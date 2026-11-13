//! The recording container, `.bfrec` (Task 6).
//!
//! A recording is one fight: who fought, under which rules, every event, and the state of the ring at every frame.
//! The fight engine writes it; picture, sound and story read only it. `studio/docs/recording-format.md` is the
//! contract, shared with the Python kit's writer (`tools/studio_export/recording.py`).
//!
//! A `.bfrec` is a zip: `meta.json` (every scalar field, the events, and which arrays there are), one `<KEY>.npy` per
//! rectangular numeric per-frame array ([`crate::npy`]) and one `<KEY>.json` per ragged one. Reading checks
//! everything the document says a reader must refuse and never panics; writing is deterministic (members in name
//! order, one fixed timestamp, deflate level 9) and round-trips bit for bit.
//!
//! JSON is read and written by the small exact codec in `recording/json.rs` rather than `serde_json`: the format
//! decides `Int` or `Float` by the text of a number, and `serde_json` (without `arbitrary_precision`) reads `-0` as a
//! float and an integer beyond 64 bits as a float.
//!
//! Equality on [`Recording`] and [`Array`] is bit-exact (floats compare by their bits), because `K` is all NaN and
//! `0.0` and `-0.0` hash differently.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::event::{Event, Value};
use crate::npy;

mod json;

/// The format this app reads and writes.
pub const RECORDING_FORMAT: u32 = 1;

const HASH_TAG: &[u8; 13] = b"BFREC-HASH-1\n";
const META: &str = "meta.json";
const META_KEYS: [&str; 12] = [
    "arrays", "corner", "events", "f0", "format", "fps", "names", "rules", "seed", "source", "t_ko", "winner",
];
const ZIP_LEVEL: i64 = 9;
/// The most one zip member may unpack to. The declared size comes from the untrusted file and deflate packs about
/// 1000:1, so without a cap a small file could ask for gigabytes. Today's largest member (`K.npy`) is about 5.6 MB.
const MAX_MEMBER_BYTES: u64 = 256 << 20;
/// The most all members of one file together may unpack to.
const MAX_TOTAL_BYTES: u64 = 1 << 30;

/// One per-frame array. The first dimension (or the length of the list) is the frame count.
#[derive(Debug, Clone)]
pub enum Array {
    /// `<f8`, C order.
    F64 { shape: Vec<usize>, data: Vec<f64> },
    /// `<i8`, C order.
    I64 { shape: Vec<usize>, data: Vec<i64> },
    /// Ragged per-frame lists (`WAVES`, `SLAMS`, `COB`, ...): a `Value::List` with one entry per frame.
    Nested(Value),
}

impl PartialEq for Array {
    /// Bit-exact: `F64` data compares by its bits, so NaN equals the same NaN and `0.0 != -0.0`.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Array::F64 { shape: a, data: x }, Array::F64 { shape: b, data: y }) => {
                a == b && x.len() == y.len() && x.iter().zip(y).all(|(p, q)| p.to_bits() == q.to_bits())
            }
            (Array::I64 { shape: a, data: x }, Array::I64 { shape: b, data: y }) => a == b && x == y,
            (Array::Nested(a), Array::Nested(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Array {}

impl Array {
    /// The number of frames: the first dimension, or the number of entries of a nested list.
    fn frames(&self) -> Option<usize> {
        match self {
            Array::F64 { shape, .. } | Array::I64 { shape, .. } => shape.first().copied(),
            Array::Nested(Value::List(v)) => Some(v.len()),
            Array::Nested(_) => None,
        }
    }
}

/// Who made a recording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// This app, at this version.
    Studio { version: String },
    /// The Python kit, at this commit (40 hex digits, maybe with `-dirty`).
    Python { commit: String },
}

impl Source {
    /// This app at its own version. The fight engine gives every recording it makes this source;
    /// [`Recording::write`] never stamps it on its own, so a recording imported from the Python kit stays marked as
    /// one.
    pub fn studio() -> Source {
        Source::Studio {
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Why a recording could not be read or written. The message names the file and the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecError {
    message: String,
}

impl RecError {
    fn new(path: &Path, problem: impl fmt::Display) -> RecError {
        RecError {
            message: format!("{}: {problem}", path.display()),
        }
    }

    /// The whole message, file name first.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for RecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RecError {}

/// One fight (`docs/recording-format.md` section 2 has the meaning of every field).
#[derive(Debug, Clone)]
pub struct Recording {
    pub format: u32,
    pub source: Source,
    pub names: Vec<String>,
    pub seed: u64,
    pub winner: Option<usize>,
    pub t_ko: Option<f64>,
    pub f0: usize,
    pub fps: u32,
    pub corner: Vec<[f64; 2]>,
    pub rules: BTreeMap<String, Value>,
    pub events: Vec<Event>,
    pub arrays: BTreeMap<String, Array>,
}

impl PartialEq for Recording {
    /// Bit-exact: every float compares by its bits (see [`Value`], [`Event`] and [`Array`]).
    fn eq(&self, other: &Self) -> bool {
        let bits = |c: &[[f64; 2]]| c.iter().flatten().map(|x| x.to_bits()).collect::<Vec<_>>();
        self.format == other.format
            && self.source == other.source
            && self.names == other.names
            && self.seed == other.seed
            && self.winner == other.winner
            && self.t_ko.map(f64::to_bits) == other.t_ko.map(f64::to_bits)
            && self.f0 == other.f0
            && self.fps == other.fps
            && self.corner.len() == other.corner.len()
            && bits(&self.corner) == bits(&other.corner)
            && self.rules == other.rules
            && self.events == other.events
            && self.arrays == other.arrays
    }
}

impl Eq for Recording {}

impl Recording {
    /// Reads a `.bfrec`. Every way a file can break the format is an error that names the file; none panics.
    pub fn read(path: &Path) -> Result<Recording, RecError> {
        read_recording(path).map_err(|problem| RecError::new(path, problem))
    }

    /// Writes this recording to `path` (through a temporary file next to it, then a rename). The same recording
    /// always gives the same bytes. A recording the format cannot hold (a non-finite float in a JSON value, arrays
    /// that disagree on the frame count, a bad array name, ...) is an error and nothing is written.
    ///
    /// `source` is written as it is: provenance survives a read and a write, so a Python recording written by the
    /// Studio still says `Source::Python` with its commit. Recordings the Studio makes carry [`Source::studio()`],
    /// set where they are made.
    pub fn write(&self, path: &Path) -> Result<(), RecError> {
        self.check()
            .map_err(|problem| RecError::new(path, format!("cannot write this recording: {problem}")))?;
        let bytes = self.zip_bytes().map_err(|problem| RecError::new(path, problem))?;
        let mut tmp = PathBuf::from(path.as_os_str());
        tmp.as_mut_os_string().push(format!(".{}.tmp", std::process::id()));
        fs::write(&tmp, &bytes)
            .and_then(|()| fs::rename(&tmp, path))
            .map_err(|e| {
                let _ = fs::remove_file(&tmp);
                RecError::new(path, format!("cannot write the file ({e})"))
            })
    }

    /// The number of recorded frames: the length of `HP` (0 when there is no `HP`).
    pub fn frames(&self) -> usize {
        self.arrays.get("HP").and_then(Array::frames).unwrap_or(0)
    }

    /// The content hash: the first 16 lowercase hex digits of the SHA-256 of [`Recording::canonical_bytes`].
    pub fn hash(&self) -> String {
        Sha256::digest(self.canonical_bytes())
            .iter()
            .take(8)
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// The byte string the content hash is taken over (`recording-format.md` section 7). `format` and `source` are
    /// left out, and so is everything about the zip.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = HASH_TAG.to_vec();
        let list_of = |out: &mut Vec<u8>, n: usize| {
            out.push(b'L');
            put_u64(out, n as u64);
        };
        list_of(&mut out, self.names.len());
        for n in &self.names {
            put_str(&mut out, n);
        }
        out.push(b'I');
        put_u64(&mut out, self.seed);
        put_value(&mut out, &self.winner.map_or(Value::Null, |w| Value::Int(w as i64)));
        put_value(&mut out, &self.t_ko.map_or(Value::Null, Value::Float));
        put_value(&mut out, &Value::Int(self.f0 as i64));
        put_value(&mut out, &Value::Int(i64::from(self.fps)));
        list_of(&mut out, self.corner.len());
        for c in &self.corner {
            put_value(&mut out, &Value::List(vec![Value::Float(c[0]), Value::Float(c[1])]));
        }
        list_of(&mut out, self.rules.len());
        for (k, v) in &self.rules {
            list_of(&mut out, 2);
            put_str(&mut out, k);
            put_value(&mut out, v);
        }
        list_of(&mut out, self.events.len());
        for e in &self.events {
            list_of(&mut out, 2 + e.fields.len());
            put_value(&mut out, &Value::Float(e.t));
            put_str(&mut out, &e.kind);
            for v in &e.fields {
                put_value(&mut out, v);
            }
        }
        put_u64(&mut out, self.arrays.len() as u64);
        for (key, a) in &self.arrays {
            put_str(&mut out, key);
            match a {
                Array::F64 { shape, data } => {
                    put_shape(&mut out, b'f', shape);
                    data.iter()
                        .for_each(|x| out.extend_from_slice(&x.to_bits().to_le_bytes()));
                }
                Array::I64 { shape, data } => {
                    put_shape(&mut out, b'i', shape);
                    data.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes()));
                }
                Array::Nested(v) => {
                    out.push(b'j');
                    put_value(&mut out, v);
                }
            }
        }
        out
    }

    /// Everything the format promises about a recording, beyond what the Rust types already guarantee.
    fn check(&self) -> Result<(), String> {
        if self.format != RECORDING_FORMAT {
            return Err(format!("format {}, this app writes {RECORDING_FORMAT}", self.format));
        }
        if i64::try_from(self.seed).is_err() {
            return Err(format!("seed {} is not below 2^63", self.seed));
        }
        if let Some(w) = self.winner
            && w >= self.names.len()
        {
            return Err(format!(
                "winner {w} is not an index into names ({} fighters)",
                self.names.len()
            ));
        }
        if self.winner.is_none() != self.t_ko.is_none() {
            return Err("winner and t_ko must both be set or both be null".into());
        }
        if self.corner.len() != self.names.len() {
            return Err(format!(
                "{} corners for {} fighters",
                self.corner.len(),
                self.names.len()
            ));
        }
        if i64::try_from(self.f0).is_err() {
            return Err(format!("f0 {} does not fit 64 bits", self.f0));
        }
        if self.fps == 0 {
            return Err("fps is 0".into());
        }
        let finite = |x: f64, what: &dyn Fn() -> String| {
            if x.is_finite() {
                Ok(())
            } else {
                Err(format!("{} is {x}, not a finite float", what()))
            }
        };
        if let Some(t) = self.t_ko {
            finite(t, &|| "t_ko".into())?;
        }
        for (k, c) in self.corner.iter().enumerate() {
            finite(c[0], &|| format!("corner[{k}]"))?;
            finite(c[1], &|| format!("corner[{k}]"))?;
        }
        for (k, v) in &self.rules {
            finite_value(v).map_err(|x| format!("rules.{k} holds {x}, not a finite float"))?;
        }
        for (n, e) in self.events.iter().enumerate() {
            finite(e.t, &|| format!("event {n} ({}): t", e.kind))?;
            for v in &e.fields {
                finite_value(v).map_err(|x| format!("event {n} ({}) holds {x}, not a finite float", e.kind))?;
            }
        }
        let frames = match self.arrays.get("HP") {
            None => return Err("there is no HP array (it gives the frame count)".into()),
            Some(hp) => hp.frames().ok_or("HP has no frame dimension")?,
        };
        for (key, a) in &self.arrays {
            if !valid_key(key) {
                return Err(format!(
                    "{key:?} cannot name an array (ASCII letters, digits and _, never \"meta\")"
                ));
            }
            match a {
                Array::F64 { shape, data } => check_shape(key, shape, data.len())?,
                Array::I64 { shape, data } => check_shape(key, shape, data.len())?,
                Array::Nested(Value::List(rows)) => {
                    for row in rows {
                        finite_value(row).map_err(|x| format!("{key} holds {x}, not a finite float"))?;
                    }
                }
                Array::Nested(_) => return Err(format!("{key} is nested but not a per-frame list")),
            }
            match a.frames() {
                Some(n) if n == frames => {}
                Some(n) => return Err(format!("{key} has {n} frames, HP has {frames}")),
                None => return Err(format!("{key} has no frame dimension")),
            }
        }
        Ok(())
    }

    /// The zip file's bytes: members in byte order of their names, all deflated at level 9 and dated
    /// 1980-01-01 00:00:00, made on "MS-DOS" whatever the platform, so the bytes depend only on the recording.
    fn zip_bytes(&self) -> Result<Vec<u8>, String> {
        let mut members: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for (key, a) in &self.arrays {
            match a {
                Array::Nested(Value::List(rows)) => {
                    let mut text = String::new();
                    if rows.is_empty() {
                        text.push_str("[]\n");
                    } else {
                        text.push_str("[\n");
                        for (n, row) in rows.iter().enumerate() {
                            if n > 0 {
                                text.push_str(",\n");
                            }
                            json::write(&mut text, row);
                        }
                        text.push_str("\n]\n");
                    }
                    members.insert(format!("{key}.json"), text.into_bytes());
                }
                _ => {
                    let bytes = npy::write(a).map_err(|e| format!("{key}.npy: {e}"))?;
                    members.insert(format!("{key}.npy"), bytes);
                }
            }
        }
        members.insert(META.to_string(), self.meta_text().into_bytes());

        let options = zip::write::SimpleFileOptions::DEFAULT
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(ZIP_LEVEL))
            .last_modified_time(zip::DateTime::DEFAULT)
            .system(zip::System::Dos)
            .large_file(false);
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in &members {
            z.start_file(name.as_str(), options)
                .map_err(|e| format!("zip: {name}: {e}"))?;
            z.write_all(data).map_err(|e| format!("zip: {name}: {e}"))?;
        }
        Ok(z.finish().map_err(|e| format!("zip: {e}"))?.into_inner())
    }

    /// `meta.json`: sorted keys, one key per line, one event per line, ASCII only.
    fn meta_text(&self) -> String {
        let mut s = String::from("{\n");
        let field = |s: &mut String, key: &str, body: &dyn Fn(&mut String)| {
            if s.len() > 2 {
                s.push_str(",\n");
            }
            s.push(' ');
            json::write_str(s, key);
            s.push_str(": ");
            body(s);
        };
        let object = |s: &mut String, pairs: &[(&str, &Value)]| {
            s.push_str("{\n");
            for (n, (k, v)) in pairs.iter().enumerate() {
                s.push_str(if n > 0 { ",\n  " } else { "  " });
                json::write_str(s, k);
                s.push_str(": ");
                json::write(s, v);
            }
            s.push_str("\n }");
        };
        let kinds: Vec<(&str, Value)> = self
            .arrays
            .iter()
            .map(|(k, a)| {
                (
                    k.as_str(),
                    Value::Str(if matches!(a, Array::Nested(_)) { "json" } else { "npy" }.into()),
                )
            })
            .collect();
        // Keys in byte order, as META_KEYS lists them.
        field(&mut s, "arrays", &|s| {
            object(s, &kinds.iter().map(|(k, v)| (*k, v)).collect::<Vec<_>>())
        });
        let corner = Value::List(
            self.corner
                .iter()
                .map(|c| Value::List(vec![Value::Float(c[0]), Value::Float(c[1])]))
                .collect(),
        );
        field(&mut s, "corner", &|s| json::write(s, &corner));
        field(&mut s, "events", &|s| {
            if self.events.is_empty() {
                s.push_str("[]");
                return;
            }
            s.push_str("[\n");
            for (n, e) in self.events.iter().enumerate() {
                s.push_str(if n > 0 { ",\n  " } else { "  " });
                let mut row = vec![Value::Float(e.t), Value::Str(e.kind.clone())];
                row.extend(e.fields.iter().cloned());
                json::write(s, &Value::List(row));
            }
            s.push_str("\n ]");
        });
        field(&mut s, "f0", &|s| json::write(s, &Value::Int(self.f0 as i64)));
        field(&mut s, "format", &|s| {
            json::write(s, &Value::Int(i64::from(self.format)))
        });
        field(&mut s, "fps", &|s| json::write(s, &Value::Int(i64::from(self.fps))));
        let names = Value::List(self.names.iter().map(|n| Value::Str(n.clone())).collect());
        field(&mut s, "names", &|s| json::write(s, &names));
        field(&mut s, "rules", &|s| {
            if self.rules.is_empty() {
                s.push_str("{}");
            } else {
                object(s, &self.rules.iter().map(|(k, v)| (k.as_str(), v)).collect::<Vec<_>>());
            }
        });
        field(&mut s, "seed", &|s| json::write(s, &Value::Int(self.seed as i64)));
        let (kind, extra_key, extra) = match &self.source {
            Source::Python { commit } => ("python", "commit", commit),
            Source::Studio { version } => ("studio", "version", version),
        };
        let (kind, extra) = (Value::Str(kind.into()), Value::Str(extra.clone()));
        let mut source = [("kind", &kind), (extra_key, &extra)];
        source.sort_by_key(|(k, _)| *k);
        field(&mut s, "source", &|s| object(s, &source));
        field(&mut s, "t_ko", &|s| {
            json::write(s, &self.t_ko.map_or(Value::Null, Value::Float))
        });
        field(&mut s, "winner", &|s| {
            json::write(s, &self.winner.map_or(Value::Null, |w| Value::Int(w as i64)))
        });
        s.push_str("\n}\n");
        s
    }
}

// ---------------------------------------------------------------- reading

fn read_recording(path: &Path) -> Result<Recording, String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read the file ({e})"))?;
    let mut members = unzip(&bytes)?;
    let meta_bytes = members.remove(META).ok_or("the zip has no meta.json")?;
    let meta = json::parse(&meta_bytes).map_err(|e| format!("{META}: {e}"))?;
    let rec = recording_from_meta(meta, &mut members).map_err(|e| format!("{META}: {e}"))?;
    let rec = rec?;
    rec.check()?;
    Ok(rec)
}

/// Every member of the zip, by name. A file that is not a complete zip, an encrypted member, a compression method
/// other than stored or deflated, a folder, a repeated name, or a member (or all members together) unpacking to
/// more than [`MAX_MEMBER_BYTES`] ([`MAX_TOTAL_BYTES`]) is an error.
fn unzip(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, String> {
    unzip_capped(bytes, MAX_MEMBER_BYTES, MAX_TOTAL_BYTES)
}

/// [`unzip`] with the two size caps given (the tests use small ones).
fn unzip_capped(bytes: &[u8], member_cap: u64, total_cap: u64) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("not a complete zip archive, so not a recording ({e})"))?;
    let mut members = BTreeMap::new();
    let mut total: u64 = 0;
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| format!("zip member {i}: {e}"))?;
        let name =
            String::from_utf8(f.name_raw().to_vec()).map_err(|_| format!("zip member {i}: name is not UTF-8"))?;
        if f.encrypted() {
            return Err(format!("{name}: encrypted zip members are not read"));
        }
        if f.is_dir() {
            return Err(format!("{name}: a recording holds no folders"));
        }
        let method = f.compression();
        if !matches!(
            method,
            zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
        ) {
            return Err(format!(
                "{name}: compression {method:?}; only stored and deflated are read"
            ));
        }
        // The declared size is the file's own claim: check it before reading, then never read past the caps
        // whatever the data really unpacks to.
        let declared = f.size();
        if declared > member_cap {
            return Err(format!(
                "{name}: declares {declared} unpacked bytes, more than the {member_cap} a member may have"
            ));
        }
        let room = member_cap.min(total_cap - total);
        if declared > room {
            return Err(format!(
                "{name}: declares {declared} unpacked bytes; with the members before it that is more than the \
                 {total_cap} a whole file may have"
            ));
        }
        let mut data = Vec::new();
        (&mut f)
            .take(room + 1)
            .read_to_end(&mut data)
            .map_err(|e| format!("{name}: cannot unpack ({e})"))?;
        if data.len() as u64 > room {
            return Err(format!("{name}: unpacks to more than the {room} bytes it may have"));
        }
        total += data.len() as u64;
        if members.insert(name.clone(), data).is_some() {
            return Err(format!("{name} is in the zip twice"));
        }
    }
    Ok(members)
}

/// Builds the recording from `meta.json` and reads every array it names out of `members`. The outer error is about
/// `meta.json` itself; the inner one is about a member, and names it.
fn recording_from_meta(
    meta: json::Json,
    members: &mut BTreeMap<String, Vec<u8>>,
) -> Result<Result<Recording, String>, String> {
    let json::Json::Obj(pairs) = meta else {
        return Err("not a JSON object".into());
    };
    let mut m: BTreeMap<String, json::Json> = pairs.into_iter().collect();
    // The format first: a newer file may differ in everything else.
    let format = match m.remove("format") {
        None => return Err("no \"format\"".into()),
        Some(json::Json::Int(n)) => n,
        Some(other) => return Err(format!("\"format\" is {}, not an integer", other.describe())),
    };
    if format != i64::from(RECORDING_FORMAT) {
        let why = if format > i64::from(RECORDING_FORMAT) {
            "written by a newer app"
        } else {
            "not a known format"
        };
        return Err(format!("format {format} ({why}); this app reads {RECORDING_FORMAT}"));
    }
    if let Some(k) = m.keys().find(|k| !META_KEYS.contains(&k.as_str())) {
        return Err(format!(
            "unknown key {k:?} (format {RECORDING_FORMAT} has no place for it)"
        ));
    }
    let mut take = |key: &str| m.remove(key).ok_or_else(|| format!("no {key:?}"));

    let source = match take("source")? {
        json::Json::Obj(pairs) => {
            let mut s: BTreeMap<String, json::Json> = pairs.into_iter().collect();
            let kind = match s.remove("kind") {
                Some(json::Json::Str(k)) => k,
                _ => return Err("\"source\" has no \"kind\" string".into()),
            };
            let (key, make): (&str, fn(String) -> Source) = match kind.as_str() {
                "python" => ("commit", |commit| Source::Python { commit }),
                "studio" => ("version", |version| Source::Studio { version }),
                other => {
                    return Err(format!(
                        "\"source\" kind {other:?} is neither \"python\" nor \"studio\""
                    ));
                }
            };
            let text = match s.remove(key) {
                Some(json::Json::Str(t)) => t,
                _ => return Err(format!("\"source\" of kind {kind:?} has no {key:?} string")),
            };
            if let Some(extra) = s.keys().next() {
                return Err(format!("\"source\" has an unknown key {extra:?}"));
            }
            make(text)
        }
        other => return Err(format!("\"source\" is {}, not an object", other.describe())),
    };
    let names = match take("names")? {
        json::Json::List(items) => items
            .into_iter()
            .map(|n| match n {
                json::Json::Str(s) => Ok(s),
                other => Err(format!("\"names\" holds {}, not a string", other.describe())),
            })
            .collect::<Result<Vec<_>, _>>()?,
        other => return Err(format!("\"names\" is {}, not a list", other.describe())),
    };
    let seed = u64::try_from(int(take("seed")?, "seed")?).map_err(|_| "\"seed\" is negative".to_string())?;
    let winner = match take("winner")? {
        json::Json::Null => None,
        w => Some(usize::try_from(int(w, "winner")?).map_err(|_| "\"winner\" is negative".to_string())?),
    };
    let t_ko = match take("t_ko")? {
        json::Json::Null => None,
        json::Json::Float(t) => Some(t),
        other => return Err(format!("\"t_ko\" is {}, not a float or null", other.describe())),
    };
    let f0 = usize::try_from(int(take("f0")?, "f0")?).map_err(|_| "\"f0\" is negative".to_string())?;
    let fps = u32::try_from(int(take("fps")?, "fps")?).map_err(|_| "\"fps\" does not fit 0 to 2^32 - 1".to_string())?;
    let corner = match take("corner")? {
        json::Json::List(items) => items
            .into_iter()
            .map(|c| match c {
                json::Json::List(xy) => match xy.as_slice() {
                    [json::Json::Float(x), json::Json::Float(y)] => Ok([*x, *y]),
                    _ => Err("a \"corner\" is not a list of two floats".to_string()),
                },
                other => Err(format!("\"corner\" holds {}, not an [x, y] list", other.describe())),
            })
            .collect::<Result<Vec<_>, _>>()?,
        other => return Err(format!("\"corner\" is {}, not a list", other.describe())),
    };
    let rules = match take("rules")? {
        json::Json::Obj(pairs) => pairs
            .into_iter()
            .map(|(k, v)| {
                v.into_value()
                    .map(|v| (k.clone(), v))
                    .map_err(|e| format!("rules.{k}: {e}"))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?,
        other => return Err(format!("\"rules\" is {}, not an object", other.describe())),
    };
    let events = match take("events")? {
        json::Json::List(items) => items
            .into_iter()
            .enumerate()
            .map(|(n, e)| event(n, e))
            .collect::<Result<Vec<_>, _>>()?,
        other => return Err(format!("\"events\" is {}, not a list", other.describe())),
    };
    let kinds = match take("arrays")? {
        json::Json::Obj(pairs) => pairs
            .into_iter()
            .map(|(k, v)| match v {
                json::Json::Str(kind) if kind == "npy" || kind == "json" => Ok((k, kind)),
                other => Err(format!(
                    "\"arrays\".{k} is {}, not \"npy\" or \"json\"",
                    other.describe()
                )),
            })
            .collect::<Result<Vec<_>, _>>()?,
        other => return Err(format!("\"arrays\" is {}, not an object", other.describe())),
    };

    // From here on the problems are about members, not about meta.json.
    Ok(arrays_from_members(&kinds, members).map(|arrays| Recording {
        format: RECORDING_FORMAT,
        source,
        names,
        seed,
        winner,
        t_ko,
        f0,
        fps,
        corner,
        rules,
        events,
        arrays,
    }))
}

fn arrays_from_members(
    kinds: &[(String, String)],
    members: &mut BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, Array>, String> {
    let want: BTreeSet<String> = kinds.iter().map(|(k, kind)| format!("{k}.{kind}")).collect();
    if let Some(extra) = members.keys().find(|n| !want.contains(*n)) {
        return Err(format!(
            "the zip holds {extra}, which meta.json \"arrays\" does not name"
        ));
    }
    let mut arrays = BTreeMap::new();
    for (key, kind) in kinds {
        if !valid_key(key) {
            return Err(format!("meta.json names an array {key:?}, which cannot name a member"));
        }
        let name = format!("{key}.{kind}");
        let data = members
            .remove(&name)
            .ok_or_else(|| format!("{name} is missing (meta.json names {key})"))?;
        let array = if kind == "npy" {
            npy::read(&data).map_err(|e| format!("{name}: {e}"))?
        } else {
            match json::parse(&data).map_err(|e| format!("{name}: {e}"))?.into_value() {
                Ok(v @ Value::List(_)) => Array::Nested(v),
                Ok(_) => return Err(format!("{name} is not a per-frame list")),
                Err(e) => return Err(format!("{name}: {e}")),
            }
        };
        arrays.insert(key.clone(), array);
    }
    Ok(arrays)
}

fn int(j: json::Json, key: &str) -> Result<i64, String> {
    match j {
        json::Json::Int(n) => Ok(n),
        other => Err(format!("{key:?} is {}, not an integer", other.describe())),
    }
}

/// `[t, kind, ...]`: `t` a float, `kind` a string, then the fields.
fn event(n: usize, e: json::Json) -> Result<Event, String> {
    let json::Json::List(items) = e else {
        return Err(format!("event {n} is {}, not a list", e.describe()));
    };
    let mut items = items.into_iter();
    let t = match items.next() {
        Some(json::Json::Float(t)) => t,
        Some(other) => return Err(format!("event {n}: t is {}, not a float", other.describe())),
        None => return Err(format!("event {n} is empty")),
    };
    let kind = match items.next() {
        Some(json::Json::Str(k)) => k,
        Some(other) => return Err(format!("event {n}: kind is {}, not a string", other.describe())),
        None => return Err(format!("event {n} has no kind")),
    };
    let fields = items.map(json::Json::into_value).collect::<Result<Vec<_>, _>>();
    Ok(Event {
        t,
        fields: fields.map_err(|e| format!("event {n} ({kind}): {e}"))?,
        kind,
    })
}

/// ASCII letters, digits and `_`, not empty, never `meta`.
fn valid_key(key: &str) -> bool {
    !key.is_empty() && key != "meta" && key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

fn check_shape(key: &str, shape: &[usize], len: usize) -> Result<(), String> {
    if shape.is_empty() {
        return Err(format!("{key} has shape (), so no frame dimension"));
    }
    match shape.iter().try_fold(1usize, |n, &d| n.checked_mul(d)) {
        Some(n) if n == len => Ok(()),
        _ => Err(format!("{key} has shape {shape:?} but {len} elements")),
    }
}

/// The first non-finite float inside a value, if any.
fn finite_value(v: &Value) -> Result<(), f64> {
    match v {
        Value::Float(x) if !x.is_finite() => Err(*x),
        Value::List(items) => items.iter().try_for_each(finite_value),
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------- the content hash

fn put_u64(out: &mut Vec<u8>, n: u64) {
    out.extend_from_slice(&n.to_le_bytes());
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    out.push(b'S');
    put_u64(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}

fn put_shape(out: &mut Vec<u8>, tag: u8, shape: &[usize]) {
    out.push(tag);
    put_u64(out, shape.len() as u64);
    for &d in shape {
        put_u64(out, d as u64);
    }
}

fn put_value(out: &mut Vec<u8>, v: &Value) {
    match v {
        Value::Null => out.push(b'N'),
        Value::Bool(b) => out.push(if *b { b'T' } else { b'F' }),
        Value::Int(n) => {
            out.push(b'I');
            out.extend_from_slice(&n.to_le_bytes());
        }
        Value::Float(x) => {
            out.push(b'D');
            out.extend_from_slice(&x.to_bits().to_le_bytes());
        }
        Value::Str(s) => put_str(out, s),
        Value::List(items) => {
            out.push(b'L');
            put_u64(out, items.len() as u64);
            items.iter().for_each(|item| put_value(out, item));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deflated zip of these members.
    fn zip_of(members: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let options = zip::write::SimpleFileOptions::DEFAULT.compression_method(zip::CompressionMethod::Deflated);
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in members {
            z.start_file(*name, options).unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap().into_inner()
    }

    #[test]
    fn member_and_total_sizes_are_capped() {
        let bytes = zip_of(&[("A.npy", vec![0; 600]), ("B.npy", vec![0; 600])]);
        assert_eq!(unzip_capped(&bytes, 600, 1200).unwrap().len(), 2);
        let err = unzip_capped(&bytes, 599, 10_000).unwrap_err();
        assert!(err.contains("A.npy") && err.contains("declares 600"), "{err}");
        let err = unzip_capped(&bytes, 1000, 1000).unwrap_err();
        assert!(err.contains("B.npy") && err.contains("whole file"), "{err}");
    }
}
