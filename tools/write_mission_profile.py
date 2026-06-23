#!/usr/bin/env python3
"""Write Aether mission profile to microSD sector 2047 (512 bytes, magic AEPR)."""

from __future__ import annotations

import argparse
import struct
import sys

MAGIC = b"AEPR"
VERSION = 1
SECTOR = 2047
SECTOR_SIZE = 512


def build_profile(
    mission_id: int,
    payload_slot: int,
    wake_min: int,
    wake_max: int,
    pressure_limit: float,
    dose_limit: int,
    leak_rate: float,
) -> bytes:
    buf = bytearray(SECTOR_SIZE)
    buf[0:4] = MAGIC
    buf[4] = VERSION
    buf[5] = min(payload_slot, 1)
    buf[6] = max(1, wake_min)
    buf[7] = min(120, max(buf[6], wake_max))
    struct.pack_into("<I", buf, 8, mission_id)
    struct.pack_into("<f", buf, 12, pressure_limit)
    struct.pack_into("<I", buf, 16, max(100, dose_limit))
    struct.pack_into("<f", buf, 20, leak_rate)
    return bytes(buf)


def main() -> None:
    p = argparse.ArgumentParser(description="Write Aether mission profile to SD sector 2047")
    p.add_argument("device", help=r"block device or image (e.g. \\.\PhysicalDrive3)")
    p.add_argument("--mission-id", type=int, default=1)
    p.add_argument("--payload", choices=("strict", "relaxed"), default="strict")
    p.add_argument("--wake-min", type=int, default=5)
    p.add_argument("--wake-max", type=int, default=60)
    p.add_argument("--pressure-limit", type=float, default=0.15)
    p.add_argument("--dose-limit", type=int, default=1000)
    p.add_argument("--leak-rate", type=float, default=0.003, help="atm/s leak wake threshold")
    args = p.parse_args()

    slot = 1 if args.payload == "relaxed" else 0
    if slot == 1 and args.pressure_limit == 0.15:
        args.pressure_limit = 0.10
    if slot == 1 and args.dose_limit == 1000:
        args.dose_limit = 2000

    blob = build_profile(
        args.mission_id,
        slot,
        args.wake_min,
        args.wake_max,
        args.pressure_limit,
        args.dose_limit,
        args.leak_rate,
    )

    try:
        with open(args.device, "r+b", buffering=0) as f:
            f.seek(SECTOR * SECTOR_SIZE)
            f.write(blob)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(2) from exc

    print(
        f"wrote sector {SECTOR}: mission={args.mission_id} payload={args.payload} "
        f"P<{args.pressure_limit} D>{args.dose_limit} leak={args.leak_rate}atm/s"
    )


if __name__ == "__main__":
    main()
