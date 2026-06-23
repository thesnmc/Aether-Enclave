# Aether Enclave — Capability Limits

**Read this before demo, customer pilot, or sales handoff.**  
This device is a **Phase 0 witness POC** on reference hardware. It is **not** a certified NBC monitor, dosimeter, or cryptographic HSM.

---

## What this product does

| Capability | Detail |
|------------|--------|
| **Pressure witness** | BMP390 inside a sealed volume; wakes on measurable pressure change (~0.015 atm default) |
| **Policy check** | WASM guest compares pressure/dose against mission limits |
| **Local breach signal** | On policy fail: OLED **ALERT**, GPIO10 **solid ON**, state **latched in RTC** until GPIO2 ACK |
| **Audit log** | Hash-linked records on microSD + USB serial JSON |
| **Log integrity check** | `verify_log.py` detects edits to exported log files |
| **Low duty cycle** | Deep sleep between events; event-only default |
| **Air-gap default** | No Wi‑Fi; no cloud |

---

## What this product does not do

| Limit | Honest statement |
|-------|------------------|
| **NBC / hazmat certification** | Not qualified for operational sign-off. Pilot / evaluation use only. |
| **Guaranteed seal breach detection** | Pressure is a **proxy**. Slow leaks, temperature drift, and partial openings may not trip thresholds. |
| **Certified dosimetry** | Breadboard **dose channel = potentiometer demo**. Phase 2 = qualified analog front-end with named sensor. |
| **Anti-implant security** | No secure boot, no signed firmware, no HSM. A motivated attacker with physical access can reflash or swap SD. |
| **Cryptographic proof chain** | Chain uses FNV-style hashing — **integrity on export**, not non-repudiation against a compromised device. |
| **Real-time remote alert** | Radio TX is **not enabled** in prototype. Optional encrypted uplink is Phase 2 with second board validation. |
| **Radiation-hardened / flight qualified** | Consumer MCU; bounded RAM wipe is hygiene, not rad-hard. |
| **CCTV replacement** | Complements perimeter surveillance; does not record video. |

---

## Alert vs log (operator vs auditor)

| Role | Need | How Aether addresses it |
|------|------|---------------------------|
| **Operator at the box** | Know **now** | GPIO10 latched + OLED ALERT until GPIO2 acknowledge |
| **Auditor later** | Prove **what happened** | SD export + `verify_log.py` |

A passive logger without local alert is **incomplete** for field use. This prototype implements **both**.

---

## Sensor and environment limits

- **BMP390** — consumer barometric sensor; not calibrated to a national pressure standard on breadboard.  
- **Temperature** — pressure inside a sealed box shifts with heat; mission limits must account for this.  
- **Placement** — sensor must sit **inside** the monitored volume; wiring through the seal is an installation problem (Phase 1 PCB + feedthrough).  
- **Battery** — sleep current depends on PCB layout; breadboard numbers are indicative only.

---

## Phase 0 vs product deliverables

| Item | Today (Phase 0) | Product (AE-CM1) |
|------|-----------------|------------------|
| MCU | ESP32-C6 module (reference) | Custom PCB, same firmware pinout |
| Violation sensor | BMP390 pressure (demo) | Reed / Hall lid switch + optional pressure |
| Dosimeter | Pot demo | Qualified front-end (sensor TBD per vertical) |
| Enclosure | Clear demo box | Field enclosure + limitations label |
| Radio | Software scaffold only | Validated burst when customer policy allows |
| Units | 1 breadboard | Pilot kits → boxed SKU |

---

## One sentence for sceptics

> *We sell an honest offline custody witness with local alert and export-verifiable logs — not certified hazmat equipment until a customer lab scopes qualification with us.*

---

**The SNMC** · Ship with every pilot kit · 2026
