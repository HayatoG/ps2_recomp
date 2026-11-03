"""Resolve o histograma do amostrador do runtime (PS2X_PROFILE_*) em nomes de funcao usando o mapa do linker.

Uso: python -I tools/resolve_profile.py <ps2_profile.txt> <ps2EntryRunner.map> [--top 30] [--base 0x140000000]
Entrada: linhas '0xRVA contagem' (RVA relativo a base do modulo). Mapa MSVC (/MAP): secao 'Publics by Value'
com colunas 'secao:offset  nome  Rva+Base  [f] [i] lib:objeto'.
Saida: ranking por funcao (tempo proprio, em % das amostras do modulo) e por objeto/biblioteca, em texto.
"""
from __future__ import annotations

import argparse
import bisect
import re
import sys
from collections import Counter
from pathlib import Path

PUBLIC_LINE = re.compile(r"^\s*[0-9A-Fa-f]{4}:[0-9A-Fa-f]{8}\s+(\S+)\s+([0-9A-Fa-f]{8,16})\s+(?:f\s+)?(?:i\s+)?(\S+)\s*$")


def load_map(path: Path, base: int) -> tuple[list[int], list[tuple[str, str]]]:
    """Retorna (rvas ordenados, [(nome, objeto)]) dos simbolos publicos."""
    entries: dict[int, tuple[str, str]] = {}
    in_publics = False
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if "Publics by Value" in line:
            in_publics = True
            continue
        if not in_publics:
            continue
        match = PUBLIC_LINE.match(line)
        if not match:
            continue
        name, rva_plus_base, owner = match.groups()
        rva = int(rva_plus_base, 16) - base
        if rva >= 0:
            entries.setdefault(rva, (name, owner))
    rvas = sorted(entries)
    return rvas, [entries[r] for r in rvas]


def read_profile(path: Path) -> tuple[dict[str, str], Counter[int]]:
    header: dict[str, str] = {}
    counts: Counter[int] = Counter()
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("#"):
            header = dict(item.split("=", 1) for item in line.lstrip("# ").split() if "=" in item)
        elif line.strip():
            address, count = line.split()
            counts[int(address, 16)] += int(count)
    return header, counts


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("profile", type=Path)
    parser.add_argument("map", type=Path)
    parser.add_argument("--top", type=int, default=30)
    parser.add_argument("--base", type=lambda text: int(text, 0), default=0x140000000,
                        help="ImageBase do mapa (padrao 0x140000000, MSVC x64)")
    args = parser.parse_args(argv)

    rvas, symbols = load_map(args.map, args.base)
    if not rvas:
        print("nenhum simbolo publico no mapa: confira se o arquivo e o .map do MSVC", file=sys.stderr)
        return 2
    header, counts = read_profile(args.profile)

    by_function: Counter[str] = Counter()
    by_owner: Counter[str] = Counter()
    unresolved = 0
    for rva, count in counts.items():
        index = bisect.bisect_right(rvas, rva) - 1
        if index < 0:
            unresolved += count
            continue
        name, owner = symbols[index]
        by_function[name] += count
        by_owner[owner.split(":")[0] if ":" in owner else owner] += count

    total = sum(counts.values())
    print(f"amostras no modulo: {total}   {' '.join(f'{k}={v}' for k, v in header.items())}   sem simbolo: {unresolved}")
    print(f"\n== Top {args.top} funcoes (tempo proprio)")
    for name, count in by_function.most_common(args.top):
        print(f"{100.0 * count / total:6.2f}%  {count:7d}  {name}")
    print(f"\n== Top {min(args.top, 15)} objetos/bibliotecas")
    for owner, count in by_owner.most_common(min(args.top, 15)):
        print(f"{100.0 * count / total:6.2f}%  {count:7d}  {owner}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
