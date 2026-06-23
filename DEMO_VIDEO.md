# 5-Minute Product Demo Script

Record for customer pilots, website, or investor deck. One take is fine; serial on screen optional.

**Props:** breadboard kit, **clear plastic crate** (tape lid), laptop + `verify_log.py`, microSD.

---

## 0:00 — Title (5 s)

**On screen:** Aether Enclave — Custody Witness for Sealed Logistics | The SNMC

---

## 0:05 — Problem (20 s)

> Sealed shipments still use breakable seals and paper logs. When the crate is opened in transit, nobody has machine-verifiable proof at the checkpoint — without calling a vendor cloud.

---

## 0:25 — Hardware (20 s)

- Module + sensor inside **sealed box**
- Point to SD, USB, alert LED (GPIO10)
- Text: *Reference board today. Product = boxed AE-CM1 module.*

---

## 0:45 — Boot (30 s)

- Power on (GPIO2 **not** held)
- Serial: `mode=EVENT_ONLY`, `baseline`, `WITNESS READY`

---

## 1:15 — Violation (45 s)

- Tape lid shut on crate
- **Open lid** → violation event
- Serial: `event`, JSON `proof`
- OLED ALERT if policy fails

---

## 2:00 — Breach latch (45 s) — **KEY MOMENT**

- GPIO10 **stays ON** through sleep
- Wake → `ALERT ACTIVE` → GPIO2 ACK → LED off

---

## 2:45 — Custody receipt (90 s)

1. Export SD → `python tools/sd_export.py ...`
2. `python tools/verify_log.py export.txt` → **ALL OK**
3. Edit one byte → **CHAIN FAIL**
4. Text: *Custody proof. No vendor server.*

---

## 4:15 — vs alternatives (20 s)

| Cloud tracker | Aether |
|---------------|--------|
| Needs network | Offline verify |
| Vendor portal | Your laptop |
| Map only | Seal violated |

---

## 4:35 — Close (25 s)

**Text:** The SNMC · aether-enclave · Pilot kits available

---

## Checklist

- [ ] Latest firmware flashed
- [ ] BMP390 INT → GPIO1
- [ ] GPIO10 LED wired
- [ ] SD logs OK
- [ ] `verify_log.py` rehearsed on PC
- [ ] Crate demo rehearsed (open lid, not “blow on sensor”)

---

## Avoid on camera

- Demo mode (GPIO2 held at boot)
- Claiming “first digital seal in the world”
- Dose pot unless labeled **demo channel only**
