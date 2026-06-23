# iDEX Technical Review — Prepared Answers

**Use this document** to rehearse the application interview, booth defence, and written follow-ups.  
Every answer below matches **what is in the repo today** and what is **committed in the grant**.

**Supporting docs:** [LIMITS.md](LIMITS.md) · [EVALUATOR_TEST.md](EVALUATOR_TEST.md) · [DEMO_VIDEO.md](DEMO_VIDEO.md)

---

## How to use this in the room

1. **Lead with demo**, not slides — sealed box → event → ALERT → SD → `verify_log.py`.  
2. When challenged, say **“correct, and here is our limit sheet”** — never argue physics you do not have.  
3. Separate **prototype honesty** from **grant deliverable** (“Phase 2 qualified dose front-end”).  
4. If you do not know, say **“out of scope for this grant; we document it in LIMITS.md.”**

---

## Hard questions and solid answers

### “This is a science-fair bench, not defence hardware.”

**Answer:** Correct for Phase 0. iDEX funds **productization**: custom PCB, enclosure, 125 units, qualified sensor path. Today we prove **architecture and procedure** — event witness, local alert, offline log, verify tool. The breadboard is the **evaluation platform** named explicitly in the application; the grant deliverable is **boxed PCB product**.

**Show:** Working firmware, [LIMITS.md](LIMITS.md), PCB quote in annex, milestone table.

---

### “Pressure is not seal integrity.”

**Answer:** Pressure is a **proxy metric** for compartment integrity, not a replacement for full NBC qualification. We detect **measurable changes inside the volume** when the seal is disturbed — lid open, leak, rapid depressurization. We document false-positive paths (temperature, slow leak) in [LIMITS.md](LIMITS.md). Phase 2 adds threshold tuning with evaluator data and optional multi-sensor fusion if the pilot site requires it.

**Show:** Open clear box → pressure event on serial. Explain 0.015 atm threshold and leak-rate path in firmware.

**Do not say:** “Proves NBC envelope combat readiness.”

---

### “Your dosimeter is a potentiometer.”

**Answer:** On breadboard, the dose channel is a **demonstration input** only — labeled `DOSE DEMO` on OLED and serial. It proves the **WASM policy path and alert chain** share the same runtime as pressure. Phase 2 deliverable is a **named qualified front-end**; we request Service input on acceptable sensor class in the application.

**Show:** Turn pot → `DOSE DEMO HI` → ALERT → GPIO10 ON. Immediately state it is not radiological certification.

**Do not say:** “Operational dosimeter” on breadboard.

---

### “Your tamper-evident chain is not tamper-evident.”

**Answer:** We are precise: the chain provides **integrity on exported logs** — if someone edits the file after export, `verify_log.py` fails. It does **not** claim the device is tamper-proof against physical reflash or SD swap. That requires secure boot and signed firmware — **roadmap with a national lab partner**, not claimed today. [LIMITS.md](LIMITS.md) states this explicitly.

**Show:** `verify_log.py` PASS → edit one byte → FAIL. Then say: “This catches custody edits on the audit file; physical attack is a different threat model documented in LIMITS.”

---

### “Why WebAssembly for an if-statement?”

**Answer:** WASM is a **policy container**, not the security boundary. Mission limits ship in a **sandboxed module** (strict vs relaxed) updatable from SD without reflashing the Rust host. Field units can receive **policy revisions** under configuration control while the witness runtime stays fixed. The host still owns sensors; the guest owns threshold logic we can audit separately.

**Do not say:** “WASM makes it unhackable.”

---

### “RAM wipe is security theatre.”

**Answer:** We do not claim rad-hard or anti-implant. Per-cycle arena wipe **limits blast radius** of a single guest fault and clears transient mission data before sleep. Persistent state is **only** cycle count, proof hash, and alert latch in RTC — documented. Honest hygiene, not HSM.

---

### “Operator misses the alert because the device sleeps.”

**Answer:** On policy fail: OLED full-screen **ALERT**, GPIO10 **stays ON through deep sleep**, latch survives in RTC until operator presses **GPIO2 to ACK**. On next wake, **ALERT ACTIVE** reminder on OLED. This is **local annunciation** without cloud. PCB v1 adds **piezo buzzer** on the same alert line.

**Show:** Trigger fail (low pressure via WASM) → LED stays on after sleep → wake → reminder → GPIO2 ACK → LED off.

---

### “Radio / remote alert does not work.”

**Answer:** Correct. Encrypted uplink is **implemented in software** but **TX disabled** until a second board validates 802.15.4 on the bench. Default deployment is **air-gap**: local alert + SD. We do not demo radio in the iDEX video unless hardware is validated.

---

### “Indigenous RISC-V on Espressif is not indigenous.”

**Answer:** We use ESP32-C6 as **reference silicon** to prove the witness **software architecture** on open RISC-V toolchain. Grant deliverable is **Indian-designed PCB + BOM + assembly** with documented second-source MCU strategy. We do not claim domestic fab on the reference chip. Porting to **Indian or strategic RISC-V MCUs** is an architecture goal, not this grant’s silicon sign-off.

---

### “₹1.5 Cr is too much for one person.”

**Answer:** ₹1.5 **crore total** over 24 months — programme ceiling matched to deliverables: PCB spins, enclosure, 125 units, vendor EMC/layout, founder full-time at ₹1.5 **lakh/month**, travel, compliance. Not ₹1.5 Cr per month. Solo founder with **contract vendors** for layout and EMS is standard for iDEX hardware teams at this scale.

**Show:** Budget table in [IDEX_APPLICATION.md](IDEX_APPLICATION.md) — 72L people/vendors, 78L product.

---

### “You keep the units or give them away?”

**Answer:** Grant deliverables: **25 evaluator kits + 100 boxed units** to Services per milestones. The SNMC **retains IP**, gerbers, firmware, flash jig, and sells/supports post-grant under separate contracts. iDEX buys **capability emergence**, not a one-off science project.

---

### “Is this a viable business?”

**Answer:** **Niche, not mass market.** Defence sealed-volume witness: slow tenders, real need. Commercial sealed logistics: smaller volumes, competes on air-gap + battery + verifiable log. Revenue path = evaluator pilot → repeat order → EMS batch. We are honest that scale is **years**, not viral.

---

### “Why not buy a commercial data logger?”

**Answer:** Commercial loggers assume **cloud or mains**, continuous logging, and weak offline custody proof. We target **µA event duty**, **no network**, **hash-linked export verify**, and **local latched alert** — a different procurement spec for forward depots and air-gapped crates.

---

### “AGPL — defence will never adopt.”

**Answer:** Source is public for **audit transparency**. Production deployment can use **signed binaries** with documented BOM; license terms are negotiable with adopting agency. Many defence integrations separate **audit source** from **field binary**.

---

## Demo script that answers the critics (5 min)

| Time | Action | Answers |
|------|--------|---------|
| 0:30 | Boot event-only, no demo mode | Not a timer spam logger |
| 1:00 | Seal box → open → pressure event | Inside-the-volume physics |
| 1:30 | Policy fail → OLED ALERT + GPIO10 ON | Local alert, not log-only |
| 2:00 | Sleep; LED still ON | Latched annunciation |
| 2:30 | Wake → ALERT ACTIVE → GPIO2 ACK | Operator workflow |
| 3:00 | SD export → verify PASS → tamper FAIL | Audit chain honesty |
| 3:30 | Hold up [LIMITS.md](LIMITS.md) | Credibility |
| 4:00 | PCB render / quote + milestone table | Productization path |

---

## Phrases to use

- “**Witness**, not CCTV replacement.”  
- “**Proxy metric** with documented limits.”  
- “**Integrity on export**, not anti-implant.”  
- “**Detect → alert → log → optional uplink.**”  
- “**Reference board today; PCB product in grant.**”  
- “**Dose channel demo on breadboard; qualified sensor in Phase 2.**”

---

## Phrases to avoid

- “NBC certified” / “battle-ready” / “tamper-proof device”  
- “Military-grade encryption” (radio not validated)  
- “Indigenous silicon” (when you mean indigenous **design**)  
- “Zero-allocation hardened firmware” (uses bounded `alloc` arena)  
- “Satellite / flight ready”

---

## Pre-interview checklist

- [ ] Flash latest firmware; breach latch + ACK tested  
- [ ] [LIMITS.md](LIMITS.md) printed or on tablet  
- [ ] `verify_log.py` rehearsed  
- [ ] Pot demo labeled “DOSE DEMO” out loud  
- [ ] GPIO10 LED wired  
- [ ] Two PCB quotes in annex  
- [ ] 1000-cycle log + verify output attached  
- [ ] One email thread showing evaluator workshop interest (if any)

---

**The SNMC** · iDEX interview prep · 2026
