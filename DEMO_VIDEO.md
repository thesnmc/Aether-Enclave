# 5-Minute iDEX Demo Video Script

Record this **exact flow** for the application annex. One take is fine; audio optional if serial is on screen.

**Props:** breadboard kit, **clear plastic box** (sealed compartment), laptop with serial + `verify_log.py`, microSD.

---

## 0:00 — Title card (5 s)

**On screen:** Aether Enclave — Sealed Compartment Witness | The SNMC | Indigenous RISC-V

---

## 0:05 — Problem (20 s)

**Voice or text overlay:**

> CCTV cannot measure pressure inside a sealed NBC tent or crate. We witness compartment integrity offline — tamper-linked proof, no cloud.

---

## 0:25 — Hardware (20 s)

- Show ESP32-C6 + BMP390 inside **clear box**
- Point to INT wire GPIO1, SD card, USB cable
- Text: *ESP32-C6 = reference board only. Product = custom PCB.*

---

## 0:45 — Boot (30 s)

- Power on (button **not** held)
- Serial: `mode=EVENT_ONLY`, `baseline — P=...`, `WITNESS READY`
- OLED: Aether Enclave boot animation

---

## 1:15 — Event (45 s)

- Close box lid lightly sealed (tape optional)
- **Open lid / squeeze / blow on BMP390** → pressure event
- Serial: `event — pressure drop` (or rise), `running WASM cycle`, JSON `proof` line
- OLED: cycle + proof hash (or **ALERT** if policy fails)

---

## 2:00 — Breach latch (45 s) — **ALERT MOMENT**

- Force policy fail (pressure or pot dose demo)
- OLED: inverted **ALERT**; serial: `BREACH`
- Device sleeps; **GPIO10 stays ON**
- Wake → `ALERT ACTIVE` → press GPIO2 → `BREACH ACK`, LED off

---

## 2:45 — Sleep (15 s)

- Shutdown animation → `entering deep sleep (button or BMP390 INT on change)`
- Text: *CPU off ~µA. No log until next real event.*

---

## 3:00 — Tamper proof (90 s) — **WOW MOMENT**

1. Wake again (button or event); run 2–3 cycles for chain
2. Remove SD → laptop → `python tools/sd_export.py ...`
3. `python tools/verify_log.py export.txt` → **ALL OK**
4. Open export in editor → **change one hex character** → save
5. `python tools/verify_log.py export.txt` → **CHAIN FAIL**
6. Text: *Custody proof. Not a camera.*

---

## 3:45 — Complements CCTV (20 s)

| CCTV | Aether |
|------|--------|
| Who was there | Pressure inside seal |
| Needs NVR/network | Air-gapped SD |

---

## 4:05 — Roadmap (30 s)

- PCB v1 → local breach alert → 25 evaluator kits → 100 boxed units
- Phase 2: qualified dose front-end
- Radio: optional encrypted burst when site policy allows

---

## 4:35 — Close (25 s)

**Text:** iDEX Open | The SNMC | GitHub: Aether-Enclave | AGPL audit

---

## Checklist before recording

- [ ] `cargo +esp build --release` flashed
- [ ] BMP390 INT → GPIO1 wired
- [ ] SD logs OK
- [ ] `verify_log.py` tested on PC
- [ ] Box big enough for board + sensor
- [ ] GPIO10 LED wired (330 Ω → GND)
- [ ] Breach latch + GPIO2 ACK rehearsed

---

## Failures to avoid on camera

- Demo mode (GPIO2 held at boot) — use **normal event-only boot**
- Claiming satellite / flight cert
- Showing Wi‑Fi (there is none)
