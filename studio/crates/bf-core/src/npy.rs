//! Reader and writer for the two `.npy` dtypes a recording uses (Task 6).
//!
//! NumPy's `.npy` format, version 1.0, restricted to little-endian `<f8` ([`Array::F64`]) and `<i8`
//! ([`Array::I64`]) in C order (`docs/recording-format.md` section 4). The writer gives the bytes
//! `numpy.lib.format.write_array` gives (numpy 2.5.3): the same header text, the same spare space and padding.
//! Errors are plain messages; the recording reader adds the file and member names.

use crate::recording::Array;

const MAGIC: &[u8; 6] = b"\x93NUMPY";
/// numpy aligns the data on this many bytes.
const ARRAY_ALIGN: usize = 64;
/// numpy leaves this many characters of room for the first dimension, so a file can grow in place.
const GROWTH_AXIS_MAX_DIGITS: usize = 21;

/// Reads a whole `.npy` file. Anything but version 1.0, `<f8` or `<i8`, C order, and exactly
/// `8 * product(shape)` data bytes is an error.
pub fn read(bytes: &[u8]) -> Result<Array, String> {
    if bytes.len() < 10 || &bytes[..6] != MAGIC {
        return Err("not a .npy file (no \\x93NUMPY magic)".into());
    }
    if bytes[6..8] != [1, 0] {
        return Err(format!("npy version {}.{}, only 1.0 is read", bytes[6], bytes[7]));
    }
    let header_len = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    let data_at = 10 + header_len;
    let Some(header) = bytes.get(10..data_at) else {
        return Err(format!(
            "the header is {header_len} bytes but the file has only {}",
            bytes.len() - 10
        ));
    };
    let header = std::str::from_utf8(header)
        .ok()
        .filter(|h| h.is_ascii())
        .ok_or("the header is not ASCII text")?;
    if !header.ends_with('\n') {
        return Err("the header does not end with a newline".into());
    }
    let (descr, fortran_order, shape) = parse_header(header)?;
    if fortran_order {
        return Err("fortran_order is True; only C order is read".into());
    }
    let count = shape
        .iter()
        .try_fold(1usize, |n, &d| n.checked_mul(d))
        .and_then(|n| n.checked_mul(8))
        .ok_or_else(|| format!("shape {shape:?} is too large"))?;
    let data = &bytes[data_at..];
    if data.len() != count {
        return Err(format!(
            "shape {shape:?} wants {count} data bytes, the file has {}",
            data.len()
        ));
    }
    let words = data.as_chunks::<8>().0.iter().copied();
    match descr.as_str() {
        "<f8" => Ok(Array::F64 {
            shape,
            data: words.map(|w| f64::from_bits(u64::from_le_bytes(w))).collect(),
        }),
        "<i8" => Ok(Array::I64 {
            shape,
            data: words.map(i64::from_le_bytes).collect(),
        }),
        other => Err(format!("dtype {other:?}; only '<f8' and '<i8' are read")),
    }
}

/// Writes an `F64` or `I64` array as numpy would. A `Nested` array, or data whose length does not match the
/// shape, is an error.
pub fn write(array: &Array) -> Result<Vec<u8>, String> {
    let (descr, shape, len) = match array {
        Array::F64 { shape, data } => ("<f8", shape, data.len()),
        Array::I64 { shape, data } => ("<i8", shape, data.len()),
        Array::Nested(_) => return Err("a nested array has no .npy form".into()),
    };
    let count = shape.iter().try_fold(1usize, |n, &d| n.checked_mul(d));
    if count != Some(len) {
        return Err(format!("shape {shape:?} does not hold {len} elements"));
    }
    let shape_repr = match shape.as_slice() {
        [] => "()".to_string(),
        [n] => format!("({n},)"),
        dims => format!("({})", dims.iter().map(usize::to_string).collect::<Vec<_>>().join(", ")),
    };
    let mut header = format!("{{'descr': '{descr}', 'fortran_order': False, 'shape': {shape_repr}, }}");
    if let Some(first) = shape.first() {
        let room = GROWTH_AXIS_MAX_DIGITS.saturating_sub(first.to_string().len());
        header.push_str(&" ".repeat(room));
    }
    // numpy's _wrap_header: pad with 1 to 64 spaces so that magic + length + header + "\n" is aligned.
    let pad = ARRAY_ALIGN - (10 + header.len() + 1) % ARRAY_ALIGN;
    header.push_str(&" ".repeat(pad));
    header.push('\n');
    let header_len = u16::try_from(header.len()).map_err(|_| format!("the header for shape {shape:?} is too long"))?;
    let mut out = Vec::with_capacity(10 + header.len() + 8 * len);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&[1, 0]);
    out.extend_from_slice(&header_len.to_le_bytes());
    out.extend_from_slice(header.as_bytes());
    match array {
        Array::F64 { data, .. } => data
            .iter()
            .for_each(|x| out.extend_from_slice(&x.to_bits().to_le_bytes())),
        Array::I64 { data, .. } => data.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
        Array::Nested(_) => unreachable!("refused above"),
    }
    Ok(out)
}

/// The three keys of the header's dict literal, in any order and spacing: `descr`, `fortran_order`, `shape`.
fn parse_header(text: &str) -> Result<(String, bool, Vec<usize>), String> {
    let mut p = Lexer {
        s: text.as_bytes(),
        i: 0,
    };
    let bad = |what: &str| format!("the header is not a dict of descr, fortran_order and shape ({what}): {text:?}");
    let (mut descr, mut fortran, mut shape) = (None, None, None);
    p.expect(b'{').map_err(|_| bad("no '{'"))?;
    loop {
        if p.eat(b'}') {
            break;
        }
        let key = p.string().ok_or_else(|| bad("a key is not a string"))?;
        p.expect(b':').map_err(|_| bad("no ':'"))?;
        let seen = match key.as_str() {
            "descr" => descr
                .replace(p.string().ok_or_else(|| bad("descr is not a string"))?)
                .is_some(),
            "fortran_order" => fortran
                .replace(p.boolean().ok_or_else(|| bad("fortran_order is not a bool"))?)
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
            p.expect(b'}').map_err(|_| bad("no ',' or '}'"))?;
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
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
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

    fn expect(&mut self, c: u8) -> Result<(), ()> {
        if self.eat(c) { Ok(()) } else { Err(()) }
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
            self.skip_space();
            let start = self.i;
            while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
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
mod tests {
    use super::*;

    #[test]
    fn header_matches_numpy() {
        let a = Array::F64 {
            shape: vec![1394, 2, 2],
            data: vec![0.0; 1394 * 4],
        };
        let bytes = write(&a).unwrap();
        assert_eq!(&bytes[..10], b"\x93NUMPY\x01\x00\x76\x00");
        let header = std::str::from_utf8(&bytes[10..128]).unwrap();
        assert_eq!(
            header.trim_end(),
            "{'descr': '<f8', 'fortran_order': False, 'shape': (1394, 2, 2), }"
        );
        assert!(header.ends_with(" \n"));
        let empty = Array::I64 {
            shape: vec![],
            data: vec![5],
        };
        let bytes = write(&empty).unwrap();
        assert_eq!(bytes.len(), 128 + 8); // a 57-byte header pads to 128, as numpy does
        assert_eq!(read(&bytes).unwrap(), empty);
    }

    #[test]
    fn header_keys_in_any_order_and_spacing() {
        let header = "{ \"shape\" : ( 2 , 1 ) , 'fortran_order':False,'descr':'<i8'}";
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&[1, 0]);
        let mut h = header.to_string();
        h.push('\n');
        bytes.extend_from_slice(&(h.len() as u16).to_le_bytes());
        bytes.extend_from_slice(h.as_bytes());
        bytes.extend_from_slice(&3i64.to_le_bytes());
        bytes.extend_from_slice(&(-4i64).to_le_bytes());
        assert_eq!(
            read(&bytes).unwrap(),
            Array::I64 {
                shape: vec![2, 1],
                data: vec![3, -4]
            }
        );
    }

    #[test]
    fn nan_payloads_survive() {
        let a = Array::F64 {
            shape: vec![2],
            data: vec![f64::from_bits(0x7FF8_0000_0000_0001), -0.0],
        };
        let back = read(&write(&a).unwrap()).unwrap();
        let Array::F64 { data, .. } = back else { panic!() };
        assert_eq!(data[0].to_bits(), 0x7FF8_0000_0000_0001);
        assert_eq!(data[1].to_bits(), (-0.0f64).to_bits());
    }

    #[test]
    fn bad_files_are_errors() {
        let good = write(&Array::F64 {
            shape: vec![2],
            data: vec![1.0, 2.0],
        })
        .unwrap();
        assert!(read(&good[..5]).unwrap_err().contains("magic"));
        assert!(read(&good[..20]).unwrap_err().contains("header"));
        assert!(read(&good[..good.len() - 1]).unwrap_err().contains("data bytes"));
        let mut v2 = good.clone();
        v2[6] = 2;
        assert!(read(&v2).unwrap_err().contains("version 2.0"));
        let swap = |from: &[u8], to: &[u8]| {
            let mut b = good.clone();
            let at = b.windows(from.len()).position(|w| w == from).unwrap();
            b[at..at + to.len()].copy_from_slice(to);
            read(&b).unwrap_err()
        };
        assert!(swap(b"<f8", b">f8").contains("dtype"));
        assert!(swap(b"False", b"True ").contains("fortran_order"));
        assert!(swap(b"'shape'", b"'shapf'").contains("unknown key"));
        assert!(
            write(&Array::F64 {
                shape: vec![3],
                data: vec![1.0]
            })
            .is_err()
        );
    }
}
