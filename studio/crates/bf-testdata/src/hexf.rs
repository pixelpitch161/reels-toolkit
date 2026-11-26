//! Python `float.hex()` strings, the form the sidecars give every number Rust must reproduce bit for bit.

use serde_json::Value;

/// Parses exactly what Python's `float.hex()` prints: `[-]0x1.<13 hex digits>p<exp>` for a normal number,
/// `[-]0x0.<13 hex digits>p-1022` for a subnormal, `[-]0x0.0p+0` for zero, and `inf`, `-inf`, `nan`. Every bit
/// is kept; anything else panics naming the text.
pub fn parse_hex_f64(s: &str) -> f64 {
    parse(s).unwrap_or_else(|e| panic!("{e}"))
}

fn parse(s: &str) -> Result<f64, String> {
    let bad = |why: &str| format!("{s:?} is not a Python float.hex() string: {why}");
    match s {
        "inf" => return Ok(f64::INFINITY),
        "-inf" => return Ok(f64::NEG_INFINITY),
        "nan" => return Ok(f64::NAN),
        _ => {}
    }
    let (sign, body) = match s.strip_prefix('-') {
        Some(rest) => (1u64 << 63, rest),
        None => (0, s),
    };
    let body = body.strip_prefix("0x").ok_or_else(|| bad("no 0x"))?;
    let (mant, exp) = body.split_once('p').ok_or_else(|| bad("no p exponent"))?;
    let (lead, frac) = mant.split_once('.').ok_or_else(|| bad("no '.'"))?;
    let digits = exp
        .strip_prefix(['+', '-'])
        .ok_or_else(|| bad("the exponent has no sign"))?;
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return Err(bad("the exponent is not a number"));
    }
    let exp: i64 = exp.parse().map_err(|_| bad("the exponent is out of range"))?;
    let magnitude = if (lead, frac) == ("0", "0") {
        if exp != 0 {
            return Err(bad("zero with an exponent other than +0"));
        }
        0
    } else {
        if frac.len() != 13 || !frac.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(bad("the fraction is not 13 hex digits"));
        }
        let f = u64::from_str_radix(frac, 16).map_err(|_| bad("the fraction is not hex"))?;
        match lead {
            "1" if (-1022..=1023).contains(&exp) => (((exp + 1023) as u64) << 52) | f,
            "1" => return Err(bad("the exponent is outside -1022..=1023")),
            "0" if exp == -1022 && f != 0 => f,
            "0" => return Err(bad("a subnormal must be non-zero with exponent -1022")),
            _ => return Err(bad("the leading digit is not 0 or 1")),
        }
    };
    Ok(f64::from_bits(sign | magnitude))
}

/// A sidecar argument as an `f64`: a JSON string in `float.hex()` form, or a plain JSON number. Panics on any
/// other JSON value.
pub fn hex_arg(v: &Value) -> f64 {
    match v {
        Value::String(s) => parse_hex_f64(s),
        Value::Number(n) => n
            .as_f64()
            .unwrap_or_else(|| panic!("the JSON number {n} has no f64 form")),
        other => panic!("expected a float.hex() string or a JSON number, got {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `float.hex()` and the IEEE bits of each value, printed by Python 3.13.18 on the reference PC.
    const PYTHON: [(&str, u64); 35] = [
        ("0x0.0p+0", 0x0000000000000000),
        ("-0x0.0p+0", 0x8000000000000000),
        ("0x1.0000000000000p+0", 0x3ff0000000000000),
        ("-0x1.0000000000000p+0", 0xbff0000000000000),
        ("0x1.999999999999ap-4", 0x3fb999999999999a),
        ("-0x1.999999999999ap-4", 0xbfb999999999999a),
        ("0x1.0000000000000p-1", 0x3fe0000000000000),
        ("0x1.0000000000000p+1", 0x4000000000000000),
        ("0x1.8000000000000p+1", 0x4008000000000000),
        ("-0x1.8000000000000p+1", 0xc008000000000000),
        ("0x1.5555555555555p-2", 0x3fd5555555555555),
        ("0x1.5555555555555p-1", 0x3fe5555555555555),
        ("0x1.921fb54442d18p+1", 0x400921fb54442d18),
        ("0x1.5bf0a8b145769p+1", 0x4005bf0a8b145769),
        ("0x1.56e1fc2f8f359p-997", 0x01a56e1fc2f8f359),
        ("0x1.7e43c8800759cp+996", 0x7e37e43c8800759c),
        ("0x0.0000000000001p-1022", 0x0000000000000001), // the smallest subnormal
        ("-0x0.0000000000001p-1022", 0x8000000000000001),
        ("0x0.012688b70e62bp-1022", 0x000012688b70e62b),
        ("0x1.0000000000000p-1022", 0x0010000000000000), // the smallest normal
        ("0x0.fffffffffffffp-1022", 0x000fffffffffffff), // the largest subnormal
        ("0x1.fffffffffffffp+1023", 0x7fefffffffffffff), // the largest finite
        ("-0x1.fffffffffffffp+1023", 0xffefffffffffffff),
        ("0x1.0000000000001p+0", 0x3ff0000000000001),
        ("0x1.fffffffffffffp-1", 0x3fefffffffffffff),
        ("0x1.d6f3454000000p+26", 0x419d6f3454000000),
        ("-0x1.a36e2eb1c432dp-14", 0xbf1a36e2eb1c432d),
        ("0x1.4f8b588e368f1p-17", 0x3ee4f8b588e368f1),
        ("0x1.fffe000000000p+15", 0x40efffe000000000),
        ("0x1.0000000000000p+32", 0x41f0000000000000),
        ("0x1.0000000000000p+53", 0x4340000000000000),
        ("0x1.3333333333334p-2", 0x3fd3333333333334),
        ("0x1.8000000000000p+0", 0x3ff8000000000000),
        ("-0x1.12e0be826d695p-32", 0xbdf12e0be826d695),
        ("0x1.fe185ca57c517p+78", 0x44dfe185ca57c517),
    ];

    #[test]
    fn python_float_hex_is_read_bit_exactly() {
        for (text, bits) in PYTHON {
            let got = parse_hex_f64(text);
            assert_eq!(got.to_bits(), f64::from_bits(bits).to_bits(), "{text}");
        }
        assert_eq!(parse_hex_f64("0x1.999999999999ap-4"), 0.1);
        assert_eq!(parse_hex_f64("0x0.0000000000001p-1022"), 5e-324);
        assert_eq!(parse_hex_f64("0x1.fffffffffffffp+1023"), f64::MAX);
        assert_eq!(parse_hex_f64("0x1.0000000000000p-1022"), f64::MIN_POSITIVE);
    }

    #[test]
    fn specials() {
        assert_eq!(parse_hex_f64("inf"), f64::INFINITY);
        assert_eq!(parse_hex_f64("-inf"), f64::NEG_INFINITY);
        assert!(parse_hex_f64("nan").is_nan());
        assert!(parse_hex_f64("-0x0.0p+0").is_sign_negative());
    }

    #[test]
    fn malformed_text_panics_naming_it() {
        for bad in [
            "",
            "0.1",
            "1e5",
            "0x1.999p-4",              // too few fraction digits
            "0x1.999999999999ap",      // no exponent
            "0x2.0000000000000p+0",    // leading digit other than 0 or 1
            "0x1.0000000000000p+1024", // exponent out of range
            "0x0.0000000000001p-1021", // subnormal with the wrong exponent
            "0x0.0p+1",                // zero with an exponent
            "0x1.000000000000gp+0",    // not hex
            " 0x1.0000000000000p+0",   // stray space
            "0x1.0000000000000p+0 ",
        ] {
            let text = parse(bad).expect_err(bad);
            assert!(text.contains(&format!("{bad:?}")), "{bad:?}: {text}");
        }
    }

    #[test]
    #[should_panic(expected = "\"0x1.8p+1\"")]
    fn parse_hex_f64_panics_on_malformed_text() {
        parse_hex_f64("0x1.8p+1");
    }

    #[test]
    fn hex_arg_takes_strings_and_numbers() {
        assert_eq!(hex_arg(&Value::from("0x1.8000000000000p+1")), 3.0);
        assert_eq!(hex_arg(&serde_json::json!(0.1)), 0.1);
        assert_eq!(hex_arg(&serde_json::json!(-7)), -7.0);
        assert_eq!(hex_arg(&Value::from("-0x0.0p+0")).to_bits(), (-0.0f64).to_bits());
    }

    #[test]
    #[should_panic(expected = "true")]
    fn hex_arg_refuses_other_values() {
        hex_arg(&Value::Bool(true));
    }
}
