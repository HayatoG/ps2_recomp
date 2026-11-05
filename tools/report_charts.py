"""Gera graficos SVG (escritos a mao, so stdlib) a partir de um JSON de dados.

Uso (a partir do diretorio do jogo):
  python -I ../tools/report_charts.py --data docs/perf/data.json --out docs/img [--base .]

Os SVG tem fundo transparente, cores do conjunto Okabe-Ito (contrastantes e seguras para daltonismo) e
texto que se adapta ao tema claro/escuro via `prefers-color-scheme` (funciona no GitHub e em navegadores).
Cada SVG leva <title> e <desc> (acessibilidade). Nenhum dado do jogo alem de numeros entra nos graficos.
Log em logs/ (relativo a --base).

Schema do JSON (so as chaves usadas aqui; outras, como "fontes" e "fatos", sao ignoradas):
{
  "charts": [ {
      "id": "fps-gameplay",            # nome do arquivo: <out>/<id>.svg
      "type": "grouped_bars" | "stacked_bars" | "hbars" | "lines",
      "title": "...", "subtitle": "...",   # subtitle opcional
      "unit": "vsync/s",                   # sufixo nos rotulos
      "alt": "descricao para leitores de tela (opcional; senao usa o titulo)",
      ... campos do tipo abaixo ... } ]
}
Valor ausente: use null (ou omita) e o grafico desenha um marcador tracejado "[PENDENTE]".

grouped_bars: "categories": ["Menu", ...], "series": [{"name": "Antes", "values": [40.5, null]}],
              opcional "reference": {"value": 50, "label": "alvo 50 Hz"}
stacked_bars: "categories": ["Antes", "Depois"], "series": [{"name": "Rasterizador", "values": [52, null]}]
              (cada barra e uma pilha; o total e desenhado acima; categoria com todos null = pendente)
hbars:        "items": [{"label": "Tocar 1 .cpp", "value": 14.6, "group": "Antes"}], "scale": "linear"|"log",
              "groups": ["Antes", "Depois"] (ordem e cor por grupo; opcional)
lines:        "series": [{"name": "baseline", "csv": "logs/final/x.csv", "x": "t_s", "y": "guest_vsync_hz"}]
              ou "points": [[x, y], ...]; "x_mode": "t" (padrao, coluna x) | "vsync" (x = vsync acumulado
              = soma de guest_vsync_hz * dt); "xrange": [lo, hi]; "x_label", "y_label"; "band": [lo, hi]
              (faixa sombreada em unidades de x, ex. janela de vsync 7530-7640); "reference" como acima.
              Serie sem arquivo/pontos e desenhada so na legenda como "[PENDENTE]".
Caminhos "csv" sao relativos a --base.
"""
from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from datetime import datetime
from pathlib import Path
from xml.sax.saxutils import escape

PALETTE = ["#0072B2", "#E69F00", "#009E73", "#D55E00", "#CC79A7", "#56B4E9", "#7A7A7A"]
WIDTH = 820
FONT = "system-ui, -apple-system, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif"
CSS = f"""
.t{{font:600 18px {FONT};fill:#57606a}} .s{{font:13px {FONT};fill:#6e7781}}
.l{{font:13px {FONT};fill:#57606a}} .v{{font:600 13px {FONT};fill:#57606a}}
.p{{font:italic 12px {FONT};fill:#8c959f}}
.ax{{stroke:#8c959f;stroke-width:1}} .gr{{stroke:#8c959f;stroke-opacity:.25;stroke-width:1}}
.pend{{fill:none;stroke:#8c959f;stroke-width:1.5;stroke-dasharray:5 4}}
@media (prefers-color-scheme: dark){{.t,.l,.v{{fill:#c9d1d9}} .s,.p{{fill:#8b949e}}}}
"""


def log(base: Path, level: str, message: str) -> None:
    folder = base / "logs"
    folder.mkdir(exist_ok=True)
    line = f"{datetime.now().isoformat(timespec='seconds')} {level} report_charts: {message}"
    with (folder / "report_charts.log").open("a", encoding="utf-8") as handle:
        handle.write(line + "\n")
    print(line, file=sys.stderr if level != "INFO" else sys.stdout)


def num(value: float, decimals: int | None = None) -> str:
    """Formata numero em pt-BR (virgula decimal)."""
    if decimals is None:
        decimals = 0 if abs(value) >= 100 else (1 if abs(value) >= 10 else 2)
    text = f"{value:,.{decimals}f}"
    return text.replace(",", "X").replace(".", ",").replace("X", ".")


def tick_text(value: float) -> str:
    return num(value, 0) if abs(value - round(value)) < 1e-9 else num(value, 1)


def nice_ticks(low: float, high: float, count: int = 5) -> list[float]:
    span = max(high - low, 1e-9)
    raw = span / count
    magnitude = 10 ** math.floor(math.log10(raw))
    step = next(m * magnitude for m in (1, 2, 2.5, 5, 10) if m * magnitude >= raw)
    start = math.floor(low / step) * step
    ticks, value = [], start
    while value <= high + step * 0.999:
        ticks.append(round(value, 10))
        value += step
        if ticks[-1] >= high:
            break
    return ticks


def text(x: float, y: float, content: str, css: str, anchor: str = "start", extra: str = "") -> str:
    return f'<text x="{x:.1f}" y="{y:.1f}" class="{css}" text-anchor="{anchor}" {extra}>{escape(content)}</text>'


class Svg:
    def __init__(self, chart: dict, height: int) -> None:
        self.chart, self.height, self.parts = chart, height, []

    def add(self, part: str) -> None:
        self.parts.append(part)

    def header(self) -> float:
        """Titulo e subtitulo; devolve o y onde o conteudo pode comecar."""
        self.add(text(20, 28, self.chart["title"], "t"))
        y = 28
        if self.chart.get("subtitle"):
            y += 20
            self.add(text(20, y, self.chart["subtitle"], "s"))
        return y + 14

    def legend(self, y: float, entries: list[tuple[str, str]]) -> float:
        """Legenda em linhas que quebram; devolve o y abaixo dela."""
        x, row_h = 20.0, 22
        for name, color in entries:
            width = 26 + 7.2 * len(name)
            if x + width > WIDTH - 10:
                x, y = 20.0, y + row_h
            self.add(f'<rect x="{x:.1f}" y="{y - 11:.1f}" width="14" height="14" rx="3" fill="{color}"/>')
            self.add(text(x + 20, y, name, "l"))
            x += width
        return y + row_h

    def render(self) -> str:
        title = escape(self.chart["title"])
        desc = escape(self.chart.get("alt") or self.chart.get("subtitle") or self.chart["title"])
        body = "\n".join(self.parts)
        return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{self.height}" '
                f'viewBox="0 0 {WIDTH} {self.height}" role="img" aria-labelledby="t d">\n'
                f'<title id="t">{title}</title><desc id="d">{desc}</desc>\n<style>{CSS}</style>\n{body}\n</svg>\n')


def unit_of(chart: dict) -> str:
    return (" " + chart["unit"]) if chart.get("unit") else ""


def y_axis(svg: Svg, left: float, right: float, top: float, bottom: float, high: float, chart: dict) -> float:
    """Eixo Y linear de 0 a `high` (arredondado); devolve o maximo real usado."""
    ticks = nice_ticks(0, max(high, 1.0))
    top_val = ticks[-1]
    for tick in ticks:
        y = bottom - (bottom - top) * tick / top_val
        svg.add(f'<line x1="{left}" x2="{right}" y1="{y:.1f}" y2="{y:.1f}" class="gr"/>')
        svg.add(text(left - 8, y + 4, tick_text(tick), "l", "end"))
    svg.add(f'<line x1="{left}" x2="{left}" y1="{top}" y2="{bottom}" class="ax"/>')
    svg.add(f'<line x1="{left}" x2="{right}" y1="{bottom}" y2="{bottom}" class="ax"/>')
    if chart.get("unit"):
        svg.add(text(left - 8, top - 10, chart["unit"], "s", "end"))
    return top_val


def reference_line(svg: Svg, chart: dict, left: float, right: float, top: float, bottom: float, top_val: float) -> None:
    ref = chart.get("reference")
    if ref and ref["value"] <= top_val:
        y = bottom - (bottom - top) * ref["value"] / top_val
        svg.add(f'<line x1="{left}" x2="{right}" y1="{y:.1f}" y2="{y:.1f}" stroke="#D55E00" '
                f'stroke-width="1.5" stroke-dasharray="6 4"/>')
        svg.add(text(right, y - 5, ref["label"], "s", "end"))


def pending_mark(svg: Svg, x: float, w: float, bottom: float, label: str = "[PENDENTE]") -> None:
    svg.add(f'<rect x="{x:.1f}" y="{bottom - 34:.1f}" width="{w:.1f}" height="34" rx="3" class="pend"/>')
    svg.add(text(x + w / 2, bottom - 14, label, "p", "middle"))


def grouped_bars(chart: dict) -> str:
    cats, series = chart["categories"], chart["series"]
    svg = Svg(chart, 0)
    top0 = svg.header()
    legend_end = svg.legend(top0 + 8, [(s["name"], PALETTE[i % 7]) for i, s in enumerate(series)])
    top, bottom, left, right = legend_end + 26, legend_end + 26 + 270, 70.0, WIDTH - 20.0
    svg.height = int(bottom + 50)
    values = [v for s in series for v in s["values"] if v is not None]
    ref = chart.get("reference", {}).get("value", 0)
    top_val = y_axis(svg, left, right, top, bottom, max(values + [ref]) * 1.12, chart)
    group_w = (right - left) / len(cats)
    bar_w = min(70.0, group_w * 0.8 / len(series))
    for ci, cat in enumerate(cats):
        gx = left + group_w * ci + (group_w - bar_w * len(series)) / 2
        for si, serie in enumerate(series):
            x, value = gx + bar_w * si, serie["values"][ci] if ci < len(serie["values"]) else None
            if value is None:
                pending_mark(svg, x + 3, bar_w - 6, bottom)
                continue
            h = (bottom - top) * value / top_val
            svg.add(f'<rect x="{x + 3:.1f}" y="{bottom - h:.1f}" width="{bar_w - 6:.1f}" height="{h:.1f}" '
                    f'rx="3" fill="{PALETTE[si % 7]}"><title>{escape(serie["name"])}: {num(value)}{unit_of(chart)}</title></rect>')
            svg.add(text(x + bar_w / 2, bottom - h - 6, num(value), "v", "middle"))
        svg.add(text(left + group_w * (ci + .5), bottom + 22, cat, "l", "middle"))
    reference_line(svg, chart, left, right, top, bottom, top_val)
    return svg.render()


def stacked_bars(chart: dict) -> str:
    cats, series = chart["categories"], chart["series"]
    svg = Svg(chart, 0)
    top0 = svg.header()
    legend_end = svg.legend(top0 + 8, [(s["name"], PALETTE[i % 7]) for i, s in enumerate(series)])
    top, bottom, left, right = legend_end + 26, legend_end + 26 + 300, 70.0, WIDTH - 20.0
    svg.height = int(bottom + 50)
    totals = [sum((s["values"][ci] or 0) for s in series) for ci in range(len(cats))]
    top_val = y_axis(svg, left, right, top, bottom, max(totals + [1]) * 1.1, chart)
    group_w = (right - left) / len(cats)
    bar_w = min(150.0, group_w * 0.6)
    for ci, cat in enumerate(cats):
        x = left + group_w * ci + (group_w - bar_w) / 2
        svg.add(text(x + bar_w / 2, bottom + 22, cat, "l", "middle"))
        if all(s["values"][ci] is None for s in series):
            pending_mark(svg, x, bar_w, bottom)
            continue
        y = bottom
        for si, serie in enumerate(series):
            value = serie["values"][ci]
            if not value:
                continue
            h = (bottom - top) * value / top_val
            y -= h
            svg.add(f'<rect x="{x:.1f}" y="{y:.1f}" width="{bar_w:.1f}" height="{h:.1f}" fill="{PALETTE[si % 7]}" '
                    f'stroke="#ffffff" stroke-opacity=".6"><title>{escape(serie["name"])}: {num(value)}{unit_of(chart)}</title></rect>')
            if h >= 17:
                svg.add(text(x + bar_w / 2, y + h / 2 + 4.5, num(value), "v", "middle", 'fill="#ffffff" style="fill:#ffffff;paint-order:stroke;stroke:#000;stroke-opacity:.45;stroke-width:2.5px"'))
        svg.add(text(x + bar_w / 2, y - 7, "total " + num(totals[ci]) + unit_of(chart), "v", "middle"))
    return svg.render()


def hbars(chart: dict) -> str:
    items = chart["items"]
    groups = chart.get("groups") or list(dict.fromkeys(i.get("group", "") for i in items))
    color_of = {g: PALETTE[i % 7] for i, g in enumerate(groups)}
    svg = Svg(chart, 0)
    top0 = svg.header()
    top = svg.legend(top0 + 8, [(g, color_of[g]) for g in groups]) + 6 if len(groups) > 1 and groups[0] else top0 + 6
    left, right, row = 300.0, WIDTH - 90.0, 36
    svg.height = int(top + row * len(items) + 24)
    values = [i["value"] for i in items if i.get("value") is not None]
    log_scale = chart.get("scale") == "log"
    high = max(values) * 1.05
    low_log = min(values) / 3 if log_scale else 0

    def pos(value: float) -> float:
        if log_scale:
            return left + (right - left) * (math.log10(value) - math.log10(low_log)) / (math.log10(high) - math.log10(low_log))
        return left + (right - left) * value / high

    if log_scale:
        decade = 10 ** math.ceil(math.log10(low_log))
        while decade <= high:
            svg.add(f'<line x1="{pos(decade):.1f}" x2="{pos(decade):.1f}" y1="{top}" y2="{top + row * len(items)}" class="gr"/>')
            svg.add(text(pos(decade), top + row * len(items) + 16, tick_text(decade) + unit_of(chart), "s", "middle"))
            decade *= 10
    else:
        for tick in nice_ticks(0, high):
            if tick <= high:
                svg.add(f'<line x1="{pos(tick):.1f}" x2="{pos(tick):.1f}" y1="{top}" y2="{top + row * len(items)}" class="gr"/>')
                svg.add(text(pos(tick), top + row * len(items) + 16, tick_text(tick), "s", "middle"))
    for index, item in enumerate(items):
        y = top + row * index
        svg.add(text(left - 10, y + row / 2 + 4, item["label"], "l", "end"))
        if item.get("value") is None:
            svg.add(f'<rect x="{left}" y="{y + 7}" width="{(right - left) * .3:.1f}" height="{row - 14}" rx="3" class="pend"/>')
            svg.add(text(left + 10, y + row / 2 + 4, "[PENDENTE]", "p"))
            continue
        end = pos(item["value"])
        svg.add(f'<rect x="{left}" y="{y + 7}" width="{end - left:.1f}" height="{row - 14}" rx="3" '
                f'fill="{color_of.get(item.get("group", ""), PALETTE[0])}"><title>{escape(item["label"])}: {num(item["value"])}{unit_of(chart)}</title></rect>')
        svg.add(text(end + 6, y + row / 2 + 4, num(item["value"]) + unit_of(chart), "v"))
    svg.add(f'<line x1="{left}" x2="{left}" y1="{top}" y2="{top + row * len(items)}" class="ax"/>')
    return svg.render()


def read_series(serie: dict, base: Path, x_mode: str) -> list[tuple[float, float]]:
    if "points" in serie:
        return [(float(x), float(y)) for x, y in serie["points"]]
    if "csv" not in serie:
        return []
    path = base / serie["csv"]
    if not path.is_file():
        log(base, "WARN", f"csv ausente: {path}")
        return []
    points, cumulative, previous = [], 0.0, 0.0
    with path.open(encoding="utf-8", newline="") as handle:
        for row in csv.DictReader(handle):
            try:
                x, y = float(row[serie.get("x", "t_s")]), float(row[serie["y"]])
            except (KeyError, ValueError):
                continue
            if x_mode == "vsync":
                cumulative += float(row["guest_vsync_hz"]) * (x - previous)
                previous = x
                points.append((cumulative, y))
            else:
                points.append((x, y))
    return points


def lines(chart: dict, base: Path) -> str:
    series = chart["series"]
    x_mode = chart.get("x_mode", "t")
    data = [read_series(s, base, x_mode) for s in series]
    svg = Svg(chart, 0)
    top0 = svg.header()
    entries = [(s["name"] + ("" if data[i] else " [PENDENTE]"), PALETTE[i % 7]) for i, s in enumerate(series)]
    legend_end = svg.legend(top0 + 8, entries)
    top, bottom, left, right = legend_end + 26, legend_end + 26 + 290, 70.0, WIDTH - 24.0
    svg.height = int(bottom + 56)
    xs = [p[0] for d in data for p in d] or [0, 1]
    x_lo, x_hi = chart.get("xrange", [min(xs), max(xs)])
    visible = [p[1] for d in data for p in d if x_lo <= p[0] <= x_hi] or [1]
    ref = chart.get("reference", {}).get("value", 0)
    top_val = y_axis(svg, left, right, top, bottom, max(visible + [ref]) * 1.1, chart)

    def px(x: float) -> float:
        return left + (right - left) * (x - x_lo) / max(x_hi - x_lo, 1e-9)

    for tick in nice_ticks(x_lo, x_hi, 6):
        if x_lo <= tick <= x_hi:
            svg.add(f'<line x1="{px(tick):.1f}" x2="{px(tick):.1f}" y1="{bottom}" y2="{bottom + 5}" class="ax"/>')
            svg.add(text(px(tick), bottom + 20, num(tick, 0), "s", "middle"))
    if chart.get("x_label"):
        svg.add(text((left + right) / 2, bottom + 42, chart["x_label"], "s", "middle"))
    if chart.get("band"):
        lo, hi = chart["band"]
        svg.add(f'<rect x="{px(lo):.1f}" y="{top}" width="{px(hi) - px(lo):.1f}" height="{bottom - top}" fill="#8c959f" fill-opacity=".18"/>')
        svg.add(text(px(lo) + 4, top + 14, chart.get("band_label", "janela de medicao"), "s"))
    for i, points in enumerate(data):
        inside = [(px(x), bottom - (bottom - top) * min(y, top_val) / top_val) for x, y in points if x_lo <= x <= x_hi]
        if len(inside) > 1:
            path = " ".join(f"{'M' if k == 0 else 'L'}{x:.1f},{y:.1f}" for k, (x, y) in enumerate(inside))
            svg.add(f'<path d="{path}" fill="none" stroke="{PALETTE[i % 7]}" stroke-width="2" stroke-linejoin="round"/>')
    reference_line(svg, chart, left, right, top, bottom, top_val)
    if not any(data):
        svg.add(f'<rect x="{left + 40}" y="{(top + bottom) / 2 - 22}" width="{right - left - 80}" height="44" rx="6" class="pend"/>')
        svg.add(text((left + right) / 2, (top + bottom) / 2 + 5, "[PENDENTE] aguardando a medicao final", "p", "middle"))
    return svg.render()


def build(chart: dict, base: Path) -> str:
    kind = chart["type"]
    if kind == "grouped_bars":
        return grouped_bars(chart)
    if kind == "stacked_bars":
        return stacked_bars(chart)
    if kind == "hbars":
        return hbars(chart)
    if kind == "lines":
        return lines(chart, base)
    raise ValueError(f"tipo desconhecido: {kind}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--data", required=True, type=Path, help="JSON de dados")
    parser.add_argument("--out", required=True, type=Path, help="pasta de saida dos SVG")
    parser.add_argument("--base", type=Path, default=Path("."), help="base dos caminhos csv e de logs/")
    args = parser.parse_args()
    data = json.loads(args.data.read_text(encoding="utf-8"))
    args.out.mkdir(parents=True, exist_ok=True)
    failed = 0
    for chart in data["charts"]:
        try:
            target = args.out / f"{chart['id']}.svg"
            target.write_text(build(chart, args.base), encoding="utf-8", newline="\n")
            log(args.base, "INFO", f"escrito {target}")
        except Exception as error:  # um grafico ruim nao derruba os outros
            failed += 1
            log(args.base, "ERROR", f"{chart.get('id')}: {error!r}")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
