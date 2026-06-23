//! Last N witness events in RTC fast RAM — survives deep sleep for OLED browser.

const SLOTS: usize = 4;
const WORDS_PER_SLOT: usize = 5;
const HEADER_WORDS: usize = 2;
const TOTAL_WORDS: usize = HEADER_WORDS + SLOTS * WORDS_PER_SLOT;

#[derive(Clone, Copy, Debug, Default)]
pub struct EventRecord {
    pub cycle: u32,
    pub proof: u64,
    pub guest: i32,
    pub vector: u8,
    pub pressure_bits: u32,
}

#[esp_hal::ram(unstable(rtc_fast, persistent))]
static mut EVENT_RTC: [u32; TOTAL_WORDS] = [0; TOTAL_WORDS];

fn read_word(i: usize) -> u32 {
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!(EVENT_RTC).cast::<u32>().add(i)) }
}

fn write_word(i: usize, v: u32) {
    unsafe {
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(EVENT_RTC).cast::<u32>().add(i),
            v,
        );
    }
}

fn slot_base(slot: usize) -> usize {
    HEADER_WORDS + slot * WORDS_PER_SLOT
}

fn total_count() -> u32 {
    read_word(0)
}

fn head_slot() -> usize {
    (read_word(1) as usize) % SLOTS
}

/// Number of events available in the ring (max 4).
pub fn count() -> usize {
    total_count().min(SLOTS as u32) as usize
}

/// Push after each completed witness cycle (newest first in browser index 0).
pub fn push(cycle: u32, proof: u64, guest: i32, vector: u8, pressure_bits: u32) {
    let n = total_count().saturating_add(1);
    write_word(0, n);
    let slot = head_slot();
    let base = slot_base(slot);
    write_word(base, cycle);
    write_word(base + 1, proof as u32);
    write_word(base + 2, (proof >> 32) as u32);
    write_word(
        base + 3,
        u32::from(vector) << 8 | (guest.max(0).min(255) as u32),
    );
    write_word(base + 4, pressure_bits);
    write_word(1, ((slot + 1) % SLOTS) as u32);
}

/// `index` 0 = newest recorded event.
pub fn get(index: usize) -> Option<EventRecord> {
    let n = count();
    if index >= n || n == 0 {
        return None;
    }
    let slot = (head_slot() + SLOTS - 1).wrapping_sub(index) % SLOTS;
    let base = slot_base(slot);
    let proof = u64::from(read_word(base + 1)) | (u64::from(read_word(base + 2)) << 32);
    let meta = read_word(base + 3);
    Some(EventRecord {
        cycle: read_word(base),
        proof,
        guest: (meta & 0xFF) as i32,
        vector: ((meta >> 8) & 0xFF) as u8,
        pressure_bits: read_word(base + 4),
    })
}
