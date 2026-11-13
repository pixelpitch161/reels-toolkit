//! `.npy` arrays and `.npz` archives in the six dtypes the reference vectors use.
//!
//! NumPy's `.npy` format, versions 1.0, 2.0 and 3.0, little-endian (or byte-sized) dtypes `|u1`, `|b1`, `<i4`,
//! `<i8`, `<f4`, `<f8`, C order only. The export tool writes version 1.0 (`tools/studio_export/common.py`,
//! `npy_bytes`); the later versions are read so a file written by plain numpy also opens. An `.npz` is a zip whose
//! members are `<name>.npy` (stored or deflated); [`Npz`] keys them by `<name>`.

use std::collections::BTreeMap;
use std::path::Path;

/// The elements of an array, flat in C order.
#[derive(Clone, Debug, PartialEq)]
pub enum NpData {
    U8(Vec<u8>),
    Bool(Vec<bool>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
}

/// One array: its shape (empty for a 0-d array, which holds one element) and its elements.
#[derive(Clone, Debug, PartialEq)]
pub struct NpArray {
    pub shape: Vec<usize>,
    pub data: NpData,
}

/// The dtypes read, as numpy writes them in the header (`dtype.str`).
const DTYPES: [&str; 6] = ["|u1", "|b1", "<i4", "<i8", "<f4", "<f8"];

/// A typed accessor: the elements if the array has that dtype, else a panic naming the dtype it has.
macro_rules! accessor {
    ($(#[$doc:meta])* $name:ident, $variant:ident, $t:ty, $descr:literal) => {
        $(#[$doc])*
        pub fn $name(&self) -> &[$t] {
            match &self.data {
                NpData::$variant(v) => v,
                _ => panic!("the array is {}, not {}", self.dtype(), $descr),
            }
        }
    };
}

impl NpArray {
    /// The number of elements (the product of the shape; 1 for a 0-d array).
    pub fn len(&self) -> usize {
        match &self.data {
            NpData::U8(v) => v.len(),
            NpData::Bool(v) => v.len(),
            NpData::I32(v) => v.len(),
            NpData::I64(v) => v.len(),
            NpData::F32(v) => v.len(),
            NpData::F64(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The numpy dtype string: `|u1`, `|b1`, `<i4`, `<i8`, `<f4` or `<f8`.
    pub fn dtype(&self) -> &'static str {
        match self.data {
            NpData::U8(_) => "|u1",
            NpData::Bool(_) => "|b1",
            NpData::I32(_) => "<i4",
            NpData::I64(_) => "<i8",
            NpData::F32(_) => "<f4",
            NpData::F64(_) => "<f8",
        }
    }

    accessor!(
        /// The elements of a `|u1` array.
        u8, U8, u8, "|u1"
    );
    accessor!(
        /// The elements of a `|b1` array.
        bool, Bool, bool, "|b1"
    );
    accessor!(
        /// The elements of an `<i4` array.
        i32, I32, i32, "<i4"
    );
    accessor!(
        /// The elements of an `<i8` array.
        i64, I64, i64, "<i8"
    );
    accessor!(
        /// The elements of an `<f4` array.
        f32, F32, f32, "<f4"
    );
    accessor!(
        /// The elements of an `<f8` array.
        f64, F64, f64, "<f8"
    );
}

/// Reads a whole `.npy` file. Errors name what is wrong (the version, the dtype, the order, the data length).
pub fn read_npy(bytes: &[u8]) -> Result<NpArray, String> {
    if bytes.len() < 10 || &bytes[..6] != b"\x93NUMPY" {
        return Err("not a .npy file (no \\x93NUMPY magic)".into());
    }
    let (major, minor) = (bytes[6], bytes[7]);
    let (header_len, header_at) = match (major, minor) {
        (1, 0) => (u16::from_le_bytes([bytes[8], bytes[9]]) as usize, 10),
        (2, 0) | (3, 0) if bytes.len() >= 12 => (
            u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize,
            12,
        ),
        (2, 0) | (3, 0) => return Err("the file ends inside the header length".into()),
        _ => return Err(format!("npy version {major}.{minor}; only 1.0, 2.0 and 3.0 are read")),
    };
    let data_at = header_at + header_len;
    let header = bytes.get(header_at..data_at).ok_or_else(|| {
        format!(
            "the header is {header_len} bytes but the file has only {} after the length",
            bytes.len() - header_at
        )
    })?;
    // Versions 1.0 and 2.0 hold latin-1 text, 3.0 UTF-8; a numeric header is plain ASCII either way.
    let header = std::str::from_utf8(header).map_err(|_| "the header is not text".to_string())?;
    let (descr, fortran_order, shape) = parse_header(header)?;
    if fortran_order {
        return Err(format!(
            "fortran_order is True (dtype '{descr}', shape {shape:?}); only C order is read"
        ));
    }
    if !DTYPES.contains(&descr.as_str()) {
        let why = if descr.starts_with('>') {
            " is big-endian"
        } else {
            " is not one of the six"
        };
        return Err(format!("dtype '{descr}'{why}; only {} are read", DTYPES.join(" ")));
    }
    let size = if descr.ends_with('1') {
        1
    } else if descr.ends_with('4') {
        4
    } else {
        8
    };
    let count = shape
        .iter()
        .try_fold(1usize, |n, &d| n.checked_mul(d))
        .ok_or_else(|| format!("shape {shape:?} is too large"))?;
    let data = &bytes[data_at..];
    if count.checked_mul(size) != Some(data.len()) {
        return Err(format!(
            "dtype '{descr}' shape {shape:?} wants {count} x {size} data bytes, the file has {}",
            data.len()
        ));
    }
    let data = match descr.as_str() {
        "|u1" => NpData::U8(data.to_vec()),
        "|b1" => NpData::Bool(
            data.iter()
                .map(|&b| match b {
                    0 => Ok(false),
                    1 => Ok(true),
                    other => Err(format!("a bool element is the byte {other}, not 0 or 1")),
                })
                .collect::<Result<_, _>>()?,
        ),
        "<i4" => NpData::I32(data.as_chunks::<4>().0.iter().map(|&w| i32::from_le_bytes(w)).collect()),
        "<i8" => NpData::I64(data.as_chunks::<8>().0.iter().map(|&w| i64::from_le_bytes(w)).collect()),
        "<f4" => NpData::F32(data.as_chunks::<4>().0.iter().map(|&w| f32::from_le_bytes(w)).collect()),
        _ => NpData::F64(data.as_chunks::<8>().0.iter().map(|&w| f64::from_le_bytes(w)).collect()),
    };
    Ok(NpArray { shape, data })
}

/// An `.npz` archive read whole: member name (without `.npy`) to array, in name order.
#[derive(Clone, Debug)]
pub struct Npz {
    arrays: BTreeMap<String, NpArray>,
    origin: String,
}

impl Npz {
    /// Reads the archive at `path`; panics naming the file (and the member) if it is missing or malformed.
    pub fn open(path: &Path) -> Npz {
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        Npz::from_bytes(&bytes, &path.display().to_string()).unwrap_or_else(|e| panic!("{e}"))
    }

    /// The member names, in ascending order.
    pub fn names(&self) -> Vec<&str> {
        self.arrays.keys().map(String::as_str).collect()
    }

    pub fn has(&self, name: &str) -> bool {
        self.arrays.contains_key(name)
    }

    /// The array `name`; panics listing the names the archive does have.
    pub fn get(&self, name: &str) -> &NpArray {
        self.arrays.get(name).unwrap_or_else(|| {
            panic!(
                "{} has no array {name:?}; available: {}",
                self.origin,
                self.names().join(", ")
            )
        })
    }

    /// Reads an archive from its bytes; `origin` (the file) goes into every error.
    pub(crate) fn from_bytes(bytes: &[u8], origin: &str) -> Result<Npz, String> {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes))
            .map_err(|e| format!("{origin} is not a readable zip: {e}"))?;
        let mut arrays = BTreeMap::new();
        for i in 0..z.len() {
            let mut member = z.by_index(i).map_err(|e| format!("{origin}: zip member {i}: {e}"))?;
            let file = member
                .name()
                .map_err(|e| format!("{origin}: zip member {i} has no readable name: {e}"))?
                .to_string();
            let name = file
                .strip_suffix(".npy")
                .ok_or_else(|| format!("{origin}: member {file:?} is not a .npy"))?
                .to_string();
            let mut data = Vec::new();
            std::io::Read::read_to_end(&mut member, &mut data)
                .map_err(|e| format!("{origin}: member {file:?}: {e}"))?;
            let array = read_npy(&data).map_err(|e| format!("{origin}: member {file:?}: {e}"))?;
            if arrays.insert(name, array).is_some() {
                return Err(format!("{origin}: member {file:?} appears twice"));
            }
        }
        Ok(Npz {
            arrays,
            origin: origin.to_string(),
        })
    }
}

/// The three keys of the header's dict literal, in any order and spacing: `descr`, `fortran_order`, `shape`.
fn parse_header(text: &str) -> Result<(String, bool, Vec<usize>), String> {
    let mut p = Lexer {
        s: text.as_bytes(),
        i: 0,
    };
    let bad = |what: &str| format!("the header is not a dict of descr, fortran_order and shape ({what}): {text:?}");
    let (mut descr, mut fortran, mut shape) = (None, None, None);
    if !p.eat(b'{') {
        return Err(bad("no '{'"));
    }
    loop {
        if p.eat(b'}') {
            break;
        }
        let key = p.string().ok_or_else(|| bad("a key is not a string"))?;
        if !p.eat(b':') {
            return Err(bad("no ':'"));
        }
        let seen = match key.as_str() {
            "descr" => descr
                .replace(
                    p.string()
                        .ok_or_else(|| bad("descr is not a dtype string; a structured dtype?"))?,
                )
                .is_some(),
            "fortran_order" => fortran
                .replace(p.boolean().ok_or_else(|| bad("fortran_order is not True or False"))?)
                .is_some(),
            "shape" => shape
                .replace(p.tuple().ok_or_else(|| bad("shape is not a tuple of integers"))?)
                .is_some(),
            other => return Err(bad(&format!("unknown key {other:?}"))),
        };
        if seen {
            return Err(bad(&format!("{key} given twice")));
        }
        if !p.eat(b',') {
            if !p.eat(b'}') {
                return Err(bad("no ',' or '}'"));
            }
            break;
        }
    }
    p.skip_space();
    if p.i != p.s.len() {
        return Err(bad("text after the '}'"));
    }
    match (descr, fortran, shape) {
        (Some(d), Some(f), Some(s)) => Ok((d, f, s)),
        _ => Err(bad("a key is missing")),
    }
}

/// A tiny reader for the Python literals a `.npy` header holds.
struct Lexer<'a> {
    s: &'a [u8],
    i: usize,
}

impl Lexer<'_> {
    fn skip_space(&mut self) {
        while self
            .s
            .get(self.i)
            .is_some_and(|c| matches!(c, b' ' | b'\t' | b'\n' | b'\r'))
        {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> bool {
        self.skip_space();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    /// A quoted string without escapes, in `'` or `"`.
    fn string(&mut self) -> Option<String> {
        self.skip_space();
        let q = *self.s.get(self.i).filter(|&&q| q == b'\'' || q == b'"')?;
        let start = self.i + 1;
        let len = self.s[start..].iter().position(|&c| c == q)?;
        let text = &self.s[start..start + len];
        if text.contains(&b'\\') {
            return None;
        }
        self.i = start + len + 1;
        Some(String::from_utf8_lossy(text).into_owned())
    }

    fn boolean(&mut self) -> Option<bool> {
        self.skip_space();
        for (word, value) in [(&b"True"[..], true), (&b"False"[..], false)] {
            if self.s[self.i..].starts_with(word) {
                self.i += word.len();
                return Some(value);
            }
        }
        None
    }

    /// `()`, `(n,)` or `(a, b, ...)` with an optional trailing comma: non-negative integers.
    fn tuple(&mut self) -> Option<Vec<usize>> {
        if !self.eat(b'(') {
            return None;
        }
        let mut dims = Vec::new();
        loop {
            if self.eat(b')') {
                return Some(dims);
            }
            let start = self.i;
            while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
                self.i += 1;
            }
            dims.push(std::str::from_utf8(&self.s[start..self.i]).ok()?.parse().ok()?);
            if !self.eat(b',') {
                return self.eat(b')').then_some(dims);
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A `.npy` file as numpy lays it out: magic, version, header length (2 bytes for 1.0, 4 for 2.0 and 3.0),
    /// the dict text padded with spaces and a newline to a multiple of 64 bytes, then the data.
    pub(crate) fn npy(version: u8, descr: &str, fortran: bool, shape: &str, data: &[u8]) -> Vec<u8> {
        let order = if fortran { "True" } else { "False" };
        let mut header = format!("{{'descr': '{descr}', 'fortran_order': {order}, 'shape': {shape}, }}");
        let fixed = if version == 1 { 10 } else { 12 };
        while (fixed + header.len() + 1) % 64 != 0 {
            header.push(' ');
        }
        header.push('\n');
        let mut out = b"\x93NUMPY".to_vec();
        out.extend_from_slice(&[version, 0]);
        if version == 1 {
            out.extend_from_slice(&(header.len() as u16).to_le_bytes());
        } else {
            out.extend_from_slice(&(header.len() as u32).to_le_bytes());
        }
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(data);
        out
    }

    fn le<T: Copy, const N: usize>(xs: &[T], f: impl Fn(T) -> [u8; N]) -> Vec<u8> {
        xs.iter().flat_map(|&x| f(x)).collect()
    }

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn reads_each_dtype() {
        let a = read_npy(&npy(1, "|u1", false, "(3,)", &[0, 7, 255])).unwrap();
        assert_eq!(
            (a.dtype(), a.shape.as_slice(), a.u8()),
            ("|u1", &[3][..], &[0, 7, 255][..])
        );
        let a = read_npy(&npy(1, "|b1", false, "(3,)", &[1, 0, 1])).unwrap();
        assert_eq!((a.dtype(), a.bool()), ("|b1", &[true, false, true][..]));
        let xs = [i32::MIN, -1, 0, 5, i32::MAX];
        let a = read_npy(&npy(1, "<i4", false, "(5,)", &le(&xs, i32::to_le_bytes))).unwrap();
        assert_eq!((a.dtype(), a.i32()), ("<i4", &xs[..]));
        let xs = [i64::MIN, -3, i64::MAX];
        let a = read_npy(&npy(1, "<i8", false, "(3,)", &le(&xs, i64::to_le_bytes))).unwrap();
        assert_eq!((a.dtype(), a.i64()), ("<i8", &xs[..]));
        let xs = [0.1f32, -0.0, f32::MAX, f32::from_bits(1)];
        let a = read_npy(&npy(1, "<f4", false, "(4,)", &le(&xs, f32::to_le_bytes))).unwrap();
        assert_eq!(a.dtype(), "<f4");
        assert_eq!(
            a.f32().iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            xs.map(f32::to_bits)
        );
        let xs = [0.1f64, -0.0, f64::from_bits(0x7FF8_0000_0000_0001)];
        let a = read_npy(&npy(1, "<f8", false, "(3,)", &le(&xs, f64::to_le_bytes))).unwrap();
        assert_eq!(a.dtype(), "<f8");
        assert_eq!(
            a.f64().iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            xs.map(f64::to_bits)
        );
    }

    #[test]
    fn reads_0d_1d_and_3d_shapes() {
        // np.array(-0.5) through tools/studio_export/common.py npy_bytes (numpy 2.5.3), byte for byte.
        let zero_d = hex(
            "934e554d5059010076007b276465736372273a20273c6638272c2027666f727472616e5f6f72646572273a2046616c73652c\
             20277368617065273a2028292c207d2020202020202020202020202020202020202020202020202020202020202020202020\
             2020202020202020202020202020202020202020202020202020200a000000000000e0bf",
        );
        let a = read_npy(&zero_d).unwrap();
        assert_eq!(
            (a.shape.as_slice(), a.f64(), a.len(), a.is_empty()),
            (&[][..], &[-0.5][..], 1, false)
        );
        let a = read_npy(&npy(1, "<i8", false, "(2,)", &le(&[4i64, 9], i64::to_le_bytes))).unwrap();
        assert_eq!((a.shape.as_slice(), a.len()), (&[2][..], 2));
        let bytes: Vec<u8> = (0..24).collect();
        let a = read_npy(&npy(1, "|u1", false, "(2, 3, 4)", &bytes)).unwrap();
        assert_eq!((a.shape.as_slice(), a.u8(), a.len()), (&[2, 3, 4][..], &bytes[..], 24));
        let a = read_npy(&npy(1, "<f4", false, "(0,)", &[])).unwrap();
        assert_eq!((a.shape.as_slice(), a.len(), a.is_empty()), (&[0][..], 0, true));
        let a = read_npy(&npy(1, "|b1", false, "(2, 0, 3)", &[])).unwrap();
        assert!(a.is_empty() && a.bool().is_empty());
    }

    #[test]
    fn reads_format_2_and_3_headers() {
        let xs = [1.5f64, -2.25];
        for version in [2, 3] {
            let a = read_npy(&npy(version, "<f8", false, "(1, 2)", &le(&xs, f64::to_le_bytes))).unwrap();
            assert_eq!(
                (a.shape.as_slice(), a.f64()),
                (&[1, 2][..], &xs[..]),
                "version {version}"
            );
        }
    }

    #[test]
    fn refuses_what_it_cannot_read() {
        let eight = [0u8; 8];
        let err = |b: Vec<u8>| read_npy(&b).unwrap_err();
        assert!(err(npy(1, "<f8", true, "(1,)", &eight)).contains("fortran_order"));
        let e = err(npy(1, ">f8", false, "(1,)", &eight));
        assert!(e.contains("'>f8'") && e.contains("big-endian"), "{e}");
        let e = err(npy(1, "|O", false, "(1,)", &eight));
        assert!(e.contains("'|O'"), "{e}");
        let e = err(npy(1, "<U3", false, "(1,)", &[0; 12]));
        assert!(e.contains("'<U3'"), "{e}");
        assert!(err(npy(1, "<f8", false, "(2,)", &eight)).contains("data bytes"));
        assert!(err(npy(1, "|b1", false, "(1,)", &[2])).contains("bool"));
        let mut v4 = npy(1, "<f8", false, "(1,)", &eight);
        v4[6] = 4;
        assert!(err(v4).contains("version 4.0"));
        assert!(err(b"PK\x03\x04".to_vec()).contains("magic"));
        let e = err(npy(1, "<f8", false, "(1,)", &eight)[..20].to_vec());
        assert!(e.contains("header"), "{e}");
    }

    #[test]
    #[should_panic(expected = "the array is <i4, not <f8")]
    fn accessors_name_the_real_dtype() {
        let a = read_npy(&npy(1, "<i4", false, "(1,)", &[1, 0, 0, 0])).unwrap();
        a.f64();
    }

    /// `zip_bytes` of three `npy_bytes` members, made by tools/studio_export/common.py (deflate level 9, the
    /// 1980 timestamp, sorted names): `c000__in` = int32 [[1,2,3],[4,5,6]], `c000__out` = float64 -0.5 (0-d),
    /// `mask` = bool [True, False, True].
    pub(crate) const PYTHON_NPZ: &str = "504b030414000000080000002100d34b2eae57000000980000000c000000633030305f5f696e2e6e70799bec17ea1b10c9c850c650ad9e925a9c5ca46ea5a06e9369a2aea3a09e965f54529498179f5f94920a12774bcc294e058a17672416a402f91a463a0ac69a3a0ab50a64032e460606062620660662162066056236200600504b030414000000080000002100506d2a8447000000880000000d000000633030305f5f6f75742e6e70799bec17ea1b10c9c850c650ad9e925a9c5ca46ea5a06e9366a1aea3a09e965f54529498179f5f94920a12774bcc294e058a17672416a402f91a9a3a0ab50a14012e063078b01f00504b03041400000008000000210016938cbb4700000083000000080000006d61736b2e6e70799bec17ea1b10c9c850c650ad9e925a9c5ca46ea5a05e9364a8aea3a09e965f54529498179f5f94920a12774bcc294e058a17672416a402f91ac63a9a3a0ab50a14002e46064600504b0102140014000000080000002100d34b2eae57000000980000000c0000000000000000000000800100000000633030305f5f696e2e6e7079504b0102140014000000080000002100506d2a8447000000880000000d0000000000000000000000800181000000633030305f5f6f75742e6e7079504b010214001400000008000000210016938cbb47000000830000000800000000000000000000008001f30000006d61736b2e6e7079504b05060000000003000300ab000000600100000000";

    #[test]
    fn reads_an_npz_the_export_tool_wrote() {
        let z = Npz::from_bytes(&hex(PYTHON_NPZ), "python.npz").unwrap();
        assert_eq!(z.names(), ["c000__in", "c000__out", "mask"]);
        assert!(z.has("mask") && !z.has("mask.npy"));
        let a = z.get("c000__in");
        assert_eq!((a.shape.as_slice(), a.i32()), (&[2, 3][..], &[1, 2, 3, 4, 5, 6][..]));
        assert_eq!(z.get("c000__out").f64(), [-0.5]);
        assert_eq!(z.get("mask").bool(), [true, false, true]);
    }

    #[test]
    fn a_bad_member_is_named() {
        let mut bytes = hex(PYTHON_NPZ);
        bytes.truncate(100);
        assert!(Npz::from_bytes(&bytes, "cut.npz").unwrap_err().contains("cut.npz"));
    }

    #[test]
    #[should_panic(expected = "available: c000__in, c000__out, mask")]
    fn get_lists_the_names_it_has() {
        Npz::from_bytes(&hex(PYTHON_NPZ), "python.npz").unwrap().get("c001__in");
    }
}
