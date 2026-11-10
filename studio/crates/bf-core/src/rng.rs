// This file is a port of parts of numpy v2.5.3's random module. The notices of the sources follow.
//
// numpy (LICENSE.txt), for numpy/random/_generator.pyx, _bounded_integers.pyx.in, _pcg64.pyx, _common.pxd and
// src/distributions/distributions.c:
//
// Copyright (c) 2005-2025, NumPy Developers.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are
// met:
//
//     * Redistributions of source code must retain the above copyright
//        notice, this list of conditions and the following disclaimer.
//
//     * Redistributions in binary form must reproduce the above
//        copyright notice, this list of conditions and the following
//        disclaimer in the documentation and/or other materials provided
//        with the distribution.
//
//     * Neither the name of the NumPy Developers nor the names of any
//        contributors may be used to endorse or promote products derived
//        from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
// OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
// LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
// THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
//
// numpy's random module (numpy/random/LICENSE.md, the 3-Clause BSD License of its two licences):
//
// Copyright (c) 2019 Kevin Sheppard. All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice,
//    this list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its contributors
//    may be used to endorse or promote products derived from this software
//    without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
// LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
// SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
// CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF
// THE POSSIBILITY OF SUCH DAMAGE.
//
// SeedSequence (numpy/random/bit_generator.pyx), derived from Melissa E. O'Neill's C++11 `std::seed_seq`
// implementation, and PCG64 (numpy/random/src/pcg64/), both under the MIT License:
//
// Copyright (c) 2015 Melissa E. O'Neill
// Copyright (c) 2019 NumPy Developers
//
// PCG Random Number Generation for C.
// Copyright 2014 Melissa O'Neill <oneill@pcg-random.org>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! numpy's `default_rng`, ported: the same numbers as `np.random.default_rng(seed)`, bit for bit.
//!
//! The fight engine, the picture effects and the sound all draw from this generator, because the Python kit that
//! the Studio rebuilds seeds them with numpy's. The port follows numpy v2.5.3's source function by function:
//!
//! - **Seeding.** `SeedSequence` (`numpy/random/bit_generator.pyx`) splits the seed into 32-bit words, hashes them
//!   into a pool of four and hands PCG64 four 64-bit words; `pcg64_set_seed` (`numpy/random/src/pcg64/`) turns those
//!   into the 128-bit state and increment.
//! - **The bit generator.** PCG64: a 128-bit linear congruential step and the XSL-RR output function.
//! - **The methods.** `Generator.random`, `uniform`, `normal`, `standard_normal`, `integers`, `permutation` (which
//!   is `shuffle` on `arange(n)`) and `choice`, from `numpy/random/_generator.pyx`, `_bounded_integers.pyx.in` and
//!   `src/distributions/distributions.c`.
//!
//! Two details of numpy are part of the stream and are modelled exactly:
//!
//! - **The buffered 32-bit half.** A 32-bit draw takes a whole 64-bit output, returns its low half and keeps the
//!   high half for the next 32-bit draw (`has_uint32` and `uinteger` in numpy's PCG64 state). 64-bit draws in
//!   between neither use nor disturb the kept half. `integers` with a bound up to 2**32, `choice_index` and
//!   `permutation` draw 32 bits; `random`, `uniform` and the normals draw 64.
//! - **The bounded-integer method of each call.** `integers` (and so `choice`) uses Lemire's method;
//!   `permutation` shuffles with masked rejection (numpy's `random_interval`). They give different numbers.
//!
//! One place cannot follow numpy to the letter. In the tail of the normal distribution (about one draw in 3,900)
//! numpy calls the C library's `log1p(-u)`, and in the wedges (about one draw in 68) it compares against the C
//! library's `exp`. The engine may only use [`crate::dmath`], so the tail takes `dmath::ln(1.0 - u)`, which is the
//! logarithm of exactly the same real number because `1.0 - u` is exact for a draw `u`, and the wedges take
//! `dmath::exp`. Where the platform's `log1p` and `dmath::ln` round differently, a tail draw differs from that
//! platform's numpy by one unit in the last place. Against numpy 2.5.3 on the reference PC, over 40 million
//! normals, that was 54 of the 10,333 tail draws (one normal in about 740,000); every other draw was bit-equal
//! and the two streams never fell out of step. Every draw in `studio/reference/rng/vectors.json` is bit-equal.

use crate::dmath;
use crate::ziggurat::{FI_DOUBLE, KI_DOUBLE, WI_DOUBLE, ZIGGURAT_NOR_INV_R, ZIGGURAT_NOR_R};

/// The number of 32-bit words in SeedSequence's entropy pool (`DEFAULT_POOL_SIZE`).
const POOL_SIZE: usize = 4;

// SeedSequence's hashing constants (`bit_generator.pyx`).
const INIT_A: u32 = 0x43b0_d7e5;
const MULT_A: u32 = 0x931e_8875;
const INIT_B: u32 = 0x8b51_f9dd;
const MULT_B: u32 = 0x58f3_8ded;
const MIX_MULT_L: u32 = 0xca01_f9dd;
const MIX_MULT_R: u32 = 0x4973_f715;
const XSHIFT: u32 = 16;

/// `PCG_DEFAULT_MULTIPLIER_128` (`pcg64.h`).
const PCG_DEFAULT_MULTIPLIER_128: u128 = (2549297995355413924_u128 << 64) + 4865540595714422341_u128;

/// `_int_to_uint32_array`: appends a non-negative integer as 32-bit words, lowest first. Zero is one zero word; any
/// other value takes as many words as it needs and no more.
fn push_uint32_words(n: u64, out: &mut Vec<u32>) {
    if n == 0 {
        out.push(0);
    }
    let mut n = n;
    while n > 0 {
        out.push((n & 0xFFFF_FFFF) as u32);
        n >>= 32;
    }
}

/// `_coerce_to_uint32_array` for a sequence of integers: each entry's words, concatenated. A bare integer seed is
/// the one-entry sequence.
fn entropy_words(seed: &[u64]) -> Vec<u32> {
    let mut words = Vec::with_capacity(2 * seed.len());
    for &n in seed {
        push_uint32_words(n, &mut words);
    }
    words
}

/// SeedSequence's `hashmix`. The multiplier changes with every call, so it is passed in and out.
fn hashmix(value: u32, hash_const: &mut u32) -> u32 {
    let mut value = value ^ *hash_const;
    *hash_const = hash_const.wrapping_mul(MULT_A);
    value = value.wrapping_mul(*hash_const);
    value ^ (value >> XSHIFT)
}

/// SeedSequence's `mix`.
fn mix(x: u32, y: u32) -> u32 {
    let result = MIX_MULT_L.wrapping_mul(x).wrapping_sub(MIX_MULT_R.wrapping_mul(y));
    result ^ (result >> XSHIFT)
}

/// `SeedSequence.mix_entropy` on a fresh pool: the pool that `SeedSequence(entropy)` holds.
fn entropy_pool(entropy: &[u32]) -> [u32; POOL_SIZE] {
    let mut hash_const = INIT_A;
    let mut pool = [0_u32; POOL_SIZE];
    // Add in the entropy up to the pool size; a shorter entropy keeps running the hash out on zeros.
    for (i, slot) in pool.iter_mut().enumerate() {
        *slot = hashmix(entropy.get(i).copied().unwrap_or(0), &mut hash_const);
    }
    // Mix all bits together so late bits can affect earlier bits.
    for i_src in 0..POOL_SIZE {
        for i_dst in 0..POOL_SIZE {
            if i_src != i_dst {
                pool[i_dst] = mix(pool[i_dst], hashmix(pool[i_src], &mut hash_const));
            }
        }
    }
    // Add any remaining entropy, mixing each new entropy word with each pool word.
    for &word in entropy.iter().skip(POOL_SIZE) {
        for slot in &mut pool {
            *slot = mix(*slot, hashmix(word, &mut hash_const));
        }
    }
    pool
}

/// `SeedSequence(entropy).generate_state(4, np.uint64)`: the four 64-bit words that seed PCG64. numpy makes eight
/// 32-bit words and reads them as little-endian pairs.
fn seed_sequence_state(entropy: &[u32]) -> [u64; 4] {
    let pool = entropy_pool(entropy);
    let mut hash_const = INIT_B;
    let mut words = [0_u32; 8];
    for (i_dst, word) in words.iter_mut().enumerate() {
        let mut data_val = pool[i_dst % POOL_SIZE];
        data_val ^= hash_const;
        hash_const = hash_const.wrapping_mul(MULT_B);
        data_val = data_val.wrapping_mul(hash_const);
        data_val ^= data_val >> XSHIFT;
        *word = data_val;
    }
    [0, 1, 2, 3].map(|i| u64::from(words[2 * i]) | (u64::from(words[2 * i + 1]) << 32))
}

/// `Generator.uniform`'s argument checks. Returns `high - low`, the range the draws are scaled by.
fn uniform_range(lo: f64, hi: f64) -> f64 {
    let range = hi - lo;
    assert!(
        range.is_finite(),
        "uniform({lo}, {hi}): high - low range exceeds valid bounds"
    );
    assert!(!range.is_sign_negative(), "uniform({lo}, {hi}): high - low < 0");
    range
}

/// `Generator.normal`'s argument check: a scale with the sign bit set is refused (so is `-0.0`); NaN passes.
fn check_scale(scale: f64) {
    assert!(
        scale.is_nan() || !scale.is_sign_negative(),
        "normal: scale < 0 ({scale})"
    );
}

/// numpy's `Generator` on its default bit generator, PCG64: what `np.random.default_rng` returns.
///
/// Cloning copies the whole state, the buffered 32-bit half included; the clone and the original then draw
/// independently of each other.
#[derive(Clone, Debug)]
pub struct Generator {
    /// The 128-bit state of the linear congruential generator.
    state: u128,
    /// Its increment, fixed by the seed and always odd.
    inc: u128,
    /// Whether `uinteger` holds the unused high half of a 64-bit output that a 32-bit draw took.
    has_uint32: bool,
    /// That half. As in numpy it keeps its value after it has been handed out.
    uinteger: u32,
}

impl Generator {
    /// `np.random.default_rng(seed)` for a non-negative integer seed, including seeds above 2**32.
    pub fn new(seed: u64) -> Self {
        Self::from_entropy(&entropy_words(&[seed]))
    }

    /// `np.random.default_rng([a, b, c])`: a list seed, as the kit makes mid-fight. Each entry gives one 32-bit
    /// word of entropy, or two if it needs more than 32 bits, exactly as SeedSequence splits a Python integer.
    pub fn from_seq(seed: &[u64]) -> Self {
        Self::from_entropy(&entropy_words(seed))
    }

    /// Resumes from `rng.bit_generator.state["state"]`: its `state` and `inc`. No 32-bit half is buffered, so
    /// this continues a numpy generator exactly only where numpy's `has_uint32` is 0.
    pub fn from_state(state: u128, inc: u128) -> Self {
        Generator {
            state,
            inc,
            has_uint32: false,
            uinteger: 0,
        }
    }

    /// `rng.random()`: a float in [0, 1) with 53 random bits.
    pub fn random(&mut self) -> f64 {
        self.next_double()
    }

    /// `rng.uniform(lo, hi)`: `lo + (hi - lo) * random()`, in that order of operations.
    ///
    /// # Panics
    /// Where numpy raises: `hi - lo` is negative (that includes `-0.0`), infinite or NaN.
    pub fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        let range = uniform_range(lo, hi);
        lo + range * self.next_double()
    }

    /// `rng.standard_normal()`: a draw from the normal distribution with mean 0 and deviation 1, by numpy's
    /// ziggurat (`random_standard_normal`). About 985 draws in 1,000 take one 64-bit output; the rest take more.
    pub fn standard_normal(&mut self) -> f64 {
        loop {
            // r = e3n52sb8: 8 bits pick the layer, 1 the sign, 52 the position.
            let mut r = self.next_u64();
            let idx = (r & 0xff) as usize;
            r >>= 8;
            let sign = r & 0x1;
            let rabs = (r >> 1) & 0x000f_ffff_ffff_ffff;
            let mut x = rabs as f64 * WI_DOUBLE[idx];
            if (sign & 0x1) != 0 {
                x = -x;
            }
            if rabs < KI_DOUBLE[idx] {
                return x; // most of the time return here
            }
            if idx == 0 {
                // The tail. numpy takes log1p(-u) to avoid log(0); the module docs say why this is ln(1.0 - u).
                loop {
                    let xx = -ZIGGURAT_NOR_INV_R * dmath::ln(1.0 - self.next_double());
                    let yy = -dmath::ln(1.0 - self.next_double());
                    if yy + yy > xx * xx {
                        return if ((rabs >> 8) & 0x1) != 0 {
                            -(ZIGGURAT_NOR_R + xx)
                        } else {
                            ZIGGURAT_NOR_R + xx
                        };
                    }
                }
            } else if (FI_DOUBLE[idx - 1] - FI_DOUBLE[idx]) * self.next_double() + FI_DOUBLE[idx]
                < dmath::exp(-0.5 * x * x)
            {
                return x;
            }
        }
    }

    /// `rng.normal(loc, scale)`: `loc + scale * standard_normal()`.
    ///
    /// # Panics
    /// Where numpy raises: `scale` is negative (that includes `-0.0`).
    pub fn normal(&mut self, loc: f64, scale: f64) -> f64 {
        check_scale(scale);
        loc + scale * self.standard_normal()
    }

    /// `rng.integers(high)`: a whole number in `0..high`, as numpy's default `int64` draw gives it. A bound of 1
    /// draws nothing; bounds up to 2**32 take 32-bit draws (the buffered half applies), larger ones 64-bit draws.
    ///
    /// # Panics
    /// Where numpy raises: `high` is 0 or above 2**63.
    pub fn integers(&mut self, high: u64) -> u64 {
        assert!(high > 0, "integers: high <= 0");
        // numpy subtracts 1 for the closed interval and wants the result to fit an int64.
        let rng = high - 1;
        assert!(rng <= i64::MAX as u64, "integers: high is out of bounds for int64");
        self.bounded_uint64(rng)
    }

    /// `rng.permutation(n)`: the numbers `0..n` in random order (numpy shuffles `np.arange(n)`).
    pub fn permutation(&mut self, n: usize) -> Vec<usize> {
        let mut out: Vec<usize> = (0..n).collect();
        // `_shuffle_raw` with first = 1: for i = n - 1 down to 1, swap item i with a random item at or below it.
        for i in (1..n).rev() {
            let j = self.interval(i as u64) as usize;
            out.swap(i, j);
        }
        out
    }

    /// The index that `rng.choice(seq)` picks in a sequence of `n` items: `rng.integers(0, n)`.
    ///
    /// # Panics
    /// Where numpy raises: `n` is 0.
    pub fn choice_index(&mut self, n: usize) -> usize {
        assert!(n > 0, "choice: a cannot be empty unless no samples are taken");
        self.integers(n as u64) as usize
    }

    /// `rng.normal(loc, scale, out.len())`: fills `out` with normal draws, in order.
    ///
    /// # Panics
    /// As [`Generator::normal`].
    pub fn fill_normal(&mut self, loc: f64, scale: f64, out: &mut [f64]) {
        check_scale(scale);
        for v in out {
            *v = loc + scale * self.standard_normal();
        }
    }

    /// `rng.uniform(lo, hi, out.len())`: fills `out` with uniform draws, in order.
    ///
    /// # Panics
    /// As [`Generator::uniform`].
    pub fn fill_uniform(&mut self, lo: f64, hi: f64, out: &mut [f64]) {
        let range = uniform_range(lo, hi);
        for v in out {
            *v = lo + range * self.next_double();
        }
    }
}

impl Generator {
    /// `PCG64(SeedSequence(entropy))`: `pcg64_set_seed` on the four words SeedSequence makes.
    fn from_entropy(entropy: &[u32]) -> Self {
        let words = seed_sequence_state(entropy);
        let initstate = (u128::from(words[0]) << 64) | u128::from(words[1]);
        let initseq = (u128::from(words[2]) << 64) | u128::from(words[3]);
        // pcg_setseq_128_srandom_r
        let mut g = Generator {
            state: 0,
            inc: (initseq << 1) | 1,
            has_uint32: false,
            uinteger: 0,
        };
        g.step();
        g.state = g.state.wrapping_add(initstate);
        g.step();
        g
    }

    /// `pcg_setseq_128_step_r`: one step of the 128-bit linear congruential generator.
    fn step(&mut self) {
        self.state = self
            .state
            .wrapping_mul(PCG_DEFAULT_MULTIPLIER_128)
            .wrapping_add(self.inc);
    }

    /// `pcg64_next64`: step, then the XSL-RR output of the new state (xor the two halves, rotate right by the top
    /// six bits).
    fn next_u64(&mut self) -> u64 {
        self.step();
        let high = (self.state >> 64) as u64;
        let low = self.state as u64;
        (high ^ low).rotate_right((self.state >> 122) as u32)
    }

    /// `pcg64_next32`: the buffered half if there is one; otherwise the low half of a new 64-bit output, keeping
    /// the high half for the next call.
    fn next_u32(&mut self) -> u32 {
        if self.has_uint32 {
            self.has_uint32 = false;
            return self.uinteger;
        }
        let next = self.next_u64();
        self.has_uint32 = true;
        self.uinteger = (next >> 32) as u32;
        (next & 0xffff_ffff) as u32
    }

    /// `next_double` (`uint64_to_double`): the top 53 bits of a 64-bit output, scaled into [0, 1).
    fn next_double(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9007199254740992.0)
    }

    /// `random_bounded_uint64(bitgen, 0, rng, 0, use_masked = false)`: a number in the closed range `0..=rng`.
    /// `rng` is below 2**63 here (see `integers`), so the full 64-bit range, which numpy serves with a raw output,
    /// cannot be asked for.
    fn bounded_uint64(&mut self, rng: u64) -> u64 {
        if rng == 0 {
            0
        } else if rng <= 0xFFFF_FFFF {
            // Call the 32-bit generator if the range fits 32 bits.
            if rng == 0xFFFF_FFFF {
                // The 32-bit Lemire method does not handle rng = 0xFFFFFFFF: numpy takes the raw 32 bits.
                u64::from(self.next_u32())
            } else {
                u64::from(self.bounded_lemire_uint32(rng as u32))
            }
        } else {
            self.bounded_lemire_uint64(rng)
        }
    }

    /// `buffered_bounded_lemire_uint32`: Lemire's method on 32-bit draws, for `rng` below 0xFFFFFFFF.
    fn bounded_lemire_uint32(&mut self, rng: u32) -> u32 {
        let rng_excl = rng + 1;
        // Generate a scaled random number.
        let mut m = u64::from(self.next_u32()) * u64::from(rng_excl);
        // Rejection sampling to remove any bias.
        let mut leftover = (m & 0xFFFF_FFFF) as u32;
        if leftover < rng_excl {
            // `rng_excl` is a simple upper bound for `threshold`.
            let threshold = (u32::MAX - rng) % rng_excl;
            while leftover < threshold {
                m = u64::from(self.next_u32()) * u64::from(rng_excl);
                leftover = (m & 0xFFFF_FFFF) as u32;
            }
        }
        (m >> 32) as u32
    }

    /// `bounded_lemire_uint64`: Lemire's method on 64-bit draws, for `rng` below 0xFFFFFFFFFFFFFFFF.
    fn bounded_lemire_uint64(&mut self, rng: u64) -> u64 {
        let rng_excl = rng + 1;
        // Generate a scaled random number.
        let mut m = u128::from(self.next_u64()) * u128::from(rng_excl);
        // Rejection sampling to remove any bias.
        let mut leftover = (m & 0xFFFF_FFFF_FFFF_FFFF) as u64;
        if leftover < rng_excl {
            // `rng_excl` is a simple upper bound for `threshold`.
            let threshold = (u64::MAX - rng) % rng_excl;
            while leftover < threshold {
                m = u128::from(self.next_u64()) * u128::from(rng_excl);
                leftover = (m & 0xFFFF_FFFF_FFFF_FFFF) as u64;
            }
        }
        (m >> 64) as u64
    }

    /// `random_interval`: a number in the closed range `0..=max` by masked rejection, on 32-bit draws when `max`
    /// fits 32 bits. This is what `shuffle` (and so `permutation`) draws with.
    fn interval(&mut self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }
        // Smallest bit mask >= max.
        let mut mask = max;
        mask |= mask >> 1;
        mask |= mask >> 2;
        mask |= mask >> 4;
        mask |= mask >> 8;
        mask |= mask >> 16;
        mask |= mask >> 32;
        // Search a random value in [0..mask] <= max.
        if max <= 0xffff_ffff {
            loop {
                let value = u64::from(self.next_u32()) & mask;
                if value <= max {
                    return value;
                }
            }
        } else {
            loop {
                let value = self.next_u64() & mask;
                if value <= max {
                    return value;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! What the public API cannot show: SeedSequence's words and the four fields of numpy's PCG64 state, compared
    //! field for field with `studio/reference/rng/vectors.json`. The streams themselves are checked in
    //! `tests/rng_vectors.rs`.

    use super::*;
    use serde_json::Value;

    /// The reference file, relative to this crate.
    const VECTORS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference/rng/vectors.json");

    fn vectors() -> Value {
        let text = std::fs::read_to_string(VECTORS).unwrap_or_else(|e| panic!("cannot read {VECTORS}: {e}"));
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{VECTORS} is not JSON: {e}"))
    }

    /// A JSON integer or a decimal string.
    fn uint(v: &Value) -> u64 {
        match v {
            Value::Number(n) => n.as_u64().unwrap_or_else(|| panic!("{n} is not a u64")),
            Value::String(s) => s.parse().unwrap_or_else(|e| panic!("{s:?} is not a u64: {e}")),
            _ => panic!("expected an integer, found {v}"),
        }
    }

    /// A 128-bit decimal string.
    fn wide(v: &Value) -> u128 {
        let s = v
            .as_str()
            .unwrap_or_else(|| panic!("expected a decimal string, found {v}"));
        s.parse().unwrap_or_else(|e| panic!("{s:?} is not a u128: {e}"))
    }

    /// A seed of the file as the list `from_seq` takes: an integer seed is the one-entry list.
    fn seed_list(seed: &Value) -> Vec<u64> {
        match seed {
            Value::Array(items) => items.iter().map(uint).collect(),
            _ => vec![uint(seed)],
        }
    }

    /// Fails unless the generator's four state fields are exactly numpy's.
    fn assert_state(g: &Generator, want: &Value, what: &str) {
        assert_eq!(g.state, wide(&want["state"]), "{what}: state");
        assert_eq!(g.inc, wide(&want["inc"]), "{what}: inc");
        assert_eq!(u64::from(g.has_uint32), uint(&want["has_uint32"]), "{what}: has_uint32");
        assert_eq!(u64::from(g.uinteger), uint(&want["uinteger"]), "{what}: uinteger");
    }

    /// The examples in the docstring of numpy's `_coerce_to_uint32_array`, and the edges.
    #[test]
    fn integers_split_into_32_bit_words_lowest_first() {
        assert_eq!(entropy_words(&[12345]), [12345]);
        assert_eq!(entropy_words(&[0]), [0]);
        assert_eq!(entropy_words(&[]), [0_u32; 0]);
        assert_eq!(
            entropy_words(&[12345, 0x10_dead_beef, 67890, 0xdead_beef]),
            [12345, 3735928559, 16, 67890, 3735928559]
        );
        assert_eq!(entropy_words(&[u64::MAX, 1 << 32]), [u32::MAX, u32::MAX, 0, 1]);
    }

    #[test]
    fn seed_sequence_words_and_start_state_match_numpy() {
        let doc = vectors();
        let seeds = doc["seeds"].as_array().expect("seeds");
        assert!(!seeds.is_empty());
        for entry in seeds {
            let seed = seed_list(&entry["seed"]);
            let init = &entry["init"];
            let want: Vec<u64> = init["seed_sequence_u64"]
                .as_array()
                .expect("seed_sequence_u64")
                .iter()
                .map(uint)
                .collect();
            assert_eq!(
                seed_sequence_state(&entropy_words(&seed)).to_vec(),
                want,
                "seed {seed:?}: seed_sequence_u64"
            );
            let g = Generator::from_seq(&seed);
            assert_eq!(g.state, wide(&init["state"]), "seed {seed:?}: state");
            assert_eq!(g.inc, wide(&init["inc"]), "seed {seed:?}: inc");
            assert!(!g.has_uint32, "seed {seed:?}: a fresh generator has no buffered half");
            assert_eq!(g.uinteger, 0, "seed {seed:?}: uinteger");
        }
    }

    /// The words and the state numpy 2.5.3 gives for `[2**40 + 7, 44, 2**64 - 1, 0, 2**32]`: eight 32-bit words,
    /// so the words past the pool of four are mixed in too (no seed in the vectors file is that long).
    #[test]
    fn a_seed_longer_than_the_pool_matches_numpy() {
        let seed = [(1 << 40) + 7, 44, u64::MAX, 0, 1 << 32];
        assert_eq!(entropy_words(&seed).len(), 8);
        assert_eq!(
            seed_sequence_state(&entropy_words(&seed)),
            [
                9094895445731885702,
                17238801770118456847,
                9076380365828138456,
                11512641171255053895
            ]
        );
        let g = Generator::from_seq(&seed);
        assert_eq!(g.state, 6063138665990521868792550225979893029);
        assert_eq!(g.inc, 334859331448147890334716078557963197583);
    }

    /// After a stream the generator stands exactly where numpy's does: state, increment, whether a 32-bit half is
    /// buffered, and the half itself (which numpy keeps, stale, after it has been used).
    #[test]
    fn state_after_a_stream_matches_numpy_field_for_field() {
        let doc = vectors();
        for entry in doc["seeds"].as_array().expect("seeds") {
            let seed = seed_list(&entry["seed"]);
            let streams = &entry["streams"];
            let check = |name: &str, draw: &dyn Fn(&mut Generator)| {
                let stream = &streams[name];
                assert!(stream.is_object(), "seed {seed:?}: stream {name:?} is missing");
                let mut g = Generator::from_seq(&seed);
                for _ in 0..uint(&stream["calls"]) {
                    draw(&mut g);
                }
                assert_state(&g, &stream["end"], &format!("seed {seed:?}, after stream {name:?}"));
            };
            // 64-bit draws only: nothing is ever buffered.
            check("random", &|g| {
                g.random();
            });
            check("normal_0_1", &|g| {
                g.normal(0.0, 1.0);
            });
            check("integers_2p40", &|g| {
                g.integers(1 << 40);
            });
            // A bound of 1 draws nothing: the generator has not moved.
            check("integers_1", &|g| {
                g.integers(1);
            });
            // 32-bit draws: a half is buffered after an odd number of them and stays in `uinteger` afterwards.
            check("integers_7", &|g| {
                g.integers(7);
            });
            check("integers_2p32", &|g| {
                g.integers(1 << 32);
            });
            check("integers_2p31_plus_1", &|g| {
                g.integers((1 << 31) + 1);
            });
            check("permutation_2", &|g| {
                g.permutation(2);
            });
            check("permutation_5", &|g| {
                g.permutation(5);
            });
            check("permutation_100", &|g| {
                g.permutation(100);
            });
            // All five methods on one generator.
            check("interleaved", &|g| {
                g.uniform(-1.0, 1.0);
                g.normal(0.0, 1.0);
                g.random();
                g.integers(5);
                g.permutation(4);
            });
        }
    }

    #[test]
    fn from_state_holds_no_buffered_half() {
        let mut g = Generator::new(1);
        g.integers(7);
        assert!(g.has_uint32, "one 32-bit draw leaves the other half buffered");
        let resumed = Generator::from_state(g.state, g.inc);
        assert!(!resumed.has_uint32);
        assert_eq!(resumed.uinteger, 0);
        assert_eq!((resumed.state, resumed.inc), (g.state, g.inc));
    }
}
