//! The recording container (`.bfrec`): the five Python reference recordings, the worked example of
//! `docs/recording-format.md` section 7, bit-exact round trips, and every way a bad file must be refused.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use bf_core::event::{Event, Value};
use bf_core::npy;
use bf_core::recording::{Array, RECORDING_FORMAT, Recording, Source};

fn fights() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference/fights/flute_vs_trombone")
}

fn ep392() -> PathBuf {
    fights().join("ep392.bfrec")
}

/// A fresh, empty folder for one test under the system temp dir, removed again when the test ends.
struct TempDir(PathBuf);

impl TempDir {
    fn join(&self, name: impl AsRef<Path>) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_dir(test: &str) -> TempDir {
    let dir = std::env::temp_dir().join(format!("bf-core-recording-{}-{test}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    TempDir(dir)
}

/// Every member of a zip, by name.
fn unzip(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = BTreeMap::new();
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        out.insert(f.name().unwrap().to_string(), data);
    }
    out
}

/// A zip of these members, stored.
fn rezip(members: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::DEFAULT.compression_method(zip::CompressionMethod::Stored);
    for (name, data) in members {
        z.start_file(name.as_str(), opts).unwrap();
        z.write_all(data).unwrap();
    }
    z.finish().unwrap().into_inner()
}

/// Writes ep392 with its members edited by `edit` to `<test dir>/bad.bfrec`, and returns the reader's message.
fn read_edited(test: &str, edit: impl FnOnce(&mut BTreeMap<String, Vec<u8>>)) -> String {
    let mut members = unzip(&fs::read(ep392()).unwrap());
    edit(&mut members);
    let dir = temp_dir(test);
    let path = dir.join("bad.bfrec");
    fs::write(&path, rezip(&members)).unwrap();
    let message = match Recording::read(&path) {
        Ok(_) => panic!("{test}: the edited file was accepted"),
        Err(e) => e.to_string(),
    };
    eprintln!("{test}: {message}");
    assert!(
        message.contains("bad.bfrec"),
        "{test}: the message does not name the file: {message}"
    );
    message
}

fn meta_text(members: &BTreeMap<String, Vec<u8>>) -> String {
    String::from_utf8(members["meta.json"].clone()).unwrap()
}

/// Replaces exactly one occurrence of `old` in meta.json.
fn edit_meta(members: &mut BTreeMap<String, Vec<u8>>, old: &str, new: &str) {
    let text = meta_text(members);
    assert_eq!(text.matches(old).count(), 1, "meta.json should hold {old:?} once");
    members.insert("meta.json".into(), text.replace(old, new).into_bytes());
}

#[test]
fn reads_python_recording() {
    let rec = Recording::read(&ep392()).unwrap();
    assert_eq!(rec.format, RECORDING_FORMAT);
    assert_eq!(
        rec.source,
        Source::Python {
            commit: "88a51b9f99b4a8367ab5087da92b1fc79aa595c4".into()
        }
    );
    assert_eq!(rec.names, ["FLUTE", "TROMBONE"]);
    assert_eq!(rec.seed, 392);
    assert_eq!(rec.winner, Some(1));
    let t_ko = rec.t_ko.unwrap();
    assert!((49.0..=53.0).contains(&t_ko));
    assert_eq!(t_ko.to_bits(), 50.021568627450975f64.to_bits());
    assert_eq!(rec.fps, 30);
    assert_eq!(rec.f0, 108);
    assert_eq!(rec.corner, [[-1.85, 1.85], [1.85, -1.85]]);
    assert_eq!(rec.frames(), 1394);
    match &rec.arrays["P"] {
        Array::F64 { shape, data } => {
            assert_eq!(shape, &[rec.frames(), 2, 2]);
            assert_eq!(data.len(), rec.frames() * 4);
        }
        other => panic!("P is not <f8: {other:?}"),
    }
    assert!(matches!(&rec.arrays["LV"], Array::I64 { shape, .. } if shape == &[1394, 2]));
    assert!(matches!(&rec.arrays["COB"], Array::Nested(Value::List(v)) if v.len() == 1394));
    assert_eq!(rec.arrays.len(), 19);
    assert_eq!(rec.events.len(), 522);
    assert!(
        rec.events.windows(2).all(|w| w[0].t <= w[1].t),
        "events are not sorted by time"
    );
    assert_eq!(rec.hash(), "038e1f40fc3373eb");
}

#[test]
fn ints_and_floats_keep_their_kind() {
    let rec = Recording::read(&ep392()).unwrap();
    assert_eq!(rec.rules["FPS"], Value::Int(30));
    assert_eq!(rec.rules["A"], Value::Float(3.0));
    assert_eq!(rec.rules["swat_cobras"], Value::Bool(true));
    assert_eq!(
        rec.rules["LV_DMG"],
        Value::List(vec![Value::Float(14.0), Value::Float(36.0)])
    );
    // [4.48824, "hit", 1, 0, 1.4000000000000001, -0.653158143981096, -0.732012545889741, "blade", 1]
    let hit: &Event = rec.events.iter().find(|e| e.kind == "hit").unwrap();
    assert_eq!(hit.t.to_bits(), 4.48824f64.to_bits());
    assert_eq!(hit.who(), Some(1));
    assert_eq!(hit.at(3), &Value::Int(0));
    assert_eq!(hit.f(4).to_bits(), 1.4000000000000001f64.to_bits());
    assert_eq!(hit.s(7), "blade");
    assert_eq!(hit.i(8), 1);
    let clash = rec.events.iter().find(|e| e.kind == "clash").unwrap();
    assert_eq!(clash.at(2), &Value::Int(-1));
    assert_eq!(clash.who(), None);
}

#[test]
fn k_is_all_nan_and_keeps_its_bits() {
    let rec = Recording::read(&ep392()).unwrap();
    let Array::F64 { shape, data } = &rec.arrays["K"] else {
        panic!("K is not <f8")
    };
    assert_eq!(shape, &[1394, 72, 7]);
    assert!(data.iter().all(|x| x.to_bits() == 0x7FF8_0000_0000_0000));
}

#[test]
fn every_reference_recording_matches_the_document() {
    // recording-format.md section 7, "The reference recordings".
    let table: [(&str, u64, usize, f64, usize, usize, &str); 5] = [
        ("ep392", 392, 1, 50.021568627450975, 522, 1394, "038e1f40fc3373eb"),
        ("seed1", 1, 0, 40.63823529411764, 398, 1113, "8689c1af755ed173"),
        ("seed2", 2, 1, 52.871568627450976, 505, 1480, "ec9e2ac0d1ca33fc"),
        ("seed3", 3, 0, 50.95490196078431, 510, 1422, "9bff0f539177070e"),
        ("seed500", 500, 0, 55.88823529411764, 528, 1570, "5a00a0b08c4d1bac"),
    ];
    for (name, seed, winner, t_ko, events, frames, hash) in table {
        let rec = Recording::read(&fights().join(format!("{name}.bfrec"))).unwrap();
        assert_eq!(rec.seed, seed, "{name}");
        assert_eq!(rec.winner, Some(winner), "{name}");
        assert_eq!(rec.t_ko.map(f64::to_bits), Some(t_ko.to_bits()), "{name}");
        assert_eq!(rec.events.len(), events, "{name}");
        assert_eq!(rec.frames(), frames, "{name}");
        assert_eq!(rec.hash(), hash, "{name}");
    }
}

fn worked_example() -> Recording {
    let f = Value::Float;
    let i = Value::Int;
    let s = |x: &str| Value::Str(x.into());
    let mut rules = BTreeMap::new();
    rules.insert("FPS".to_string(), i(30));
    rules.insert("swat".to_string(), Value::Bool(true));
    let mut arrays = BTreeMap::new();
    arrays.insert(
        "HP".to_string(),
        Array::F64 {
            shape: vec![2, 2],
            data: vec![100.0, 100.0, 0.0, 97.5],
        },
    );
    arrays.insert(
        "LV".to_string(),
        Array::I64 {
            shape: vec![2, 2],
            data: vec![1, 1, 1, 2],
        },
    );
    arrays.insert(
        "W".to_string(),
        Array::Nested(Value::List(vec![
            Value::List(vec![]),
            Value::List(vec![Value::List(vec![i(0), f(0.5), s("wave"), Value::Null])]),
        ])),
    );
    Recording {
        format: RECORDING_FORMAT,
        source: Source::Studio {
            version: "0.1.0".into(),
        },
        names: vec!["A".into(), "B".into()],
        seed: 7,
        winner: Some(1),
        t_ko: Some(2.5),
        f0: 3,
        fps: 30,
        corner: vec![[-1.0, 1.0], [1.0, -1.0]],
        rules,
        events: vec![
            Event {
                t: 1.0,
                kind: "hit".into(),
                fields: vec![i(0), i(1), f(2.5), s("blade")],
            },
            Event {
                t: 2.5,
                kind: "ko".into(),
                fields: vec![i(0), i(1)],
            },
        ],
        arrays,
    }
}

const WORKED_HEX: &str = "\
42465245432d484153482d310a4c020000000000000053010000000000000041\
5301000000000000004249070000000000000049010000000000000044000000\
0000000440490300000000000000491e000000000000004c0200000000000000\
4c020000000000000044000000000000f0bf44000000000000f03f4c02000000\
0000000044000000000000f03f44000000000000f0bf4c02000000000000004c\
0200000000000000530300000000000000465053491e000000000000004c0200\
00000000000053040000000000000073776174544c02000000000000004c0600\
00000000000044000000000000f03f5303000000000000006869744900000000\
0000000049010000000000000044000000000000044053050000000000000062\
6c6164654c04000000000000004400000000000004405302000000000000006b\
6f49000000000000000049010000000000000003000000000000005302000000\
0000000048506602000000000000000200000000000000020000000000000000\
0000000000594000000000000059400000000000000000000000000060584053\
02000000000000004c5669020000000000000002000000000000000200000000\
0000000100000000000000010000000000000001000000000000000200000000\
000000530100000000000000576a4c02000000000000004c0000000000000000\
4c01000000000000004c04000000000000004900000000000000004400000000\
0000e03f530400000000000000776176654e";

#[test]
fn worked_example_gives_the_documented_bytes_and_hash() {
    let rec = worked_example();
    let hex: String = rec.canonical_bytes().iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hex.len(), 2 * 562);
    assert_eq!(hex, WORKED_HEX);
    assert_eq!(rec.hash(), "e95b0e26c6a45d73");
}

#[test]
fn worked_example_round_trips() {
    let rec = worked_example();
    let dir = temp_dir("worked");
    let path = dir.join("worked.bfrec");
    rec.write(&path).unwrap();
    let back = Recording::read(&path).unwrap();
    assert_eq!(back, rec);
    assert_eq!(back.hash(), "e95b0e26c6a45d73");
}

#[test]
fn round_trip_is_bit_exact() {
    let dir = temp_dir("round-trip");
    for name in ["ep392", "seed1", "seed2", "seed3", "seed500"] {
        let rec = Recording::read(&fights().join(format!("{name}.bfrec"))).unwrap();
        let path = dir.join(format!("{name}.bfrec"));
        rec.write(&path).unwrap();
        let back = Recording::read(&path).unwrap();
        assert!(back == rec, "{name}: the recording changed in a round trip");
        assert_eq!(back.hash(), rec.hash(), "{name}");
        // Writing is deterministic: the same recording gives the same bytes.
        let again = dir.join(format!("{name}-again.bfrec"));
        back.write(&again).unwrap();
        assert_eq!(
            fs::read(&path).unwrap(),
            fs::read(&again).unwrap(),
            "{name}: writing is not deterministic"
        );
    }
}

#[test]
fn written_file_follows_the_zip_rules() {
    let rec = Recording::read(&ep392()).unwrap();
    let dir = temp_dir("zip-rules");
    let path = dir.join("out.bfrec");
    rec.write(&path).unwrap();
    let bytes = fs::read(&path).unwrap();
    let mut z = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    assert!(z.comment().is_empty());
    let mut names = Vec::new();
    for i in 0..z.len() {
        let f = z.by_index(i).unwrap();
        assert_eq!(f.compression(), zip::CompressionMethod::Deflated);
        assert!(!f.encrypted() && !f.is_dir());
        let t = f.last_modified().unwrap();
        assert_eq!(
            (t.year(), t.month(), t.day(), t.hour(), t.minute(), t.second()),
            (1980, 1, 1, 0, 0, 0)
        );
        names.push(f.name().unwrap().to_string());
    }
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "members are not in name order");
    assert_eq!(names.last().map(String::as_str), Some("meta.json"));
    // Every .npy member is byte for byte what numpy wrote.
    let ours = unzip(&bytes);
    let theirs = unzip(&fs::read(ep392()).unwrap());
    assert_eq!(ours.keys().collect::<Vec<_>>(), theirs.keys().collect::<Vec<_>>());
    for (name, data) in &theirs {
        if name.ends_with(".npy") {
            assert!(&ours[name] == data, "{name} differs from numpy's bytes");
        }
    }
}

#[test]
fn npy_reads_and_writes_numpy_bytes() {
    let members = unzip(&fs::read(ep392()).unwrap());
    let p = &members["P.npy"];
    assert_eq!(&p[..10], b"\x93NUMPY\x01\x00\x76\x00");
    assert_eq!(
        &p[10..75],
        b"{'descr': '<f8', 'fortran_order': False, 'shape': (1394, 2, 2), }".as_slice()
    );
    for (name, data) in members.iter().filter(|(n, _)| n.ends_with(".npy")) {
        let a = npy::read(data).unwrap();
        assert!(&npy::write(&a).unwrap() == data, "{name}");
    }
    let one_d = Array::I64 {
        shape: vec![3],
        data: vec![-1, 0, 7],
    };
    let bytes = npy::write(&one_d).unwrap();
    assert_eq!(bytes.len() % 64, 24);
    assert!(
        String::from_utf8_lossy(&bytes[10..128])
            .starts_with("{'descr': '<i8', 'fortran_order': False, 'shape': (3,), }")
    );
    assert_eq!(npy::read(&bytes).unwrap(), one_d);
}

#[test]
fn bit_exact_equality_tells_nan_and_minus_zero_apart() {
    let a = Array::F64 {
        shape: vec![1],
        data: vec![f64::NAN],
    };
    assert_eq!(a, a.clone());
    assert_ne!(Value::Float(0.0), Value::Float(-0.0));
    assert_ne!(Value::Float(1.0), Value::Int(1));
    // Two recordings that differ only in the sign of a zero are different recordings.
    let mut plus = worked_example();
    plus.t_ko = Some(0.0);
    let mut minus = worked_example();
    minus.t_ko = Some(-0.0);
    assert_ne!(plus, minus);
    assert_ne!(plus.hash(), minus.hash());
}

#[test]
fn a_python_recording_written_by_rust_stays_python() {
    let rec = Recording::read(&ep392()).unwrap();
    let dir = temp_dir("provenance");
    let path = dir.join("again.bfrec");
    rec.write(&path).unwrap();
    let back = Recording::read(&path).unwrap();
    assert_eq!(
        back.source,
        Source::Python {
            commit: "88a51b9f99b4a8367ab5087da92b1fc79aa595c4".into()
        }
    );
}

/// The offsets of every zip header with this 4-byte signature.
fn signatures(bytes: &[u8], sig: &[u8; 4]) -> Vec<usize> {
    bytes
        .windows(4)
        .enumerate()
        .filter(|(_, w)| *w == sig)
        .map(|(i, _)| i)
        .collect()
}

#[test]
fn a_member_declaring_a_huge_size_is_an_error() {
    // A stored zip of ep392 whose HP.npy claims 4,026,531,840 unpacked bytes in both its local header and its
    // central directory entry. Nothing that large may be read into memory.
    let members = unzip(&fs::read(ep392()).unwrap());
    let mut bytes = rezip(&members);
    let huge = 0xF000_0000u32.to_le_bytes();
    let name = b"HP.npy";
    for (sig, size_at, name_at) in [(b"PK\x03\x04", 22, 30), (b"PK\x01\x02", 24, 46)] {
        let at = signatures(&bytes, sig)
            .into_iter()
            .find(|&i| bytes[i + name_at..].starts_with(name))
            .unwrap();
        bytes[at + size_at..at + size_at + 4].copy_from_slice(&huge);
    }
    let dir = temp_dir("huge-member");
    let path = dir.join("huge.bfrec");
    fs::write(&path, &bytes).unwrap();
    let err = Recording::read(&path).unwrap_err().to_string();
    eprintln!("huge member: {err}");
    assert!(err.contains("huge.bfrec") && err.contains("HP.npy"), "{err}");
    assert!(err.contains("4026531840"), "{err}");
}

// ---------------------------------------------------------------- refusals (Review Focus 4 and the document)

#[test]
fn truncated_file_is_an_error() {
    let bytes = fs::read(ep392()).unwrap();
    let dir = temp_dir("truncated");
    for cut in [0, 10, bytes.len() / 2, bytes.len() - 1] {
        let path = dir.join("cut.bfrec");
        fs::write(&path, &bytes[..cut]).unwrap();
        let err = Recording::read(&path).unwrap_err().to_string();
        eprintln!("cut at {cut}: {err}");
        assert!(err.contains("cut.bfrec"), "{err}");
        assert!(err.contains("zip"), "{err}");
    }
    let err = Recording::read(&dir.join("missing.bfrec")).unwrap_err().to_string();
    assert!(err.contains("missing.bfrec"), "{err}");
}

#[test]
fn newer_format_is_an_error() {
    let err = read_edited("newer-format", |m| edit_meta(m, "\"format\": 1,", "\"format\": 2,"));
    assert!(err.contains("format 2"), "{err}");
    assert!(err.contains("this app reads 1"), "{err}");
}

#[test]
fn mismatched_array_lengths_is_an_error() {
    let err = read_edited("mismatched", |m| {
        let Array::F64 { shape, data } = npy::read(&m["SLW.npy"]).unwrap() else {
            panic!()
        };
        let short = Array::F64 {
            shape: vec![shape[0] - 1, shape[1]],
            data: data[..data.len() - shape[1]].to_vec(),
        };
        m.insert("SLW.npy".into(), npy::write(&short).unwrap());
    });
    assert!(
        err.contains("SLW") && err.contains("1393") && err.contains("1394"),
        "{err}"
    );
    let err = read_edited("mismatched-json", |m| {
        let text = String::from_utf8(m["WAVES.json"].clone()).unwrap();
        m.insert("WAVES.json".into(), text.replacen("[\n[],\n", "[\n", 1).into_bytes());
    });
    assert!(err.contains("WAVES") && err.contains("1393"), "{err}");
}

#[test]
fn missing_or_unnamed_member_is_an_error() {
    let err = read_edited("missing-member", |m| {
        m.remove("WAVES.json");
    });
    assert!(err.contains("WAVES.json"), "{err}");
    let err = read_edited("extra-member", |m| {
        m.insert("EXTRA.npy".into(), m["HP.npy"].clone());
    });
    assert!(err.contains("EXTRA.npy"), "{err}");
    let err = read_edited("no-meta", |m| {
        m.remove("meta.json");
    });
    assert!(err.contains("meta.json"), "{err}");
    let err = read_edited("no-hp", |m| {
        m.remove("HP.npy");
        edit_meta(m, "  \"HP\": \"npy\",\n", "");
    });
    assert!(err.contains("HP"), "{err}");
}

#[test]
fn wrong_dtype_is_an_error() {
    let err = read_edited("dtype", |m| {
        let mut p = m["HP.npy"].clone();
        let at = p.windows(5).position(|w| w == b"'<f8'").unwrap();
        p[at + 2..at + 4].copy_from_slice(b"f4");
        m.insert("HP.npy".into(), p);
    });
    assert!(err.contains("HP.npy") && err.contains("<f4"), "{err}");
    let err = read_edited("fortran", |m| {
        let text = m["HP.npy"].clone();
        let at = text.windows(5).position(|w| w == b"False").unwrap();
        let mut p = text.clone();
        p[at..at + 5].copy_from_slice(b"True ");
        m.insert("HP.npy".into(), p);
    });
    assert!(err.contains("HP.npy") && err.contains("fortran"), "{err}");
    let err = read_edited("short-data", |m| {
        let mut p = m["HP.npy"].clone();
        p.truncate(p.len() - 8);
        m.insert("HP.npy".into(), p);
    });
    assert!(err.contains("HP.npy"), "{err}");
}

#[test]
fn malformed_meta_json_is_an_error() {
    let err = read_edited("malformed", |m| {
        let half = m["meta.json"].len() / 2;
        m.get_mut("meta.json").unwrap().truncate(half);
    });
    assert!(err.contains("meta.json"), "{err}");
    let err = read_edited("wrong-type", |m| edit_meta(m, "\"seed\": 392,", "\"seed\": 392.0,"));
    assert!(err.contains("seed"), "{err}");
    let err = read_edited("source", |m| edit_meta(m, "\"kind\": \"python\"", "\"kind\": \"perl\""));
    assert!(err.contains("perl"), "{err}");
    let err = read_edited("unknown-key", |m| {
        edit_meta(m, "\"seed\": 392,", "\"seed\": 392,\n \"extra\": 1,")
    });
    assert!(err.contains("extra"), "{err}");
    let err = read_edited("repeated-key", |m| {
        edit_meta(m, "\"seed\": 392,", "\"seed\": 392,\n \"seed\": 393,")
    });
    assert!(err.contains("twice"), "{err}");
}

#[test]
fn non_finite_float_in_meta_is_an_error() {
    for (test, bad) in [("nan", "NaN"), ("inf", "Infinity"), ("huge", "1e999")] {
        let err = read_edited(test, |m| {
            edit_meta(m, "\"t_ko\": 50.021568627450975,", &format!("\"t_ko\": {bad},"))
        });
        assert!(err.contains("meta.json") && err.contains("finite"), "{test}: {err}");
    }
}

#[test]
fn writing_a_bad_recording_is_an_error() {
    let dir = temp_dir("write-bad");
    let mut rec = worked_example();
    rec.t_ko = Some(f64::NAN);
    assert!(
        rec.write(&dir.join("a.bfrec"))
            .unwrap_err()
            .to_string()
            .contains("finite")
    );
    let mut rec = worked_example();
    rec.arrays.insert(
        "X".into(),
        Array::I64 {
            shape: vec![3],
            data: vec![1, 2, 3],
        },
    );
    assert!(rec.write(&dir.join("b.bfrec")).unwrap_err().to_string().contains("X"));
    let mut rec = worked_example();
    rec.arrays.insert(
        "bad name".into(),
        Array::I64 {
            shape: vec![2],
            data: vec![1, 2],
        },
    );
    assert!(rec.write(&dir.join("c.bfrec")).is_err());
    assert!(!dir.join("a.bfrec").exists());
}
