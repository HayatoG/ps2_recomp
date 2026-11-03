"""Sincroniza o C++ gerado pelo ps2_recomp com uma arvore de build do PS2Recomp.

Uso: python -I tools/sync_recomp_output.py <output_do_recompilador> <ps2xRuntime>
- Copia SO os .cpp que mudaram para <ps2xRuntime>/src/runner (arquivos identicos mantem o timestamp, entao o
  ninja faz build incremental de verdade) e remove os orfaos (funcoes renomeadas deixam arquivos antigos que
  geram definicoes duplicadas).
- Copia os .h gerados (ps2_recompiled_*.h) para <ps2xRuntime>/include.
- So toca arquivos gerados; nao mexe em main.cpp (que fica em src/) nem em outros fontes.
Logs em ./logs (execute a partir do diretorio do jogo).
"""
from __future__ import annotations

import argparse
import logging
import shutil
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

log = logging.getLogger("sync_recomp_output")
HEADER_PREFIX = "ps2_recompiled_"


def same_content(src: Path, dst: Path) -> bool:
    """Mesmo conteudo? (tamanho primeiro; so entao compara os bytes)"""
    try:
        if src.stat().st_size != dst.stat().st_size:
            return False
        return src.read_bytes() == dst.read_bytes()
    except OSError:
        return False


def sync_files(sources: list[Path], dest_dir: Path, workers: int) -> tuple[int, int]:
    """Copia so o que mudou: arquivos identicos mantem o timestamp, e o ninja nao recompila o que nao mudou.

    Retorna (copiados, inalterados)."""
    dest_dir.mkdir(parents=True, exist_ok=True)

    def one(src: Path) -> bool:
        dst = dest_dir / src.name
        if same_content(src, dst):
            return False
        shutil.copyfile(src, dst)
        return True

    with ThreadPoolExecutor(max_workers=workers) as pool:
        changed = sum(pool.map(one, sources))
    return changed, len(sources) - changed


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("output", type=Path, help="diretorio de saida do ps2_recomp")
    parser.add_argument("runtime", type=Path, help="diretorio ps2xRuntime da arvore de build")
    parser.add_argument("--workers", type=int, default=8)
    args = parser.parse_args(argv)

    Path("logs").mkdir(exist_ok=True)
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s",
                        handlers=[logging.FileHandler("logs/sync_recomp_output.log", encoding="utf-8"),
                                  logging.StreamHandler()])

    runner_dir = args.runtime / "src" / "runner"
    include_dir = args.runtime / "include"
    if not args.output.is_dir() or not (args.runtime / "CMakeLists.txt").is_file():
        log.error("caminhos invalidos: output=%s runtime=%s", args.output, args.runtime)
        return 2

    new_cpp = sorted(args.output.glob("*.cpp"))
    new_headers = sorted(p for p in args.output.glob("*.h") if p.name.startswith(HEADER_PREFIX))
    if not new_cpp or not new_headers:
        log.error("saida do recompilador sem .cpp ou sem %s*.h; abortando sem alterar nada", HEADER_PREFIX)
        return 2

    # Orfaos: funcoes renomeadas deixam arquivos antigos que gerariam definicoes duplicadas.
    keep = {p.name for p in new_cpp}
    removed = 0
    for old in runner_dir.glob("*.cpp"):
        if old.name not in keep:
            old.unlink()
            removed += 1
    copied_cpp, same_cpp = sync_files(new_cpp, runner_dir, args.workers)
    copied_h, same_h = sync_files(new_headers, include_dir, args.workers)
    log.info("runner: %d .cpp copiados, %d inalterados, %d orfaos removidos; include: %d cabecalhos copiados, %d inalterados",
             copied_cpp, same_cpp, removed, copied_h, same_h)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
