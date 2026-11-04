"""Gera um relatorio HTML autocontido (graficos SVG interativos) de uma execucao do runtime.

Fontes (todas opcionais, exceto o log):
  --run-log    logs/run_*.log    linhas [hb], [perf] e [gs-feat]
  --profile    PS2X_GUEST_PROFILE_OUT   histograma de chamadas do guest por janela de tempo
  --names      dir do C++ gerado (<nome>_0x<addr>.cpp), para dar nome aos enderecos

Uso (a partir do diretorio do jogo):
  python -I ../tools/profile_report.py --run-log logs/run_x.log --profile logs/p.txt \
      --names work/recomp/output --out logs/reports/run_x.html --title "Abertura"

Nenhum byte do jogo entra no relatorio: so contadores, enderecos e nomes nossos.
"""
from __future__ import annotations

import argparse
import json
import re
from datetime import datetime, timezone
from pathlib import Path

HB = re.compile(r"^\[hb\] frame=(\d+) .*?vsync=(\d+) dma=(\d+) gif=(\d+)")
PERF = re.compile(r"^\[perf\] (.*)")
GS_WINDOW = re.compile(r"^\[gs-feat\] draws=(\d+) pixels~=(\d+) presents=(\d+)")
GS_FEATURE = re.compile(r"^\[gs-feat\] (prim\.triangle|prim\.sprite|tex\.on|tex\.render_to_texture)\s+(\d+)\s")
WINDOW = re.compile(r"window t=([\d.]+) vsync=(\d+) total=(\d+) distinct=(\d+)")
GENERATED = re.compile(r"^(?P<name>.+)_0x(?P<addr>[0-9a-f]+)\.cpp$")

STACK_SERIES = 7  # categorias nomeadas no empilhado; o resto vira "Outras" (paleta tem 8 cores)


def load_names(directory: Path | None) -> dict[int, str]:
    if not directory or not directory.is_dir():
        return {}
    names: dict[int, str] = {}
    for path in directory.iterdir():
        match = GENERATED.match(path.name)
        if match:
            names[int(match["addr"], 16)] = match["name"]
    return names


def parse_run_log(path: Path) -> dict:
    heartbeats, gs_windows, perf = [], [], {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if match := HB.match(line):
            heartbeats.append([int(group) for group in match.groups()])
        elif match := GS_WINDOW.match(line):
            draws, pixels, presents = (int(group) for group in match.groups())
            gs_windows.append({"draws": draws, "pixels": pixels, "presents": presents, "triangles": 0, "rtt": 0})
        elif (match := GS_FEATURE.match(line)) and gs_windows:
            if match[1] == "prim.triangle":
                gs_windows[-1]["triangles"] = int(match[2])
            elif match[1] == "tex.render_to_texture":
                gs_windows[-1]["rtt"] = int(match[2])
        elif match := PERF.match(line):
            perf = dict(item.split("=", 1) for item in match[1].split() if "=" in item)
    return {"heartbeats": heartbeats, "gs": gs_windows, "perf": perf}


def parse_profile(path: Path | None) -> list[dict]:
    windows: list[dict] = []
    if not path or not path.is_file():
        return windows
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if match := WINDOW.match(line):
            windows.append({"t": float(match[1]), "vsync": int(match[2]), "total": int(match[3]),
                            "distinct": int(match[4]), "counts": {}})
        elif line.startswith("0x") and windows:
            address, count = line.split()
            windows[-1]["counts"][int(address, 16)] = int(count)
    return windows


def label(address: int, names: dict[int, str]) -> str:
    return f"{names.get(address, 'sem nome')} 0x{address:06x}"


def speed_series(windows: list[dict]) -> list[dict]:
    series, previous_t, previous_v = [], 0.0, 0
    for window in windows:
        elapsed = window["t"] - previous_t
        if elapsed > 0:
            series.append({"x": round(window["t"], 1), "y": round((window["vsync"] - previous_v) / elapsed, 1)})
        previous_t, previous_v = window["t"], window["vsync"]
    return series


def stack_series(windows: list[dict], names: dict[int, str]) -> dict:
    totals: dict[int, int] = {}
    for window in windows:
        for address, count in window["counts"].items():
            totals[address] = totals.get(address, 0) + count
    top = sorted(totals, key=lambda address: -totals[address])[:STACK_SERIES]
    keys = [label(address, names) for address in top] + ["Outras"]
    rows = []
    for window in windows:
        values = [window["counts"].get(address, 0) for address in top]
        rows.append({"x": round(window["t"], 1), "vsync": window["vsync"],
                     "values": values + [max(0, window["total"] - sum(values))]})
    return {"keys": keys, "rows": rows}


def build_payload(args: argparse.Namespace) -> dict:
    run = parse_run_log(args.run_log)
    windows = parse_profile(args.profile)
    names = load_names(args.names)
    hb = run["heartbeats"]
    return {
        "title": args.title or args.run_log.name,
        "source": args.run_log.name,
        "generated": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "perf": run["perf"],
        "speed": speed_series(windows),
        "calls": [{"x": round(window["t"], 1), "y": window["total"]} for window in windows],
        "stack": stack_series(windows, names) if windows else None,
        "gif": [{"x": row[1], "y": row[3]} for row in hb[:: max(1, len(hb) // 400)]],
        "dma": [{"x": row[1], "y": row[2]} for row in hb[:: max(1, len(hb) // 400)]],
        "gsDraws": [{"x": index + 1, "y": window["draws"] / max(1, window["presents"])}
                    for index, window in enumerate(run["gs"])],
        "gsPixels": [{"x": index + 1, "y": window["pixels"] / max(1, window["presents"])}
                     for index, window in enumerate(run["gs"])],
    }


PAGE = r"""<!DOCTYPE html>
<html lang="pt-BR"><head><meta charset="utf-8"><title>__TITLE__</title>
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>
.viz-root{color-scheme:light;--surface-1:#fcfcfb;--text-primary:#0b0b0b;--text-secondary:#52514e;--grid:#e6e5e0;
--s1:#2a78d6;--s2:#eb6834;--s3:#1baf7a;--s4:#eda100;--s5:#e87ba4;--s6:#008300;--s7:#6250d6;--s8:#e34948}
@media (prefers-color-scheme:dark){.viz-root{color-scheme:dark;--surface-1:#1a1a19;--text-primary:#f0efec;--text-secondary:#c3c2b7;--grid:#33332f;
--s1:#3987e5;--s2:#d95926;--s3:#199e70;--s4:#c98500;--s5:#d55181;--s6:#008300;--s7:#9085e9;--s8:#e66767}}
*{box-sizing:border-box}body{margin:0}
.viz-root{background:var(--surface-1);color:var(--text-primary);font:14px/1.45 system-ui,sans-serif;padding:24px;max-width:1100px;margin:auto}
h1{font-size:22px;margin:0 0 4px}h2{font-size:15px;margin:0 0 2px}.sub{color:var(--text-secondary);font-size:13px;margin:0 0 10px}
.tiles{display:flex;flex-wrap:wrap;gap:24px;margin:16px 0 24px}.tile b{display:block;font-size:24px;font-weight:600}.tile span{color:var(--text-secondary);font-size:12px}
section{margin:0 0 30px}svg{display:block;width:100%;height:auto}svg text{fill:var(--text-secondary);font-size:11px}
.legend{display:flex;flex-wrap:wrap;gap:6px 14px;margin:6px 0;font-size:12px;color:var(--text-secondary)}.legend i{display:inline-block;width:10px;height:10px;border-radius:2px;margin-right:5px}
#tip{position:fixed;pointer-events:none;background:var(--surface-1);border:1px solid var(--grid);border-radius:6px;padding:6px 9px;font-size:12px;display:none;box-shadow:0 2px 8px #0003;max-width:360px;z-index:9}
details{margin-top:6px;font-size:12px}table{border-collapse:collapse;font-size:12px}td,th{padding:2px 10px 2px 0;text-align:right;border-bottom:1px solid var(--grid)}td:first-child,th:first-child{text-align:left}
</style></head><body><div class="viz-root"><h1 id="t"></h1><p class="sub" id="m"></p><div class="tiles" id="tiles"></div><div id="charts"></div><div id="tip"></div></div>
<script>
const D=__DATA__;const W=1040,H=240,L=62,R=16,T=12,B=30;const tip=document.getElementById('tip');
const fmt=v=>v>=1e9?(v/1e9).toFixed(1)+' G':v>=1e6?(v/1e6).toFixed(1)+' M':v>=1e3?(v/1e3).toFixed(1)+' k':(+v.toFixed(1)).toString();
const svgNS='http://www.w3.org/2000/svg';
function el(n,a,p){const e=document.createElementNS(svgNS,n);for(const k in a)e.setAttribute(k,a[k]);if(p)p.appendChild(e);return e}
function nice(m){if(m<=0)return 1;const p=10**Math.floor(Math.log10(m)),f=m/p;return (f<=1?1:f<=2?2:f<=5?5:10)*p}
function show(ev,html){tip.innerHTML=html;tip.style.display='block';tip.style.left=Math.min(ev.clientX+14,innerWidth-370)+'px';tip.style.top=(ev.clientY+14)+'px'}
function hide(){tip.style.display='none'}
function sec(title,sub){const s=document.createElement('section');s.innerHTML='<h2></h2><p class="sub"></p>';s.children[0].textContent=title;s.children[1].textContent=sub;document.getElementById('charts').appendChild(s);return s}
function frame(s,ymax,xmin,xmax,xlabel){const v=el('svg',{viewBox:`0 0 ${W} ${H}`,role:'img'},s);const y=nice(ymax);
 for(let i=0;i<=4;i++){const yy=T+(H-T-B)*i/4;el('line',{x1:L,x2:W-R,y1:yy,y2:yy,stroke:'var(--grid)'},v);const t=el('text',{x:L-8,y:yy+4,'text-anchor':'end'},v);t.textContent=fmt(y*(1-i/4))}
 for(let i=0;i<=5;i++){const xv=xmin+(xmax-xmin)*i/5,t=el('text',{x:L+(W-L-R)*i/5,y:H-10,'text-anchor':i==0?'start':i==5?'end':'middle'},v);t.textContent=fmt(xv)}
 const xl=el('text',{x:W-R,y:H-22+18,'text-anchor':'end',opacity:0},v);xl.textContent=xlabel;
 return {v,sx:x=>L+(W-L-R)*(x-xmin)/((xmax-xmin)||1),sy:yv=>T+(H-T-B)*(1-yv/y),y}}
function line(s,pts,color,unit,xname,ref){if(!pts.length)return;const xs=pts.map(p=>p.x),ymax=Math.max(...pts.map(p=>p.y),ref?ref.y:0),f=frame(s,ymax,Math.min(...xs),Math.max(...xs),xname);
 if(ref){const yy=f.sy(ref.y);el('line',{x1:L,x2:W-R,y1:yy,y2:yy,stroke:'var(--text-secondary)','stroke-dasharray':'4 4'},f.v);const t=el('text',{x:W-R,y:yy-5,'text-anchor':'end'},f.v);t.textContent=ref.label}
 el('polyline',{points:pts.map(p=>f.sx(p.x)+','+f.sy(p.y)).join(' '),fill:'none',stroke:color,'stroke-width':2,'stroke-linejoin':'round'},f.v);
 const c=el('line',{y1:T,y2:H-B,stroke:'var(--text-secondary)',opacity:0},f.v),dot=el('circle',{r:4,fill:color,stroke:'var(--surface-1)','stroke-width':2,opacity:0},f.v);
 const hit=el('rect',{x:L,y:T,width:W-L-R,height:H-T-B,fill:'transparent'},f.v);
 hit.addEventListener('mousemove',ev=>{const b=f.v.getBoundingClientRect(),mx=(ev.clientX-b.left)*W/b.width;let best=pts[0];for(const p of pts)if(Math.abs(f.sx(p.x)-mx)<Math.abs(f.sx(best.x)-mx))best=p;
  c.setAttribute('x1',f.sx(best.x));c.setAttribute('x2',f.sx(best.x));c.setAttribute('opacity',.5);dot.setAttribute('cx',f.sx(best.x));dot.setAttribute('cy',f.sy(best.y));dot.setAttribute('opacity',1);
  show(ev,`${xname} <b>${best.x}</b><br>${fmt(best.y)} ${unit}`)});
 hit.addEventListener('mouseleave',()=>{hide();c.setAttribute('opacity',0);dot.setAttribute('opacity',0)})}
function stacked(s,d){const totals=d.rows.map(r=>r.values.reduce((a,b)=>a+b,0)),ymax=Math.max(...totals),f=frame(s,ymax,0,d.rows.length,'janela');
 const cw=(W-L-R)/d.rows.length,bw=Math.max(3,cw-3),colors=d.keys.map((k,i)=>i==d.keys.length-1?'var(--s8)':`var(--s${i+1})`);
 d.rows.forEach((r,ri)=>{let acc=0;r.values.forEach((val,ki)=>{if(!val)return;const y0=f.sy(acc+val),y1=f.sy(acc);acc+=val;el('rect',{x:L+ri*cw+1.5,y:y0,width:bw,height:Math.max(0,y1-y0-(y1-y0>4?2:0)),fill:colors[ki]},f.v)});
  const hit=el('rect',{x:L+ri*cw,y:T,width:cw,height:H-T-B,fill:'transparent'},f.v);
  hit.addEventListener('mousemove',ev=>show(ev,`<b>t=${r.x}s</b> vsync ${r.vsync}<br>`+r.values.map((val,i)=>val?`<i style="display:inline-block;width:8px;height:8px;border-radius:2px;background:${colors[i]};margin-right:4px"></i>${d.keys[i]}: ${fmt(val)}`:'').filter(Boolean).join('<br>')));
  hit.addEventListener('mouseleave',hide)});
 const lg=document.createElement('div');lg.className='legend';lg.innerHTML=d.keys.map((k,i)=>`<span><i style="background:${colors[i]}"></i>${k}</span>`).join('');s.appendChild(lg);
 const det=document.createElement('details');det.innerHTML='<summary>Ver como tabela</summary>';const tb=document.createElement('table');tb.innerHTML='<tr><th>t (s)</th><th>vsync</th>'+d.keys.map(k=>`<th>${k.split(' ')[0]}</th>`).join('')+'</tr>'+d.rows.map(r=>`<tr><td>${r.x}</td><td>${r.vsync}</td>${r.values.map(v=>`<td>${fmt(v)}</td>`).join('')}</tr>`).join('');det.appendChild(tb);s.appendChild(det)}
document.getElementById('t').textContent=D.title;document.getElementById('m').textContent='Fonte: '+D.source+' · gerado em '+D.generated;
const P=D.perf,tiles=[['realtime_pct','% do tempo real (guest)'],['guest_vsync_hz','vsync do guest (Hz)'],['avg_fps','FPS médio do host'],['one_percent_low_fps','FPS 1% low'],['duration_s','duração (s)']];
document.getElementById('tiles').innerHTML=tiles.filter(t=>P[t[0]]).map(t=>`<div class="tile"><b>${P[t[0]]}</b><span>${t[1]}</span></div>`).join('');
if(D.speed.length){line(sec('Velocidade do jogo','Vsyncs do guest por segundo do relógio real. 50 é a velocidade real no PAL: abaixo é lento, acima não tem limitador.'),D.speed,'var(--s1)','vsyncs/s','t (s)',{y:50,label:'50 = tempo real'})}
if(D.stack){stacked(sec('O que o jogo está fazendo','Chamadas entre funções do guest por janela de tempo, por função. Mudança de cor = mudança de fase.'),D.stack)}
if(D.calls.length){line(sec('Atividade total','Total de chamadas entre funções do guest por janela.'),D.calls,'var(--s1)','chamadas','t (s)')}
if(D.gif.length){line(sec('Envios ao GS (GIF)','Contador acumulado por vsync. Linha reta = o jogo parou de mandar dados ao GS.'),D.gif,'var(--s1)','envios','vsync')}
if(D.dma.length){line(sec('Inícios de DMA','Contador acumulado por vsync.'),D.dma,'var(--s1)','DMAs','vsync')}
if(D.gsDraws.length){line(sec('Draws por quadro','Média por janela de 600 quadros apresentados.'),D.gsDraws,'var(--s1)','draws/quadro','janela')}
if(D.gsPixels.length){line(sec('Pixels por quadro','Média por janela de 600 quadros apresentados (estimativa do contador).'),D.gsPixels,'var(--s1)','pixels/quadro','janela')}
</script></body></html>
"""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--run-log", type=Path, required=True)
    parser.add_argument("--profile", type=Path)
    parser.add_argument("--names", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--title")
    args = parser.parse_args()

    payload = build_payload(args)
    html = PAGE.replace("__TITLE__", payload["title"]).replace("__DATA__", json.dumps(payload).replace("</", "<\\/"))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(html, encoding="utf-8")
    stamp = datetime.now(timezone.utc).isoformat(timespec="seconds")
    print(f"{stamp} INFO relatorio: {args.out} (janelas={len(payload['speed'])} heartbeats={len(payload['gif'])} gs={len(payload['gsDraws'])})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
