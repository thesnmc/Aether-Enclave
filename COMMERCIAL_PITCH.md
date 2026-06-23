# Aether Enclave — Commercial Pitch

**Company:** The SNMC  
**Product:** Offline custody witness module for sealed logistics  
**Status:** Reference firmware on ESP32-C6 · PCB product in development

---

## One line

> When a **sealed shipment is opened in transit**, issue a **local alert** and an **offline custody receipt** any auditor can verify on a laptop — **no vendor cloud, no subscription dashboard**.

---

## Problem

High-value and regulated shipments still rely on **breakable seals and paper logs**:

- Pilferage and **disputed handoffs** between warehouse, carrier, and receiver  
- Cloud trackers need **network + vendor portal** — useless at air-gapped sites or when contracts forbid uplink  
- Cheap loggers record numbers; they do **not** issue **tamper-evident custody receipts**  
- Port e-seals solve **containers**; they do not solve **every internal crate and bonded store**

---

## Solution

**Aether Enclave** is a low-power **custody witness module** mounted on or inside a sealed crate:

```text
Sleep (µA) → seal violated → policy check → LOCAL alert → custody receipt → verify → sleep
```

| Capability | Benefit |
|------------|---------|
| **Violation detect** | Lid open / pressure change (reed switch on production PCB) |
| **Latched alert** | GPIO10 + OLED until operator acknowledges |
| **Custody receipt** | Hash-linked record on SD / USB export |
| **Offline verify** | `verify_log.py` — edit one byte → chain fails |
| **Policy packs** | WASM strict/relaxed per customer via SD profile |
| **Air-gap default** | No Wi‑Fi; optional encrypted uplink when policy allows |

**Reference board today:** ESP32-C6 breadboard. **Product:** boxed module, enclosure, flash tooling.

---

## Target customers

| Segment | Use case |
|---------|----------|
| **Defence logistics contractors** | Sealed spares / ammo-style crates on internal moves |
| **Bonded warehouses & customs-adjacent stores** | Custody chain without cloud dependency |
| **Pharma / hazmat logistics** | Air-gapped sites; audit export after delivery |
| **High-value equipment shippers** | Proof seal intact at receiver |
| **OEM integrators** | Embed witness firmware in their crate or lock |

---

## Why not a generic data logger?

| Generic logger / cloud IoT | Aether Enclave |
|----------------------------|----------------|
| Continuous graphs | **Event-only** — months on battery |
| Trust vendor cloud | **Verify file locally** |
| App notification only | **Physical latched alert at crate** |
| Fixed firmware | **WASM policy** per route/customer |
| ₹2k sensor on Amazon | **Custody system** — alert + receipt + procedure |

We are **not** a GPS map. We are **checkpoint proof** when the seal dies.

---

## SKUs (roadmap)

| SKU | Form | Target price @ volume |
|-----|------|------------------------|
| **AE-CM1** | Reusable witness module on crate | ₹8k–15k early · ₹3–6k @ 1k+ |
| **AE-PL1** | Pressurized / canister variant (BMP390) | Module + env sensor pack |
| **AE-TG1** | Disposable tamper tag (Phase 2) | ₹40–120 @ 100k+ |

---

## Demo (5 minutes)

1. Sealed crate (clear box + tape)  
2. **Open lid** → ALERT → GPIO10 latched  
3. Export SD → `verify_log.py` **PASS**  
4. Edit one byte → **FAIL**  
5. *“Custody receipt without our server.”*

Script: [DEMO_VIDEO.md](DEMO_VIDEO.md) · Procedure: [PILOT_TEST.md](PILOT_TEST.md)

---

## Business model

| Stream | Description |
|--------|-------------|
| **Hardware** | Module per crate or fleet rollout |
| **Pilot kits** | Boxed eval unit + written test procedure |
| **OEM licence** | Witness runtime in partner lock/crate |
| **Policy packs** | WASM profiles per vertical (pharma, hazmat, defence contractor) |
| **Support** | Flash jig, EN/HI SOP, verify tooling |

---

## Honest limits

See [LIMITS.md](LIMITS.md). We do **not** claim port e-seal replacement, certified hazmat instrumentation, or anti-implant HSM on the reference board.

---

## Contact / next step

**Pilot offer:** 30-minute sealed-crate test with customer SOP — module + `verify_log.py` + export procedure.

**The SNMC** · [GitHub](https://github.com/thesnmc/Aether-Enclave) · 2026
