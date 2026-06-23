# Aether Enclave — iDEX Open Roadmap (up to ₹1.5 crore)

Spend plan for an **iDEX Open Challenge** application: move from the working breadboard prototype to a **field-ready sensor node** that Service evaluators can test on a written scope.

**Team:** two people — founder (firmware, product, iDEX milestones) and one friend (ECE graduate, software + hardware bring-up). Grant covers **salaries and vendor work** (PCB fab, EMS); no extra hires.

**Current status:** ESP32-C6 firmware, WASM host, BMP390 + ADS1115 on I2C, SSD1306 OLED + microSD SPI module, USB flash/debug, QEMU bench.

*(L = lakh = ₹100,000; Cr = crore = ₹10,000,000)*

---

## What we are building

A **sleep-heavy diagnostic node** for unmanned / sheltered platforms:

| Piece | Role |
|-------|------|
| ESP32-C6 (RISC-V) | Bare-metal host, deep sleep, USB debug |
| BMP390 | Pressure (atm), altitude proxy |
| ADS1115 + pot | Dose channel (pot = dosimeter stand-in at demo) |
| SSD1306 OLED | Live status: cycle, flags, proof |
| microSD (SPI) | Append-only proof log every wake |
| WASM payload | Swappable mission logic without rewriting the Rust host |

One USB-C cable. No Wi-Fi. No cloud. Proof on **serial + OLED + SD**.

---

## iDEX Open fit

Problem statement for the application:

> *Low-power wake node that runs isolated health checks on pressure and radiation inputs, logs tamper-evident proof per cycle, and leaves no mission data in RAM between runs.*

| iDEX expectation | Our answer |
|------------------|------------|
| Indian innovation | Rust host + RISC-V ESP32-C6, built and tested in India |
| Defence use case | Platform bay / shelter monitor; pot → engineering front-end in Phase 2 |
| Prototype → product | Breadboard today → PCB → 100 boxed units |
| Lean team | Founder + ECE partner; grant pays person-months, not a hiring spree |
| Milestone reports | Git tags, SD log samples, demo video, evaluator units |

We deliver **evaluator-ready hardware and documentation** in 24 months. Full platform qualification follows a named Service customer — not promised in this grant window alone.

---

## Team and salaries (24 months)

The main cost today is **engineering time**. Hardware for the demo is **under ₹10,000**; there is **no booth rental** in the budget — DefExpo is walk-up with a breadboard and laptop.

| Person | Role | Monthly (grant) | 24 months |
|--------|------|---------------|-----------|
| **Founder** | Firmware, WASM host, iDEX milestones, PCB spec, expo demo | ₹1.50 L | **₹36 L** |
| **Friend (ECE + software)** | Bring-up, sensors, SD/OLED, test scripts, EMS liaison | ₹1.50 L | **₹36 L** |
| **Total personnel** | | **₹3.00 L/mo** | **₹72 L** |

No other employees. PCB layout and EMS assembly are vendor line items.

---

## Budget overview (₹1.5 crore)

| Line | Budget | Notes |
|------|--------|-------|
| **Founder salary** (24 × ₹1.50 L/mo) | **₹36 L** | Full-time product + firmware |
| **Friend salary** (24 × ₹1.50 L/mo) | **₹36 L** | Full-time hardware + software |
| Phase 1 — iDEX Open + PCB v1 | ₹18 L | Fab, parts, bench tools, prototypes |
| Phase 2 — Evaluator pilot (25 units) | ₹22 L | PCB v2, parts, travel, spares |
| Phase 3 — Product pack | ₹16 L | Enclosure tooling, docs, PCB v3 |
| Phase 4 — 100-unit EMS run | ₹12 L | Production batch + flash jig |
| Travel, compliance, IP, iDEX reporting | ₹10 L | Reviews, expos, LLP, trademark |
| **Total** | **₹150 L (₹1.5 Cr)** | |

**Phase 0 (done):** prototype hardware **under ₹10,000** out of pocket — no booth fees. Engineering hours not counted in that figure.

### Phase timeline

| Phase | Months | Goal |
|-------|--------|------|
| 0 — Done | — | Working breadboard demo + repo |
| 1 | 1–6 | iDEX Open submission, PCB v1, reliability run, SD export tool |
| 2 | 7–14 | 25 evaluator units, PCB v2, Service feedback |
| 3 | 15–20 | Boxed product, enclosure, Hindi/English docs |
| 4 | 21–24 | 100 EMS units, serial numbers, training kits |

---

## Phase 0 — Done (hardware under ₹10,000)

| Part | ~₹ |
|------|-----|
| ESP32-C6-DevKitC-1 | 900 |
| BMP390 breakout | 800 |
| ADS1115 breakout | 350 |
| SSD1306 128×64 I2C OLED | 250 |
| microSD SPI module | 150 |
| microSD card (dedicated log) | 200 |
| Breadboard, wires, button, 10 kΩ pot, USB-C | 500 |
| **Total parts** | **~₹3,150** |

Headroom under ₹10k covers spares, extra wire, and a second DevKit if needed. **No booth or stall cost** — demo fits on a table with USB power and serial.

**Delivered in firmware:**

- [x] WASM host, wipe-between-cycles, RTC proof chain  
- [x] OLED boot + per-cycle display  
- [x] microSD append-only proof sectors (GPIO3/4/5/15)  
- [x] Demo mode, pressure-drop wake, JSON serial line  
- [x] QEMU regression path  

---

## Phase 1 — iDEX Open + PCB v1 (months 1–6, ₹18 L)

| Item | Cost | Outcome |
|------|------|---------|
| iDEX Open application + legal | ₹1 L | Submitted package + company docs |
| PCB v1 fab (50 boards, 4-layer) | ₹5 L | ESP32-C6, sensors, OLED, SD socket |
| Parts for 15 assembled units | ₹4 L | OLED + microSD on every board |
| Bench tools (logic analyser, DMM, reflow) | ₹4 L | Bring-up and debug capability |
| Prototype enclosures (15) | ₹1 L | Pre-PCB mechanical check |
| Contingency / re-spin | ₹3 L | One fab recovery |

**Goals:**

- [x] microSD proof sectors in firmware  
- [ ] iDEX Open application submitted  
- [ ] PCB v1 runs same firmware as breadboard  
- [ ] SD export tool — proof readable on a laptop  
- [ ] 1000-cycle reliability run with SD log  
- [ ] Demo video for iDEX milestone  

---

## Phase 2 — Evaluator pilot (months 7–14, ₹22 L)

Ship **25 assembled units** to **iDEX-linked Service evaluators** against a one-page test scope: wake reliability, proof log integrity, sleep current, WASM payload swap.

| Item | Cost |
|------|------|
| PCB v2 + 40 bare boards | ₹5 L |
| Parts + assembly (25 units) | ₹7 L |
| Engineering dosimeter / pressure port (5 units) | ₹3 L |
| Travel (DefExpo, Aero India, iDEX reviews) | ₹4 L |
| Spares + RMA | ₹3 L |

**Goals:**

- [ ] 25 serial-numbered units with calibration card  
- [ ] Signed evaluator test scope  
- [ ] Written feedback — wake count, false wakes, SD vs serial proof match  
- [ ] PCB v2 from findings  

---

## Phase 3 — Product pack (months 15–20, ₹16 L)

| Item | Cost |
|------|------|
| PCB v3 (30 boards) | ₹5 L |
| Enclosure tooling + 30 shells | ₹8 L |
| Documentation (Hindi + English) | ₹2 L |
| Supply-chain review | ₹1 L |

**Goals:**

- [ ] Boxed unit: board + OLED + SD + quick-start sheet  
- [ ] Known-limitations sheet for evaluators  
- [ ] AGPL source pack + BOM for audit  
- [ ] Temperature and mechanical checks documented to agreed limits  

---

## Phase 4 — Small series (months 21–24, ₹12 L)

| Item | Cost |
|------|------|
| 100× PCB v3 at Indian EMS | ₹8 L |
| Enclosure + kitting (OLED, SD, card) | ₹3 L |
| USB flash jig | ₹1 L |

**Goals:**

- [ ] 100 boxed units with serial numbers  
- [ ] Each unit ships with SD card and readout instructions  
- [ ] DefExpo demo script passes on every release tag  

---

## Travel, compliance, IP (₹10 L)

| Item | Cost |
|------|------|
| Travel (iDEX, DefExpo, Aero India, evaluator meetings) | ₹4 L |
| LLP / GST / accounting | ₹1.5 L |
| Trademark + documentation | ₹1.5 L |
| iDEX milestone reporting | ₹1 L |
| Contingency | ₹2 L |

---

## Grant spend priorities

| Priority | Spend |
|----------|-------|
| Person-months (founder + friend) | ₹72 L |
| PCBs, parts, EMS, enclosures | ₹68 L |
| Travel, legal, IP, reporting | ₹10 L |

We do **not** budget for: extra headcount, classic ESP32 boards, Wi-Fi/cloud uplink without a security review, or paid marketing agencies.

---

## Success at ₹1.5 Cr

1. **100 production units** — OLED + microSD + proof log in every box.  
2. **25+ evaluator units** with written Service feedback.  
3. **Custom PCB + enclosure** — production shape, not breadboard.  
4. **SD proof export** — evaluator matches laptop log to serial hash.  
5. **Two full-time salaries** for 24 months of R&D.  
6. **Working expo demo** on every firmware release.  

---

## Next steps

1. DefExpo demo — breadboard with OLED + microSD (GPIO6/7 I2C, GPIO3/4/5/15 SPI).  
2. Record demo video: wake → WASM → OLED → SD line on serial.  
3. Draft iDEX Open problem statement (pressure + radiation wake node).  
4. PCB v1 quotes from two Indian fabs.  
5. SD export script for laptop proof readout.  

---

*iDEX Open planning draft — revise when sanction letter and EMS quotes are final.*
