#!/usr/bin/env python3
"""Embute arquivos de texto (shaders) num header C++ como raw strings constexpr.

Uso: python -I tools/embed_text.py <saida.h> <identificador>=<arquivo> [...] [--ns NAMESPACE]
Generico (sem nada de um jogo). Cada arquivo vira `inline constexpr const char <ident>[] = R"EMBED(...)EMBED";`.
Strings literais do MSVC limitam-se a ~16 KB por pedaco: o texto e dividido em pedacos concatenados.
"""
import sys
from pathlib import Path

CHUNK = 12000  # bytes por literal (limite do MSVC: 16380)


def chunks(text: str):
    out, cur = [], ""
    for line in text.splitlines(keepends=True):
        if len(cur) + len(line) > CHUNK:
            out.append(cur)
            cur = ""
        cur += line
    if cur:
        out.append(cur)
    return out


def main(argv):
    ns = "embedded_text"
    args = []
    it = iter(argv[1:])
    for a in it:
        if a == "--ns":
            ns = next(it)
        else:
            args.append(a)
    if len(args) < 2:
        print(__doc__)
        return 2
    out_path, items = Path(args[0]), args[1:]
    lines = ["#pragma once", "// GERADO por tools/embed_text.py - nao edite; edite os arquivos de origem.", "",
             f"namespace {ns}", "{"]
    for item in items:
        ident, _, src = item.partition("=")
        text = Path(src).read_text(encoding="utf-8")
        lines.append(f"    // origem: {Path(src).name}")
        lines.append(f"    inline constexpr const char {ident}[] =")
        for c in chunks(text):
            lines.append(f'        R"EMBED({c})EMBED"')
        lines[-1] += ";"
        lines.append("")
    lines.append("}")
    out_path.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    print(f"escrito {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
