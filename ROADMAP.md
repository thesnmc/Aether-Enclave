# Aether Enclave — iDEX Open plan (up to ₹1.5 crore)

Planning document for an **iDEX Open Challenge** application: breadboard prototype today → custom PCB → small batch of field units evaluators can test with a written procedure.

**Team:** two people — founder (firmware, product, milestones) and one ECE graduate (bring-up, test, EMS liaison). Grant pays **salaries and vendors** (PCB fab, assembly); no other hires planned.

**Today:** working ESP32-C6 firmware, BMP390 + ADS1115 on I2C, OLED, optional microSD log, USB flash/debug, QEMU regression build.

*(L = lakh = ₹100,000; Cr = crore = ₹10,000,000)*

---

## What we are building

A **low-duty-cycle sensor node** that wakes on a schedule or event, runs an isolated WASM check, logs a chained proof hash, and clears RAM before sleeping again.

| Part | Role today | Phase 2+ |
|------|------------|----------|
| ESP32-C6 | Bare-metal host, deep sleep | Same chip on PCB |
| BMP390 | Pressure (atm) | Production sensor port |
| ADS1115 + pot | Demo “dose” channel | Real front-end on AIN0 |
| OLED | Operator readout at demo | Same on PCB |
| microSD | Offline proof + mission profile | Socket on every unit |
| WASM payload | Strict / relaxed slots in flash | OTA or SD swap TBD |

One USB cable for power and logs. **No Wi‑Fi in this firmware** — any radio would be a separate design review.

**Honest scope:** this grant window targets **evaluator-ready prototypes** (bay / shelter / lab bench), not satellite qualification or flight clearance. Those need different parts, test houses, and a named Service customer.

---

## Problem statement (draft for application)

> Low-power node that wakes on pressure or timer, runs a sandboxed health check, appends a hash-linked log each cycle, and does not keep mission data in RAM between runs.

---

## Why iDEX Open fits

| They ask for | We have / plan |
|--------------|----------------|
| Indian R&D | Rust + RISC-V ESP32-C6, built in India |
| Defence relevance | Platform bay / shelter monitoring; radiation path on roadmap |
| Prototype → product | Breadboard → PCB → 100 boxed units |
| Small team | Founder + one engineer; salaries in budget |
| Milestone proof | Git tags, SD logs, demo video, evaluator test sheet |

We commit to **hardware + docs evaluators can run in 24 months**. Full platform sign-off is **out of scope** unless a Service lab agrees a test plan later.

---

## Team salaries (24 months)

Hardware for the table demo is **under ₹10,000** parts; **no booth rental** in the budget — USB power and a laptop are enough.

| Person | Role | ₹/month | 24 months |
|--------|------|---------|-----------|
| Founder | Firmware, WASM host, PCB spec, iDEX reporting | 1.50 L | **36 L** |
| Engineer (ECE) | Wiring, sensors, SD/OLED, test scripts, EMS | 1.50 L | **36 L** |
| **Total** | | **3.00 L/mo** | **72 L** |

PCB layout and assembly are paid to vendors, not extra headcount.

---

## Budget (₹1.5 crore)

| Line | ₹ | Notes |
|------|---|--------|
| Founder salary (24 mo) | 36 L | |
| Engineer salary (24 mo) | 36 L | |
| Phase 1 — iDEX + PCB v1 | 18 L | Fab, parts, bench gear |
| Phase 2 — 25 evaluator units | 22 L | PCB v2, travel, spares |
| Phase 3 — Product pack | 16 L | Enclosure, docs |
| Phase 4 — 100-unit EMS | 12 L | Batch + flash jig |
| Travel, compliance, IP | 10 L | Reviews, LLP, reporting |
| **Total** | **150 L** | |

**Phase 0 (self-funded parts ~₹3–4k):** breadboard demo working in repo.

---

## Timeline

| Phase | Months | Deliverable |
|-------|--------|-------------|
| 0 — Done | — | Breadboard firmware + repo + evaluator test sheet |
| 1 | 1–6 | iDEX submission, PCB v1, reliability log |
| 2 | 7–14 | 25 units to evaluators, PCB v2 from feedback |
| 3 | 15–20 | Boxed unit, Hindi/English quick-start |
| 4 | 21–24 | 100 EMS units, serial numbers |

---

## Phase 0 — Done

**Parts (~₹3,150 core):** WeAct ESP32-C6-A-N4, BMP390L, ADS1115, I2C OLED, SD module, card, breadboard, pot, button, USB cable.

**Firmware shipped in repo:**

- [x] WASM host, RAM wipe each cycle  
- [x] Chained proof hash (serial + RTC + verify script)  
- [x] Strict + relaxed WASM slots (flash, no host reflash to swap)  
- [x] Mission profile on SD sector 2047  
- [x] OLED boot + per-cycle display  
- [x] SD proof log sectors 2048+  
- [x] Demo mode, pressure / leak wake, JSON serial  
- [x] Power timing on serial (active ms + sleep interval)  
- [x] `tools/sd_export.py`, `verify_log.py`, `write_mission_profile.py`  
- [x] [EVALUATOR_TEST.md](EVALUATOR_TEST.md)  
- [x] QEMU x86 bench  

---

## Phase 1 — iDEX + PCB v1 (months 1–6, ₹18 L)

| Spend | ₹ | Outcome |
|-------|---|---------|
| Application + legal | 1 L | Submitted package |
| PCB v1 (50 boards, 4-layer) | 5 L | C6 + sensor footprints + SD + OLED |
| Parts, 15 assemblies | 4 L | Same pin map as breadboard |
| Bench tools | 4 L | DMM, logic analyser, reflow |
| Enclosure mock-ups | 1 L | Fit check |
| Re-spin buffer | 3 L | One fab recovery |

**Goals:**

- [ ] iDEX Open application submitted  
- [ ] PCB v1 runs **same firmware** as breadboard (pin-compatible)  
- [ ] 1000-cycle log on SD without watchdog reset  
- [ ] Demo video (wake → WASM → OLED → SD line)  
- [x] PC tools for SD export and proof verify  

---

## Phase 2 — Evaluator pilot (months 7–14, ₹22 L)

Ship **25 assembled boards** with [EVALUATOR_TEST.md](EVALUATOR_TEST.md) — wake reliability, proof chain match (serial vs SD), sleep current measurement on their meter.

| Spend | ₹ |
|-------|---|
| PCB v2 + bare boards | 5 L |
| Assembly + parts (25) | 7 L |
| Engineering dosimeter samples (5) | 3 L |
| Travel (DefExpo, reviews) | 4 L |
| Spares | 3 L |

**Goals:**

- [ ] 25 serial-numbered units  
- [ ] Written evaluator feedback (false wakes, log integrity)  
- [ ] PCB v2 from findings  

---

## Phase 3 — Product pack (months 15–20, ₹16 L)

| Spend | ₹ |
|-------|---|
| PCB v3 (30) | 5 L |
| Enclosure tooling + shells | 8 L |
| Hindi + English docs | 2 L |
| Supply review | 1 L |

**Goals:**

- [ ] Boxed kit: board, OLED, SD, quick-start, limitations sheet  
- [ ] AGPL source + BOM for audit  
- [ ] Temperature / mechanical limits documented to agreed test levels  

---

## Phase 4 — Small series (months 21–24, ₹12 L)

| Spend | ₹ |
|-------|---|
| 100× PCB at EMS | 8 L |
| Enclosure + kitting | 3 L |
| USB flash jig | 1 L |

**Goals:**

- [ ] 100 boxed units with serial numbers and SD card  
- [ ] Release tag passes evaluator script on sample units  

---

## Travel, compliance, IP (₹10 L)

Reviews, DefExpo / Aero India travel, LLP/GST, trademark, milestone reports, contingency.

---

## What we are not buying

- Extra hires beyond the two-person team  
- Classic ESP32 (we stay on C6 RISC-V)  
- Wi‑Fi / cloud uplink without a security review  
- Paid marketing or booth rental  
- Claims of flight or orbital qualification in this grant window  

---

## Success at ₹1.5 Cr (measurable)

1. **100 production units** with OLED + SD in every box.  
2. **25+ evaluator units** with written test results.  
3. **Custom PCB + enclosure** replacing breadboard.  
4. **Proof chain** verified on PC from SD export.  
5. **Two salaries** funded for 24 months of R&D.  
6. **Table demo** that works on every release tag.  

---

## Next actions

1. Solder breadboard when parts arrive — follow [README.md](README.md) wiring.  
2. Record demo video.  
3. Finalise iDEX problem statement and submit.  
4. Get two Indian PCB quotes for v1.  
5. Run 1000-cycle SD reliability test on breadboard.  

---

*Draft — update when sanction letter and vendor quotes are firm.*
