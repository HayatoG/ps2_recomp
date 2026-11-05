"""Remove atualizacoes mortas do MAC do VU0 no C++ gerado pelo ps2_recomp (flags "preguicosas").

Uso: python -I tools/lazy_vu0_mac.py <dir_ou_arquivo.cpp> [--apply]
- Sem --apply so conta (dry run). Com --apply reescreve os .cpp no lugar (so os que mudam).
- Regra (liveness em linha reta, conservadora): uma chamada `ps2_vu0_update_mac(ctx, res, N);` e' morta
  quando, antes de qualquer leitura possivel de ctx->vu0_mac_flags, outra atualizacao completa acontece
  na mesma sequencia reta. Tudo o que nao for uma instrucao simples conhecida e' barreira: rotulos,
  desvios, return, chamadas ao runtime, outros helpers do VU0, leituras do MAC (cfc2, vopmula/vopmsub),
  e tambem READ*/WRITE* (o caminho lento chama o runtime).
- Valido porque a atualizacao sobrescreve o MAC inteiro (lanes fora de `dest` zeradas) e o runtime so
  le o MAC dentro de chamadas explicitas (`runtime->...`, microprograma do VU0).
- O lugar definitivo desta regra e' o gerador (ps2xRecomp); este passe e' o prototipo medido em
  ghost_pc/docs/guest-codegen-opt.md.
Logs em ./logs (execute a partir do diretorio do jogo).
"""
from __future__ import annotations

import argparse
import logging
import re
import sys
from pathlib import Path

log = logging.getLogger("lazy_vu0_mac")

CALL_RE = re.compile(r" ps2_vu0_update_mac\(ctx, res, (?:0x[0-9A-Fa-f]+u?|\d+u?)\);")
DEAD_MARK = " /*mac morto*/"
BARRIER_TOKENS = ("vu0_mac_flags", "runtime->", "label_", "goto", "return", "case ", "switch",
                  "FUN_", "ps2_vu0_", "#", "dispatch", "throw", "PS_LOG", "READ", "WRITE")


def is_transparent(line: str) -> bool:
    """A linha nao le o MAC nem desvia o fluxo?"""
    s = line.strip()
    if not s or s.startswith("//"):
        return True
    if s in ("{", "}") or s.endswith(":"):
        return False
    if any(t in s for t in BARRIER_TOKENS):
        return False
    if s.startswith(("if", "else", "while", "for", "do")):
        return False
    return s.endswith(";") or s.endswith("}")


def is_plain_update(line: str) -> bool:
    """Exatamente uma atualizacao completa e nenhuma outra mencao ao MAC na linha."""
    if len(CALL_RE.findall(line)) != 1:
        return False
    return is_transparent(CALL_RE.sub("", line))


def process(lines: list[str]) -> tuple[list[str], int, int]:
    out = list(lines)
    pending = None
    removed = total = 0
    for i, line in enumerate(lines):
        if CALL_RE.search(line):
            total += 1
        if is_plain_update(line):
            if pending is not None:
                out[pending] = CALL_RE.sub(DEAD_MARK, out[pending])
                removed += 1
            pending = i
        elif not is_transparent(line):
            pending = None
    return out, removed, total


def process_file(path: Path, apply: bool) -> tuple[int, int]:
    raw = path.read_bytes().decode("utf-8")
    if "ps2_vu0_update_mac" not in raw:
        return 0, 0
    nl = "\r\n" if "\r\n" in raw else "\n"
    out, removed, total = process(raw.split(nl))
    if apply and removed:
        path.write_bytes(nl.join(out).encode("utf-8"))
    return removed, total


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("target", type=Path)
    ap.add_argument("--apply", action="store_true")
    args = ap.parse_args()
    Path("logs").mkdir(exist_ok=True)
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s",
                        handlers=[logging.FileHandler("logs/lazy_vu0_mac.log", encoding="utf-8"),
                                  logging.StreamHandler()])
    files = [args.target] if args.target.is_file() else sorted(args.target.glob("*.cpp"))
    removed = total = touched = 0
    for f in files:
        r, t = process_file(f, args.apply)
        removed += r
        total += t
        touched += 1 if r else 0
    log.info("%s: %d/%d atualizacoes do MAC mortas em %d arquivos (%s)", args.target, removed, total,
             touched, "aplicado" if args.apply else "dry run")
    return 0


if __name__ == "__main__":
    sys.exit(main())
