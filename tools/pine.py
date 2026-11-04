"""Cliente minimo do PINE (IPC do PCSX2): le a memoria do EE de um emulador em execucao.

Uso:
  python -I tools/pine.py status
  python -I tools/pine.py read 0x38A4D8 0x38A3DC              # le palavras de 32 bits
  python -I tools/pine.py watch 2 0x38A4D8 0x38A3DC --log L   # imprime so quando algo muda, a cada 2 s
  python -I tools/pine.py list 0x38958C 0x14 64               # percorre lista encadeada (cabeca, offset do next, limite)

Somente leitura de valores pontuais para diagnostico; nada e gravado no emulador.
Protocolo: [u32 tamanho total][opcode][args]; resposta: [u32 tamanho][u8 codigo][dados]. Codigo 0 = ok.
"""
from __future__ import annotations

import argparse
import socket
import struct
import time
from datetime import datetime, timezone
from pathlib import Path

OP_READ32 = 2
OP_STATUS = 15
OP_TITLE = 11
STATUS_NAMES = {0: "running", 1: "paused", 2: "shutdown"}


class Pine:
    def __init__(self, port: int) -> None:
        self.sock = socket.create_connection(("127.0.0.1", port), timeout=5)

    def request(self, payload: bytes) -> bytes:
        self.sock.sendall(struct.pack("<I", len(payload) + 4) + payload)
        header = self._recv(4)
        size = struct.unpack("<I", header)[0]
        body = self._recv(size - 4)
        if body[0] != 0:
            raise RuntimeError(f"PINE erro {body[0]}")
        return body[1:]

    def _recv(self, count: int) -> bytes:
        data = b""
        while len(data) < count:
            chunk = self.sock.recv(count - len(data))
            if not chunk:
                raise ConnectionError("PINE fechou a conexao")
            data += chunk
        return data

    def read32(self, address: int) -> int:
        return struct.unpack("<I", self.request(struct.pack("<BI", OP_READ32, address)))[0]

    def status(self) -> str:
        return STATUS_NAMES.get(struct.unpack("<I", self.request(bytes([OP_STATUS])))[0], "?")


def emit(line: str, log: Path | None) -> None:
    stamped = f"{datetime.now(timezone.utc).isoformat(timespec='seconds')} INFO {line}"
    print(stamped, flush=True)
    if log:
        log.parent.mkdir(parents=True, exist_ok=True)
        with log.open("a", encoding="utf-8") as handle:
            handle.write(stamped + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("command", choices=["status", "read", "watch", "list"])
    parser.add_argument("args", nargs="*")
    parser.add_argument("--port", type=int, default=28011)
    parser.add_argument("--log", type=Path)
    parser.add_argument("--seconds", type=float, default=0.0, help="watch: encerra depois deste tempo (0 = sem limite)")
    options = parser.parse_args()

    pine = Pine(options.port)
    if options.command == "status":
        emit(f"status={pine.status()}", options.log)
    elif options.command == "read":
        emit(" ".join(f"{int(a, 16):#x}={pine.read32(int(a, 16)):#x}" for a in options.args), options.log)
    elif options.command == "watch":
        period, addresses = float(options.args[0]), [int(a, 16) for a in options.args[1:]]
        previous, start = None, time.monotonic()
        while not options.seconds or time.monotonic() - start < options.seconds:
            values = [pine.read32(a) for a in addresses]
            if values != previous:
                emit(f"t={time.monotonic() - start:.0f}s " + " ".join(f"{a:#x}={v:#x}" for a, v in zip(addresses, values)), options.log)
                previous = values
            time.sleep(period)
    elif options.command == "list":
        node, next_offset, limit = int(options.args[0], 16), int(options.args[1], 16), int(options.args[2])
        node = pine.read32(node)
        for _ in range(limit):
            if node == 0:
                break
            emit(f"node={node:#x} next={pine.read32(node + next_offset):#x} state={pine.read32(node):#x} vt={pine.read32(node + 0x120):#x}", options.log)
            node = pine.read32(node + next_offset)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
