//! The PCG hash every effect's randomness comes from, the same on the CPU as in `shaders/hash.wgsl`.

/// One round of PCG (O'Neill's `pcg_hash`).
pub fn pcg(v: u32) -> u32 {
    let state = v.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277_803_737);
    (word >> 22) ^ word
}

/// A number in [0, 1) from a seed, an index and a channel: `h3` in the shaders.
pub fn h3(seed: u32, i: u32, k: u32) -> f32 {
    (pcg(seed ^ pcg(i.wrapping_mul(16).wrapping_add(k))) >> 8) as f32 / 16_777_216.0
}

/// A record seed from an event's step and its index in the event list.
pub fn seed_of(step: u32, index: usize, salt: u32) -> u32 {
    pcg(pcg(step ^ 0x5EED_0000) ^ (index as u32).wrapping_mul(0x9E37_79B9) ^ salt)
}
