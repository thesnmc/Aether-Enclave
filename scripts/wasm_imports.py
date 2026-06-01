#!/usr/bin/env python3
"""Dump (module, kind, name) from a .wasm import section."""
import sys
from pathlib import Path


def leb128(data: bytes, pos: int) -> tuple[int, int]:
    result, shift = 0, 0
    while True:
        b = data[pos]
        pos += 1
        result |= (b & 0x7F) << shift
        shift += 7
        if b < 0x80:
            return result, pos


def read_name(data: bytes, pos: int) -> tuple[str, int]:
    n, pos = leb128(data, pos)
    s = data[pos : pos + n].decode()
    return s, pos + n


def main() -> None:
    path = Path(sys.argv[1])
    data = path.read_bytes()
    assert data[:4] == b"\x00asm"
    pos = 8
    while pos < len(data):
        sid = data[pos]
        pos += 1
        size, pos = leb128(data, pos)
        sec = data[pos : pos + size]
        pos += size
        if sid != 2:
            continue
        p = 0
        count, p = leb128(sec, p)
        for _ in range(count):
            mod, p = read_name(sec, p)
            kind = sec[p]
            p += 1
            field, p = read_name(sec, p)
            if kind == 0:
                _, p = leb128(sec, p)
                print(f"{mod}::func::{field}")
            elif kind == 1:
                flags = sec[p]
                p += 1
                _, p = leb128(sec, p)
                if flags & 1:
                    _, p = leb128(sec, p)
                print(f"{mod}::memory::{field}")
            elif kind == 2:
                _, p = leb128(sec, p)
                _, p = leb128(sec, p)
                print(f"{mod}::table::{field}")
            elif kind == 3:
                _, p = leb128(sec, p)
                print(f"{mod}::global::{field}")
        return
    print("(no import section)", file=sys.stderr)


if __name__ == "__main__":
    main()
