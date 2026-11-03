"""Launcher genérico de ports PS2, fora do jogo (regras 4-6 do AGENTS.md).

Uso (a partir da raiz do repo ou de qualquer lugar):
    python -I launcher/ps2launcher.py <info|verify|extract|run> [--game ghost_pc] [--iso caminho.iso] [--force]

Cada jogo tem um ``game.toml`` no seu diretório (ver ``ghost_pc/game.toml``).
- Exige o dump original do usuário: sem ISO válida (serial + hash do ELF de boot) nada roda.
- Extrai só o necessário para ``<jogo>/work/`` (ignorado pelo git).
- Registra tudo em ``<jogo>/logs/launcher_<data>.log``.
"""
from __future__ import annotations

import argparse
import datetime
import hashlib
import logging
import os
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import extract_iso  # noqa: E402  (biblioteca de leitura de ISO da raiz)

log = logging.getLogger("launcher")

VIDEO_HZ_BY_REGION = {"PAL": "50", "NTSC": "60"}


class LauncherError(Exception):
    """Erro esperado: mensagem pensada para o usuário."""


@dataclass(frozen=True)
class GameConfig:
    root: Path
    name: str
    serial: str
    boot_elf_sha256: str
    iso_glob: str
    work_dir: Path
    extract_files: tuple[str, ...]
    run_executable: Path | None
    run_args: tuple[str, ...]
    run_env: tuple[tuple[str, str], ...] = ()
    region: str = "NTSC"

    @classmethod
    def load(cls, game_dir: Path) -> "GameConfig":
        cfg_path = game_dir / "game.toml"
        if not cfg_path.is_file():
            raise LauncherError(f"game.toml não encontrado em {game_dir}")
        with cfg_path.open("rb") as fh:
            raw = tomllib.load(fh)
        try:
            game, dump = raw["game"], raw["dump"]
            extract = raw.get("extract", {})
            run = raw.get("run", {})
            exe = run.get("executable")
            return cls(
                root=game_dir,
                name=game["name"],
                serial=game["serial"],
                boot_elf_sha256=dump["boot_elf_sha256"].lower(),
                iso_glob=dump.get("iso_glob", "*.iso"),
                work_dir=game_dir / extract.get("work_dir", "work"),
                extract_files=tuple(extract.get("files", [game["serial"], "SYSTEM.CNF"])),
                run_executable=(game_dir / exe) if exe else None,
                run_args=tuple(run.get("args", [])),
                run_env=tuple((str(k), str(v)) for k, v in run.get("env", {}).items()),
                region=str(game.get("region", "NTSC")).upper(),
            )
        except KeyError as err:
            raise LauncherError(f"game.toml sem o campo obrigatório {err}") from err


def setup_logging(log_dir: Path) -> None:
    log_dir.mkdir(parents=True, exist_ok=True)
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(levelname)s %(name)s: %(message)s",
        handlers=[
            logging.FileHandler(log_dir / f"launcher_{datetime.date.today()}.log", encoding="utf-8"),
            logging.StreamHandler(),
        ],
    )


def find_iso(cfg: GameConfig, explicit: Path | None) -> Path:
    """Localiza o dump do usuário; nunca baixa nem embute nada (regra 4)."""
    if explicit:
        if not explicit.is_file():
            raise LauncherError(f"ISO não encontrada: {explicit}")
        return explicit
    matches = sorted(cfg.root.glob(cfg.iso_glob))
    if not matches:
        raise LauncherError(
            f"Dump original não encontrado em {cfg.root} ({cfg.iso_glob}). "
            "Coloque a sua imagem do jogo ali ou use --iso.")
    if len(matches) > 1:
        raise LauncherError(f"Mais de uma ISO em {cfg.root}; use --iso. Candidatas: {[m.name for m in matches]}")
    return matches[0]


def verify_dump(cfg: GameConfig, iso_path: Path) -> None:
    """Confere serial (BOOT2) e sha256 do ELF de boot contra o game.toml."""
    try:
        with extract_iso.Iso(str(iso_path)) as iso:
            boot_name, _ = iso.boot_info()
            if boot_name != cfg.serial:
                raise LauncherError(f"Serial do disco ({boot_name}) difere do esperado ({cfg.serial}).")
            digest = hashlib.sha256(iso.read(boot_name)).hexdigest()
    except extract_iso.IsoError as err:
        raise LauncherError(f"ISO inválida: {err}") from err
    if digest != cfg.boot_elf_sha256:
        raise LauncherError(
            f"Hash do {cfg.serial} não confere (obtido {digest}). Dump diferente ou corrompido.")
    log.info("dump verificado: %s serial=%s sha256=%s", iso_path.name, cfg.serial, digest)


def extract_assets(cfg: GameConfig, iso_path: Path, force: bool) -> None:
    missing = [n for n in cfg.extract_files if not (cfg.work_dir / n).exists()]
    if not missing and not force:
        log.info("work/ já contém %s; nada a extrair (use --force)", ", ".join(cfg.extract_files))
        return
    names = cfg.extract_files if force else tuple(missing)
    log.info("extraindo %s para %s", ", ".join(names), cfg.work_dir)
    extract_iso.extract(str(iso_path), str(cfg.work_dir), names)


def build_run_env(cfg: GameConfig, opts: argparse.Namespace) -> dict[str, str]:
    """Ambiente do port: base do processo + [run.env] do game.toml + opções da linha de comando."""
    env = dict(os.environ)
    # A lógica do jogo é guiada pelo vblank: PAL = 50 Hz, NTSC = 60 Hz (o runtime assume 60 por padrão).
    env["PS2X_VIDEO_HZ"] = VIDEO_HZ_BY_REGION.get(cfg.region, "60")
    env.update(cfg.run_env)
    if opts.fps is not None:
        env["PS2X_TARGET_FPS"] = str(opts.fps)  # 0 = ilimitado
    if opts.pad_script is not None:
        env["PS2X_PAD_SCRIPT"] = str(opts.pad_script.resolve())
    if opts.fps_log is not None:
        env["PS2X_FPS_LOG"] = str(opts.fps_log.resolve())
    if opts.shot_dir is not None:
        env["PS2X_SHOT_DIR"] = str(opts.shot_dir.resolve())
    if opts.missing_log is not None:
        env["PS2X_MISSING_LOG"] = str(opts.missing_log.resolve())
    return env


def run_game(cfg: GameConfig, opts: argparse.Namespace, extra_args: list[str]) -> int:
    if cfg.run_executable is None:
        raise LauncherError("game.toml não define [run].executable (port ainda não construído).")
    if not cfg.run_executable.is_file():
        raise LauncherError(f"Executável do port não encontrado: {cfg.run_executable}")
    boot_elf = cfg.work_dir / cfg.serial
    args = [str(cfg.run_executable)] + [a.format(elf=boot_elf, work=cfg.work_dir) for a in cfg.run_args] + extra_args
    env = build_run_env(cfg, opts)
    log.info("iniciando: %s", args)
    log.info("opções de execução: %s", {k: v for k, v in env.items() if k.startswith("PS2X_")})
    run_log = cfg.root / "logs" / f"run_{datetime.datetime.now():%Y%m%d_%H%M%S}.log"
    with run_log.open("wb") as out:
        try:
            proc = subprocess.run(args, cwd=cfg.root, env=env, stdout=out, stderr=subprocess.STDOUT,
                                  check=False, timeout=opts.timeout)
        except subprocess.TimeoutExpired:
            log.warning("tempo limite de %ss atingido; processo encerrado. Saída em %s", opts.timeout, run_log)
            return 124
    log.info("encerrado com código %d; saída em %s", proc.returncode, run_log)
    return proc.returncode


def parse_args(argv: list[str]) -> tuple[argparse.Namespace, list[str]]:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("command", choices=("info", "verify", "extract", "run"))
    parser.add_argument("--game", type=Path, default=Path("."), help="diretório do jogo (com game.toml)")
    parser.add_argument("--iso", type=Path, help="caminho do dump (padrão: busca iso_glob no diretório do jogo)")
    parser.add_argument("--force", action="store_true", help="reextrair mesmo se work/ já tiver os arquivos")
    run_opts = parser.add_argument_group("opções do comando run")
    run_opts.add_argument("--fps", type=int, metavar="N", help="limite de FPS de apresentação (0 = ilimitado)")
    run_opts.add_argument("--pad-script", type=Path, metavar="ARQ", help="script do gamepad virtual (por quadro)")
    run_opts.add_argument("--fps-log", type=Path, metavar="CSV", help="grava o FPS por segundo neste CSV")
    run_opts.add_argument("--shot-dir", type=Path, metavar="DIR", help="onde salvar os PNG do comando 'shot' do pad virtual")
    run_opts.add_argument("--missing-log", type=Path, metavar="ARQ",
                          help="acrescenta aqui cada alvo de desvio sem função gerada (para CreateFunctionsAt)")
    run_opts.add_argument("--timeout", type=float, metavar="S", help="encerra o jogo após S segundos (testes automatizados)")
    return parser.parse_known_args(argv)


def main(argv: list[str]) -> int:
    args, extra = parse_args(argv)
    try:
        cfg = GameConfig.load(args.game.resolve())
        setup_logging(cfg.root / "logs")
        log.info("jogo: %s (%s)", cfg.name, cfg.serial)
        if args.command == "info":
            log.info("work=%s extrair=%s executável=%s", cfg.work_dir, cfg.extract_files, cfg.run_executable)
            return 0
        iso_path = find_iso(cfg, args.iso)
        verify_dump(cfg, iso_path)
        if args.command in ("extract", "run"):
            extract_assets(cfg, iso_path, args.force)
        if args.command == "run":
            return run_game(cfg, args, extra)
        return 0
    except LauncherError as err:
        logging.getLogger("launcher").error("%s", err)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
