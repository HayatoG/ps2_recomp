#!/usr/bin/env python3
"""Compara duas pastas de capturas PNG quadro a quadro (regressao visual).

Generica: nada de jogo hardcoded. Para cada PNG da pasta de referencia procura o de mesmo nome
na pasta candidata e informa MD5 (do arquivo e dos pixels), pixels diferentes, erro maximo por
canal e PSNR. Escreve resumo em texto e JSON lines em <logs>/ (timestamp e nivel, regra 6).

Saida 0 = tudo dentro da tolerancia; 1 = alguma diferenca acima da tolerancia (ou arquivo
faltando); 2 = erro de uso/leitura.

Uso:
  python -I tools/image_regress.py REF_DIR CAND_DIR [--logs logs] [--max-err 0] [--max-diff-px 0]
                                   [--min-psnr 0] [--name regress]

Tolerancias (todas padrao 0 = byte-identico nos pixels): --max-err e o maior erro absoluto
permitido em qualquer canal (0..255); --max-diff-px, o numero de pixels diferentes permitido
por quadro; --min-psnr (dB, 0 = ignorar), PSNR minimo por quadro. Um quadro falha se violar
qualquer uma. Pillow e usado se instalado; senao, decodificador PNG proprio (8 bits, sem
entrelacamento).
"""
import argparse
import datetime
import hashlib
import json
import math
import os
import struct
import sys
import zlib

try:  # Pillow e opcional
    from PIL import Image
except ImportError:  # pragma: no cover
    Image = None


def decode_png_pure(data):
    """Decodifica PNG 8 bits (gray/RGB/RGBA/gray+alpha/paleta) sem entrelacamento -> (w, h, ch, bytes)."""
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("nao e PNG")
    pos, idat, plte = 8, [], None
    w = h = depth = ctype = interlace = None
    while pos < len(data):
        n, typ = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        pos += 12 + n
        if typ == b"IHDR":
            w, h, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
        elif typ == b"PLTE":
            plte = body
        elif typ == b"IDAT":
            idat.append(body)
        elif typ == b"IEND":
            break
    if depth != 8 or interlace != 0 or ctype not in (0, 2, 3, 4, 6):
        raise ValueError("PNG nao suportado sem Pillow (depth=%s tipo=%s interlace=%s)" % (depth, ctype, interlace))
    ch = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[ctype]
    raw = zlib.decompress(b"".join(idat))
    stride = w * ch
    out = bytearray(h * stride)
    prev = bytearray(stride)
    p = 0
    for y in range(h):
        f = raw[p]
        line = bytearray(raw[p + 1:p + 1 + stride])
        p += 1 + stride
        if f == 1:
            for i in range(ch, stride):
                line[i] = (line[i] + line[i - ch]) & 255
        elif f == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 255
        elif f == 3:
            for i in range(stride):
                a = line[i - ch] if i >= ch else 0
                line[i] = (line[i] + ((a + prev[i]) >> 1)) & 255
        elif f == 4:
            for i in range(stride):
                a = line[i - ch] if i >= ch else 0
                b = prev[i]
                c = prev[i - ch] if i >= ch else 0
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[i] = (line[i] + pr) & 255
        elif f != 0:
            raise ValueError("filtro PNG invalido")
        out[y * stride:(y + 1) * stride] = line
        prev = line
    if ctype == 3:
        pal = [plte[i:i + 3] for i in range(0, len(plte), 3)]
        out = bytearray(b"".join(pal[i] for i in out))
        ch = 3
    return w, h, ch, bytes(out)


def load_pixels(path):
    """-> (w, h, canais, bytes) com RGB(A) 8 bits."""
    if Image is not None:
        with Image.open(path) as im:
            if im.mode not in ("RGB", "RGBA", "L", "LA"):
                im = im.convert("RGBA")
            ch = len(im.getbands())
            return im.width, im.height, ch, im.tobytes()
    with open(path, "rb") as f:
        return decode_png_pure(f.read())


def compare_pixels(a, b, ch):
    """-> (pixels diferentes, erro maximo por canal [lista], erro quadratico total)."""
    if a == b:
        return 0, [0] * ch, 0
    maxerr = [0] * ch
    sq = 0
    diff_px = 0
    for i in range(0, len(a), ch):
        pa, pb = a[i:i + ch], b[i:i + ch]
        if pa == pb:
            continue
        diff_px += 1
        for c in range(ch):
            d = abs(pa[c] - pb[c])
            if d > maxerr[c]:
                maxerr[c] = d
            sq += d * d
    return diff_px, maxerr, sq


def md5_file(path):
    h = hashlib.md5()
    with open(path, "rb") as f:
        for blk in iter(lambda: f.read(1 << 20), b""):
            h.update(blk)
    return h.hexdigest()


def compare_frame(ref_path, cand_path, tol):
    r = {"name": os.path.basename(ref_path), "md5_ref": md5_file(ref_path), "md5_cand": md5_file(cand_path)}
    r["md5_equal"] = r["md5_ref"] == r["md5_cand"]
    rw, rh, rc, rp = load_pixels(ref_path)
    cw, chh, cc, cp = load_pixels(cand_path)
    r["pixel_md5_equal"] = hashlib.md5(rp).digest() == hashlib.md5(cp).digest() and (rw, rh, rc) == (cw, chh, cc)
    if (rw, rh, rc) != (cw, chh, cc):
        r.update(ok=False, reason="dimensoes/canais diferentes: %dx%dx%d vs %dx%dx%d" % (rw, rh, rc, cw, chh, cc))
        return r
    diff_px, maxerr, sq = compare_pixels(rp, cp, rc)
    n = rw * rh * rc
    psnr = float("inf") if sq == 0 else 10 * math.log10(255.0 * 255.0 * n / sq)
    r.update(width=rw, height=rh, channels=rc, diff_pixels=diff_px, max_err=maxerr,
             psnr_db=None if psnr == float("inf") else round(psnr, 2))
    reasons = []
    if max(maxerr) > tol["max_err"]:
        reasons.append("erro maximo %d > %d" % (max(maxerr), tol["max_err"]))
    if diff_px > tol["max_diff_px"]:
        reasons.append("pixels diferentes %d > %d" % (diff_px, tol["max_diff_px"]))
    if tol["min_psnr"] > 0 and psnr < tol["min_psnr"]:
        reasons.append("PSNR %.2f < %.2f" % (psnr, tol["min_psnr"]))
    r["ok"] = not reasons
    r["reason"] = "; ".join(reasons)
    return r


def main(argv=None):
    ap = argparse.ArgumentParser(description="Compara pastas de capturas PNG quadro a quadro.")
    ap.add_argument("ref_dir")
    ap.add_argument("cand_dir")
    ap.add_argument("--logs", default="logs", help="pasta dos logs (padrao: logs)")
    ap.add_argument("--name", default="image_regress", help="prefixo dos arquivos de log")
    ap.add_argument("--max-err", type=int, default=0, help="erro maximo por canal permitido (padrao 0)")
    ap.add_argument("--max-diff-px", type=int, default=0, help="pixels diferentes permitidos por quadro (padrao 0)")
    ap.add_argument("--min-psnr", type=float, default=0.0, help="PSNR minimo em dB (0 = ignorar)")
    args = ap.parse_args(argv)

    for d in (args.ref_dir, args.cand_dir):
        if not os.path.isdir(d):
            print("erro: pasta inexistente: %s" % d, file=sys.stderr)
            return 2
    os.makedirs(args.logs, exist_ok=True)
    stamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
    txt_path = os.path.join(args.logs, "%s_%s.log" % (args.name, stamp))
    jl_path = os.path.join(args.logs, "%s_%s.jsonl" % (args.name, stamp))
    txt, jl = open(txt_path, "w", encoding="utf-8"), open(jl_path, "w", encoding="utf-8")

    def log(level, msg, **extra):
        ts = datetime.datetime.now().isoformat(timespec="seconds")
        line = "%s [%s] %s" % (ts, level, msg)
        print(line)
        txt.write(line + "\n")
        jl.write(json.dumps(dict(ts=ts, level=level, msg=msg, **extra), ensure_ascii=False) + "\n")

    tol = {"max_err": args.max_err, "max_diff_px": args.max_diff_px, "min_psnr": args.min_psnr}
    log("INFO", "ref=%s cand=%s tolerancia=%s pillow=%s" % (args.ref_dir, args.cand_dir, tol, Image is not None), tol=tol)
    refs = sorted(f for f in os.listdir(args.ref_dir) if f.lower().endswith(".png"))
    cands = {f for f in os.listdir(args.cand_dir) if f.lower().endswith(".png")}
    if not refs:
        log("ERROR", "a pasta de referencia nao tem PNG")
        return 2
    failed = 0
    for name in refs:
        if name not in cands:
            failed += 1
            log("ERROR", "%s: FALTA na pasta candidata" % name, name=name, ok=False)
            continue
        try:
            r = compare_frame(os.path.join(args.ref_dir, name), os.path.join(args.cand_dir, name), tol)
        except (ValueError, OSError, zlib.error) as e:
            failed += 1
            log("ERROR", "%s: erro de leitura: %s" % (name, e), name=name, ok=False)
            continue
        if r["ok"]:
            level = "INFO"
            state = "IGUAL (md5)" if r["md5_equal"] else ("IGUAL (pixels)" if r["pixel_md5_equal"] else "DENTRO DA TOLERANCIA")
        else:
            level, state = "ERROR", "DIFERENTE (%s)" % r["reason"]
            failed += 1
        extra = "" if "diff_pixels" not in r else " diff_px=%d max_err=%s psnr=%s" % (
            r["diff_pixels"], r["max_err"], "inf" if r["psnr_db"] is None else "%.2fdB" % r["psnr_db"])
        log(level, "%s: %s md5 %s/%s%s" % (name, state, r["md5_ref"][:8], r["md5_cand"][:8], extra), **r)
    for name in sorted(cands - set(refs)):
        log("WARN", "%s: so existe na candidata (ignorado)" % name, name=name)
    log("ERROR" if failed else "INFO", "resumo: %d quadros, %d falhas -> %s" % (len(refs), failed, "FALHA" if failed else "OK"),
        frames=len(refs), failed=failed)
    txt.close()
    jl.close()
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
