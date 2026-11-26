//! `bf_core::rng::Generator` gives numpy's numbers, bit for bit.
//!
//! `studio/reference/rng/vectors.json` is written by `tools/studio_export/rng_vectors.py` from numpy 2.5.3. For each
//! seed (an integer for `default_rng(seed)`, a list for `default_rng([a, b, c])`) it holds how the seed becomes a
//! generator ("init") and a set of named streams. Every stream starts from a fresh generator; only "interleaved"
//! mixes methods on one generator. A last "resume" case gives a generator's state after 1,000 normals and the
//! normals that follow.
//!
//! `matches_numpy` replays all of it through the public API and compares every float by `to_bits()`. It fails when
//! the file holds a stream name, a stream key or a top-level key this test does not know, and when a known stream
//! is missing for a seed, so nothing in the file can go unchecked without this test being changed.
//!
//! Where a generator stands after a stream (the stream's "end") is checked through its behaviour: a buffered 32-bit
//! half must come out of the next 32-bit draw, and after that the generator must give the same 64-bit outputs as
//! one resumed with `Generator::from_state(end.state, end.inc)`. Raw outputs are read with `integers(2**32)`, which
//! numpy serves with one unmodified 32-bit draw: two of them are the low and the high half of one 64-bit output.

use bf_core::rng::Generator;
use serde_json::{Map, Value};

/// The reference file, relative to this crate.
const VECTORS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference/rng/vectors.json");

/// The numpy call a stream records.
#[derive(Clone, Copy, Debug)]
enum Call {
    /// `rng.random()`, once per value.
    Random,
    /// `rng.uniform(lo, hi)`, once per value.
    Uniform(f64, f64),
    /// `rng.normal(loc, scale)`, once per value.
    Normal(f64, f64),
    /// `rng.uniform(lo, hi, size)`: one array call.
    UniformArray(f64, f64),
    /// `rng.normal(loc, scale, size)`: one array call.
    NormalArray(f64, f64),
    /// `rng.integers(high)`, once per value.
    Integers(u64),
    /// `rng.permutation(n)`: one call.
    Permutation(usize),
    /// `rng.choice(items)`, once per value.
    Choice(&'static [i64]),
    /// Each round: `uniform(-1, 1)`, `normal(0, 1)`, `random()`, `integers(5)`, `permutation(4)` on one generator.
    Interleaved,
}

/// Every stream `streams()` in `rng_vectors.py` writes, with the call that made it.
const STREAMS: [(&str, Call); 22] = [
    ("random", Call::Random),
    ("uniform_m1_1", Call::Uniform(-1.0, 1.0)),
    ("uniform_5_9", Call::Uniform(5.0, 9.0)),
    ("normal_0_1", Call::Normal(0.0, 1.0)),
    ("normal_2_0p5", Call::Normal(2.0, 0.5)),
    ("normal_0_1_size1000", Call::NormalArray(0.0, 1.0)),
    ("uniform_0_1_size1000", Call::UniformArray(0.0, 1.0)),
    ("integers_7", Call::Integers(7)),
    ("integers_1000", Call::Integers(1000)),
    ("integers_2p40", Call::Integers(1 << 40)),
    ("integers_1", Call::Integers(1)),
    ("integers_2p31_plus_1", Call::Integers((1 << 31) + 1)),
    ("integers_2p32", Call::Integers(1 << 32)),
    ("integers_2p32_plus_1", Call::Integers((1 << 32) + 1)),
    ("integers_3x2p61", Call::Integers(3 << 61)),
    ("permutation_2", Call::Permutation(2)),
    ("permutation_5", Call::Permutation(5)),
    ("permutation_16", Call::Permutation(16)),
    ("permutation_100", Call::Permutation(100)),
    ("choice_m1_1", Call::Choice(&[-1, 1])),
    ("choice_5", Call::Choice(&[10, 20, 30, 40, 50])),
    ("interleaved", Call::Interleaved),
];

/// The keys a stream may have.
const STREAM_KEYS: [&str; 9] = ["call", "op", "args", "calls", "size", "values", "indices", "row", "end"];

/// The keys of the whole file.
const TOP_KEYS: [&str; 5] = ["about", "format", "numbers", "resume", "seeds"];

fn load() -> Value {
    let text = std::fs::read_to_string(VECTORS).unwrap_or_else(|e| panic!("cannot read {VECTORS}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{VECTORS} is not JSON: {e}"))
}

fn object<'a>(v: &'a Value, what: &str) -> &'a Map<String, Value> {
    v.as_object()
        .unwrap_or_else(|| panic!("{what}: expected an object, found {v}"))
}

fn array<'a>(v: &'a Value, what: &str) -> &'a [Value] {
    v.as_array()
        .unwrap_or_else(|| panic!("{what}: expected a list, found {v}"))
}

fn text<'a>(v: &'a Value, what: &str) -> &'a str {
    v.as_str()
        .unwrap_or_else(|| panic!("{what}: expected a string, found {v}"))
}

/// A whole number: a JSON integer, or a decimal string where the value can pass 2**53.
fn uint(v: &Value, what: &str) -> u64 {
    match v {
        Value::Number(n) => n.as_u64().unwrap_or_else(|| panic!("{what}: {n} is not a u64")),
        Value::String(s) => s.parse().unwrap_or_else(|e| panic!("{what}: {s:?} is not a u64: {e}")),
        _ => panic!("{what}: expected an integer, found {v}"),
    }
}

/// A signed whole number written as a JSON integer.
fn int(v: &Value, what: &str) -> i64 {
    v.as_i64()
        .unwrap_or_else(|| panic!("{what}: expected an integer, found {v}"))
}

/// A 128-bit number written as a decimal string.
fn wide(v: &Value, what: &str) -> u128 {
    let s = text(v, what);
    s.parse().unwrap_or_else(|e| panic!("{what}: {s:?} is not a u128: {e}"))
}

/// A plain JSON number (an argument such as `-1` or `0.5`) as an `f64`.
fn number(v: &Value, what: &str) -> f64 {
    v.as_f64()
        .unwrap_or_else(|| panic!("{what}: expected a number, found {v}"))
}

/// A Python `float.hex()` string as the `f64` with exactly those bits: `[-]0x1.<hex digits>p<exponent>`, with
/// `0x0.0p+0` for zero and a leading `0x0.` with exponent -1022 for subnormals.
fn hexf(s: &str) -> f64 {
    fn bad(s: &str) -> ! {
        panic!("not a float.hex() string: {s:?}")
    }
    let (negative, rest) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let Some(rest) = rest.strip_prefix("0x") else { bad(s) };
    let Some((mantissa, exponent)) = rest.split_once('p') else {
        bad(s)
    };
    let Some((lead, fraction)) = mantissa.split_once('.') else {
        bad(s)
    };
    if fraction.is_empty() || fraction.len() > 13 {
        bad(s);
    }
    let Ok(fraction_digits) = u64::from_str_radix(fraction, 16) else {
        bad(s)
    };
    let fraction_bits = fraction_digits << (4 * (13 - fraction.len()));
    let Ok(exponent) = exponent.parse::<i64>() else { bad(s) };
    let magnitude = match lead {
        "1" if (-1022..=1023).contains(&exponent) => (((exponent + 1023) as u64) << 52) | fraction_bits,
        "0" if exponent == -1022 || (exponent == 0 && fraction_bits == 0) => fraction_bits,
        _ => bad(s),
    };
    f64::from_bits(magnitude | (u64::from(negative) << 63))
}

/// Fails unless `got` has exactly the bits of the hex float `want`.
fn same_bits(got: f64, want: &Value, what: &str) {
    let want_text = text(want, what);
    let want = hexf(want_text);
    assert!(
        got.to_bits() == want.to_bits(),
        "{what}: got {got:e} (bits {:#018x}), numpy gives {want:e} = {want_text} (bits {:#018x})",
        got.to_bits(),
        want.to_bits()
    );
}

/// The seed of one entry: a JSON integer for `default_rng(seed)`, a list for `default_rng([a, b, c])`.
#[derive(Clone, Debug)]
enum Seed {
    Int(u64),
    List(Vec<u64>),
}

impl Seed {
    fn parse(v: &Value) -> Seed {
        match v {
            Value::Array(items) => Seed::List(items.iter().map(|x| uint(x, "a list seed's entry")).collect()),
            _ => Seed::Int(uint(v, "seed")),
        }
    }

    /// A fresh generator, as `np.random.default_rng(seed)` makes it.
    fn fresh(&self) -> Generator {
        match self {
            Seed::Int(seed) => Generator::new(*seed),
            Seed::List(seed) => Generator::from_seq(seed),
        }
    }
}

/// One raw 32-bit draw: `rng.integers(2**32)` is numpy's `next_uint32` and nothing else.
fn raw_u32(g: &mut Generator) -> u64 {
    let v = g.integers(1 << 32);
    assert!(v <= u64::from(u32::MAX), "integers(2**32) gave {v}");
    v
}

/// One raw 64-bit output, read as its low then its high 32-bit half. The generator must hold no buffered half.
fn raw_u64(g: &mut Generator) -> u64 {
    let low = raw_u32(g);
    let high = raw_u32(g);
    (high << 32) | low
}

/// Checks where `g` stands against a stream's "end": the buffered 32-bit half if numpy holds one, then the same
/// 64-bit outputs as a generator resumed from the end state. A generator that wrongly holds (or lacks) a buffered
/// half is off by half an output from the first comparison on.
fn check_end(g: &mut Generator, end: &Value, what: &str) {
    let end = object(end, what);
    let mut keys: Vec<&str> = end.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["has_uint32", "inc", "state", "uinteger"],
        "{what}: keys of \"end\""
    );
    match uint(&end["has_uint32"], what) {
        0 => {}
        1 => assert_eq!(
            raw_u32(g),
            uint(&end["uinteger"], what),
            "{what}: the buffered 32-bit half after the stream"
        ),
        other => panic!("{what}: has_uint32 is {other}"),
    }
    let mut resumed = Generator::from_state(wide(&end["state"], what), wide(&end["inc"], what));
    for k in 0..4 {
        assert_eq!(
            raw_u64(g),
            raw_u64(&mut resumed),
            "{what}: 64-bit output {k} after the stream, against a generator resumed from the end state"
        );
    }
}

/// Fails unless the stream's "op", "args" and counts are what `call` says, so a changed file cannot be replayed
/// with the wrong call. Returns the stream's values.
fn check_shape<'a>(stream: &'a Map<String, Value>, call: Call, what: &str) -> &'a [Value] {
    for key in stream.keys() {
        assert!(
            STREAM_KEYS.contains(&key.as_str()),
            "{what}: unknown stream key {key:?}"
        );
    }
    let op = text(&stream["op"], what);
    let args = array(&stream["args"], what);
    let calls = uint(&stream["calls"], what) as usize;
    let values = array(&stream["values"], what);
    let pair = |lo: f64, hi: f64| {
        assert_eq!(args.len(), 2, "{what}: args");
        assert_eq!(
            (number(&args[0], what), number(&args[1], what)),
            (lo, hi),
            "{what}: args"
        );
    };
    let want_op = match call {
        Call::Random => {
            assert!(args.is_empty(), "{what}: args");
            "random"
        }
        Call::Uniform(lo, hi) | Call::UniformArray(lo, hi) => {
            pair(lo, hi);
            "uniform"
        }
        Call::Normal(loc, scale) | Call::NormalArray(loc, scale) => {
            pair(loc, scale);
            "normal"
        }
        Call::Integers(high) => {
            assert_eq!(args.len(), 1, "{what}: args");
            assert_eq!(uint(&args[0], what), high, "{what}: args");
            "integers"
        }
        Call::Permutation(n) => {
            assert_eq!(args.len(), 1, "{what}: args");
            assert_eq!(uint(&args[0], what) as usize, n, "{what}: args");
            "permutation"
        }
        Call::Choice(items) => {
            assert_eq!(args.len(), 1, "{what}: args");
            let listed: Vec<i64> = array(&args[0], what).iter().map(|v| int(v, what)).collect();
            assert_eq!(listed, items, "{what}: args");
            "choice"
        }
        Call::Interleaved => {
            assert!(args.is_empty(), "{what}: args");
            "interleaved"
        }
    };
    assert_eq!(op, want_op, "{what}: op");
    match call {
        Call::UniformArray(..) | Call::NormalArray(..) => {
            assert_eq!(calls, 1, "{what}: an array stream is one call");
            assert_eq!(
                uint(&stream["size"], what) as usize,
                values.len(),
                "{what}: size against the number of values"
            );
        }
        Call::Permutation(n) => {
            assert_eq!(calls, 1, "{what}: a permutation stream is one call");
            assert_eq!(values.len(), n, "{what}: number of values");
            assert!(!stream.contains_key("size"), "{what}: unexpected \"size\"");
        }
        _ => {
            assert_eq!(calls, values.len(), "{what}: calls against the number of values");
            assert!(!stream.contains_key("size"), "{what}: unexpected \"size\"");
        }
    }
    assert!(!values.is_empty(), "{what}: no values");
    assert_eq!(
        stream.contains_key("indices"),
        matches!(call, Call::Choice(_)),
        "{what}: \"indices\" belongs to choice streams only"
    );
    values
}

/// Replays one stream on a fresh generator and compares every value, then where the generator stands.
fn check_stream(seed: &Seed, name: &str, call: Call, stream: &Value) {
    let what = format!("seed {seed:?}, stream {name:?}");
    let stream = object(stream, &what);
    let values = check_shape(stream, call, &what);
    let mut g = seed.fresh();
    match call {
        Call::Random => {
            for (i, want) in values.iter().enumerate() {
                same_bits(g.random(), want, &format!("{what}, value {i}"));
            }
        }
        Call::Uniform(lo, hi) => {
            for (i, want) in values.iter().enumerate() {
                same_bits(g.uniform(lo, hi), want, &format!("{what}, value {i}"));
            }
        }
        Call::Normal(loc, scale) => {
            // `standard_normal()` is the same draw before numpy's `loc + scale * z`: a second generator checks it.
            let mut standard = seed.fresh();
            for (i, want) in values.iter().enumerate() {
                same_bits(g.normal(loc, scale), want, &format!("{what}, value {i}"));
                let z = standard.standard_normal();
                same_bits(
                    loc + scale * z,
                    want,
                    &format!("{what}, value {i}, as loc + scale * standard_normal()"),
                );
            }
        }
        Call::UniformArray(lo, hi) => {
            let mut out = vec![f64::NAN; values.len()];
            g.fill_uniform(lo, hi, &mut out);
            for (i, want) in values.iter().enumerate() {
                same_bits(out[i], want, &format!("{what}, value {i}"));
            }
        }
        Call::NormalArray(loc, scale) => {
            let mut out = vec![f64::NAN; values.len()];
            g.fill_normal(loc, scale, &mut out);
            for (i, want) in values.iter().enumerate() {
                same_bits(out[i], want, &format!("{what}, value {i}"));
            }
        }
        Call::Integers(high) => {
            for (i, want) in values.iter().enumerate() {
                let at = format!("{what}, value {i}");
                assert_eq!(g.integers(high), uint(want, &at), "{at}");
            }
        }
        Call::Permutation(n) => {
            let want: Vec<usize> = values.iter().map(|v| uint(v, &what) as usize).collect();
            assert_eq!(g.permutation(n), want, "{what}");
        }
        Call::Choice(items) => {
            let indices = array(&stream["indices"], &what);
            assert_eq!(indices.len(), values.len(), "{what}: indices against values");
            for (i, want) in indices.iter().enumerate() {
                let at = format!("{what}, value {i}");
                let index = g.choice_index(items.len());
                assert_eq!(index, uint(want, &at) as usize, "{at}: index");
                assert_eq!(items[index], int(&values[i], &at), "{at}: item");
            }
        }
        Call::Interleaved => {
            for (i, row) in values.iter().enumerate() {
                let at = format!("{what}, round {i}");
                let row = array(row, &at);
                assert_eq!(
                    row.len(),
                    8,
                    "{at}: a row is [uniform, normal, random, integers, 4 permuted values]"
                );
                same_bits(g.uniform(-1.0, 1.0), &row[0], &format!("{at}, uniform(-1, 1)"));
                same_bits(g.normal(0.0, 1.0), &row[1], &format!("{at}, normal(0, 1)"));
                same_bits(g.random(), &row[2], &format!("{at}, random()"));
                assert_eq!(g.integers(5), uint(&row[3], &at), "{at}, integers(5)");
                let want: Vec<usize> = row[4..].iter().map(|v| uint(v, &at) as usize).collect();
                assert_eq!(g.permutation(4), want, "{at}, permutation(4)");
            }
        }
    }
    check_end(&mut g, &stream["end"], &what);
}

/// Checks how a seed becomes a generator: the first raw 64-bit outputs, and that the recorded starting state
/// resumes to the same outputs.
fn check_init(seed: &Seed, init: &Value) {
    let what = format!("seed {seed:?}, init");
    let init = object(init, &what);
    let mut keys: Vec<&str> = init.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["inc", "raw_u64", "seed_sequence_u64", "state"], "{what}: keys");
    let raw: Vec<u64> = array(&init["raw_u64"], &what).iter().map(|v| uint(v, &what)).collect();
    assert!(!raw.is_empty(), "{what}: no raw outputs");
    let state = wide(&init["state"], &what);
    let inc = wide(&init["inc"], &what);

    // The four 64-bit words SeedSequence hands PCG64 fix the starting state (pcg64_set_seed in numpy's pcg64.c):
    // this ties "seed_sequence_u64" to "state" and "inc", which the outputs below then pin.
    let words: Vec<u128> = array(&init["seed_sequence_u64"], &what)
        .iter()
        .map(|v| u128::from(uint(v, &what)))
        .collect();
    assert_eq!(words.len(), 4, "{what}: seed_sequence_u64");
    let multiplier: u128 = (2549297995355413924_u128 << 64) + 4865540595714422341_u128;
    let want_inc = (((words[2] << 64) | words[3]) << 1) | 1;
    let want_state = want_inc
        .wrapping_add((words[0] << 64) | words[1])
        .wrapping_mul(multiplier)
        .wrapping_add(want_inc);
    assert_eq!(
        (state, inc),
        (want_state, want_inc),
        "{what}: state and inc from seed_sequence_u64"
    );

    let mut fresh = seed.fresh();
    let mut resumed = Generator::from_state(state, inc);
    for (i, &want) in raw.iter().enumerate() {
        assert_eq!(
            raw_u64(&mut fresh),
            want,
            "{what}: raw 64-bit output {i} of a fresh generator"
        );
        assert_eq!(
            raw_u64(&mut resumed),
            want,
            "{what}: raw 64-bit output {i} of from_state(state, inc)"
        );
    }

    // A one-entry list is the same seed as the bare integer (SeedSequence splits both into the same 32-bit words).
    if let Seed::Int(n) = seed {
        let mut listed = Generator::from_seq(&[*n]);
        for (i, &want) in raw.iter().enumerate() {
            assert_eq!(
                raw_u64(&mut listed),
                want,
                "{what}: raw 64-bit output {i} of from_seq(&[seed])"
            );
        }
    }
}

/// The resume case: a state taken after some normals, and the normals that follow.
fn check_resume(resume: &Value) {
    let what = "resume";
    let resume = object(resume, what);
    let mut keys: Vec<&str> = resume.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["has_uint32", "inc", "next", "seed", "skipped", "state", "uinteger"],
        "{what}: keys"
    );
    assert_eq!(
        uint(&resume["has_uint32"], what),
        0,
        "{what}: from_state resumes with no buffered 32-bit half"
    );
    let skipped = object(&resume["skipped"], what);
    assert_eq!(text(&skipped["call"], what), "rng.normal(0, 1)", "{what}: skipped call");
    let skip = uint(&skipped["calls"], what);
    let next = object(&resume["next"], what);
    let values = check_shape(next, Call::Normal(0.0, 1.0), "resume, next");

    let mut resumed = Generator::from_state(wide(&resume["state"], what), wide(&resume["inc"], what));
    let mut played = Generator::new(uint(&resume["seed"], what));
    for _ in 0..skip {
        played.normal(0.0, 1.0);
    }
    for (i, want) in values.iter().enumerate() {
        same_bits(
            resumed.normal(0.0, 1.0),
            want,
            &format!("{what}: normal {i} after from_state(state, inc)"),
        );
        same_bits(
            played.normal(0.0, 1.0),
            want,
            &format!("{what}: normal {i} after {skip} normals from the seed"),
        );
    }
    check_end(&mut resumed, &next["end"], "resume, from_state");
    check_end(&mut played, &next["end"], "resume, played from the seed");
}

#[test]
fn matches_numpy() {
    let doc = load();
    let top = object(&doc, "the vectors file");
    for key in top.keys() {
        assert!(TOP_KEYS.contains(&key.as_str()), "unknown top-level key {key:?}");
    }
    assert_eq!(uint(&top["format"], "format"), 1, "vectors format");

    let seeds = array(&top["seeds"], "seeds");
    assert!(!seeds.is_empty(), "no seeds in the vectors file");
    let (mut wide_int_seed, mut list_seed) = (false, false);
    for entry in seeds {
        let entry = object(entry, "a seed entry");
        let mut keys: Vec<&str> = entry.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["init", "seed", "streams"], "keys of a seed entry");
        let seed = Seed::parse(&entry["seed"]);
        match &seed {
            Seed::Int(n) => wide_int_seed |= *n > u64::from(u32::MAX),
            Seed::List(_) => list_seed = true,
        }
        check_init(&seed, &entry["init"]);

        let streams = object(&entry["streams"], "streams");
        for name in streams.keys() {
            assert!(
                STREAMS.iter().any(|(known, _)| *known == name.as_str()),
                "seed {seed:?}: the vectors file has a stream this test does not know: {name:?}"
            );
        }
        for (name, call) in STREAMS {
            let stream = streams
                .get(name)
                .unwrap_or_else(|| panic!("seed {seed:?}: stream {name:?} is missing from the vectors file"));
            check_stream(&seed, name, call, stream);
        }
    }
    assert!(wide_int_seed, "the vectors file has no integer seed above 2**32");
    assert!(list_seed, "the vectors file has no list seed");

    check_resume(&top["resume"]);
}

/// A list seed whose entries need more than 32 bits, and more words than SeedSequence's pool of four: the vectors
/// file has no such seed. The numbers are numpy 2.5.3's for `seed = [2**40 + 7, 44, 2**64 - 1, 0, 2**32]`:
/// `g = np.random.default_rng(seed)`, then `g.bit_generator.state["state"]` (state and inc) and
/// `g.bit_generator.random_raw(4)`.
#[test]
fn wide_list_seed_matches_numpy() {
    let seed = [(1 << 40) + 7, 44, u64::MAX, 0, 1 << 32];
    let raw: [u64; 4] = [
        5084789623536395166,
        4634792655478571521,
        4579134360560066414,
        2454427435776407686,
    ];
    let state: u128 = 6063138665990521868792550225979893029;
    let inc: u128 = 334859331448147890334716078557963197583;
    let mut fresh = Generator::from_seq(&seed);
    let mut resumed = Generator::from_state(state, inc);
    for (i, &want) in raw.iter().enumerate() {
        assert_eq!(raw_u64(&mut fresh), want, "raw 64-bit output {i} of from_seq");
        assert_eq!(raw_u64(&mut resumed), want, "raw 64-bit output {i} of from_state");
    }
}

/// The tail of the normal distribution (beyond about 3.654, one draw in 3,900): the vectors file holds only four
/// such draws, alike in the bits that matter and none of them looping. These are seeds whose first normal is a
/// tail draw, with numpy 2.5.3's `np.random.default_rng(seed).normal(0, 1).hex()` on the reference PC. The first
/// eight cover every combination of three bits of the 64-bit output: the ziggurat's sign bit (which the tail
/// ignores), the bit that picks the tail's side, and the bit above it. The last two reject their first pair of
/// uniforms and loop. (About one tail draw in 190 differs from numpy in its last bit, see the docs of
/// `bf_core::rng`; none of these do.)
#[test]
fn normal_tail_matches_numpy() {
    const TAIL: [(u64, &str); 10] = [
        (1950, "0x1.2380a29cff10bp+2"),
        (11779, "0x1.d8a2906dccca5p+1"),
        (5097, "-0x1.0ec4274039ea0p+2"),
        (19593, "-0x1.e1746e8fffac0p+1"),
        (13497, "0x1.df84cb1ddabb6p+1"),
        (2429, "0x1.d6eebeb21e2dap+1"),
        (755, "-0x1.1878d558cc268p+2"),
        (34481, "-0x1.31a6d7dd6021dp+2"),
        (141511, "-0x1.de6091a784909p+1"),
        (507423, "0x1.1c7bbea82bd8bp+2"),
    ];
    for (seed, want) in TAIL {
        let want = hexf(want);
        assert!(want.abs() > 3.654, "seed {seed}: {want} is not a tail draw");
        let got = Generator::new(seed).normal(0.0, 1.0);
        assert!(
            got.to_bits() == want.to_bits(),
            "seed {seed}: normal(0, 1) gave {got:e} (bits {:#018x}), numpy gives {want:e} (bits {:#018x})",
            got.to_bits(),
            want.to_bits()
        );
        let standard = Generator::new(seed).standard_normal();
        assert_eq!(standard.to_bits(), want.to_bits(), "seed {seed}: standard_normal()");
    }
}

/// A clone carries the whole state, the buffered 32-bit half included, and from then on the two generators do
/// not affect each other.
#[test]
fn clone_is_independent() {
    /// A mixed run of 32-bit and 64-bit draws, as bits.
    fn run(g: &mut Generator) -> Vec<u64> {
        let mut out = Vec::new();
        for _ in 0..8 {
            out.push(g.integers(1 << 32));
            out.push(g.random().to_bits());
            out.push(g.integers(7));
            out.push(g.normal(0.0, 1.0).to_bits());
            out.extend(g.permutation(5).into_iter().map(|v| v as u64));
        }
        out
    }

    let mut original = Generator::new(392);
    original.integers(5); // one 32-bit draw: the other half of that output is now buffered
    let mut copy = original.clone();

    let from_original = run(&mut original);
    // Drawing from the original did not move the copy: it gives the same run from the same point.
    let from_copy = run(&mut copy);
    assert_eq!(from_copy, from_original);

    // The same run on a generator that was never cloned: cloning changed nothing.
    let mut plain = Generator::new(392);
    plain.integers(5);
    assert_eq!(run(&mut plain), from_original);

    // Advance only the copy; the original continues as the plain generator does.
    let extra = run(&mut copy);
    let next_original = run(&mut original);
    assert_eq!(
        next_original, extra,
        "both were at the same point before the copy drew again"
    );
    assert_eq!(run(&mut plain), next_original);
    assert_ne!(run(&mut copy), next_original, "the copy is now one run further on");
}

#[test]
fn hexf_reads_python_float_hex() {
    assert_eq!(hexf("0x1.0000000000000p+0").to_bits(), 1.0_f64.to_bits());
    assert_eq!(hexf("-0x1.8000000000000p+1").to_bits(), (-3.0_f64).to_bits());
    assert_eq!(hexf("0x1.999999999999ap-4").to_bits(), 0.1_f64.to_bits());
    assert_eq!(hexf("0x1.fffffffffffffp+1023").to_bits(), f64::MAX.to_bits());
    assert_eq!(hexf("0x0.0p+0").to_bits(), 0.0_f64.to_bits());
    assert_eq!(hexf("-0x0.0p+0").to_bits(), (-0.0_f64).to_bits());
    assert_eq!(hexf("0x0.0000000000001p-1022").to_bits(), 1);
    assert_eq!(hexf("0x1.0000000000000p-1022").to_bits(), f64::MIN_POSITIVE.to_bits());
}
