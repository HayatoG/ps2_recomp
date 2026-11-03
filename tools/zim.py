"""Leitor generico de texturas "wZIM" (zim) de PS2: info e conversao para PNG (sem dependencias).

Formato (ver <jogo>/docs/v3a-zim-format.md): cabecalho de 0x20 bytes, N registros de imagem
(0x28 B: pacote, indice, blocos, largura/altura, tamanho, TEX0, TEX1), M registros de paleta
(0x10 B: pacote, tamanho, reg. GS), cadeia DMA e pacotes GIF prontos para o GS. Os pixels estao
linearmente no pacote de upload (BITBLTBUF/TRXREG + GIFtag IMAGE), sem swizzle no arquivo;
paletas CT32 (RGBA, alfa PS2 0..0x80) em CSM1 (PSMT8: troca dos blocos 8..15 <-> 16..23).

Uso:
  python -I ../tools/zim.py info   <arq.zim>
  python -I ../tools/zim.py to-png <arq.zim> <saida.png|dir> [--index N] [--alpha ps2|raw]
  Origem dentro de um fpk (inclusive aninhado, inclusive bloco V3a):
  python -I ../tools/zim.py to-png --iso <iso> --lba N --size N --path 4.26 <saida.png>
  python -I ../tools/zim.py to-png --fpk <arq.BIN> --path 4.26 <saida.png>
Logs em logs/zim_<data>.log.
"""
import argparse, datetime, importlib.util, logging, os, struct, sys, zlib

log = logging.getLogger("zim")
PSM_NAMES = {0x00: "PSMCT32", 0x01: "PSMCT24", 0x02: "PSMCT16", 0x0A: "PSMCT16S",
             0x13: "PSMT8", 0x14: "PSMT4", 0x1B: "PSMT8H", 0x24: "PSMT4HL", 0x2C: "PSMT4HH"}
BPP = {0x00: 32, 0x01: 24, 0x02: 16, 0x0A: 16, 0x13: 8, 0x14: 4}


class ZimError(Exception):
    pass


def decode_tex0(v):
    return dict(tbp0=v & 0x3FFF, tbw=(v >> 14) & 0x3F, psm=(v >> 20) & 0x3F, tw=(v >> 26) & 0xF,
                th=(v >> 30) & 0xF, tcc=(v >> 34) & 1, tfx=(v >> 35) & 3, cbp=(v >> 37) & 0x3FFF,
                cpsm=(v >> 51) & 0xF, csm=(v >> 55) & 1, csa=(v >> 56) & 0x1F, cld=(v >> 61) & 7)


def _image_data(b, pkt, size):
    """Coleta os bytes de pixels a partir do pacote GIF: procura GIFtags FLG=IMAGE apos pkt."""
    out = bytearray()
    q = pkt
    end = len(b) - 16
    while len(out) < size and q <= end:
        lo = struct.unpack_from("<Q", b, q)[0]
        nloop, flg = lo & 0x7FFF, (lo >> 58) & 3
        # GIFtag IMAGE precedido do VIFcode DIRECT (0x50) com nloop+1 qwords
        direct = q >= 4 and b[q - 4:q] == struct.pack("<I", 0x50000000 | (nloop + 1))
        if flg == 2 and nloop and direct and q + 16 + nloop * 16 <= len(b):
            out += b[q + 16:q + 16 + nloop * 16]
            q += 16 + nloop * 16
        else:
            q += 16
    if len(out) < size:
        raise ZimError(f"dados de imagem incompletos ({len(out)} < {size}) no pacote 0x{pkt:x}")
    return bytes(out[:size])


def parse(b):
    if b[:4] != b"wZIM":
        raise ZimError("magic != wZIM")
    ver, nimg, npal, img_off, pal_off, dma_off, _r, pkt_off = struct.unpack_from("<8I", b, 4)
    z = dict(version=ver, nimg=nimg, npal=npal, img_off=img_off, pal_off=pal_off,
             dma_off=dma_off, pkt_off=pkt_off, images=[], palettes=[])
    for i in range(nimg):
        o = img_off + 0x28 * i
        pkt, idx, bw, bh, w, h, unk, size, tex0, tex1 = struct.unpack_from("<II4HIIQQ", b, o)
        z["images"].append(dict(pkt=pkt, index=idx, blocks=(bw, bh), w=w, h=h, unk=unk, size=size,
                                tex0=tex0, tex1=tex1, **{"t_" + k: v for k, v in decode_tex0(tex0).items()}))
    for i in range(npal):
        o = pal_off + 0x10 * i
        pkt, size, reg = struct.unpack_from("<IIQ", b, o)
        z["palettes"].append(dict(pkt=pkt, size=size, reg=reg))
    return z


def _unswizzle_csm1(pal):
    if len(pal) != 256:
        return pal
    return [pal[(i & 0xE7) | ((i & 8) << 1) | ((i & 0x10) >> 1)] for i in range(256)]


def to_rgba(b, z, n=0, alpha="ps2"):
    im = z["images"][n]
    psm, w, h = im["t_psm"], im["w"], im["h"]
    px = _image_data(b, im["pkt"], im["size"])
    pal = None
    if psm in (0x13, 0x14):
        p = z["palettes"][min(im["index"], len(z["palettes"]) - 1)]
        raw = _image_data(b, p["pkt"], p["size"])
        pal = [tuple(raw[i:i + 4]) for i in range(0, len(raw), 4)]
        if psm == 0x13:
            pal = _unswizzle_csm1(pal)
    def fa(a):
        return min(255, a * 2) if alpha == "ps2" else a
    rows = []
    if psm == 0x14:
        for y in range(h):
            r = bytearray()
            base = y * w
            for x in range(w):
                k = base + x
                v = px[k >> 1]
                c = pal[(v >> 4) if (k & 1) else (v & 0xF)]
                r += bytes((c[0], c[1], c[2], fa(c[3])))
            rows.append(bytes(r))
    elif psm == 0x13:
        lut = [bytes((c[0], c[1], c[2], fa(c[3]))) for c in pal]
        for y in range(h):
            rows.append(b"".join(lut[v] for v in px[y * w:(y + 1) * w]))
    elif psm == 0x00:
        for y in range(h):
            r = bytearray(px[y * w * 4:(y + 1) * w * 4])
            for x in range(3, len(r), 4):
                r[x] = fa(r[x])
            rows.append(bytes(r))
    else:
        raise ZimError(f"PSM 0x{psm:x} nao suportado")
    return w, h, rows


def write_png(path, w, h, rows):
    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    raw = b"".join(b"\0" + r for r in rows)
    data = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    with open(path, "wb") as f:
        f.write(data)


def _load(name):
    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location(name, os.path.join(here, name + ".py"))
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def read_input(a):
    if not (a.iso or a.fpk):
        return open(a.input, "rb").read()
    fpk, v3a = _load("fpk"), _load("v3a")
    src = fpk.Src(a.iso, a.lba * fpk.SEC, a.size) if a.iso else fpk.Src(a.fpk)
    for step in a.path.split("."):
        _hdr, ents = fpk.parse(src)
        e = ents[int(step)]
        src = src.sub(e["abs"], e["size"])
        if src.read(0, 4) == b"\0V3a":
            blob = v3a.decompress(src.read(0, src.size))
            tmp = fpk.Src.__new__(fpk.Src)
            import io
            tmp.f, tmp.base, tmp.size = io.BytesIO(blob), 0, len(blob)
            src = tmp
    return src.read(0, src.size)


def cmd_info(a):
    b = read_input(a)
    z = parse(b)
    print(f"wZIM v{z['version']} imagens={z['nimg']} paletas={z['npal']} tamanho={len(b)}")
    for i, im in enumerate(z["images"]):
        print(f"  img{i}: {im['w']}x{im['h']} {PSM_NAMES.get(im['t_psm'], hex(im['t_psm']))} "
              f"size={im['size']} pkt=0x{im['pkt']:x} idx={im['index']} blocks={im['blocks']} "
              f"TEX0: tbw={im['t_tbw']} tw=2^{im['t_tw']} th=2^{im['t_th']} tcc={im['t_tcc']} "
              f"cpsm={im['t_cpsm']} csm={im['t_csm']} cld={im['t_cld']} TEX1=0x{im['tex1']:x}")
    for i, p in enumerate(z["palettes"]):
        print(f"  pal{i}: size={p['size']} ({p['size'] // 4} cores) pkt=0x{p['pkt']:x} reg=0x{p['reg']:016x}")
    return 0


def cmd_png(a):
    b = read_input(a)
    z = parse(b)
    idxs = [a.index] if a.index is not None else range(z["nimg"])
    for n in idxs:
        w, h, rows = to_rgba(b, z, n, a.alpha)
        out = a.output
        if os.path.isdir(out) or len(idxs) > 1:
            os.makedirs(out, exist_ok=True)
            out = os.path.join(out, f"img{n}.png")
        write_png(out, w, h, rows)
        log.info("img%d %dx%d %s -> %s", n, w, h, PSM_NAMES.get(z["images"][n]["t_psm"]), out)
    return 0


def main():
    os.makedirs("logs", exist_ok=True)
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s",
                        handlers=[logging.FileHandler(os.path.join("logs", f"zim_{datetime.date.today()}.log"),
                                                      encoding="utf-8"), logging.StreamHandler()])
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sp = p.add_subparsers(required=True)
    for name, fn in (("info", cmd_info), ("to-png", cmd_png)):
        s = sp.add_parser(name)
        if name == "info":
            s.add_argument("input", nargs="?")
        else:
            s.add_argument("args", nargs="+", help="<entrada> <saida> ou so <saida> com --iso/--fpk")
            s.add_argument("--index", type=int)
            s.add_argument("--alpha", choices=("ps2", "raw"), default="ps2")
        s.add_argument("--iso"); s.add_argument("--lba", type=int); s.add_argument("--size", type=int)
        s.add_argument("--fpk"); s.add_argument("--path")
        s.set_defaults(fn=fn)
    a = p.parse_args()
    if a.fn is cmd_png:
        if a.iso or a.fpk:
            a.input, a.output = None, a.args[-1]
        elif len(a.args) == 2:
            a.input, a.output = a.args
        else:
            p.error("to-png <entrada> <saida>")
    sys.exit(a.fn(a))


if __name__ == "__main__":
    main()
