#!/usr/bin/env python3
"""Verify tamper-evident proof chain from Aether serial JSON or SD export lines."""

from __future__ import annotations

import argparse
import json
import sys

FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x00000100000001B3


def mix(h: int, word: int) -> int:
    h ^= word & 0xFFFFFFFF
    h = (h * FNV_PRIME) & 0xFFFFFFFFFFFFFFFF
    return h


def mix_u8(h: int, byte: int) -> int:
    h ^= byte & 0xFF
    h = (h * FNV_PRIME) & 0xFFFFFFFFFFFFFFFF
    return h


def chain_proof(
    prev: int,
    guest: int,
    pressure_bits: int,
    dose: int,
    vector: int,
    cycle: int,
    mission_id: int,
    payload_slot: int,
) -> int:
    h = FNV_OFFSET
    h = mix(h, (prev >> 32) & 0xFFFFFFFF)
    h = mix(h, prev & 0xFFFFFFFF)
    h = mix(h, guest & 0xFFFFFFFF)
    h = mix(h, pressure_bits & 0xFFFFFFFF)
    h = mix(h, dose & 0xFFFFFFFF)
    h = mix_u8(h, vector & 0xFF)
    h = mix(h, cycle & 0xFFFFFFFF)
    h = mix(h, mission_id & 0xFFFFFFFF)
    h = mix_u8(h, payload_slot & 0xFF)
    return h


def parse_hex64(s: str) -> int:
    s = s.strip().lower()
    if s.startswith("0x"):
        s = s[2:]
    return int(s, 16) & 0xFFFFFFFFFFFFFFFF


def parse_sd_line(line: str) -> dict | None:
    line = line.strip()
    if not line or line.startswith("#"):
        return None
    if line.startswith("{"):
        return json.loads(line)
    out: dict[str, object] = {}
    for part in line.split():
        if "=" not in part:
            continue
        k, v = part.split("=", 1)
        out[k] = v
    return out


def verify_record(rec: dict, prev_proof: int, payload_slot: int = 0) -> tuple[bool, int]:
    cycle = int(rec["cycle"])
    guest = int(rec["guest"])
    proof = parse_hex64(str(rec["proof"]))
    vector = int(str(rec["vector"]), 16) if "vector" in rec else int(rec.get("vector", 0))
    pressure = float(rec.get("pressure", rec.get("P", 0)))
    dose = int(rec.get("dose", rec.get("D", 0)))
    mission_id = int(rec.get("mission_id", rec.get("mission", 0)))
    if "payload" in rec:
        payload_slot = 1 if str(rec["payload"]).upper().startswith("RELAX") else 0
    elif "payload_slot" in rec:
        payload_slot = int(rec["payload_slot"])

    import struct

    pressure_bits = struct.unpack("<I", struct.pack("<f", pressure))[0]

    expected = chain_proof(
        prev_proof, guest, pressure_bits, dose, vector, cycle, mission_id, payload_slot
    )
    return expected == proof, proof


def main() -> None:
    parser = argparse.ArgumentParser(description="Verify Aether proof chain from log input")
    parser.add_argument(
        "files",
        nargs="*",
        help="log files (JSON lines or sd_export text); stdin if omitted",
    )
    args = parser.parse_args()

    streams = [open(f, encoding="utf-8", errors="replace") for f in args.files]
    if not streams:
        streams = [sys.stdin]

    prev = 0
    ok = 0
    fail = 0
    for fh in streams:
        for raw in fh:
            rec = parse_sd_line(raw)
            if not rec or "proof" not in rec:
                continue
            if "prev_proof" in rec:
                prev = parse_hex64(str(rec["prev_proof"]))
            good, proof = verify_record(rec, prev, 0)
            if good:
                ok += 1
                print(f"OK cycle {rec.get('cycle', '?')} proof=0x{proof:016X}")
            else:
                fail += 1
                print(f"FAIL cycle {rec.get('cycle', '?')}", file=sys.stderr)
            prev = proof

    for fh in streams:
        if fh is not sys.stdin:
            fh.close()

    if ok == 0 and fail == 0:
        print("no records found", file=sys.stderr)
        raise SystemExit(2)
    raise SystemExit(1 if fail else 0)


if __name__ == "__main__":
    main()
