"""Parser generico do conteiner "fpk" (magic 'fpk\\0').

Uso (rode a partir do diretorio do jogo, para logs/ ficar relativo a ele):
  python -I ../tools/fpk.py list    <arquivo.BIN>            [--json logs/x.json]
  python -I ../tools/fpk.py list    --iso <iso> --lba N --size N
  python -I ../tools/fpk.py extract <arquivo.BIN> --index N | --all --out <dir>
So grava arquivos no comando extract. Logs em logs/fpk_<data>.log.
"""
import argparse, collections, datetime, json, logging, os, struct, sys

SEC = 2048
log = logging.getLogger("fpk")


class Src:
    """Leitor com base+tamanho sobre um arquivo cru ou sobre uma extensao dentro de uma ISO."""
    def __init__(self, path, base=0, size=None):
        self.f = open(path, "rb")
        self.base = base
        if size is None:
            self.f.seek(0, 2)
            size = self.f.tell() - base
        self.size = size

    def sub(self, off, size):
        s = object.__new__(Src)
        s.f, s.base, s.size = self.f, self.base + off, size
        return s

    def read(self, off, n):
        if off < 0 or off >= self.size:
            return b""
        self.f.seek(self.base + off)
        return self.f.read(min(n, self.size - off))


def detect(b):
    """Detecta o tipo pelos primeiros bytes (heuristica; nomes sao so rotulos)."""
    if len(b) < 4:
        return "empty" if not b else "tiny"
    t = b[:4]
    known = {b"TIM2": "TIM2", b"VAGp": "VAG", b"SShd": "SShd", b"SSbd": "SSbd", b"fpk\0": "fpk",
             b"RIFF": "RIFF", b"OggS": "Ogg", b"MThd": "MIDI", b"IECS": "IECS(HD/BD)", b"\x7fELF": "ELF",
             b"\x00\x00\x01\xba": "MPEG-PS", b"\x00\x00\x01\xb3": "MPEG-ES", b"sdac": "sdac"}
    if t in known:
        return known[t]
    if t == b"\x00V3a":
        return "V3a-packed"
    if b[:19] == b"cavia stream format":
        return "cavia-stream"
    if all(32 <= c < 127 for c in t):
        return "ascii:" + t.decode("ascii")
    if b.count(0) == len(b):
        return "zeros"
    return "unknown"


def fourcc(v):
    b = struct.pack("<I", v)
    if v and all(32 <= c < 127 for c in b):
        return b.decode("ascii")
    return f"0x{v:x}" if v else ""


def ext_of(tag):
    """w3 de entradas de pacote interno: extensao em fourcc invertido (ex.: bytes 'tsl\0' = 'lst')."""
    b = struct.pack("<I", tag).rstrip(b"\x00")
    if b and all(32 <= c < 127 for c in b):
        return b[::-1].decode("ascii")
    return ""


def parse(src):
    h = src.read(0, 0x80)
    if h[:4] != b"fpk\0":
        raise ValueError("magic != fpk\\0")
    (_, _, count, align, tab_off, _, tab_size, _, _, _, data_start, total) = struct.unpack_from("<12I", h, 0)
    name = h[0x40:0x80].split(b"\0")[0].decode("ascii", "replace")
    hdr = dict(count=count, align=align, table_off=tab_off, table_size=tab_size,
               data_start=data_start, total=total, name=name)
    tab = src.read(tab_off, count * 16)
    if len(tab) < count * 16:
        raise ValueError("tabela truncada")
    ents = []
    for i in range(count):
        a, off, size, tag = struct.unpack_from("<4I", tab, i * 16)
        ents.append(dict(index=i, a=a, off=off, size=size, tag=tag, ext=ext_of(tag),
                         tagstr=fourcc(tag)))
    for e in ents:
        e["abs"] = data_start + e["off"]
        e["magic"] = detect(src.read(e["abs"], 32)) if e["size"] else "empty"
    return hdr, ents


def validate(hdr, ents, src):
    """Retorna lista de problemas: fora do arquivo, sobreposicao, desalinhamento."""
    probs = []
    if hdr["data_start"] + hdr["total"] != src.size:
        probs.append(f"data_start+total 0x{hdr['data_start'] + hdr['total']:x} != tamanho 0x{src.size:x}")
    spans = []
    for e in ents:
        if e["size"] == 0:
            continue
        if e["abs"] + e["size"] > src.size:
            probs.append(f"#{e['index']} fora do arquivo: end=0x{e['abs'] + e['size']:x}")
        if e["off"] % hdr["align"]:
            probs.append(f"#{e['index']} off 0x{e['off']:x} nao alinhado")
        spans.append((e["abs"], e["abs"] + e["size"], e["index"]))
    spans.sort()
    dup = 0
    for (s0, e0, i0), (s1, e1, i1) in zip(spans, spans[1:]):
        if s1 < e0:
            if (s0, e0) == (s1, e1):
                dup += 1  # entradas que apontam para o mesmo extent (alias)
            else:
                probs.append(f"sobreposicao #{i0} [0x{s0:x},0x{e0:x}) x #{i1} [0x{s1:x},0x{e1:x})")
    if dup:
        log.info("%d entradas sao alias exato de outra", dup)
    if spans and spans[0][0] < hdr["data_start"]:
        probs.append("dados antes de data_start")
    return probs


def open_src(a):
    if a.iso:
        if a.lba is None or a.size is None:
            sys.exit("--iso exige --lba e --size")
        return Src(a.iso, a.lba * SEC, a.size)
    if not a.file:
        sys.exit("informe arquivo ou --iso/--lba/--size")
    return Src(a.file)


def enter(src, a):
    """--into N[.M...]: abre fpk aninhado na entrada N (e depois M dentro dele)."""
    for part in (a.into.split(".") if a.into else []):
        hdr, ents = parse(src)
        e = ents[int(part)]
        src = src.sub(e["abs"], e["size"])
    return src


def cmd_list(a):
    src = open_src(a)
    src = enter(src, a)
    hdr, ents = parse(src)
    log.info("fpk '%s': %d entradas, align=0x%x, data_start=0x%x, total=0x%x",
             hdr["name"], hdr["count"], hdr["align"], hdr["data_start"], hdr["total"])
    for e in ents:
        log.info("#%-4d off=0x%08x size=0x%08x a=%d tag=%-6s ext=%-4s %s", e["index"], e["off"], e["size"], e["a"], e["tagstr"], e["ext"], e["magic"])
    stats = collections.Counter(e["magic"] for e in ents)
    log.info("tipos: %s", dict(stats))
    probs = validate(hdr, ents, src)
    for p in probs:
        log.warning(p)
    log.info("validacao: %s", "OK" if not probs else f"{len(probs)} problema(s)")
    out = a.json or os.path.join("logs", f"fpk_list_{hdr['name']}.json")
    os.makedirs(os.path.dirname(out) or ".", exist_ok=True)
    with open(out, "w") as f:
        json.dump(dict(header=hdr, types=stats, problems=probs, entries=ents), f, indent=1)
    log.info("json: %s", out)
    return 1 if probs else 0


def cmd_extract(a):
    if not a.out or (a.index is None and not a.all):
        sys.exit("extract exige --out e (--index N | --all)")
    src = open_src(a)
    src = enter(src, a)
    hdr, ents = parse(src)
    sel = ents if a.all else [ents[a.index]]
    os.makedirs(a.out, exist_ok=True)
    for e in sel:
        if not e["size"]:
            continue
        ext = {"TIM2": "tm2", "MPEG-PS": "pss", "VAG": "vag", "fpk": "fpk"}.get(e["magic"], "bin")
        p = os.path.join(a.out, f"{e['index']:04d}_{e['off']:08x}.{ext}")
        with open(p, "wb") as o:
            pos, left = e["abs"], e["size"]
            while left > 0:
                chunk = src.read(pos, min(left, 1 << 22))
                if not chunk:
                    break
                o.write(chunk)
                pos += len(chunk)
                left -= len(chunk)
        log.info("extraido %s (%d bytes)", p, e["size"])
    return 0


def main():
    os.makedirs("logs", exist_ok=True)
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s",
                        handlers=[logging.FileHandler(f"logs/fpk_{datetime.date.today()}.log"), logging.StreamHandler()])
    p = argparse.ArgumentParser(description=__doc__)
    sp = p.add_subparsers(dest="cmd", required=True)
    for n, fn in (("list", cmd_list), ("extract", cmd_extract)):
        s = sp.add_parser(n)
        s.add_argument("file", nargs="?")
        s.add_argument("--iso")
        s.add_argument("--lba", type=int)
        s.add_argument("--size", type=int)
        s.add_argument("--into", help="indice(s) de fpk aninhado, ex.: 1 ou 1.3")
        s.set_defaults(fn=fn)
        if n == "list":
            s.add_argument("--json")
        else:
            s.add_argument("--index", type=int)
            s.add_argument("--all", action="store_true")
            s.add_argument("--out")
    a = p.parse_args()
    sys.exit(a.fn(a))


if __name__ == "__main__":
    main()
