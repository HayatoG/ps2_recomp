"""Sincroniza o C++ gerado pelo ps2_recomp com uma arvore de build do PS2Recomp.

Uso: python -I tools/sync_recomp_output.py <output_do_recompilador> <ps2xRuntime>
- Remove os .cpp antigos de <ps2xRuntime>/src/runner (funcoes renomeadas deixam arquivos orfaos que
  geram definicoes duplicadas) e copia os novos .cpp para la.
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


def copy_files(sources: list[Path], dest_dir: Path, workers: int) -> int:
    dest_dir.mkdir(parents=True, exist_ok=True)
    with ThreadPoolExecutor(max_workers=workers) as pool:
        list(pool.map(lambda src: shutil.copyfile(src, dest_dir / src.name), sources))
    return len(sources)


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

    removed = 0
    for old in runner_dir.glob("*.cpp"):
        old.unlink()
        removed += 1
    copied_cpp = copy_files(new_cpp, runner_dir, args.workers)
    copied_h = copy_files(new_headers, include_dir, args.workers)
    log.info("runner: removidos %d, copiados %d .cpp; include: %d cabecalhos", removed, copied_cpp, copied_h)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
