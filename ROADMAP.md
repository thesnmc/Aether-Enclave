# Aether Enclave — iDEX Open Plan (₹1.5 Crore)

**Applicant:** The SNMC · **Duration:** 24 months · **Ask:** ₹1.5 crore (programme ceiling)

Breadboard prototype today → custom PCB → evaluator kits → 100 boxed units. Grant builds **The SNMC as a supplier**, not a one-off demo.

**Team:** solo founder (firmware, product, milestones) + **contract vendors** for PCB layout, assembly, and enclosure.

**Today:** working ESP32-C6 firmware, BMP390 + ADS1115 on I2C, OLED, optional microSD log, USB flash/debug, QEMU regression build.

*(L = lakh = ₹100,000; Cr = crore = ₹10,000,000)*

---

## What we are building

An indigenous **sealed compartment integrity witness** — event-driven, air-gapped, tamper-linked proof for NBC envelopes, stored kit, and logistics crates (UAV bay as secondary).

**Runtime:** sleep (µA) → environmental **event** → WASM sandbox check → hash-linked log → RAM wipe → sleep.

| Part | Role today | Phase 2+ |
|------|------------|----------|
| ESP32-C6 | **Reference** board only | Custom PCB / qualified MCU |
| BMP390 | Pressure inside seal | Production sensor |
| ADS1115 + pot | Demo dose channel | Qualified radiological front-end |
| OLED | Evaluator table demo | Boxed kit |
| microSD | Offline audit trail | Every unit |
| WASM | Strict / relaxed policy | SD / field updates |

**No Wi‑Fi** in firmware. Radio encrypted uplink = optional roadmap, **off by default**.

**Out of scope:** satellite flight, flight executive, CCTV replacement, certified dosimeter on breadboard.

---

## Problem statement (application)

> Indigenous low-power witness for **sealed defence compartments**: wakes on pressure/dose events, runs sandboxed health policy, appends **tamper-linked offline proof**, retains **no mission data in RAM** between cycles. ESP32-C6 demonstrates architecture; iDEX delivers **PCB + evaluator kits**.

Full text: [IDEX_APPLICATION.md](IDEX_APPLICATION.md).

---

## Why iDEX Open fits

| They ask for | We have / plan |
|--------------|----------------|
| Indian R&D | Rust + RISC-V ESP32-C6, built in India |
| Defence relevance | **Sealed NBC / compartment witness**; dose path Phase 2 |
| Prototype → product | Breadboard → PCB → 100 boxed units |
| Small team | Solo founder + contract vendors; salaries in budget |
| Milestone proof | Git tags, SD logs, demo video, evaluator test sheet |

We commit to **hardware + docs evaluators can run in 24 months**. Post-grant, The SNMC sells and supports the product line under commercial and defence contracts. Full platform sign-off is **out of scope** unless a Service lab agrees a test plan later.

---

## Post-grant commercial path

| Segment | Buyer | Why Aether fits |
|---------|-------|-----------------|
| Defence sealed volumes | NBC workshops, depots, UAV lines | Air-gap, tamper log, indigenous |
| Ordnance / sensitive stores | Unit armourers, Q-branch | Offline custody proof |
| Commercial logistics | Pharma cold chain, hazmat crates | Battery months, no cloud dependency |

**Honest limits:** Defence sales are **slow** (tenders, trials). Commercial cold-chain has **competition** (cloud loggers). Our wedge is **ultra-low sleep + air-gap + verifiable log + local alert**, not “cheaper BMP390.” Revenue in years 3–5 depends on evaluator conversion and one commercial pilot — not automatic.

---

## Team compensation and vendor services (24 months)

Hardware for the table demo is **under ₹10,000** parts; **no booth rental** in the budget.

| Line | Role | ₹/month | 24 months |
|------|------|---------|-----------|
| Founder | Firmware, WASM host, PCB spec, iDEX reporting | 1.50 L | **36 L** |
| Vendor & specialist services | PCB layout, EMC pre-scan, enclosure, test house, dosimeter samples | 1.50 L avg | **36 L** |
| **Total** | | **3.00 L/mo** | **72 L** |

The vendor line is **contract spend spread over 24 months**, not a second full-time employee at founder salary. PCB fab and EMS are additionally funded in phase budgets.

---

## Budget (₹1.5 crore)

| Line | ₹ | Notes |
|------|---|--------|
| Founder salary (24 mo) | 36 L | ₹1.5 L/mo full-time R&D |
| Vendor & specialist services (24 mo) | 36 L | PCB, EMC, enclosure, test — contract vendors |
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
- [ ] **Local breach alert** on OLED + GPIO10 (policy fail / threshold)  
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

- Extra full-time hires beyond the solo founder + contract vendors  
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
5. **Founder full-time** for 24 months; vendor network for PCB/EMS.  
6. **Table demo** that works on every release tag.  
7. **The SNMC** positioned to bid post-grant — IP, BOM, flash jig, docs.

---

## Next actions

1. Wire BMP390 **INT → GPIO1**; flash firmware — [README.md](README.md).  
2. Record demo per [DEMO_VIDEO.md](DEMO_VIDEO.md) (sealed box + tamper fail).  
3. Submit [IDEX_APPLICATION.md](IDEX_APPLICATION.md).  
4. Run **1000-cycle** event or interval soak; attach `verify_log.py` output.  
5. Get **two Indian PCB quotes** (breadboard pinout).  
6. Email **NBC / logistics / UAV** workshops for evaluator interest.  

---

**The SNMC** · 2026
