# Aether Enclave — Evaluator Test Procedure

**Product:** Sealed compartment integrity witness (NBC envelope / crate / bay mock-up).  
**Time:** ~30 minutes. **Pass:** all tests below + tamper chain fail demo.

Hardware: WeAct ESP32-C6, BMP390L (**INT → GPIO1**), ADS1115, I2C OLED, microSD (GPIO3/4/5/15).  
**Recommended:** place BMP390 inside a **clear plastic box** to simulate sealed compartment.

---

## Setup

1. Wire 3.3V/GND to all modules; I2C SDA→GPIO6, SCL→GPIO7.
2. BMP390 **INT** (top header) → **GPIO1**.
3. Wake button: GPIO2 → **3.3V** (not GND).
4. Optional review button: GPIO9 → **GND** (internal pull-up; scrolls OLED event log).
5. Pot wiper → ADS1115 AIN0.
6. Alert LED: GPIO10 → 330 Ω → LED → **GND** (breach demo).
7. Flash: `cd enclave_kernel && cargo +esp run --release`.
8. Open USB serial (native JTAG). Expect `mode=EVENT_ONLY`.

---

## Test 1 — Sealed compartment event (core demo)

| Step | Action | Pass |
|------|--------|------|
| 1 | Power on, button open | `WITNESS READY`, `baseline — P=...` |
| 2 | Close box; wait 10 s | No log lines (event-only) |
| 3 | Open lid / blow on BMP390 | `event — pressure...`, WASM cycle, JSON `proof` |
| 4 | OLED | Cycle + hash on event |

**Pass criteria:** No SD/serial **cycle log** until physical event.

---

## Test 1b — OLED event browser (GPIO9)

| Step | Action | Pass |
|------|--------|------|
| 1 | After Test 1 step 3 (≥1 logged event) | OLED shows cycle summary |
| 2 | Press **GPIO9** review button | OLED shows `EVENT LOG` / page 1/N |
| 3 | Press GPIO9 again | Scrolls to next stored event (up to last 4) |
| 4 | Release GPIO9 ~45 s | Browser exits; device returns toward sleep |

**Pass criteria:** Operator can read recent events on OLED; SD export + `verify_log.py` unchanged (audit truth).

---

## Test 1c — Breach latch + acknowledge (GPIO10 / GPIO2)

| Step | Action | Pass |
|------|--------|------|
| 1 | Force policy fail (open box until `CHAP LOW` or turn pot for `DOSE DEMO HI`) | OLED **ALERT**, serial `BREACH` |
| 2 | Wait for deep sleep | **GPIO10 LED stays ON** |
| 3 | Wake device (button or INT) | OLED `ALERT ACTIVE`, serial `BREACH latched` |
| 4 | Press **GPIO2** wake button | Serial `BREACH ACK`, **GPIO10 OFF** |

**Pass criteria:** Operator can see breach **after sleep** without reading SD; ACK clears latch.

---

## Test 2 — Tamper-evident proof chain (iDEX wow)

| Step | Action | Pass |
|------|--------|------|
| 1 | Run 3+ events (button or pressure) | JSON lines with changing `proof` |
| 2 | Export SD | `python tools/sd_export.py \\.\PhysicalDriveN` |
| 3 | Verify | `python tools/verify_log.py export.txt` → **all OK** |
| 4 | Edit **one character** in export file | Save |
| 5 | Verify again | **CHAIN FAIL** on tampered line |

**Pass criteria:** Tamper is detected offline without vendor cloud.

---

## Test 3 — WASM + strict/relaxed policy

| Step | Action | Pass |
|------|--------|------|
| 1 | Pot low, reboot, trigger event | `payload=STRICT` |
| 2 | Pot **>75%**, reboot, trigger event | `payload=RELAXED`, looser limits |
| 3 | Optional SD | `write_mission_profile.py ... --payload relaxed` |

---

## Test 4 — Wake sources

| Step | Action | Pass |
|------|--------|------|
| 1 | Sleep; press button | Full witness cycle |
| 2 | BMP390 INT event | `BMP390_INT` in wake cause when wired |
| 3 | Optional interval | SD `--interval-wake` or pot &lt;10%; timer wake logs on schedule |

---

## Test 5 — Demo mode (booth only)

| Step | Action | Pass |
|------|--------|------|
| 1 | Hold GPIO2 at power-on | `DEMO MODE` every ~2 s |

*Not used for field deployment test.*

---

## Test 6 — microSD layout

| Sector | Content |
|--------|---------|
| 2047 | Mission profile `AEPR` (v2: radio, interval flags) |
| 2048+ | Cycle records `AEC1` |

```bash
python tools/write_mission_profile.py \\.\PhysicalDriveN --mission-id 1
python tools/sd_export.py \\.\PhysicalDriveN
python tools/verify_log.py export.txt
```

---

## Test 7 — Air gap

| Step | Action | Pass |
|------|--------|------|
| 1 | Disconnect Wi‑Fi / no router needed | Device functions USB+SD only |
| 2 | Serial | `radio=OFF` unless profile enables dry-run |

---

## Test 8 — Power (document on your meter)

| Step | Action | Pass |
|------|--------|------|
| 1 | After event | `power — active_last=...ms` on serial |
| 2 | Deep sleep | Target **~10–30 µA** class on ESP32-C6 (confirm on PCB) |

---

## Fail criteria

- Panic or WDT reset over 10 consecutive events.
- `verify_log.py` fails on **unmodified** export.
- Event-only mode logs continuously with sealed box closed (false wake storm).

---

## What this is / is not

| Is | Is not |
|----|--------|
| Sealed-volume witness + audit log | CCTV replacement |
| Reference ESP32-C6 POC | Flight-certified product |
| Complements NBC / logistics QA | Live cloud dashboard |

Full application: [IDEX_APPLICATION.md](IDEX_APPLICATION.md). Video script: [DEMO_VIDEO.md](DEMO_VIDEO.md).

---

## Tools

| Tool | Purpose |
|------|---------|
| `tools/sd_export.py` | Raw SD dump |
| `tools/verify_log.py` | Proof chain + tamper detect |
| `tools/write_mission_profile.py` | Profile sector 2047 |
| `tools/decrypt_uplink.py` | Optional radio dry-run decrypt |
