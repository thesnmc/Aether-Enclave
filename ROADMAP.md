# Aether Enclave — Product Roadmap

**Company:** The SNMC · **Product:** Commercial custody witness for sealed logistics

---

## Vision

Default **offline custody receipt** for sealed shipments — alert at the box, verify at the checkpoint, no vendor cloud.

---

## Phases

| Phase | Timeline | Deliverable |
|-------|----------|-------------|
| **0 — Now** | Done | Reference firmware, tools, breadboard demo |
| **1** | 0–6 mo | PCB v1, enclosure mock, pilot kit BOM |
| **2** | 6–12 mo | 10–25 pilot units, customer feedback, PCB v2 |
| **3** | 12–18 mo | Boxed **AE-CM1** product, EN/HI quick-start |
| **4** | 18–24 mo | EMS batch, reed/lid sensor, optional NFC export |
| **5** | 24+ mo | **AE-TG1** disposable tag feasibility |

---

## Phase 0 — Complete

- Event-only witness runtime (ESP32-C6)  
- WASM policy, hash-linked receipts, `verify_log.py`  
- Latched breach alert (GPIO10, OLED, GPIO2 ACK)  
- BMP390 pressure path + optional dose demo channel  
- [PILOT_TEST.md](PILOT_TEST.md) · [COMMERCIAL_PITCH.md](COMMERCIAL_PITCH.md)

---

## Phase 1 — PCB module (months 0–6)

| Goal | Outcome |
|------|---------|
| PCB v1 | Same pinout as breadboard |
| Reed / Hall on GPIO | Lid-open = primary violation |
| 1000-cycle reliability log | Soak + verify annex |
| Demo video | Sealed crate → verify tamper fail |
| 2 fab quotes | Cost @ 100 / 1000 units |

---

## Phase 2 — Pilots (months 6–12)

| Goal | Outcome |
|------|---------|
| 10–25 pilot units | Logistics contractor or bonded store |
| Written feedback | False wakes, verify workflow |
| PCB v2 | From pilot findings |
| Target COGS | Path to ₹3–6k @ 1k units |

---

## Phase 3 — Product (months 12–18)

- Boxed **AE-CM1** kit: module, SD, USB, quick-start, limitations sheet  
- AGPL source + BOM for audit  
- Optional violation-only uplink (radio validated)

---

## Phase 4 — Scale (months 18–24)

- EMS batch (100+ units)  
- Flash jig + serial numbers  
- OEM licence discussions (crate / lock integrators)

---

## What we are not building

- Cloud dashboard SaaS (verify stays local-first)  
- GPS fleet tracker replacement  
- Port container e-seal clone as lead product  
- Certified NBC / dosimeter on reference hardware

---

## Success metrics

1. **1 paid pilot** or LOI from logistics / warehouse customer  
2. **AE-CM1** boxed SKU with documented COGS  
3. **verify_log.py** used in customer audit workflow  
4. Repeat order or OEM integration letter

---

**The SNMC** · 2026
