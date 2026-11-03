"""Leitura/extração de ISO9660 de PS2.

Biblioteca (usada pelo launcher) e CLI:
    python -I tools/extract_iso.py <iso> <destino> [nome ...]
Sem nomes: extrai SYSTEM.CNF e o ELF de BOOT2. Nome de diretório: extrai os arquivos dele.
Executar a partir do diretório do jogo; logs em ./logs/.
"""
import datetime
import hashlib
import logging
import os
import re
import struct
import sys

SEC = 2048
log = logging.getLogger("extract")


class IsoError(Exception):
    pass


def read_dir(f, lba, size):
    """Lista um diretório ISO9660: [(nome_sem_versao, lba, tamanho, eh_dir)]."""
    f.seek(lba * SEC)
    d = f.read(size)
    o, out = 0, []
    while o < size:
        rec_len = d[o]
        if rec_len == 0:
            o = (o // SEC + 1) * SEC
            continue
        rec_lba = struct.unpack_from("<I", d, o + 2)[0]
        rec_size = struct.unpack_from("<I", d, o + 10)[0]
        is_dir = bool(d[o + 25] & 2)
        name = d[o + 33:o + 33 + d[o + 32]].decode("ascii", "replace")
        if name not in ("\x00", "\x01"):
            out.append((name.split(";")[0], rec_lba, rec_size, is_dir))
        o += rec_len
    return out


class Iso:
    """Acesso somente leitura a uma imagem ISO9660 (setores de 2048 bytes)."""

    def __init__(self, path):
        self.path = path
        self.f = open(path, "rb")
        self.f.seek(16 * SEC)
        pvd = self.f.read(SEC)
        if pvd[1:6] != b"CD001":
            self.f.close()
            raise IsoError("nao e ISO9660: %s" % path)
        self.root = read_dir(self.f, struct.unpack_from("<I", pvd, 158)[0],
                             struct.unpack_from("<I", pvd, 166)[0])
        self.by_name = {e[0]: e for e in self.root}

    def close(self):
        self.f.close()

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()

    def entry(self, name):
        try:
            return self.by_name[name]
        except KeyError:
            raise IsoError("%s nao encontrado na raiz da ISO" % name)

    def listdir(self, name):
        _, lba, size, is_dir = self.entry(name)
        if not is_dir:
            raise IsoError("%s nao e um diretorio" % name)
        return read_dir(self.f, lba, size)

    def read(self, name):
        _, lba, size, is_dir = self.entry(name)
        if is_dir:
            raise IsoError("%s e um diretorio" % name)
        self.f.seek(lba * SEC)
        return self.f.read(size)

    def boot_info(self):
        """Retorna (nome do ELF de BOOT2, texto do SYSTEM.CNF)."""
        text = self.read("SYSTEM.CNF").decode("ascii", "replace")
        m = re.search(r"BOOT2\s*=\s*cdrom0:\\?([^;\s]+)", text)
        if not m:
            raise IsoError("BOOT2 ausente em SYSTEM.CNF")
        return m.group(1), text


def setup_logging(prefix="extract"):
    os.makedirs("logs", exist_ok=True)
    logging.basicConfig(
        level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s",
        handlers=[logging.FileHandler("logs/%s_%s.log" % (prefix, datetime.date.today())),
                  logging.StreamHandler()])


def _write(path, data):
    with open(path, "wb") as out:
        out.write(data)
    log.info("extraido %s (%d bytes) sha256=%s", path, len(data), hashlib.sha256(data).hexdigest())


def extract(iso_path, dest, wanted=()):
    os.makedirs(dest, exist_ok=True)
    with Iso(iso_path) as iso:
        if not wanted:
            boot, text = iso.boot_info()
            log.info("SYSTEM.CNF:\n%s", text)
            wanted = ["SYSTEM.CNF", boot]
        for name in wanted:
            if iso.entry(name)[3]:
                sub = os.path.join(dest, name)
                os.makedirs(sub, exist_ok=True)
                for en, elba, esz, edir in iso.listdir(name):
                    log.info("%s/%s lba=%d size=%d dir=%s", name, en, elba, esz, edir)
                    if not edir:
                        iso.f.seek(elba * SEC)
                        _write(os.path.join(sub, en), iso.f.read(esz))
            else:
                _write(os.path.join(dest, name), iso.read(name))


if __name__ == "__main__":
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    setup_logging()
    extract(sys.argv[1], sys.argv[2], sys.argv[3:])
