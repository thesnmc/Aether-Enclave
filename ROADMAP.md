# Aether Enclave — Grant Roadmap (up to ₹1.5 crore)

This document describes how we would spend a grant of **₹1.5 crore** (₹15 million) to move from the **DefExpo breadboard demo** to a **field-testable unit** that a lab or service branch could evaluate. Figures are planning estimates in Indian Rupees, not a formal quote.

**Current status:** Working firmware on ESP32-C6, QEMU bench, BMP390 + ADS1115 + OLED on I2C, USB flash/debug without external tools.

---

## Goal

Deliver a **small, sleep-heavy sensor node** that:

- Runs checked diagnostic logic in an isolated WASM module  
- Leaves no application data in RAM between runs  
- Records a proof hash per wake for audit  
- Uses **Indian-made or Indian-assembled** hardware where practical  
- Can be tested at a **DRDO lab, BEL, or private defence electronics partner** without rewriting the core Rust codebase  

---

## Budget overview

| Phase | Duration | Budget | Outcome |
|-------|----------|--------|---------|
| 0 — Done | — | ~₹0.3 L (self-funded) | Breadboard demo, Git repo, DefExpo booth |
| 1 — Hardening | 4 months | ₹18 L | SD logging, env tests, custom PCB v1 |
| 2 — Lab pilot | 6 months | ₹42 L | 20 pre-production units, partner lab report |
| 3 — Qualification prep | 8 months | ₹55 L | Thermal/vibe/EMI screen, docs, CEMILAC path advice |
| 4 — Small series | 6 months | ₹30 L | 100 units, spares, training kit |
| 5 — Team & compliance | ongoing | ₹5 L | iDEX reporting, IP, audit trail |
| **Total** | ~24 months | **₹1.5 Cr** | Field-evaluable product + manufacturing pack |

*(L = lakh = ₹100,000; Cr = crore = ₹10,000,000)*

---

## Phase 0 — Complete (breadboard demo)

**Spend:** ~₹25,000–40,000 (already paid by founder)

**Deliverables:**

- ESP32-C6 firmware: WASM host, sensor I2C, OLED, RTC cycle counter, demo mode  
- QEMU regression path for CI  
- DefExpo wiring guide in README  

**Not in scope yet:** SD card, sealed enclosure, radiation-hard parts, formal test reports.

---

## Phase 1 — Hardening (₹18 lakh, months 1–4)

### Engineering

| Item | Cost | Notes |
|------|------|-------|
| 2× firmware engineers (contract, 4 mo) | ₹8 L | SD SPI driver, sealed build flags, fault injection |
| PCB v1 (4-layer, 50 boards) | ₹4 L | ESP32-C6 + sensor sockets + OLED + SD + test pads |
| Parts for 10 assembled boards | ₹2 L | BMP390, ADS1115, OLED, connectors |
| Environmental chamber time (commercial lab) | ₹2 L | −10 °C to +55 °C, 10 boards |
| Test gear (logic analyser, calibrated reference) | ₹2 L | One-time |

### Milestones

- [ ] microSD append-only `PROOF.LOG` per wake (SPI on GPIO3/4/5/15)  
- [ ] PCB replaces breadboard; same firmware with pin config header  
- [ ] 1000-cycle soak test logged  
- [ ] Demo video + test summary for iDEX / incubator progress report  

---

## Phase 2 — Lab pilot (₹42 lakh, months 5–10)

### Purpose

Put **20 units** in the hands of a **partner lab** (e.g. DRDO cluster lab, IIT defence electronics group, or DPSU R&D) for a defined test plan—not a sales pitch, a written scope.

| Item | Cost | Notes |
|------|------|-------|
| 20 assembled PCB v2 units | ₹6 L | ESD-safe assembly, conformal coat option |
| Lab partnership / test programme fee | ₹15 L | Written SOW: wake reliability, proof log integrity, sleep current |
| Field sensors (real dosimeter front-end, pressure port) | ₹8 L | Replace pot with engineering-grade inputs where required |
| 1× firmware + 1× hardware engineer (6 mo) | ₹10 L | Fix findings, PCB v2 spin |
| Travel + DefExpo / Aero India follow-on | ₹3 L | |

### Milestones

- [ ] Signed test report: wake count, false wake rate, RAM wipe verification method  
- [ ] WASM payload update process documented (swap `.wasm` without full reflash if feasible)  
- [ ] Sleep current measured (< target TBD with lab, typically sub-mA domain for deep sleep)  

---

## Phase 3 — Qualification prep (₹55 lakh, months 11–18)

This phase does **not** guarantee military certification—it pays for the **evidence pack** often requested before deeper qualification.

| Item | Cost | Notes |
|------|------|-------|
| Vibration + shock (MIL-STD-810 subset or JSS equivalent) | ₹12 L | External lab, 5 boards |
| EMI/EMC pre-scan (conducted/radiated snapshot) | ₹15 L | Fix layout in PCB v3 if needed |
| PCB v3 + 30 boards | ₹8 L | Guard ring, filtered power, optional metal shield can |
| Documentation set | ₹8 L | ICD, BOM, test procedures, source release pack (AGPL compliance) |
| Security review (external) | ₹7 L | Supply chain, flash protection, debug port policy |
| Contingency | ₹5 L | Re-spin, failed samples |

### Milestones

- [ ] Qualification readiness review with lab sign-off  
- [ ] Written “known limitations” sheet for evaluators  
- [ ] CEMILAC / platform office intro meetings (advice only; timelines vary by platform)  

---

## Phase 4 — Small series (₹30 lakh, months 19–24)

| Item | Cost | Notes |
|------|------|-------|
| 100 production units (PCB v3) | ₹15 L | Through EMS in India (e.g. Bangalore/Pune cluster) |
| Enclosure (IP54-ish aluminium or ABS) | ₹6 L | Tooling + 100 shells |
| Training kit (10 units for school/lab) | ₹4 L | Documented labs using QEMU + 1 real board |
| Spares + warranty pool | ₹3 L | |
| Logistics + storage | ₹2 L | |

### Milestones

- [ ] Deliver 100 serial-numbered units with calibration card  
- [ ] Flashing jig at EMS (USB Serial/JTAG, no chip-wise manual steps)  
- [ ] End-user one-page quick start (Hindi + English)  

---

## Phase 5 — Team & compliance (₹5 lakh, spread across project)

| Item | Cost |
|------|------|
| Company / LLP compliance, accounting | ₹1.5 L |
| IP filing (trademark + optional copyright on docs) | ₹2 L |
| Grant reporting, iDEX milestone submissions | ₹1.5 L |

---

## How this maps to Indian defence funding channels

| Channel | Fit |
|---------|-----|
| **iDEX DISC / SPRINT** | Phases 1–2: prototype → lab demo |
| **TDF (Technology Development Fund)** | Phases 2–3 if aligned with specific Service problem statement |
| **Make-II / DPSU R&D** | Phase 4 if a DPSU adopts the module as a subsystem |
| **State deep-tech grant** | Phase 1 overlap (PCB + hires) |

We would **not** spend grant money on marketing fluff, generic AI, or cloud subscriptions. The spend is tied to **boards, tests, people, and written reports**.

---

## What we will not do with the grant

- Buy classic ESP32 boards (wrong CPU for this codebase)  
- Add Wi-Fi/cloud uplink without a written security review  
- Promise full military qualification in 24 months without a named platform customer  
- Outsource core firmware to closed binary; Rust host stays auditable (AGPL)  

---

## Success criteria at ₹1.5 Cr spend

1. **100 production units** with logged proof history (SD + serial).  
2. **One external lab report** on reliability and wipe behaviour.  
3. **PCB + enclosure** suitable for vehicle bay or sheltered outdoor trial—not just breadboard.  
4. **Documented path** for a Service lab to swap WASM diagnostic logic for a new mission profile.  
5. **DefExpo-ready demo** retained on every firmware release (regression in QEMU + 1 hardware smoke test).  

---

## Immediate next steps (before grant)

1. DefExpo demo with current breadboard + OLED.  
2. Order PCB v1 quote from two Indian fabs.  
3. Draft 2-page problem statement for iDEX (pressure + radiation monitor wake node for unmanned systems).  
4. Implement SD proof log (Phase 1 first code task).  

---

*Figures updated June 2025 planning cycle. Adjust with actual quotes from labs and EMS vendors.*
