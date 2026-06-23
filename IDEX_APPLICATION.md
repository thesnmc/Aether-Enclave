# iDEX Open — Application (Aether Enclave)

**Applicant:** The SNMC  
**Programme:** iDEX Open Challenge  
**Ask:** ₹1.5 crore over 24 months (programme ceiling)  
**Primary pitch:** Indigenous **sealed compartment integrity witness** for defence NBC envelopes, stored kit, and logistics crates.

---

## Title

**Aether Enclave — Indigenous RISC-V Witness for Sealed Compartment Integrity**

---

## Problem statement

Indian defence operations depend on **sealed volumes** — NBC shelter envelopes, filter plenums, stored ammunition and sensitive kit in closed cases, and UAV payload bays between sorties. These spaces must remain **pressure-tight** and (where required) within radiological monitoring policy.

**CCTV and cloud IoT do not solve this:** cameras observe **outside** the seal; they do not produce **quantitative, tamper-evident proof** of pressure integrity **inside** a closed compartment. Always-on loggers need mains, networks, and large attack surfaces. Manual inspection leaves **no machine-verifiable audit trail**.

**Gap:** A low-duty-cycle witness that **detects environmental breach**, **alerts the operator locally**, appends a **hash-linked offline log** for audit, and **retains no mission data in RAM** between checks.

---

## Proposed solution

**Aether Enclave** is bare-metal firmware on a **reference RISC-V board** (ESP32-C6 today; custom PCB in Phase 1). It is **not** a flight computer, satellite payload, or CCTV replacement.

| Capability | Description |
|------------|-------------|
| **Event-driven wake** | BMP390 pressure path + INT line; default **event-only** (no periodic log spam) |
| **Local breach indication** | On policy fail: OLED **ALERT**, status GPIO, serial line — **immediate operator signal** |
| **WASM sandbox** | Mission limits in isolated WebAssembly; strict/relaxed profiles without reflashing host |
| **Proof chain** | Each cycle: FNV-linked 64-bit proof; PC tools detect tampering |
| **Offline log** | microSD raw sectors + USB serial JSON; **no FAT, no cloud** |
| **RAM wipe** | Fixed 128 KiB host arena zeroed every cycle; deep sleep between events |
| **Optional uplink** | Encrypted one-way burst when mission profile allows; **off by default** |
| **Dose channel** | ADS1115 path today (demo pot); qualified front-end in Phase 2 |

**ESP32-C6 breadboard = evaluation platform only.** Deliverable product: **PCB, enclosure, 25 evaluator kits, 100 boxed units**, Hindi/English procedures.

### Detect → alert → log → (optional) uplink

Operators need **two different things**:

1. **Now:** “Did the seal break?” → local alert (OLED, LED, optional buzzer on PCB) at the moment WASM policy fails or pressure crosses threshold.  
2. **Later:** “Prove what happened to an auditor?” → tamper-linked SD log + `verify_log.py`.

Logging without alerting is an **audit recorder**, not a field safety tool. Aether Enclave does **both**: wake on physics, run policy, **signal breach locally**, then write proof. Air-gapped sites get the alert on the box; networked depots can enable encrypted uplink in mission profile.

---

## Why not commercial IoT / Pi / CCTV?

| Requirement | CCTV / cloud logger | Aether Enclave |
|-------------|---------------------|----------------|
| Physics **inside** sealed volume | No | Yes (pressure) |
| **Immediate local alert** on breach | No | Yes (OLED / GPIO) |
| Months on battery / µA sleep | No | Yes |
| Air-gapped audit | No | Yes |
| Tamper-evident log | Weak | Hash chain + verify tool |
| Indigenous RISC-V stack | Rare | Core design |
| Policy swap without Linux | No | WASM + SD profile |

**We complement CCTV** — we do not replace perimeter surveillance.

---

## Secondary use cases (same hardware)

- UAV **bay custody** when payload is removed or non-telemetered stores are installed  
- **Secure logistics crates** — chain-of-custody environmental witness without GPS uplink  
- **Commercial cold-chain / hazmat cases** (post-grant): sealed crate integrity where cloud loggers are unacceptable or battery life matters

Not in scope for this grant: orbital deployment, flight certification, certified dosimeter on breadboard.

---

## Innovation

1. **Witness runtime** — sleep → detect → sandboxed check → **local alert if fail** → proof log → wipe → sleep.  
2. **Verifiable integrity** — evaluator runs `verify_log.py`; one-byte tamper fails chain.  
3. **Portable architecture** — Rust `no_std` host ports to other RISC-V MCUs; reference board ≠ final silicon.  
4. **Bounded memory** — fixed arena + per-cycle wipe limits blast radius of single faults (not rad-hard claims).

---

## Current status (Phase 0 — complete)

- Working breadboard firmware (BMP390, ADS1115, OLED, SD, WASM strict/relaxed)  
- Event-only default; BMP390 INT wake on GPIO1  
- OLED event browser (GPIO9); cycle display on GPIO10 LED path  
- Optional encrypted uplink scaffold (**radio off by default**)  
- Tools: `sd_export.py`, `verify_log.py`, `write_mission_profile.py`  
- Evaluator procedure: [EVALUATOR_TEST.md](EVALUATOR_TEST.md)  
- Capability limits: [LIMITS.md](LIMITS.md)  
- Interview prep: [IDEX_REVIEW.md](IDEX_REVIEW.md)  
- Submit checklist: [IDEX_READY.md](IDEX_READY.md)  
- Demo script: [DEMO_VIDEO.md](DEMO_VIDEO.md)

---

## 24-month milestones

| Month | Deliverable | Evidence |
|-------|-------------|----------|
| 0–3 | iDEX sanction; **5-min demo video**; 1000-cycle SD log | Video link, verify output |
| 3–6 | **PCB v1** (breadboard pinout); enclosure concept; **local alert on PCB** | Gerbers, BOM quote, alert demo |
| 6–12 | **10 lab units**; evaluator test feedback | Signed test sheet |
| 12–18 | **PCB v2**; **25 numbered evaluator kits** | Serial list, feedback report |
| 18–24 | **100 boxed units**; flash jig; EN/HI quick-start | EMS tag, sample verify |

---

## What iDEX funds (and what it does not)

**iDEX is not a purchase order for 100 gadgets you hand over and walk away.**

The grant funds **Indian R&D and productization** so The SNMC becomes a **delivery-capable supplier**:

| Funded | Outcome for The SNMC |
|--------|----------------------|
| Founder salary (24 mo) | Full-time R&D; you survive while building |
| Vendor services (PCB, EMS, enclosure, test) | **Product line** you own — gerbers, BOM, flash jig |
| Evaluator + production units | **Proof of supply** to Services; units go to evaluators per milestone |
| IP + compliance | Trademark, LLP/GST, milestone reports |

**You retain the architecture, PCB design, firmware, and right to sell** post-grant to Services, ordnance factories, civil defence, and commercial sealed-logistics customers under separate contracts. AGPL source stays public for audit; production can ship signed binaries. The 100 boxed units are a **grant deliverable**, not the end of the company.

---

## Budget summary (₹1.5 Cr)

| Line | ₹ | Notes |
|------|---|--------|
| Founder salary (24 mo @ ₹1.5 L/mo) | 36 L | Full-time firmware, product, milestones |
| Vendor & specialist services (24 mo) | 36 L | PCB layout, EMC pre-scan, enclosure design, test house, dosimeter samples — **not** a second full-time salary |
| PCB, parts, bench (Phase 1) | 18 L | Fab, assemblies, tools |
| 25 evaluator units + travel (Phase 2) | 22 L | PCB v2, pilot feedback |
| Enclosure, docs (Phase 3) | 16 L | Boxed kit, EN/HI manuals |
| 100-unit EMS batch (Phase 4) | 12 L | Small-series production |
| Travel, compliance, IP, contingency | 10 L | Reviews, DefExpo, LLP, reporting |
| **Total** | **150 L** | Programme maximum |

**Why ₹1.5 Cr is justified:** 24 months, custom PCB + enclosure, 125 total units (25 evaluator + 100 production), qualified vendor work, and one founder living wage — not profit extraction. Asking below ceiling without reducing deliverables signals you cannot reach production scale.

Detail: [ROADMAP.md](ROADMAP.md).

---

## Risks and mitigation

| Risk | Mitigation |
|------|------------|
| Pressure alone misses slow leaks | Document limits; Phase 2 multi-sensor; threshold tuning in WASM |
| Operators want real-time without visiting box | Local alert on breach; optional encrypted uplink in profile |
| Dosimeter qualification | Phase 2 analog front-end; honest limits sheet on breadboard |
| Evaluator access | Written 30-min test; boxed kit with USB only |
| EMI / mechanical on PCB | v1 pin-compatible with breadboard; v2 from feedback |
| Slow defence sales post-grant | Parallel commercial crate/cold-chain pilots; evaluator relationships |

---

## What we need from iDEX / Services

1. Sanction to deliver **25 evaluator units** with written procedure.  
2. One **pilot site** (NBC shelter mock-up, logistics depot, or UAV line) for feedback.  
3. Clarity on **radiation front-end** requirements for Phase 2 (if dose channel is in scope).

---

## One-minute elevator pitch

> *When a defence compartment is sealed, CCTV cannot prove it stayed tight inside. Aether Enclave is an indigenous RISC-V witness: it sleeps on microamps, wakes on pressure events, **alerts locally if policy fails**, and writes a tamper-linked log you verify on a laptop — no Wi‑Fi required. iDEX funds the PCB, field kits, and a company that can supply them — not a one-off science project.*

---

## Footnote (space / flight — only if asked)

Software architecture may port to **qualified RISC-V** platforms for **auxiliary** monitoring programmes. **Out of scope** for this prototype and grant deliverables.

---

**The SNMC** · iDEX Open Application · 2026
