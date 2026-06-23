# Aether Enclave — Evaluator Test Procedure

One-page checklist for iDEX / Service evaluators. Hardware: breadboard kit (WeAct ESP32-C6, BMP390L, ADS1115, I2C OLED, optional microSD on GPIO3/4/5/15).

## Setup

1. Wire 3.3V and GND to all modules; I2C SDA→GPIO6, SCL→GPIO7.
2. Button: GPIO2 → **3.3V** (not GND).
3. Pot wiper → ADS1115 AIN0; pot ends → 3.3V and GND.
4. Connect **native USB** data port; flash: `cd enclave_kernel && cargo +esp run --release`.
5. Open serial monitor at default USB JTAG speed.

## Test 1 — Cold boot self-test

| Step | Action | Pass criteria |
|------|--------|---------------|
| 1 | Power on (button open) | `MISSION READY` or `DEGRADED` with sensor lines |
| 2 | Read serial | `mission — id=... payload=STRICT` (or RELAXED if pot high) |
| 3 | OLED | `AETHER ENCLAVE` / `SENSOR THEEK` or fault line |

## Test 2 — WASM cycle + proof chain

| Step | Action | Pass criteria |
|------|--------|---------------|
| 1 | Let one cycle complete | JSON line with `proof` and `prev_proof` |
| 2 | Second wake (button or timer) | `chain=LINKED`, new `proof` ≠ previous |
| 3 | Verify offline | `python tools/verify_log.py serial_capture.txt` → all OK |

## Test 3 — Swappable payload (no host reflash)

| Step | Action | Pass criteria |
|------|--------|---------------|
| 1 | Turn pot **>75%**, reboot | Serial: `payload=RELAXED`, limits P_lim=0.10, D_lim=2000 |
| 2 | Turn pot low, reboot | `payload=STRICT` again |
| 3 | Optional SD profile | `python tools/write_mission_profile.py \\.\PhysicalDriveN --payload relaxed` then reboot → `(SD profile)` |

## Test 4 — Wake sources

| Step | Action | Pass criteria |
|------|--------|---------------|
| 1 | Sleep, press button | Wake `GPIO`, vector `0x20` |
| 2 | Wait for RTC timer | Wake `RTC_TIMER`, vector `0x21` |
| 3 | Blow on BMP390 quickly | `rapid leak rate` or `pressure drop` → vector `0x20` |

## Test 5 — Demo mode

| Step | Action | Pass criteria |
|------|--------|---------------|
| 1 | Hold GPIO2 at power-on | `DEMO MODE — cycles every 2 s` |
| 2 | Watch OLED + serial | Cycle count increments, proof chain links |

## Test 6 — microSD proof log

| Step | Action | Pass criteria |
|------|--------|---------------|
| 1 | Insert dedicated microSD | Boot: `SD: OK` |
| 2 | Run 3+ cycles | `SD — cycle #N logged` |
| 3 | Export on PC | `python tools/sd_export.py \\.\PhysicalDriveN` |
| 4 | Verify chain | `python tools/verify_log.py export.txt` |

## Test 7 — Power budget (document on your PCB)

| Step | Action | Pass criteria |
|------|--------|---------------|
| 1 | Read serial after cycle | `power — active_last=...ms sleep_next=...s` |
| 2 | Measure with multimeter | Deep sleep typically **~10–30 µA** class on C6 (confirm on your build) |
| 3 | Record | Active window ms × active current + sleep × sleep current |

## Fail criteria

- Panic or watchdog reset during 10 consecutive cycles.
- Proof chain verification fails on exported log.
- RAM wipe: no prior cycle strings in memory dump after sleep (host design intent).

## Files

| Tool | Purpose |
|------|---------|
| `tools/sd_export.py` | Dump raw SD sectors |
| `tools/verify_log.py` | Verify proof chain |
| `tools/write_mission_profile.py` | Write SD sector 2047 profile |
