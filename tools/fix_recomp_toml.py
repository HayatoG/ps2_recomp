"""Pos-processa o TOML gerado por ExportPS2Functions.java (que o reescreve a cada exportacao).

Uso: python -I tools/fix_recomp_toml.py <ghost.toml> --input <elf> [--set chave=valor ...]
- --input   caminho do ELF em [general].input (o Ghidra grava o caminho de importacao, que pode estar velho)
- --set     define/atualiza uma chave escalar em [general] (ex.: low_memory_mode=true output_worker_threads=8)
Idempotente. Logs em ./logs (execute a partir do diretorio do jogo).
"""
from __future__ import annotations

import argparse
import logging
import re
import sys
from pathlib import Path

log = logging.getLogger("fix_recomp_toml")
GENERAL_ANCHOR = "single_file_output"


def set_scalar(text: str, key: str, value: str) -> str:
    """Atualiza `key = ...` dentro de [general] ou insere logo apos single_file_output."""
    line = f"{key} = {value}"
    pattern = re.compile(rf"^{re.escape(key)}\s*=.*$", re.MULTILINE)
    if pattern.search(text):
        return pattern.sub(line, text, count=1)
    anchor = re.search(rf"^{GENERAL_ANCHOR}\s*=.*$", text, re.MULTILINE)
    if not anchor:
        raise ValueError(f"nao achei '{GENERAL_ANCHOR}' em [general] para inserir {key}")
    return text[:anchor.end()] + "\n" + line + text[anchor.end():]


def toml_string(value: str) -> str:
    return '"' + value.replace("\\", "/").replace('"', '\\"') + '"'


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("toml", type=Path)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--set", action="append", default=[], metavar="CHAVE=VALOR")
    args = parser.parse_args(argv)

    Path("logs").mkdir(exist_ok=True)
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s",
                        handlers=[logging.FileHandler("logs/fix_recomp_toml.log", encoding="utf-8"),
                                  logging.StreamHandler()])

    text = args.toml.read_text(encoding="utf-8")
    text = set_scalar(text, "input", toml_string(str(args.input.resolve())))
    for item in args.set:
        key, sep, value = item.partition("=")
        if not sep or not key:
            log.error("--set invalido: %r (use chave=valor)", item)
            return 2
        text = set_scalar(text, key.strip(), value.strip())
    args.toml.write_text(text, encoding="utf-8")
    log.info("atualizado %s (input=%s, %d opcoes)", args.toml, args.input, len(args.set))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
