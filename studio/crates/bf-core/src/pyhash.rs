//! CPython `hash(str)` under `PYTHONHASHSEED=0` (Task 5).
//!
//! The kit seeds its caption puff with `hash(line) % 997` (`reelkit/ui.py`), so frames depend on this value. CPython
//! 3.13 hashes a string with SipHash-1-3 (`Python/pyhash.c`, `siphash13`) over the bytes of the string's internal
//! storage, and `PYTHONHASHSEED=0` turns hash randomisation off by zeroing the key. The storage is one byte per
//! character when every character is below U+0100, two bytes little-endian per character when every character is
//! below U+10000, and four bytes little-endian otherwise; it is never the UTF-8 bytes (except for ASCII, where the two
//! agree). `studio/reference/pyhash/vectors.json` (what Python gave on the reference PC) checks it.

/// Python `hash(s)` for a `str`, as CPython 3.13 computes it with `PYTHONHASHSEED=0`: the empty string hashes to 0, a
/// result of -1 becomes -2 (-1 is CPython's error value).
pub fn str_hash(s: &str) -> i64 {
    let widest = s.chars().map(u32::from).max().unwrap_or(0);
    let width = if widest < 0x100 {
        1
    } else if widest < 0x1_0000 {
        2
    } else {
        4
    };
    let mut bytes = Vec::with_capacity(s.len() * width);
    for c in s.chars() {
        bytes.extend_from_slice(&u32::from(c).to_le_bytes()[..width]);
    }
    hash_bytes(&bytes)
}

/// CPython `_Py_HashBytes` with the default `Py_HASH_CUTOFF` of 0 (no small-input shortcut).
fn hash_bytes(bytes: &[u8]) -> i64 {
    if bytes.is_empty() {
        return 0;
    }
    // CPython casts the unsigned result to `Py_hash_t`: the same bits as a signed 64-bit integer.
    let x = siphash13(0, 0, bytes) as i64;
    if x == -1 { -2 } else { x }
}

/// One SipRound: CPython's `DOUBLE_ROUND` is `HALF_ROUND(v0, v1, v2, v3, 13, 16)` then
/// `HALF_ROUND(v2, v1, v0, v3, 17, 21)`.
fn sip_round(v: &mut [u64; 4]) {
    v[0] = v[0].wrapping_add(v[1]);
    v[2] = v[2].wrapping_add(v[3]);
    v[1] = v[1].rotate_left(13) ^ v[0];
    v[3] = v[3].rotate_left(16) ^ v[2];
    v[0] = v[0].rotate_left(32);
    v[2] = v[2].wrapping_add(v[1]);
    v[0] = v[0].wrapping_add(v[3]);
    v[1] = v[1].rotate_left(17) ^ v[2];
    v[3] = v[3].rotate_left(21) ^ v[0];
    v[2] = v[2].rotate_left(32);
}

/// CPython `siphash13(k0, k1, src, src_sz)`: one compression round per little-endian 8-byte word, the tail bytes
/// and the length (its low byte, in the top byte) as the last word, then three finalisation rounds.
fn siphash13(k0: u64, k1: u64, src: &[u8]) -> u64 {
    let mut v = [
        k0 ^ 0x736f_6d65_7073_6575,
        k1 ^ 0x646f_7261_6e64_6f6d,
        k0 ^ 0x6c79_6765_6e65_7261,
        k1 ^ 0x7465_6462_7974_6573,
    ];
    let (words, tail) = src.as_chunks::<8>();
    for w in words {
        let mi = u64::from_le_bytes(*w);
        v[3] ^= mi;
        sip_round(&mut v);
        v[0] ^= mi;
    }
    let mut b = (src.len() as u64) << 56;
    for (i, &byte) in tail.iter().enumerate() {
        b |= u64::from(byte) << (8 * i);
    }
    v[3] ^= b;
    sip_round(&mut v);
    v[0] ^= b;
    v[2] ^= 0xff;
    sip_round(&mut v);
    sip_round(&mut v);
    sip_round(&mut v);
    v[0] ^ v[1] ^ v[2] ^ v[3]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::py::imod_py;
    use serde_json::Value;

    fn vectors() -> Value {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference/pyhash/vectors.json");
        let text = std::fs::read_to_string(path).expect("studio/reference/pyhash/vectors.json");
        serde_json::from_str(&text).expect("valid JSON")
    }

    /// Checks every `[string, hash, hash % 997]` row of one list and returns how many there were.
    fn check_rows(rows: &Value) -> usize {
        let rows = rows.as_array().expect("a list of rows");
        for row in rows {
            let s = row[0].as_str().expect("the string");
            let want: i64 = row[1].as_str().expect("the hash as text").parse().expect("an i64");
            let want_mod = row[2].as_i64().expect("hash % 997");
            let got = str_hash(s);
            assert_eq!(got, want, "hash({s:?})");
            assert_eq!(imod_py(got, 997), want_mod, "hash({s:?}) % 997");
        }
        rows.len()
    }

    #[test]
    fn every_ascii_vector_matches_python() {
        assert_eq!(check_rows(&vectors()["rows"]), 500);
    }

    #[test]
    fn every_wider_vector_matches_python() {
        assert_eq!(check_rows(&vectors()["non_ascii"]), 12);
    }

    #[test]
    fn empty_string_hashes_to_zero() {
        assert_eq!(str_hash(""), 0);
    }

    /// Computed on the reference PC with the pinned CPython 3.13.18 and `PYTHONHASHSEED=0`: strings of each storage
    /// width (1, 2 and 4 bytes per character, with the first character of the wider two) and a caption that mixes
    /// ASCII with an emoji (the whole string then stores 4 bytes per character).
    #[test]
    fn each_storage_width_matches_the_pinned_interpreter() {
        assert_eq!(str_hash("é"), 6_047_309_291_227_476_195);
        assert_eq!(str_hash("日本"), 6_243_316_497_235_261_705);
        assert_eq!(str_hash("\u{1f447}"), -8_737_224_579_150_811_114);
        assert_eq!(
            str_hash("Comment the next match-up \u{1f447}"),
            -2_367_232_345_773_644_760
        );
        assert_eq!(str_hash("\u{100}"), 75_343_234_424_780_393);
        assert_eq!(str_hash("\u{10000}"), 8_244_560_961_346_649_228);
    }
}
