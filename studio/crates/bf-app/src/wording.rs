//! Shared text formatters, so every screen writes names, counts and numbers the same way.

/// What the preview says when the PC has no sound output that works, or the one it had was lost: the transport, the
/// status bar and `open_audio` all use it.
pub const NO_DEVICE: &str = "No sound device was found, so the preview is silent.";

/// A roster name as shown, in sentence case: "BASS CLARINET" -> "Bass clarinet".
pub fn display_name(roster_name: &str) -> String {
    let lower = roster_name.to_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A count with a comma between thousands: 18240 -> "18,240".
pub fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, d) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(d);
    }
    out
}

/// A share as a whole percentage: 0.47 -> "47%".
pub fn percent(share: f64) -> String {
    format!("{:.0}%", share * 100.0)
}

/// A multiplier with two decimals: 2.0 -> "×2.00".
pub fn times(x: f64) -> String {
    format!("×{x:.2}")
}

/// Seconds with one decimal: 50.6 -> "50.6 s".
pub fn seconds(x: f64) -> String {
    format!("{x:.1} s")
}

/// A count and the word for it: (1, "video", "videos") -> "1 video". The count is written with [`thousands`].
pub fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{} {}", thousands(n), if n == 1 { one } else { many })
}

/// The name of a match-up's folder and files: ["BASS CLARINET", "BASSOON"] -> "bass_clarinet_vs_bassoon".
pub fn folder_stem(names: &[String; 2]) -> String {
    let stem = |name: &str| name.to_lowercase().split_whitespace().collect::<Vec<_>>().join("_");
    format!("{}_vs_{}", stem(&names[0]), stem(&names[1]))
}

/// A match-up as shown: ["FLUTE", "TROMBONE"] -> "Flute vs Trombone".
pub fn versus(names: &[String; 2]) -> String {
    format!("{} vs {}", display_name(&names[0]), display_name(&names[1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(a: &str, b: &str) -> [String; 2] {
        [a.to_string(), b.to_string()]
    }

    #[test]
    fn the_wording_helpers() {
        assert_eq!(display_name("BASS CLARINET"), "Bass clarinet");
        assert_eq!(thousands(18240), "18,240");
        assert_eq!(percent(0.47), "47%");
        assert_eq!(times(2.0), "×2.00");
        assert_eq!(seconds(50.6), "50.6 s");
        assert_eq!(plural(1, "video", "videos"), "1 video");
        assert_eq!(
            folder_stem(&pair("BASS CLARINET", "BASSOON")),
            "bass_clarinet_vs_bassoon"
        );
        assert_eq!(versus(&pair("FLUTE", "TROMBONE")), "Flute vs Trombone");
    }

    #[test]
    fn the_wording_helpers_at_their_edges() {
        assert_eq!(display_name("FLUTE"), "Flute");
        assert_eq!(display_name("ALTO SAX"), "Alto sax");
        assert_eq!(display_name(""), "");
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(3_000), "3,000");
        assert_eq!(thousands(1_234_567), "1,234,567");
        assert_eq!(percent(0.0), "0%");
        assert_eq!(percent(1.0), "100%");
        assert_eq!(percent(0.6), "60%");
        assert_eq!(times(0.4), "×0.40");
        assert_eq!(times(12.0), "×12.00");
        assert_eq!(seconds(49.0), "49.0 s");
        assert_eq!(plural(0, "video", "videos"), "0 videos");
        assert_eq!(plural(2, "video", "videos"), "2 videos");
        assert_eq!(plural(3_000, "fight", "fights"), "3,000 fights");
        assert_eq!(folder_stem(&pair("TUBA", "BASS DRUM")), "tuba_vs_bass_drum");
        assert_eq!(versus(&pair("ALTO SAX", "TENOR SAX")), "Alto sax vs Tenor sax");
        assert_eq!(versus(&pair("TUBA", "BASS DRUM")), "Tuba vs Bass drum");
    }
}
