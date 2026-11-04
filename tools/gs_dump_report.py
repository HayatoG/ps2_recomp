#!/usr/bin/env python3
"""Resume um dump do GS (PS2X_GS_DUMP_OUT) e, se houver, o rastreio de pixels (PS2X_GS_TRACE_PX_OUT).

Uso: python -I tools/gs_dump_report.py <dump.txt> [--px <px.txt>] [--log <saida.log>]

Mostra GIFtags que excedem o pacote (dado continuado que seria lido como tag falsa),
o contexto de cada tag desse tipo e, para cada mudança de pixel, o draw que a causou.
"""
import argparse
import collections
import logging
import re

FIELD = re.compile(r"(\w+)=(\S+)")


def parse_fields(line):
    return dict(FIELD.findall(line.split("|")[0]))


def read_dump(path):
    """Devolve eventos em ordem; draws numerados a partir de 1, como o rastreio de pixels."""
    events = []
    draw = 0
    with open(path, errors="replace") as handle:
        for line in handle:
            if line.startswith("D "):
                draw += 1
                events.append(("D", draw, parse_fields(line), line))
            elif line.startswith("G "):
                events.append(("G", draw, parse_fields(line), line))
    return events


def report_tags(events, log):
    tags = [e for e in events if e[0] == "G"]
    by_format = collections.Counter(e[2]["flg"] for e in tags)
    short = [e for e in tags if e[2]["short"] != "0"]
    log.info("tags=%d por_flg=%s tags_incompletas=%d", len(tags), dict(by_format), len(short))
    for kind, draw, fields, _ in short[:20]:
        log.info("  incompleta depois do draw %d: %s", draw, fields)
    # Formatos raros ou NREG/regs variados indicam dado bruto interpretado como tag.
    shapes = collections.Counter((e[2]["flg"], e[2]["nreg"], e[2]["regs"]) for e in tags)
    log.info("formas de tag mais comuns:")
    for shape, count in shapes.most_common(15):
        log.info("  %6d flg=%s nreg=%s regs=%s", count, *shape)
    log.info("formas de tag distintas: %d", len(shapes))


def report_pixels(path, events, log):
    draws = {e[1]: e for e in events if e[0] == "D"}
    with open(path, errors="replace") as handle:
        lines = [line.strip() for line in handle if line.startswith("P ")]
    log.info("mudancas de pixel: %d", len(lines))
    for line in lines[:120]:
        fields = parse_fields(line)
        draw = draws.get(int(fields["after_draw"]))
        detail = ""
        if draw:
            d = draw[2]
            detail = "prim=%s tme=%s fst=%s fbp=%s test=%s alpha=%s tbp=%s" % (
                d["prim"], d["tme"], d["fst"], d["fbp"], d["test"], d["alpha"], d["tbp"])
        log.info("%s | %s", line, detail)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump")
    parser.add_argument("--px")
    parser.add_argument("--log")
    args = parser.parse_args()
    handlers = [logging.StreamHandler()]
    if args.log:
        handlers.append(logging.FileHandler(args.log, encoding="utf-8"))
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s", handlers=handlers)
    log = logging.getLogger("gs_dump_report")
    events = read_dump(args.dump)
    report_tags(events, log)
    if args.px:
        report_pixels(args.px, events, log)


if __name__ == "__main__":
    main()
