# iDEX — Ready to Submit

**One page.** Do every item before you upload the application.

---

## ESP32-C6 wiring (full guide)

| Document | What it covers |
|----------|----------------|
| **[README.md → Wiring guide](README.md#wiring-guide)** | **Main reference** — pin map, ASCII diagram, step-by-step |
| **[EVALUATOR_TEST.md](EVALUATOR_TEST.md)** | Short setup list for evaluators |
| **[ARCHITECTURE.md](ARCHITECTURE.md)** | Software flow; points to README for hardware |

### Pin cheat sheet

| GPIO | Wire to |
|------|---------|
| **6 / 7** | I2C SDA / SCL (BMP390, ADS1115, OLED) |
| **1** | BMP390 **INT** (required for event wake) |
| **2** | Wake button → **3.3V** (not GND) |
| **9** | Review button → **GND** (optional) |
| **10** | LED → 330 Ω → **GND** (required for breach demo) |
| **3 / 4 / 5 / 15** | SD SPI MOSI / MISO / SCK / CS |
| **ADS AIN0** | Pot wiper |

Flash: `cd enclave_kernel && cargo +esp run --release` — see [README.md → Build and flash](README.md#build-and-flash).

---

## Submit checklist

### Hardware (you)

- [ ] BMP390 INT → GPIO1
- [ ] GPIO10 LED + 330 Ω → GND
- [ ] GPIO2 wake + GPIO9 review (optional) wired
- [ ] microSD logs cycles
- [ ] Clear plastic box for sealed demo
- [ ] `cargo +esp build --release` — latest firmware flashed

### Demo (you)

- [ ] Run full [EVALUATOR_TEST.md](EVALUATOR_TEST.md) — Tests 1, 1b, 1c, 2 pass
- [ ] Record [DEMO_VIDEO.md](DEMO_VIDEO.md) (event → breach latch → tamper fail)
- [ ] Upload video; copy link for form

### Evidence (you)

- [ ] 1000-cycle SD soak + `python tools/verify_log.py` output saved
- [ ] Two Indian **PCB quotes** (PDF) — breadboard pinout
- [ ] [LIMITS.md](LIMITS.md) ready to attach or print

### Outreach (you)

- [ ] 20 emails to NBC / depot / logistics / UAV workshops
- [ ] Save best reply thread for annex (if any)

### Application copy

- [ ] Form fields from [IDEX_APPLICATION.md](IDEX_APPLICATION.md)
- [ ] Lead pitch: **ground sealed witness** — not satellite
- [ ] Interview rehearsed from [IDEX_REVIEW.md](IDEX_REVIEW.md)
- [ ] Budget matches [ROADMAP.md](ROADMAP.md)

### Do not attach / do not lead with

- Satellite as main pitch
- “NBC certified” / “tamper-proof device”
- Operational dosimeter (say **DOSE DEMO** on breadboard)
- Demo mode in video (GPIO2 held at boot)

---

## Pitch (copy-paste)

> Indigenous sealed-compartment witness: event-driven RISC-V firmware, local breach alert (GPIO10 latched until ACK), hash-linked offline SD log, verify on laptop — air-gap default. ESP32-C6 is reference hardware; grant delivers PCB, enclosure, evaluator kits.

---

## After submit

- Keep repo tagged at submit commit
- Respond to iDEX within 48 h if they write
- If no grant in ~4 months: follow [README.md](README.md) go-home rule — archive or pivot, no guilt

---

**The SNMC** · 2026
