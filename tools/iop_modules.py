"""Analisa modulos IOP (IRX) e imagens IOPRP: nome/versao, exports, imports.
Uso: python -I tools/iop_modules.py <diretorio_MODULES> [--json saida.json]
Le apenas metadados (nomes, numeros); nao despeja bytes. Logs em logs/."""
import struct, sys, os, glob, json, logging, datetime

os.makedirs("logs", exist_ok=True)
logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s",
                    handlers=[logging.FileHandler(f"logs/iop_modules_{datetime.date.today()}.log"),
                              logging.StreamHandler()])
log = logging.getLogger("iop_modules")

EXP_MAGIC, IMP_MAGIC = 0x41C00000, 0x41E00000


def cstr(b):
    return b.split(b"\0")[0].decode("ascii", "replace")


def parse_irx(d, name="?"):
    """Retorna dict com modulo, versao, exports e imports de um ELF IRX (tipo 0xFF80)."""
    if d[:4] != b"\x7fELF":
        raise ValueError("nao e ELF")
    etype = struct.unpack_from("<H", d, 16)[0]
    shoff = struct.unpack_from("<I", d, 32)[0]
    shentsize, shnum = struct.unpack_from("<HH", d, 46)
    res = {"elf_type": hex(etype), "module": None, "version": None, "exports": [], "imports": []}
    # .iopmod (SHT 0x70000080): moduleinfo,entry,gp,text,data,bss,u16 version,nome
    for i in range(shnum):
        sh = struct.unpack_from("<10I", d, shoff + i * shentsize)
        if sh[1] == 0x70000080:
            off = sh[4]
            _, entry, gp, ts, ds, bs, ver = struct.unpack_from("<6IH", d, off)
            res.update(module=cstr(d[off + 26:off + 26 + 32]), version=f"{ver >> 8}.{ver & 0xFF}",
                       entry=hex(entry), text=ts, data=ds, bss=bs)
    # posicoes de arquivo com relocacao R_MIPS_32 (cada slot de export com funcao tem uma)
    relpos = set()
    secs = [struct.unpack_from("<10I", d, shoff + i * shentsize) for i in range(shnum)]
    for sh in secs:
        if sh[1] == 9 and sh[7] < len(secs):
            tgt = secs[sh[7]]
            for k in range(sh[5] // 8):
                ro, ri = struct.unpack_from("<II", d, sh[4] + k * 8)
                if (ri & 0xFF) == 2:
                    relpos.add(tgt[4] + ro - tgt[3])
    # Primeiros PT_LOAD: arquivo ~ memoria; varre magics alinhados em 4 bytes
    for o in range(0, len(d) - 16, 4):
        w = struct.unpack_from("<I", d, o)[0]
        if w == EXP_MAGIC and struct.unpack_from("<I", d, o + 4)[0] == 0 and d[o + 12:o + 20].isascii():
            ver = struct.unpack_from("<I", d, o + 8)[0]
            lib = cstr(d[o + 12:o + 20])
            if not lib.replace("_", "").isalnum():
                continue
            # slots 0..3 reservados (_start, reservado, shutdown, reservado); funcoes a partir do slot 4
            # fim = ultimo slot com relocacao em cadeia (lacunas <= 3 slots); slots zerados = funcoes removidas
            base, last, gap, p = o + 20, 3, 0, o + 20 + 16
            while p + 4 <= len(d) and gap <= 3:
                if p in relpos:
                    last, gap = (p - base) // 4, 0
                else:
                    gap += 1
                p += 4
            n = last - 3
            res["exports"].append({"lib": lib, "version": f"{(ver >> 8) & 0xFF}.{ver & 0xFF}", "count": n})
        elif w == IMP_MAGIC and struct.unpack_from("<I", d, o + 4)[0] == 0 and d[o + 12:o + 20].isascii():
            ver = struct.unpack_from("<I", d, o + 8)[0]
            lib = cstr(d[o + 12:o + 20])
            if not lib.replace("_", "").isalnum():
                continue
            nums, p = [], o + 20
            while p + 8 <= len(d):
                a, b = struct.unpack_from("<II", d, p)
                if a == 0x03E00008 and (b >> 16) == 0x2400:      # jr ra ; li $0,N
                    nums.append(b & 0xFFFF)
                elif a == 0 or (a >> 16) == 0x2400 and False:
                    break
                else:
                    break
                p += 8
            res["imports"].append({"lib": lib, "version": f"{(ver >> 8) & 0xFF}.{ver & 0xFF}", "funcs": nums})
    return res


def parse_ioprp(d):
    """ROMDIR: entradas de 16 bytes (nome[10], extinfo u16, tamanho u32); dados alinhados a 16."""
    out, o = [], 0
    while o + 16 <= len(d):
        n = d[o:o + 10]
        if n[:6] == b"RESET\0":
            pass
        if n[0] == 0:
            break
        out.append((cstr(n), struct.unpack_from("<H", d, o + 10)[0], struct.unpack_from("<I", d, o + 12)[0]))
        o += 16
    return out


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 1
    root = sys.argv[1]
    report = {}
    for p in sorted(glob.glob(os.path.join(root, "*.IRX"))):
        d = open(p, "rb").read()
        r = parse_irx(d)
        r["size"] = len(d)
        report[os.path.basename(p)] = r
        log.info("%s size=%d module=%s ver=%s exports=%s imports=%s", os.path.basename(p), len(d),
                 r["module"], r["version"], [(e["lib"], e["count"]) for e in r["exports"]],
                 [(i["lib"], len(i["funcs"])) for i in r["imports"]])
    ip = os.path.join(root, "IOPRP280.IMG")
    if os.path.exists(ip):
        d = open(ip, "rb").read()
        ents = parse_ioprp(d)
        # localizar offsets: dados comecam apos a tabela, cada arquivo alinhado a 16
        # extinfo fica em entrada EXTINFO; os dados reais seguem a tabela+extinfo
        mods = []
        sizes = {n: s for n, _, s in ents}
        pos = 0  # RESET(0) -> ROMDIR -> EXTINFO -> modulos, tudo alinhado a 16
        for n, ei, s in ents:
            if n in ("RESET", "ROMDIR", "EXTINFO"):
                # RESET tem tamanho 0; ROMDIR/EXTINFO ocupam espaco no inicio
                pos += (s + 15) & ~15
                continue
            blob = d[pos:pos + s]
            info = None
            if blob[:4] == b"\x7fELF":
                try:
                    info = parse_irx(blob)
                except Exception as e:  # noqa
                    log.warning("falha %s: %s", n, e)
            mods.append({"entry": n, "size": s, "module": info and info["module"],
                         "version": info and info["version"],
                         "exports": info and [(e["lib"], e["count"]) for e in info["exports"]],
                         "imports": info and [(i["lib"], len(i["funcs"])) for i in info["imports"]]})
            pos += (s + 15) & ~15
        report["IOPRP280.IMG"] = {"size": len(d), "entries": mods}
        for m in mods:
            log.info("IOPRP %s", m)
    if "--json" in sys.argv:
        out = sys.argv[sys.argv.index("--json") + 1]
        json.dump(report, open(out, "w"), indent=1)
        log.info("json em %s", out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
