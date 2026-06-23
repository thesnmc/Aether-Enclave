#!/usr/bin/env python3
"""Decrypt Aether uplink dry-run hex from serial (dev PSK only)."""

from __future__ import annotations

import argparse
import struct
import sys

from cryptography.hazmat.primitives.ciphers.aead import AESGCM

DEV_PSK = bytes(
    [
        0xAE,
        0x7E,
        0x1C,
        0x4A,
        0xD0,
        0x00,
        0x01,
        0x02,
        0x03,
        0x04,
        0x05,
        0x06,
        0x07,
        0x08,
        0x09,
        0x0A,
        0x0B,
        0x0C,
        0x0D,
        0x0E,
        0x0F,
        0x10,
        0x11,
        0x12,
        0x13,
        0x14,
        0x15,
        0x16,
        0x17,
        0x18,
        0x19,
        0x1A,
    ]
)


def decrypt_frame(hex_str: str) -> dict:
    raw = bytes.fromhex(hex_str.strip())
    if len(raw) != 45:
        raise ValueError(f"expected 45 bytes, got {len(raw)}")
    nonce, ct_tag = raw[:12], raw[12:]
    plain = AESGCM(DEV_PSK).decrypt(nonce, ct_tag, None)
    if len(plain) != 17:
        raise ValueError(f"unexpected plaintext len {len(plain)}")
    mission_id, cycle = struct.unpack_from("<II", plain, 0)
    flags = plain[8]
    proof = struct.unpack_from("<Q", plain, 9)[0]
    return {
        "mission_id": mission_id,
        "cycle": cycle,
        "flags": flags,
        "proof": f"0x{proof:016X}",
    }


def main() -> None:
    p = argparse.ArgumentParser(description="Decrypt Aether uplink dry-run hex line")
    p.add_argument("hex", help="45-byte sealed frame as hex (90 chars)")
    args = p.parse_args()
    try:
        out = decrypt_frame(args.hex)
    except Exception as exc:  # noqa: BLE001
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(2) from exc
    print(out)


if __name__ == "__main__":
    main()
