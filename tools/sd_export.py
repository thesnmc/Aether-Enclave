#!/usr/bin/env python3
"""Export Aether Enclave raw SDIO proof log from a microSD card or disk image.

Firmware stores append-only 512-byte sectors starting at LBA 2048 (no FAT).
On Windows use \\\\.\\PhysicalDriveN (admin); on Linux use /dev/sdX (unmount first).
"""

from __future__ import annotations

import argparse
import struct
import sys

LOG_META_SECTOR = 2048
LOG_FIRST_SECTOR = 2049
LOG_SECTOR_COUNT = 512
SECTOR_SIZE = 512


def read_sectors(path: str, start: int, count: int) -> bytes:
    with open(path, "rb") as f:
        f.seek(start * SECTOR_SIZE)
        data = f.read(count * SECTOR_SIZE)
    if len(data) < count * SECTOR_SIZE:
        raise OSError(f"short read at sector {start}: got {len(data)} bytes")
    return data


def export_log(path: str) -> int:
    meta = read_sectors(path, LOG_META_SECTOR, 1)
    if meta[0:4] != b"AETH":
        print("No Aether log at sector 2048 (header AETH missing)", file=sys.stderr)
        return 1

    next_sector = struct.unpack_from("<I", meta, 4)[0]
    total = struct.unpack_from("<I", meta, 8)[0]
    print(f"# Aether Enclave SD log — {total} cycles (next write sector {next_sector})")

    blob = read_sectors(path, LOG_FIRST_SECTOR, LOG_SECTOR_COUNT)
    found = 0
    for i in range(LOG_SECTOR_COUNT):
        sector = blob[i * SECTOR_SIZE : (i + 1) * SECTOR_SIZE]
        if sector[0:4] != b"AEC1":
            continue
        raw = sector[4:].split(b"\n", 1)[0].split(b"\x00", 1)[0]
        line = raw.decode("ascii", errors="replace").strip()
        if line:
            print(line)
            found += 1

    if found == 0:
        print("# (no AEC1 cycle sectors yet)", file=sys.stderr)
    return 0


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Dump raw proof sectors from an Aether field microSD card"
    )
    parser.add_argument(
        "path",
        help="block device or image (e.g. \\\\.\\PhysicalDrive3 or /dev/sdb)",
    )
    args = parser.parse_args()
    try:
        raise SystemExit(export_log(args.path))
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(2) from exc


if __name__ == "__main__":
    main()
