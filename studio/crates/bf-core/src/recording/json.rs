//! JSON text to and from [`Value`]s, exactly, for the recording container.
//!
//! A number with no `.`, `e` or `E` is an `Int` (and must fit 64 bits), any other is a `Float` read to the nearest
//! `f64` and must be finite; floats are written with the shortest text that reads back to the same bits, always
//! with a `.` or an exponent.

use std::collections::BTreeSet;

use crate::event::Value;

/// Deepest nesting a JSON member may have (the kit's deepest is 4); deeper input is refused, not a stack overflow.
const MAX_DEPTH: usize = 64;

/// A parsed JSON value: a [`Value`] or an object (keys in file order; a repeated key is an error).
#[derive(Debug)]
pub(super) enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// A short name of the kind of value, for messages.
    pub(super) fn describe(&self) -> String {
        match self {
            Json::Null => "null".into(),
            Json::Bool(b) => b.to_string(),
            Json::Int(n) => format!("the integer {n}"),
            Json::Float(x) => format!("the float {x:?}"),
            Json::Str(s) => format!("the string {s:?}"),
            Json::List(_) => "a list".into(),
            Json::Obj(_) => "an object".into(),
        }
    }

    /// As a [`Value`]: an object has no place in a value in format 1.
    pub(super) fn into_value(self) -> Result<Value, String> {
        Ok(match self {
            Json::Null => Value::Null,
            Json::Bool(b) => Value::Bool(b),
            Json::Int(n) => Value::Int(n),
            Json::Float(x) => Value::Float(x),
            Json::Str(s) => Value::Str(s),
            Json::List(items) => Value::List(items.into_iter().map(Json::into_value).collect::<Result<_, _>>()?),
            Json::Obj(_) => return Err("a JSON object inside a value (format 1 has none)".into()),
        })
    }
}

/// Parses one JSON document (UTF-8, surrounding whitespace allowed).
pub(super) fn parse(bytes: &[u8]) -> Result<Json, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("not UTF-8 text ({e})"))?;
    let mut p = Parser {
        s: text.as_bytes(),
        i: 0,
    };
    let v = p.value(0).map_err(|e| p.at(e))?;
    p.space();
    if p.i != p.s.len() {
        return Err(p.at("text after the JSON value".into()));
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    /// The message with the line and column of the current position.
    fn at(&self, problem: String) -> String {
        let upto = &self.s[..self.i.min(self.s.len())];
        let line = 1 + upto.iter().filter(|&&c| c == b'\n').count();
        let col = 1 + upto.iter().rev().take_while(|&&c| c != b'\n').count();
        format!("not valid JSON at line {line}, column {col}: {problem}")
    }

    fn space(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn word(&mut self, w: &str) -> bool {
        if self.s[self.i..].starts_with(w.as_bytes()) {
            self.i += w.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > MAX_DEPTH {
            return Err(format!("nested deeper than {MAX_DEPTH}"));
        }
        self.space();
        match self.peek() {
            None => Err("the text ends where a value should be".into()),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.list(depth),
            Some(b'"') => self.string().map(Json::Str),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) if self.word("null") => Ok(Json::Null),
            Some(_) if self.word("true") => Ok(Json::Bool(true)),
            Some(_) if self.word("false") => Ok(Json::Bool(false)),
            Some(_) if self.word("NaN") => Err("NaN is not a finite float (every float must be finite)".into()),
            Some(_) if self.word("Infinity") => {
                Err("Infinity is not a finite float (every float must be finite)".into())
            }
            Some(c) => Err(format!("unexpected {:?}", c as char)),
        }
    }

    fn list(&mut self, depth: usize) -> Result<Json, String> {
        self.i += 1;
        let mut items = Vec::new();
        self.space();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Json::List(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.space();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(Json::List(items));
                }
                _ => return Err("expected ',' or ']' in a list".into()),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, String> {
        self.i += 1;
        let mut pairs: Vec<(String, Json)> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        self.space();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Json::Obj(pairs));
        }
        loop {
            self.space();
            if self.peek() != Some(b'"') {
                return Err("expected a string key".into());
            }
            let key = self.string()?;
            if !seen.insert(key.clone()) {
                return Err(format!("the key {key:?} is given twice"));
            }
            self.space();
            if self.peek() != Some(b':') {
                return Err("expected ':' after a key".into());
            }
            self.i += 1;
            let v = self.value(depth + 1)?;
            pairs.push((key, v));
            self.space();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Json::Obj(pairs));
                }
                _ => return Err("expected ',' or '}' in an object".into()),
            }
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
        }
        self.i - start
    }

    /// `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`, as an `Int` without a fraction or an exponent.
    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
            if self.word("Infinity") {
                return Err("-Infinity is not a finite float (every float must be finite)".into());
            }
        }
        match self.peek() {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err("a number has no digits".into()),
        }
        let mut float = false;
        if self.peek() == Some(b'.') {
            self.i += 1;
            if self.digits() == 0 {
                return Err("a number has no digits after '.'".into());
            }
            float = true;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if self.digits() == 0 {
                return Err("a number has no digits in its exponent".into());
            }
            float = true;
        }
        if self.peek().is_some_and(|c| c.is_ascii_digit()) {
            return Err("a number has a leading zero".into());
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).expect("ASCII");
        if float {
            // Rust's parse is correctly rounded: the nearest f64, as Python's float() gives.
            let x: f64 = text.parse().map_err(|_| format!("{text} is not a number"))?;
            if !x.is_finite() {
                return Err(format!("{text} is not a finite float (every float must be finite)"));
            }
            Ok(Json::Float(x))
        } else {
            text.parse()
                .map(Json::Int)
                .map_err(|_| format!("the integer {text} does not fit 64 bits"))
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let digits = self.s.get(self.i..self.i + 4).ok_or("a \\u escape is cut short")?;
        let text = std::str::from_utf8(digits).map_err(|_| "a bad \\u escape".to_string())?;
        let n = u32::from_str_radix(text, 16).map_err(|_| format!("a bad \\u escape {text:?}"))?;
        if !text.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("a bad \\u escape {text:?}"));
        }
        self.i += 4;
        Ok(n)
    }

    fn string(&mut self) -> Result<String, String> {
        self.i += 1;
        let mut out = String::new();
        loop {
            let start = self.i;
            while self.peek().is_some_and(|c| c != b'"' && c != b'\\' && c >= 0x20) {
                self.i += 1;
            }
            out.push_str(std::str::from_utf8(&self.s[start..self.i]).expect("checked UTF-8"));
            match self.peek() {
                None => return Err("a string is not closed".into()),
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.i += 1;
                    let c = self.peek().ok_or("a string is not closed")?;
                    self.i += 1;
                    match c {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let mut code = self.hex4()?;
                            if (0xD800..0xDC00).contains(&code) {
                                if !self.word("\\u") {
                                    return Err("a lone surrogate in a \\u escape".into());
                                }
                                let low = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&low) {
                                    return Err("a lone surrogate in a \\u escape".into());
                                }
                                code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                            }
                            out.push(char::from_u32(code).ok_or("a lone surrogate in a \\u escape")?);
                        }
                        other => return Err(format!("a bad escape \\{}", other as char)),
                    }
                }
                Some(_) => return Err("a control character inside a string".into()),
            }
        }
    }
}

/// Appends the compact JSON text of a value (`, ` between list items, as Python writes). Floats must be finite
/// (the recording's check comes first).
pub(super) fn write(out: &mut String, v: &Value) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(n) => out.push_str(&n.to_string()),
        Value::Float(x) => {
            debug_assert!(x.is_finite());
            // Debug formatting is the shortest round-trip text and always has a '.' or an exponent.
            out.push_str(&format!("{x:?}"));
        }
        Value::Str(s) => write_str(out, s),
        Value::List(items) => {
            out.push('[');
            for (n, item) in items.iter().enumerate() {
                if n > 0 {
                    out.push_str(", ");
                }
                write(out, item);
            }
            out.push(']');
        }
    }
}

/// A JSON string, ASCII only (other characters as `\u` escapes, as Python's `ensure_ascii`).
pub(super) fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ' '..='~' => out.push(c),
            _ => {
                let mut units = [0u16; 2];
                for u in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{u:04x}"));
                }
            }
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str) -> Result<Value, String> {
        parse(text.as_bytes()).and_then(Json::into_value)
    }

    #[test]
    fn int_or_float_by_the_text() {
        assert_eq!(value("1"), Ok(Value::Int(1)));
        assert_eq!(value("-0"), Ok(Value::Int(0)));
        assert_eq!(value("1.0"), Ok(Value::Float(1.0)));
        assert_eq!(value("1e2"), Ok(Value::Float(100.0)));
        assert_eq!(value("1E+2"), Ok(Value::Float(100.0)));
        let neg_zero = value("-0.0").unwrap();
        assert!(matches!(neg_zero, Value::Float(x) if x.to_bits() == (-0.0f64).to_bits()));
        assert_eq!(value("9223372036854775807"), Ok(Value::Int(i64::MAX)));
        assert_eq!(value("-9223372036854775808"), Ok(Value::Int(i64::MIN)));
        assert!(value("9223372036854775808").unwrap_err().contains("64 bits"));
    }

    #[test]
    fn floats_read_to_the_nearest_double() {
        for (text, bits) in [
            ("1.4000000000000001", 1.4000000000000001f64.to_bits()),
            ("0.1", 0.1f64.to_bits()),
            ("5e-324", 1),
            ("2.4703282292062328e-324", 1),
            ("1.7976931348623157e308", f64::MAX.to_bits()),
        ] {
            assert!(
                matches!(value(text), Ok(Value::Float(x)) if x.to_bits() == bits),
                "{text}"
            );
        }
    }

    #[test]
    fn non_finite_and_malformed_are_errors() {
        for bad in ["NaN", "Infinity", "-Infinity", "1e999", "-1e999"] {
            assert!(value(bad).unwrap_err().contains("finite"), "{bad}");
        }
        for bad in [
            "",
            "[1,",
            "01",
            "1.",
            ".5",
            "1e",
            "[1 2]",
            "{\"a\":1,\"a\":2}",
            "\"\\x\"",
            "\"a",
            "[]x",
            "+1",
        ] {
            assert!(value(bad).is_err(), "{bad:?}");
        }
        let deep = "[".repeat(MAX_DEPTH + 2) + &"]".repeat(MAX_DEPTH + 2);
        assert!(value(&deep).unwrap_err().contains("deeper"));
    }

    #[test]
    fn strings_and_escapes() {
        assert_eq!(
            value(r#""a\"b\\c\/\n\u00e9\ud83d\ude00""#),
            Ok(Value::Str("a\"b\\c/\né\u{1F600}".into()))
        );
        assert!(value(r#""\ud83d""#).is_err());
        let mut s = String::new();
        write_str(&mut s, "a\"b\\é\u{1F600}\u{1}");
        assert_eq!(s, r#""a\"b\\\u00e9\ud83d\ude00\u0001""#);
        assert_eq!(value(&s), Ok(Value::Str("a\"b\\é\u{1F600}\u{1}".into())));
    }

    #[test]
    fn floats_write_shortest_with_a_point_or_exponent() {
        let text = |x: f64| {
            let mut s = String::new();
            write(&mut s, &Value::Float(x));
            s
        };
        assert_eq!(text(100.0), "100.0");
        assert_eq!(text(-0.0), "-0.0");
        assert_eq!(text(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(text(1e-5), "1e-5");
        assert_eq!(text(1e300), "1e300");
        for x in [
            100.0,
            -0.0,
            1e-5,
            1e300,
            5e-324,
            f64::MAX,
            1.4000000000000001,
            1e16,
            123456789012345680.0,
        ] {
            let back = value(&text(x)).unwrap();
            assert!(matches!(back, Value::Float(y) if y.to_bits() == x.to_bits()), "{x:?}");
        }
    }
}
