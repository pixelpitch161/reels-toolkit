// numpy's own float64 tanh (`np.tanh`), ported from numpy v2.5.3's
// numpy/_core/src/umath/loops_hyperbolic.dispatch.cpp.src (`simd_tanh_f64` and its table `lut18x16`). The
// notice of the source follows (numpy's LICENSE.txt).
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

// This file is not a module of its own: `dmath.rs` includes it twice, beside a software `fma` and beside the
// processor's, so the two copies run the same operations in the same order.

/// numpy's tanh: |x| picks one of 16 intervals (by its exponent and top mantissa bits), and a polynomial of degree
/// 16 in |x| - b, evaluated by Horner's rule with fused multiply-adds, gives the value; the sign is put back last.
#[inline(always)]
pub(super) fn np_tanh(x: f64) -> f64 {
    let bits = x.to_bits();
    if x.is_nan() {
        return f64::NAN;
    }
    let nd = bits & 0x7ff8_0000_0000_0000;
    let hi = ((nd >> 32) as i32).wrapping_sub(0x3fc0_0000).clamp(0, 0x78_0000);
    let row = 18 * (hi >> 19) as usize;
    let c = |k: usize| f64::from_bits(NP_TANH[row + k]);
    let y = x.abs() - c(0);
    let mut r = fma(c(17), y, c(16));
    let mut k = 15;
    while k >= 1 {
        r = fma(r, y, c(k));
        k -= 1;
    }
    if nd > 0x7fe0_0000_0000_0000 {
        r = 1.0;
    }
    f64::from_bits(r.to_bits() | (bits & 0x8000_0000_0000_0000))
}

/// numpy's `lut18x16`, copied from its source: 16 intervals of |x|, each `b, c0 .. c16` (b is where the interval
/// starts, c0 .. c16 the polynomial's coefficients).
#[rustfmt::skip]
static NP_TANH: [u64; 288] = [
    // 0
    0x0000000000000000, 0x0000000000000000, 0x3ff0000000000000, 0xbbf0b3ea3fdfaa19,
    0xbfd5555555555555, 0xbce6863ee44ed636, 0x3fc1111111112ab5, 0xbda1ea19ddddb3b4,
    0xbfaba1ba1990520b, 0xbe351ca7f096011f, 0x3f9664f94e6ac14e, 0xbea8c4c1fd7852fe,
    0xbf822404577aa9dd, 0xbefdd99a221ed573, 0x3f6e3be689423841, 0xbf2a1306713a4f3a,
    0xbf55d7e76dc56871, 0x3f35e67ab76a26e7,
    // 1
    0x3fcc000000000000, 0x3fcb8fd0416a7c92, 0x3fee842ca3f08532, 0xbfca48aaeb53bc21,
    0xbfd183afc292ba11, 0x3fc04dcd0476c75e, 0x3fb5c19efdfc08ad, 0xbfb0b8df995ce4df,
    0xbf96e37bba52f6fc, 0x3f9eaaf3320c3851, 0xbf94d3343bae39dd, 0xbfccce16b1046f13,
    0x403d8b07f7a82aa3, 0x4070593a3735bab4, 0xc0d263511f5baac1, 0xc1045e509116b066,
    0x41528c38809c90c7, 0x41848ee0627d8206,
    // 2
    0x3fd4000000000000, 0x3fd35f98a0ea650e, 0x3fed11574af58f1b, 0xbfd19921f4329916,
    0xbfcc1a4b039c9bfa, 0x3fc43d3449a80f08, 0x3fa74c98dc34fbac, 0xbfb2955cf41e8164,
    0x3ecff7df18455399, 0x3f9cf823fe761fc1, 0xbf7bc748e60df843, 0xbf81a16f224bb7b6,
    0xbf9f44ab92fbab0a, 0xbfccab654e44835e, 0x40169f73b15ebe5c, 0x4041fab9250984ce,
    0xc076d57fb5190b02, 0xc0a216d618b489ec,
    // 3
    0x3fdc000000000000, 0x3fda5729ee488037, 0x3fea945b9c24e4f9, 0xbfd5e0f09bef8011,
    0xbfc16e1e6d8d0be6, 0x3fc5c26f3699b7e7, 0xbf790d6a8eff0a77, 0xbfaf9d05c309f7c6,
    0x3f97362834d33a4e, 0x3f9022271754ff1f, 0xbf8c89372b43ba85, 0xbf62cbf00406bc09,
    0x3fb2eac604473d6a, 0x3fd13ed80037dbac, 0xc025c1dd41cd6cb5, 0xc0458d090ec3de95,
    0x4085f09f888f8ada, 0x40a5b89107c8af4f,
    // 4
    0x3fe4000000000000, 0x3fe1bf47eabb8f95, 0x3fe6284c3374f815, 0xbfd893b59c35c882,
    0xbf92426c751e48a2, 0x3fc1a686f6ab2533, 0xbfac3c021789a786, 0xbf987d27ccff4291,
    0x3f9e7f8380184b45, 0xbf731fe77c9c60af, 0xbf8129a092de747a, 0x3f75b29bb02cf69b,
    0x3f45f87d903aaac8, 0xbf6045b9076cc487, 0xbf58fd89fe05e0d1, 0xbf74949d60113d63,
    0x3fa246332a2fcba5, 0x3fb69d8374520eda,
    // 5
    0x3fec000000000000, 0x3fe686650b8c2015, 0x3fe02500a09f8d6e, 0xbfd6ba7cb7576538,
    0x3fb4f152b2bad124, 0x3faf203c316ce730, 0xbfae2196b7326859, 0x3f8b2ca62572b098,
    0x3f869543e7c420d4, 0xbf84a6046865ec7d, 0x3f60c85b4d538746, 0x3f607df0f9f90c17,
    0xbf5e104671036300, 0x3f2085ee7e8ac170, 0x3f73f7af01d5af7a, 0x3f7c9fd6200d0ade,
    0xbfb29d851a896fcd, 0xbfbded519f981716,
    // 6
    0x3ff4000000000000, 0x3feb2523bb6b2dee, 0x3fd1f25131e3a8c0, 0xbfce7291743d7555,
    0x3fbbba40cbef72be, 0xbf89c7a02788557c, 0xbf93a7a011ff8c2a, 0x3f8f1cf6c7f5b00a,
    0xbf7326bd4914222a, 0xbf4ca3f1f2b9192b, 0x3f5be9392199ec18, 0xbf4b852a6e0758d5,
    0x3f19bc98ddf0f340, 0x3f23524622610430, 0xbf1e40bdead17e6b, 0x3f02cd40e0ad0a9f,
    0x3ed9065ae369b212, 0xbef02d288b5b3371,
    // 7
    0x3ffc000000000000, 0x3fee1fbf97e33527, 0x3fbd22ca1c24a139, 0xbfbb6d85a01efb80,
    0x3fb01ba038be6a3d, 0xbf98157e26e0d541, 0x3f6e4709c7e8430e, 0x3f60379811e43dd5,
    0xbf5fc15b0a9d98fa, 0x3f4c77dee0afd227, 0xbf2a0c68a4489f10, 0xbf0078c63d1b8445,
    0x3f0d4304bc9246e8, 0xbeff12a6626911b4, 0x3ee224cd6c4513e5, 0xbe858ab8e019f311,
    0xbeb8e1ba4c98a030, 0x3eb290981209c1a6,
    // 8
    0x4004000000000000, 0x3fef9258260a71c2, 0x3f9b3afe1fba5c76, 0xbf9addae58c7141a,
    0x3f916df44871efc8, 0xbf807b55c1c7d278, 0x3f67682afa611151, 0xbf4793826f78537e,
    0x3f14cffcfa69fbb6, 0x3f04055bce68597a, 0xbf00462601dc2faa, 0x3eec12eadd55be7a,
    0xbed13c415f7b9d41, 0x3eab9008bca408af, 0xbe24b645e68eeaa3, 0xbe792fa6323b7cf8,
    0x3e6ffd0766ad4016, 0xbe567e924bf5ff6e,
    // 9
    0x400c000000000000, 0x3feff112c63a9077, 0x3f6dd37d19b22b21, 0xbf6dc59376c7aa19,
    0x3f63c6869dfc8870, 0xbf53a18d5843190f, 0x3f3ef2ee77717cbf, 0xbf2405695e36240f,
    0x3f057e48e5b79d10, 0xbee2bf0cb4a71647, 0x3eb7b6a219dea9f4, 0xbe6fa600f593181b,
    0xbe722b8d9720cdb0, 0x3e634df71865f620, 0xbe4abfebfb72bc83, 0x3e2df04d67876402,
    0xbe0c63c29f505f5b, 0x3de3f7f7de6b0eb6,
    // 10
    0x4014000000000000, 0x3fefff419668df11, 0x3f27ccec13a9ef96, 0xbf27cc5e74677410,
    0x3f1fb9aef915d828, 0xbf0fb6bbc89b1a5b, 0x3ef95a4482f180b7, 0xbee0e08de39ce756,
    0x3ec33b66d7d77264, 0xbea31eaafe73efd5, 0x3e80cbcc8d4c5c8a, 0xbe5a3c935dce3f7d,
    0x3e322666d739bec0, 0xbe05bb1bcf83ca73, 0x3dd51c38f8695ed3, 0xbd95c72be95e4d2c,
    0xbd7fab216b9e0e49, 0x3d69ed18bae3ebbc,
    // 11
    0x401c000000000000, 0x3feffffc832750f2, 0x3ecbe6c3f33250ae, 0xbecbe6c0e8b4cc87,
    0x3ec299d1e27c6e11, 0xbeb299c9c684a963, 0x3e9dc2c27da3b603, 0xbe83d709ba5f714e,
    0x3e66ac4e578b9b10, 0xbe46abb02c4368ed, 0x3e2425bb231a5e29, 0xbe001c6d95e3ae96,
    0x3dd76a553d7e7918, 0xbdaf2ac143fb6762, 0x3d8313ac38c6832b, 0xbd55a89c30203106,
    0x3d2826b62056aa27, 0xbcf7534c4f3dfa71,
    // 12
    0x4024000000000000, 0x3feffffffdc96f35, 0x3e41b4865394f75f, 0xbe41b486526b0565,
    0x3e379b5ddcca334c, 0xbe279b5dd4fb3d01, 0x3e12e2afd9f7433e, 0xbdf92e3fc5ee63e0,
    0x3ddcc74b8d3d5c42, 0xbdbcc749ca8079dd, 0x3d9992a4beac8662, 0xbd74755a00ea1fd3,
    0x3d4de0fa59416a39, 0xbd23eae52a3dbf57, 0x3cf7787935626685, 0xbccad6b3bb9eff65,
    0x3ca313e31762f523, 0xbc730b73f1eaff20,
    // 13
    0x402c000000000000, 0x3fefffffffffcf58, 0x3d8853f01bda5f28, 0xbd8853f01bef63a4,
    0x3d8037f57bc62c9a, 0xbd7037f57ae72aa6, 0x3d59f320348679ba, 0xbd414cc030f2110e,
    0x3d23c589137f92b4, 0xbd03c5883836b9d2, 0x3ce191ba5ed3fb67, 0xbcbc1c6c063bb7ac,
    0x3c948716cf3681b4, 0xbc6b5e3e9ca0955e, 0x3c401ffc49c6bc29, 0xbc12705ccd3dd884,
    0x3bea37aa21895319, 0xbbba2cff8135d462,
    // 14
    0x4034000000000000, 0x3ff0000000000000, 0x3c73953c0197ef58, 0xbc73955be519be31,
    0x3c6a2d4b50a2cff7, 0xbc5a2ca2bba78e86, 0x3c44b61d9bbcc940, 0xbc2ba022e8d82a87,
    0x3c107f8e2c8707a1, 0xbbf07a5416264aec, 0x3bc892450bad44c4, 0xbba3be9a4460fe00,
    0x3b873f9f2d2fda99, 0xbb5eca68e2c1ba2e, 0xbabf0b21acfa52ab, 0xba8e0a4c47ae75f5,
    0x3ae5c7f1fd871496, 0xbab5a71b5f7d9035,
    // 15
    0x0000000000000000, 0x3ff0000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000,
    0x0000000000000000, 0x0000000000000000,
];
