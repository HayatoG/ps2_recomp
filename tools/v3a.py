"""Descompressor generico "V3a"/"V2" (LZSS + Huffman estatico estilo LHA -lh5-/ar002).

Algoritmo reconstruido da rotina do jogo (ver <jogo>/docs/v3a-zim-format.md): e o decoder
classico "ar002" de Haruhiko Okumura (o mesmo do LHA -lh5-), com NC=510, NT=19, TBIT=5,
CBIT=9, NP=16, PBIT=4, bitbuf de 16 bits MSB-first, tabelas de 12 bits (C) e 8 bits (P/T).
Diferencas: o numero de blocos Huffman vem do cabecalho (nao ha EOF) e a janela e o proprio
buffer de saida (distancia sem limite pratico).

Cabecalhos aceitos:
  00 'V' '3' 'a' | u32 nblocos | u32 tam_descomprimido | u32 tam_comprimido | bitstream (+0x10)
  00 'V' '2'     | u16 nblocos (LE)                                        | bitstream (+5)
  b0 != 0        | b0 = nblocos                                            | bitstream (+1)

Uso (rode do diretorio do jogo):
  python -I ../tools/v3a.py decompress <in> <out>
  python -I ../tools/v3a.py info <in>
  python -I ../tools/v3a.py validate --iso <iso> --lba N --size N [--out-dir work/x] [--json logs/x.json]
  python -I ../tools/v3a.py validate <arquivo_fpk>
Sem dependencias. Logs em logs/v3a_<data>.log.
"""
import argparse, datetime, importlib.util, json, logging, os, struct, sys, time

log = logging.getLogger("v3a")

NC, NT, NP = 510, 19, 16
TBIT, CBIT, PBIT = 5, 9, 4


class V3aError(Exception):
    pass


def parse_header(data):
    """Retorna (nblocos, offset_do_bitstream, tam_descomp_ou_None, tam_comp_ou_None, variante)."""
    if len(data) < 1:
        raise V3aError("vazio")
    if data[0] != 0:
        return data[0], 1, None, None, "byte"
    if data[1:2] != b"V":
        raise V3aError("magic invalido")
    if data[2:3] == b"2":
        return struct.unpack_from("<H", data, 3)[0], 5, None, None, "V2"
    if data[2:4] == b"3a":
        n, usize, csize = struct.unpack_from("<iII", data, 4)
        return n, 16, usize, csize, "V3a"
    raise V3aError("versao desconhecida")


class _Dec:
    def __init__(self, data, pos):
        self.d = bytes(data) + b"\0" * 8  # o jogo le alem do fim; aqui le zeros
        self.p = pos * 8                  # posicao em bits
        self.left = [0] * (2 * NC)
        self.right = [0] * (2 * NC)
        self.c_len = [0] * NC
        self.pt_len = [0] * NT
        self.c_table = [0] * 4096
        self.pt_table = [0] * 256

    def peek(self):
        p = self.p
        i = p >> 3
        d = self.d
        return (((d[i] << 16) | (d[i + 1] << 8) | d[i + 2]) >> (8 - (p & 7))) & 0xFFFF

    def getbits(self, n):
        v = self.peek() >> (16 - n)
        self.p += n
        return v

    def make_table(self, nchar, bitlen, tablebits, table):
        count = [0] * 17
        for i in range(nchar):
            count[bitlen[i]] += 1
        start = [0] * 18
        for i in range(1, 17):
            start[i + 1] = (start[i] + (count[i] << (16 - i))) & 0xFFFF
        if start[17] != 0:
            raise V3aError("tabela Huffman invalida")
        jut = 16 - tablebits
        weight = [0] * 17
        for i in range(1, tablebits + 1):
            start[i] >>= jut
            weight[i] = 1 << (tablebits - i)
        for i in range(tablebits + 1, 17):
            weight[i] = 1 << (16 - i)
        i = start[tablebits + 1] >> jut
        if i != 0:
            for k in range(i, 1 << tablebits):
                table[k] = 0
        avail = nchar
        mask = 1 << (15 - tablebits)
        left, right = self.left, self.right
        for ch in range(nchar):
            ln = bitlen[ch]
            if ln == 0:
                continue
            nextcode = start[ln] + weight[ln]
            if ln <= tablebits:
                for k in range(start[ln], nextcode):
                    table[k] = ch
            else:
                k = start[ln]
                arr, idx = table, k >> jut
                for _ in range(ln - tablebits):
                    if arr[idx] == 0:
                        left[avail] = right[avail] = 0
                        arr[idx] = avail
                        avail += 1
                    node = arr[idx]
                    arr = right if (k & mask) else left
                    idx = node
                    k <<= 1
                arr[idx] = ch
            start[ln] = nextcode

    def read_pt_len(self, nn, nbit, special):
        n = self.getbits(nbit)
        if n == 0:
            c = self.getbits(nbit)
            for i in range(nn):
                self.pt_len[i] = 0
            for i in range(256):
                self.pt_table[i] = c
            return
        i = 0
        while i < n:
            bb = self.peek()
            c = bb >> 13
            if c == 7:
                m = 1 << 12
                while m & bb:
                    m >>= 1
                    c += 1
            self.p += 3 if c < 7 else c - 3
            self.pt_len[i] = c
            i += 1
            if i == special:
                z = self.getbits(2)
                for _ in range(z):
                    self.pt_len[i] = 0
                    i += 1
        while i < nn:
            self.pt_len[i] = 0
            i += 1
        self.make_table(nn, self.pt_len, 8, self.pt_table)

    def _tree(self, c, limit, mask, bb):
        left, right = self.left, self.right
        while c >= limit:
            c = right[c] if (bb & mask) else left[c]
            mask >>= 1
        return c

    def read_c_len(self):
        n = self.getbits(CBIT)
        if n == 0:
            c = self.getbits(CBIT)
            for i in range(NC):
                self.c_len[i] = 0
            for i in range(4096):
                self.c_table[i] = c
            return
        i = 0
        while i < n:
            bb = self.peek()
            c = self.pt_table[bb >> 8]
            if c >= NT:
                c = self._tree(c, NT, 0x80, bb)
            self.p += self.pt_len[c]
            if c <= 2:
                if c == 0:
                    z = 1
                elif c == 1:
                    z = self.getbits(4) + 3
                else:
                    z = self.getbits(CBIT) + 20
                for _ in range(z):
                    self.c_len[i] = 0
                    i += 1
            else:
                self.c_len[i] = c - 2
                i += 1
        while i < NC:
            self.c_len[i] = 0
            i += 1
        self.make_table(NC, self.c_len, 12, self.c_table)


def decompress(data, expected=None):
    """Descomprime um bloco V3a/V2/byte-count. Retorna bytes."""
    nblocks, pos, usize, _csize, _var = parse_header(data)
    if expected is None:
        expected = usize
    dec = _Dec(data, pos)
    out = bytearray()
    d = dec.d
    for _ in range(max(nblocks, 0)):
        bsize = dec.getbits(16)
        dec.read_pt_len(NT, TBIT, 3)
        dec.read_c_len()
        dec.read_pt_len(NP, PBIT, -1)
        c_table, c_len, pt_table, pt_len = dec.c_table, dec.c_len, dec.pt_table, dec.pt_len
        left, right = dec.left, dec.right
        p = dec.p
        for _ in range(bsize):
            i = p >> 3
            bb = (((d[i] << 16) | (d[i + 1] << 8) | d[i + 2]) >> (8 - (p & 7))) & 0xFFFF
            c = c_table[bb >> 4]
            if c >= NC:
                m = 8
                while c >= NC:
                    c = right[c] if (bb & m) else left[c]
                    m >>= 1
            p += c_len[c]
            if c < 256:
                out.append(c)
                continue
            i = p >> 3
            bb = (((d[i] << 16) | (d[i + 1] << 8) | d[i + 2]) >> (8 - (p & 7))) & 0xFFFF
            j = pt_table[bb >> 8]
            if j >= NP:
                m = 0x80
                while j >= NP:
                    j = right[j] if (bb & m) else left[j]
                    m >>= 1
            p += pt_len[j]
            if j:
                i = p >> 3
                bb = (((d[i] << 16) | (d[i + 1] << 8) | d[i + 2]) >> (8 - (p & 7))) & 0xFFFF
                p += j - 1  # getbits(j-1)
                j = (1 << (j - 1)) + (bb >> (17 - j))
            dist = j + 1
            ln = c - 253
            s = len(out) - dist
            if s < 0:
                raise V3aError(f"distancia {dist} antes do inicio (saida {len(out)})")
            if dist >= ln:
                out += out[s:s + ln]
            else:
                for k in range(ln):
                    out.append(out[s + k])
        dec.p = p
        if expected is not None and len(out) > expected:
            raise V3aError(f"saida {len(out)} excede o esperado {expected}")
    return bytes(out)


def _setup_log():
    os.makedirs("logs", exist_ok=True)
    path = os.path.join("logs", f"v3a_{datetime.date.today().isoformat()}.log")
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s",
                        handlers=[logging.FileHandler(path, encoding="utf-8"), logging.StreamHandler()])


def _load_fpk():
    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location("fpk", os.path.join(here, "fpk.py"))
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def _magic(b):
    t = b[:4]
    if all(32 <= c < 127 for c in t):
        return t.decode()
    if t == b"fpk\0":
        return "fpk"
    return t.hex()


def cmd_decompress(a):
    data = open(a.input, "rb").read()
    t = time.time()
    out = decompress(data)
    open(a.output, "wb").write(out)
    log.info("%s -> %s: %d -> %d bytes (%.1fs), magic %s", a.input, a.output, len(data), len(out),
             time.time() - t, _magic(out))
    return 0


def cmd_info(a):
    data = open(a.input, "rb").read(16)
    print(parse_header(data))
    return 0


def cmd_validate(a):
    fpk = _load_fpk()
    src = fpk.Src(a.iso, a.lba * fpk.SEC, a.size) if a.iso else fpk.Src(a.file)
    _hdr, ents = fpk.parse(src)
    res, ok, tot = [], 0, 0
    for e in ents:
        if e["size"] == 0 or src.read(e["abs"], 4) != b"\0V3a":
            continue
        if a.index is not None and e["index"] not in a.index:
            continue
        tot += 1
        blob = src.read(e["abs"], e["size"])
        n, _, usize, csize, _ = parse_header(blob)
        t = time.time()
        r = dict(index=e["index"], blocks=n, usize=usize, csize=csize, entry_size=e["size"], tag=e["tag"])
        try:
            out = decompress(blob)
            r.update(out=len(out), ok=len(out) == usize, magic=_magic(out), secs=round(time.time() - t, 1))
            if a.out_dir:
                os.makedirs(a.out_dir, exist_ok=True)
                open(os.path.join(a.out_dir, f"{e['index']:04d}.dec"), "wb").write(out)
        except Exception as ex:  # noqa
            r.update(ok=False, error=str(ex))
        ok += bool(r.get("ok"))
        log.info("#%d %s", e["index"], r)
        res.append(r)
    log.info("validacao: %d/%d com tamanho == campo +8", ok, tot)
    if a.json:
        json.dump(dict(ok=ok, total=tot, entries=res), open(a.json, "w"), indent=1)
    return 0 if ok == tot else 1


def main():
    _setup_log()
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sp = p.add_subparsers(required=True)
    s = sp.add_parser("decompress"); s.add_argument("input"); s.add_argument("output"); s.set_defaults(fn=cmd_decompress)
    s = sp.add_parser("info"); s.add_argument("input"); s.set_defaults(fn=cmd_info)
    s = sp.add_parser("validate"); s.add_argument("file", nargs="?"); s.add_argument("--iso")
    s.add_argument("--lba", type=int); s.add_argument("--size", type=int); s.add_argument("--out-dir")
    s.add_argument("--json"); s.add_argument("--index", type=int, nargs="*"); s.set_defaults(fn=cmd_validate)
    a = p.parse_args()
    sys.exit(a.fn(a))


if __name__ == "__main__":
    main()
