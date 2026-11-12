//! The two Python hypots, held to the bit (ruling M3-R35): `dmath::np_hypot` is what numpy 2.5.3's `np.hypot`
//! computes on the reference PC and `dmath::py_hypot` is what CPython 3.13's `math.hypot` computes. The table is a
//! sample of the 560,018 pairs both were measured on: the edge cases a fight or a look can meet (zeros, equal
//! components, a zero component, tiny and huge pairs, subnormals, infinities, NaN), 40 ordinary pairs on which
//! the two differ, 20 ordinary pairs drawn without choosing, and pairs around the points where a square leaves
//! the normal range.

use bf_core::dmath;

// (x, y, np.hypot(x, y), math.hypot(x, y)) as bits; CPython 3.13.18 and numpy 2.5.3 on the reference PC
#[rustfmt::skip]
const TABLE: [(u64, u64, u64, u64); 116] = [
    (0x0000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000),
    (0x8000000000000000, 0x0000000000000000, 0x0000000000000000, 0x0000000000000000),
    (0x0000000000000000, 0x8000000000000000, 0x0000000000000000, 0x0000000000000000),
    (0x4008000000000000, 0x4010000000000000, 0x4014000000000000, 0x4014000000000000),
    (0x4004cccccccccccd, 0x0000000000000000, 0x4004cccccccccccd, 0x4004cccccccccccd),
    (0x0000000000000000, 0xc004cccccccccccd, 0x4004cccccccccccd, 0x4004cccccccccccd),
    (0x3d719799812dea11, 0x3d719799812dea11, 0x3d78e10d3a69204b, 0x3d78e10d3a69204b),
    (0x3d719799812dea11, 0x0000000000000000, 0x3d719799812dea11, 0x3d719799812dea11),
    (0x3fd7ae147ae147ae, 0x3fd7ae147ae147ae, 0x3fe0be89b23f6ed0, 0x3fe0be89b23f6ed0),
    (0xbff4000000000000, 0x3ff4000000000000, 0x3ffc48c6001f0ac0, 0x3ffc48c6001f0ac0),
    (0x0000000000000001, 0x0000000000000000, 0x0000000000000001, 0x0000000000000001),
    (0x0000000000000001, 0x0000000000000001, 0x0000000000000001, 0x0000000000000001),
    (0x16687e92154ef7ac, 0x16687e92154ef7ac, 0x167151f68876f410, 0x167151f68876f410),
    (0x6974e718d7d7625a, 0x6974e718d7d7625a, 0x697d8f9811335b57, 0x697d8f9811335b57),
    (0x1eb67e9c127b6e74, 0x1e9afe54e2fa848b, 0x1eb77c2a3caeb721, 0x1eb77c2a3caeb721),
    (0x200016e0ebac2efa, 0x1ff573d68f903ea2, 0x20035646a76700c0, 0x20035646a76700c0),
    (0x5fef06d5a4bf631f, 0x5feca3d8e6d80cbb, 0x5ff51cbbf16afe40, 0x5ff51cbbf16afe40),
    (0x7e37e43c8800759c, 0x01a56e1fc2f8f359, 0x7e37e43c8800759c, 0x7e37e43c8800759c),
    (0x3ff0000000000000, 0x3c670ef54646d497, 0x3ff0000000000000, 0x3ff0000000000000),
    (0x7ff0000000000000, 0x3ff0000000000000, 0x7ff0000000000000, 0x7ff0000000000000),
    (0x7ff8000000000000, 0x3ff0000000000000, 0x7ff8000000000000, 0x7ff8000000000000),
    (0x7ff0000000000000, 0x7ff8000000000000, 0x7ff0000000000000, 0x7ff0000000000000),
    (0xfff0000000000000, 0x7ff8000000000000, 0x7ff0000000000000, 0x7ff0000000000000),
    (0xbfebdb1a6a44751c, 0x3ffbe3c6c2cfc828, 0x3fff2c9301f8ea23, 0x3fff2c9301f8ea22),
    (0x3fb746ad04b545e0, 0x3fff4933adbde04c, 0x3fff51db0bc0448c, 0x3fff51db0bc0448d),
    (0xbff552595030e7af, 0xbffa45a38811f8ca, 0x4000eaedddb1a239, 0x4000eaedddb1a23a),
    (0xbffbe563b00362b2, 0x4001fb6162dce448, 0x4006c1e35917f3ec, 0x4006c1e35917f3eb),
    (0x3fd8563fda3131d0, 0xbfd9d1b10f9b7f78, 0x3fe1bd96246e6cdf, 0x3fe1bd96246e6ce0),
    (0xbffa1f53a456edd2, 0xbfa52a433184ef80, 0x3ffa21784eceb88f, 0x3ffa21784eceb88e),
    (0xbfdea5525ff63d70, 0xbf9fc436a6250c80, 0x3fdeb5c4c27e6c05, 0x3fdeb5c4c27e6c06),
    (0x3fdda2db7eb27550, 0xbfe01377218bfe50, 0x3fe5dd1117b172f0, 0x3fe5dd1117b172ef),
    (0x40036d00eec8623e, 0x400781e0c6654da2, 0x400e7ec484ea39e4, 0x400e7ec484ea39e3),
    (0x3ff616112b234c08, 0xbff1d5489fb94067, 0x3ffc63119f72b553, 0x3ffc63119f72b554),
    (0xc006dd9aceb3062c, 0xbff36b7e0ddc657d, 0x4008d787b0c7fcbe, 0x4008d787b0c7fcbf),
    (0x3fd895cd4a32f450, 0x4002bdf6265b7a3c, 0x4002fe089e61758a, 0x4002fe089e617589),
    (0x3ffe2ad665a66e08, 0xc001880d20d93c8c, 0x40072092225b41bf, 0x40072092225b41be),
    (0xbffb48bb15d9a2d9, 0x3fecd4f64c17aa90, 0x3ffedbc10c56835a, 0x3ffedbc10c568359),
    (0x3fe07da434093400, 0xbff9149bb965313e, 0x3ffa66ae73afd0ea, 0x3ffa66ae73afd0eb),
    (0xbff446a80c0e1303, 0xbfeb9afe6fc0c8f0, 0x3ff8873681e01b77, 0x3ff8873681e01b78),
    (0x3ffa573668116ef0, 0x3fed33ed384ab660, 0x3ffe1df277f47c58, 0x3ffe1df277f47c57),
    (0xc0043cf4289666b9, 0xbffba277e118d5bb, 0x4008814d16cc179d, 0x4008814d16cc179c),
    (0x3fea5f864516f824, 0x3fc8eb81c1c5cce0, 0x3feb195790811721, 0x3feb195790811722),
    (0x3fdcccc07047bf00, 0x4006d70616b511b2, 0x40071f353b28b25a, 0x40071f353b28b259),
    (0xc007445512b115e2, 0xbffc80ada05d60de, 0x400b48dc3a7cb396, 0x400b48dc3a7cb395),
    (0xc002b6b24f400316, 0x4003f6df10305ac6, 0x400b5d200ce8fa9e, 0x400b5d200ce8fa9d),
    (0x3fe76ff8ceeaf010, 0x3ff707ccb694e428, 0x3ff9d7297e2c4b32, 0x3ff9d7297e2c4b33),
    (0x40075e4b16e30490, 0x3fa1ccdd4e4beb40, 0x40075eb78e8977e5, 0x40075eb78e8977e6),
    (0x3ff7a945af2e4d4c, 0xbf924b0544a9f800, 0x3ff7a9b6d28e9e55, 0x3ff7a9b6d28e9e56),
    (0xc0029d20a9ad06ac, 0xc001e8f25e83d5eb, 0x4009d4ba7c579aa0, 0x4009d4ba7c579aa1),
    (0xc00351bbdd05ad5a, 0xc003e724bb69e346, 0x400bbcc228d341b7, 0x400bbcc228d341b6),
    (0x3fbb8b95f422b260, 0x3ff7c655390a2f04, 0x3ff7d644d7b76165, 0x3ff7d644d7b76166),
    (0xbfe3722ab5c3b5a0, 0x3ff1822b65f515d0, 0x3ff406ed4c52c50a, 0x3ff406ed4c52c50b),
    (0xc0044cfb7fe8da3e, 0xbfe0d8fbf7c300f0, 0x4004bba8a0675a17, 0x4004bba8a0675a16),
    (0xc006d8f2e1d815a6, 0x3feecaa3366dd68c, 0x40081c0267de447f, 0x40081c0267de4480),
    (0xbff950eb8f725fb2, 0x3fee2aead5a276e0, 0x3ffd781542d4d9e9, 0x3ffd781542d4d9ea),
    (0xbff82138cba45691, 0x4004a773181c3c46, 0x4007eb7336eeac15, 0x4007eb7336eeac16),
    (0xbff19e39102919e2, 0x3ff524ba6e20dc8c, 0x3ffb858c201409ea, 0x3ffb858c201409e9),
    (0xc00468885d50f8f8, 0x3ff70d7731807c28, 0x40077036bbf97f95, 0x40077036bbf97f96),
    (0x3ffb67b73676d04c, 0xbff30d90735f4c51, 0x4000b0521eefb760, 0x4000b0521eefb761),
    (0x3ff334d2c9c8eab8, 0xbfc53fb3055668a0, 0x3ff3639e24b71553, 0x3ff3639e24b71552),
    (0x3ff87b34a7635388, 0xbffaed5f2136aa24, 0x4002323af6967783, 0x4002323af6967782),
    (0x4002fa4a15aa06bc, 0x3ff638c38c3f5a78, 0x4005fdb257bf4e51, 0x4005fdb257bf4e50),
    (0xbffd489da034545b, 0x40020d2a7c902a1a, 0x40073e357e8c8e3a, 0x40073e357e8c8e39),
    (0xbff0caa2610a9a25, 0x3fff5fa20829ce34, 0x4001cad0d2096b10, 0x4001cad0d2096b10),
    (0x3fff73a7518eab50, 0x40027968191c7be0, 0x400842d7c2c08daf, 0x400842d7c2c08db0),
    (0xbff727043f0c57fc, 0x400050454847eba6, 0x400400e66cd5aa4a, 0x400400e66cd5aa4a),
    (0xbff9df0d09b77ee1, 0xbfe34e93db582650, 0x3ffb9d17bc66c84b, 0x3ffb9d17bc66c84b),
    (0xbffef1427634fa92, 0xbfb85ffad81650c0, 0x3ffefadabfca2221, 0x3ffefadabfca2221),
    (0xc0046402c9a8db58, 0x4005edad319268fa, 0x400df1a130da52d2, 0x400df1a130da52d2),
    (0xc007604c31e1da7b, 0xbf94e9cd5223f800, 0x400760719d3bea37, 0x400760719d3bea37),
    (0x40003a83d65ed344, 0xc001a3a818ef87ec, 0x4007f80da26c0617, 0x4007f80da26c0617),
    (0x3fe2a4bfc2c44000, 0x3fc7df12f49f0880, 0x3fe3934ee8c2c21d, 0x3fe3934ee8c2c21d),
    (0x3ff8b937f6d353c8, 0x3ff310aa1457b99c, 0x3fff387d626e2e3b, 0x3fff387d626e2e3b),
    (0x3fff3387ee9e8334, 0x3ffc27bd46a80f14, 0x40050368a9ca5b2b, 0x40050368a9ca5b2b),
    (0x3fdfd176cd6d7230, 0xbfecce8f6c197b80, 0x3ff074390c0c4827, 0x3ff074390c0c4827),
    (0xc002d7eb9f1ee5a0, 0xc0002301c81ffb1a, 0x4008cf048b734a3a, 0x4008cf048b734a3a),
    (0x4001d1660f95d998, 0xbff66a7b09b8a94a, 0x40050cc6717bc181, 0x40050cc6717bc181),
    (0x4002e9d94f9bfa1e, 0xbff29d9f3da4d99d, 0x4005146a18e41257, 0x4005146a18e41257),
    (0xc005406cddb13bf2, 0xbff42905985669fe, 0x4007856685472580, 0x4007856685472580),
    (0xbfe625a7a8fb8dc8, 0xc00305fa952b4307, 0x4003d00e6ca952d5, 0x4003d00e6ca952d5),
    (0xbfdaf8bef0207ab0, 0x3ffc9a3eb1ff94c0, 0x3ffd62f6d4d7a782, 0x3ffd62f6d4d7a782),
    (0xbfe08becc9568b00, 0xbfa33faac8364c00, 0x3fe0971b31fe9e2b, 0x3fe0971b31fe9e2b),
    (0x4007f0ec08f655b8, 0xbffb544eab43d8f2, 0x400b90f733ec0f8a, 0x400b90f733ec0f8a),
    (0x0000000000000002, 0x0000000000000002, 0x0000000000000003, 0x0000000000000003),
    (0x0000000000000002, 0x0000000000000002, 0x0000000000000003, 0x0000000000000003),
    (0x0000000000000001, 0x0000000000000002, 0x0000000000000002, 0x0000000000000002),
    (0x000000000000001f, 0x0000000000000003, 0x000000000000001f, 0x000000000000001f),
    (0x0000000000000016, 0x000000000000000d, 0x000000000000001a, 0x000000000000001a),
    (0x0000000000000020, 0x0000000000000003, 0x0000000000000020, 0x0000000000000020),
    (0x00000007be249dd3, 0x0000000375e873a8, 0x000000087b1c7a33, 0x000000087b1c7a33),
    (0x00000004ab802433, 0x00000000fcf5a6e6, 0x00000004c5f842d8, 0x00000004c5f842d8),
    (0x00000007039a2551, 0x000000025541c40a, 0x000000076453cd86, 0x000000076453cd86),
    (0x0011c8d743cc8d22, 0x0006e5fa58042874, 0x00131357c94776cc, 0x00131357c94776cc),
    (0x001985dc22552d74, 0x000ca055019b64ea, 0x001c79aba4882ec1, 0x001c79aba4882ec1),
    (0x001c50cf8f01da19, 0x00022ef550a2db1e, 0x001c66545e74fdaa, 0x001c66545e74fdaa),
    (0x143c31b8c11e6b8c, 0x1438a9d7e77276ee, 0x1442bacac643de17, 0x1442bacac643de17),
    (0x14358e0bdc14bed7, 0x143825a8b1efc094, 0x14402f1e7bfe0c33, 0x14402f1e7bfe0c33),
    (0x143dbdd51286b3eb, 0x143e1c462ea099f4, 0x144529491cbce279, 0x144529491cbce279),
    (0x1ff713108d0086b8, 0x1ffd7f1877bfaa62, 0x2002b98c9ae783a2, 0x2002b98c9ae783a2),
    (0x1ff9a3482681247d, 0x1fedaebe389f010c, 0x1ffd9fa8d98fb114, 0x1ffd9fa8d98fb114),
    (0x1ff3918e13a5a2e0, 0x1fd80baf231e1e0c, 0x1ff4789a59e66cdb, 0x1ff4789a59e66cdb),
    (0x20083e1300979a66, 0x1fee4e24a52361d8, 0x20096616af9efa0f, 0x20096616af9efa0f),
    (0x2003c28479c993aa, 0x1ffbfb527eb7caf6, 0x20083622eecabb22, 0x20083622eecabb22),
    (0x2006b69a7e5d4352, 0x2007bfe679cb7450, 0x20106e635691ec43, 0x20106e635691ec42),
    (0x5fdacb58f88de870, 0x5faabfe4b0a0d2e8, 0x5fdb008d10f60bd9, 0x5fdb008d10f60bd9),
    (0x5fdef134e353ce7c, 0x5fcb75f18d216e8e, 0x5fe0ed074552a7db, 0x5fe0ed074552a7db),
    (0x5fda220254ec85fc, 0x5fa9660d2bd85404, 0x5fda5332c96d3559, 0x5fda5332c96d3559),
    (0x5fe48c310269cb68, 0x5fd3b1573bf8912a, 0x5fe6c8f51e9f5d69, 0x5fe6c8f51e9f5d69),
    (0x5fe0f1e509ea2474, 0x5fc2b14ec0afb609, 0x5fe193d6238a5a2a, 0x5fe193d6238a5a2a),
    (0x5feb557af738d4d2, 0x5fcadd3ae5344ec6, 0x5fec259afc57bf30, 0x5fec259afc57bf30),
    (0x6bb29c531c4de92e, 0x6bb9e29bc801ede8, 0x6bbfe18a281f9ca7, 0x6bbfe18a281f9ca7),
    (0x6bb685c05c230b7a, 0x6b896c677ec08eec, 0x6bb6bedd5f69fade, 0x6bb6bedd5f69fade),
    (0x6bbdccbdc5692c12, 0x6bb4249933c36c4c, 0x6bc1fc0683adb0cc, 0x6bc1fc0683adb0cc),
    (0x7fd5fd8f52975985, 0x7fa7309e9ed159b2, 0x7fd62e424141f234, 0x7fd62e424141f234),
    (0x7fdfb03450351738, 0x7fb359d2694168a2, 0x7fe00718ebc45702, 0x7fe00718ebc45702),
    (0x7fd29921382fcae0, 0x7fd2534c7f0d0528, 0x7fda1c0f095077db, 0x7fda1c0f095077db),
];

fn same(got: f64, want: f64) -> bool {
    got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan())
}

#[test]
fn np_hypot_is_numpys_and_py_hypot_is_cpythons() {
    let (mut differ, mut plain, mut musl_np, mut musl_py) = (0, 0, 0, 0);
    for (x, y, np, py) in TABLE {
        let (x, y, np, py) = (
            f64::from_bits(x),
            f64::from_bits(y),
            f64::from_bits(np),
            f64::from_bits(py),
        );
        let (got_np, got_py) = (dmath::np_hypot(x, y), dmath::py_hypot(x, y));
        assert!(
            same(got_np, np),
            "np.hypot({x:e}, {y:e}) = {np:e}, np_hypot gave {got_np:e}"
        );
        assert!(
            same(got_py, py),
            "math.hypot({x:e}, {y:e}) = {py:e}, py_hypot gave {got_py:e}"
        );
        // either argument order, either sign
        assert!(same(dmath::np_hypot(y, -x), np) && same(dmath::py_hypot(-y, x), py));
        differ += usize::from(!same(np, py));
        plain += usize::from(same((x * x + y * y).sqrt(), np));
        musl_np += usize::from(same(dmath::hypot(x, y), np));
        musl_py += usize::from(same(dmath::hypot(x, y), py));
    }
    // the sample really tells the functions apart: the two Pythons, the plain root outside the normal range, and
    // musl's hypot from both
    assert!(differ >= 40, "numpy's and CPython's hypot differ on {differ} rows");
    assert!(
        plain < TABLE.len(),
        "the plain root is numpy's on every row: the scaled rows are gone"
    );
    assert!(
        musl_np < TABLE.len() && musl_py < TABLE.len(),
        "dmath::hypot equals a Python hypot on every row"
    );
}

/// In the range of a fight's numbers numpy's hypot is exactly the plain root of the sum of the squares.
#[test]
fn np_hypot_is_the_plain_root_where_no_square_leaves_the_normal_range() {
    let mut s = 0x9E37_79B9_7F4A_7C15u64;
    let mut unit = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..200_000 {
        let (x, y) = (unit() * 6.0 - 3.0, unit() * 6.0 - 3.0);
        assert_eq!(dmath::np_hypot(x, y).to_bits(), (x * x + y * y).sqrt().to_bits());
        assert_eq!(dmath::np_hypot(x, 0.0).to_bits(), x.abs().to_bits());
    }
}
