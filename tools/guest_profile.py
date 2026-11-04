"""Le o histograma de chamadas do guest (PS2X_GUEST_PROFILE) e resolve enderecos em nomes de funcao.

Uso:
  python -I tools/guest_profile.py <perfil.txt> --names <dir_do_cpp_gerado> [--top 25]         # resumo por janela
  python -I tools/guest_profile.py <perfil.txt> --names <dir> --diff <vsyncA> <vsyncB>         # o que mudou entre duas janelas
  python -I tools/guest_profile.py <perfil.txt> --names <dir> --log logs/guest_profile.log     # tambem grava em log

Os nomes vem dos arquivos gerados pelo ps2_recomp (<nome>_0x<endereco>.cpp). Enderecos sem nome saem em hexadecimal.
"""
from __future__ import annotations

import argparse
import re
from datetime import datetime, timezone
from pathlib import Path

WINDOW = re.compile(r"window t=(?P<t>[\d.]+) vsync=(?P<vsync>\d+) total=(?P<total>\d+) distinct=(?P<distinct>\d+)")
GENERATED = re.compile(r"^(?P<name>.+)_0x(?P<addr>[0-9a-f]+)\.cpp$")


def load_names(directory: Path) -> dict[int, str]:
    names: dict[int, str] = {}
    for path in directory.iterdir():
        match = GENERATED.match(path.name)
        if match:
            names[int(match["addr"], 16)] = match["name"]
    return names


def load_windows(path: Path) -> list[dict]:
    windows: list[dict] = []
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = WINDOW.match(line)
        if match:
            windows.append({"t": float(match["t"]), "vsync": int(match["vsync"]), "total": int(match["total"]),
                            "distinct": int(match["distinct"]), "counts": {}})
        elif line.startswith("0x") and windows:
            address, count = line.split()
            windows[-1]["counts"][int(address, 16)] = int(count)
    return windows


def closest(windows: list[dict], vsync: int) -> dict:
    return min(windows, key=lambda window: abs(window["vsync"] - vsync))


def label(address: int, names: dict[int, str]) -> str:
    return f"0x{address:06x} {names.get(address, '?')}"


def summary(windows: list[dict], names: dict[int, str], top: int) -> list[str]:
    lines: list[str] = []
    for window in windows:
        lines.append(f"== t={window['t']:.0f}s vsync={window['vsync']} total={window['total']} distinct={window['distinct']}")
        ranked = sorted(window["counts"].items(), key=lambda item: -item[1])[:top]
        lines.extend(f"  {count:>10} {label(address, names)}" for address, count in ranked)
    return lines


def diff(a: dict, b: dict, names: dict[int, str], top: int) -> list[str]:
    lines = [f"A: vsync={a['vsync']} total={a['total']} distinct={a['distinct']}",
             f"B: vsync={b['vsync']} total={b['total']} distinct={b['distinct']}"]
    only_a = sorted(set(a["counts"]) - set(b["counts"]), key=lambda address: -a["counts"][address])[:top]
    only_b = sorted(set(b["counts"]) - set(a["counts"]), key=lambda address: -b["counts"][address])[:top]
    lines.append("-- so em A (parou de ser chamado):")
    lines.extend(f"  {a['counts'][address]:>10} {label(address, names)}" for address in only_a)
    lines.append("-- so em B (passou a ser chamado):")
    lines.extend(f"  {b['counts'][address]:>10} {label(address, names)}" for address in only_b)
    return lines


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("profile", type=Path)
    parser.add_argument("--names", type=Path, required=True)
    parser.add_argument("--top", type=int, default=25)
    parser.add_argument("--diff", nargs=2, type=int, metavar=("VSYNC_A", "VSYNC_B"))
    parser.add_argument("--log", type=Path)
    args = parser.parse_args()

    names = load_names(args.names)
    windows = load_windows(args.profile)
    if not windows:
        print("nenhuma janela no perfil")
        return 1
    if args.diff:
        lines = diff(closest(windows, args.diff[0]), closest(windows, args.diff[1]), names, args.top)
    else:
        lines = summary(windows, names, args.top)
    print("\n".join(lines))
    if args.log:
        stamp = datetime.now(timezone.utc).isoformat(timespec="seconds")
        args.log.parent.mkdir(parents=True, exist_ok=True)
        with args.log.open("a", encoding="utf-8") as log:
            log.write(f"{stamp} INFO guest_profile {args.profile}\n" + "\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
