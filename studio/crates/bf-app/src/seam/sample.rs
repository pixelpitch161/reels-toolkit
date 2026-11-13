//! The sample data: the mock-ups' own roster, fights, numbers and wording (interface reference, section 11), which
//! the screens, the fixtures, the sample engine and `--fixtures` draw from until the real engine is behind the seam.

//!
//! Everything here is a fixed, deterministic stand-in: the same call always gives the same value, and no number
//! here ever reaches a real fight, frame or soundtrack.

use std::path::PathBuf;
use std::sync::Arc;

use crate::model::balance::BalanceStats;
use crate::model::draft::{CaptionTexts, Draft};
use crate::model::library::LibraryEntry;
use crate::model::roster::{AbilityLine, FighterCard, PairMusic, PixelMask, Roster, RosterEntry, Section, ThemeEntry};
use crate::model::story::{AbilityMark, StoryView};
use crate::model::take::{AudioTake, CheckLine, Frame, FrameSource, Lane, LaneGroup, MusicView};
use crate::model::{NA, NF, SR};

// ---------------------------------------------------------------------------------------------------------------
// The pixel font
// ---------------------------------------------------------------------------------------------------------------

/// Glyph advance of the series font: 5 px glyph and 1 px gap.
const ADVANCE: usize = 6;

/// The 5 x 7 glyphs A to Z, copied from the `_G` table of `reelkit/font.py`.
const GLYPHS: [&str; 26] = [
    ".###.|#...#|#...#|#####|#...#|#...#|#...#",
    "####.|#...#|#...#|####.|#...#|#...#|####.",
    ".###.|#...#|#....|#....|#....|#...#|.###.",
    "####.|#...#|#...#|#...#|#...#|#...#|####.",
    "#####|#....|#....|####.|#....|#....|#####",
    "#####|#....|#....|####.|#....|#....|#....",
    ".###.|#...#|#....|#.###|#...#|#...#|.####",
    "#...#|#...#|#...#|#####|#...#|#...#|#...#",
    "#####|..#..|..#..|..#..|..#..|..#..|#####",
    "..###|...#.|...#.|...#.|#..#.|#..#.|.##..",
    "#...#|#..#.|#.#..|##...|#.#..|#..#.|#...#",
    "#....|#....|#....|#....|#....|#....|#####",
    "#...#|##.##|#.#.#|#.#.#|#...#|#...#|#...#",
    "#...#|##..#|#.#.#|#..##|#...#|#...#|#...#",
    ".###.|#...#|#...#|#...#|#...#|#...#|.###.",
    "####.|#...#|#...#|####.|#....|#....|#....",
    ".###.|#...#|#...#|#...#|#.#.#|#..#.|.##.#",
    "####.|#...#|#...#|####.|#.#..|#..#.|#...#",
    ".####|#....|#....|.###.|....#|....#|####.",
    "#####|..#..|..#..|..#..|..#..|..#..|..#..",
    "#...#|#...#|#...#|#...#|#...#|#...#|.###.",
    "#...#|#...#|#...#|#...#|#...#|.#.#.|..#..",
    "#...#|#...#|#...#|#.#.#|#.#.#|#.#.#|.#.#.",
    "#...#|#...#|.#.#.|..#..|.#.#.|#...#|#...#",
    "#...#|#...#|.#.#.|..#..|..#..|..#..|..#..",
    "#####|....#|...#.|..#..|.#...|#....|#####",
];

/// `text` in the series' 5 x 7 pixel font, as `reelkit/font.py`'s `text_mask` draws it at scale 1: 7 tall,
/// `6 * len - 1` wide. Lower case reads as upper case; anything but A to Z is drawn as a space.
pub fn mask(text: &str) -> PixelMask {
    let chars: Vec<char> = text.chars().collect();
    let w = (chars.len() * ADVANCE).saturating_sub(1).max(1);
    let h = 7;
    let mut on = vec![false; w * h];
    for (i, ch) in chars.iter().enumerate() {
        let up = ch.to_ascii_uppercase();
        if !up.is_ascii_uppercase() {
            continue;
        }
        let glyph = GLYPHS[(up as u8 - b'A') as usize];
        for (y, row) in glyph.split('|').enumerate() {
            for (x, c) in row.chars().enumerate() {
                on[y * w + i * ADVANCE + x] = c == '#';
            }
        }
    }
    PixelMask { w, h, on }
}

// ---------------------------------------------------------------------------------------------------------------
// The roster
// ---------------------------------------------------------------------------------------------------------------

/// The roster of the interface reference (section 2), in its order: section, shown name, colour.
const FIGHTERS: [(Section, &str, u32); 30] = [
    (Section::Woodwinds, "Flute", 0xdee8f6),
    (Section::Woodwinds, "Oboe", 0x9cb034),
    (Section::Woodwinds, "Clarinet", 0xbea8ff),
    (Section::Woodwinds, "Bass clarinet", 0x6e84aa),
    (Section::Woodwinds, "Bassoon", 0x1e824c),
    (Section::Woodwinds, "Alto sax", 0xc850f0),
    (Section::Woodwinds, "Tenor sax", 0xe66e1e),
    (Section::Woodwinds, "Bari sax", 0xff765c),
    (Section::Woodwinds, "Bagpipes", 0x40c458),
    (Section::Brass, "Trumpet", 0xffd63c),
    (Section::Brass, "Horn", 0xc87040),
    (Section::Brass, "Trombone", 0x78f0b8),
    (Section::Brass, "Euphonium", 0xf0ac20),
    (Section::Brass, "Tuba", 0x685cec),
    (Section::Brass, "Sousaphone", 0xd21e3c),
    (Section::Strings, "Violin", 0xb062ff),
    (Section::Strings, "Viola", 0x24c8b0),
    (Section::Strings, "Cello", 0xc45428),
    (Section::Strings, "Contrabass", 0x466ec8),
    (Section::Strings, "Harp", 0xf4e4be),
    (Section::Strings, "Acoustic", 0xd6a064),
    (Section::Strings, "Electric", 0x64ff78),
    (Section::Strings, "Ukulele", 0xff8ab2),
    (Section::Keys, "Piano", 0x549cff),
    (Section::Keys, "Organ", 0xc43468),
    (Section::Keys, "Accordion", 0xec52d6),
    (Section::Keys, "Synth", 0x46e4ff),
    (Section::Keys, "Turntables", 0xbef53c),
    (Section::Percussion, "Drums", 0xff9228),
    (Section::Percussion, "Bass drum", 0x80808c),
];

/// FLUTE's colour.
const FLUTE: [u8; 3] = [0xde, 0xe8, 0xf6];
/// TROMBONE's colour.
const TROMBONE: [u8; 3] = [0x78, 0xf0, 0xb8];

fn rgb(hex: u32) -> [u8; 3] {
    [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8]
}

fn ability(tag: &str, name: &str, text: &str) -> AbilityLine {
    AbilityLine {
        tag: tag.to_string(),
        name: name.to_string(),
        text: text.to_string(),
        key: name.to_uppercase(),
    }
}

/// The Match-up mock-up's corner cards (interface reference, 5.1, and the Music screen's lines from 5.4) for FLUTE
/// and TROMBONE; every other fighter gets a card that says it is sample data.
fn card(name: &str, display: &str) -> FighterCard {
    match name {
        "FLUTE" => FighterCard {
            weapon: "Snake charmer".to_string(),
            text: "Cobras rise from dropped baskets and strike on the beat. Every hit that lands lets them reach \
                   further."
                .to_string(),
            abilities: [
                ability("LV2", "Flutter tongue", "Twice as fast for 8 beats"),
                ability("LV3", "Snake dance", "Lasts 4 beats"),
                ability("LOW", "Piccolo", "A wave across the ring, below 25 HP"),
            ],
            range: "C5 to A6".to_string(),
            usually_plays: "Melody".to_string(),
            colour_name: "Silver".to_string(),
            music_title: "Flute, the charmer".to_string(),
            music_text: "High and ornamented. Runs of eighths that fall back to the home note.".to_string(),
            can_play_bass: false,
            portrait: None,
        },
        "TROMBONE" => FighterCard {
            weapon: "Slide lance".to_string(),
            text: "The slide shoots out at the enemy on the beat, then pulls back. Every hit that lands makes it \
                   longer. It can parry."
                .to_string(),
            abilities: [
                ability("LV2", "Sad trombone", "A ring that hits and slows"),
                ability("LV3", "Power slide", "Twice as fast for 8 beats"),
                ability("LOW", "Pedal tone", "A heavy slam, below 25 HP"),
            ],
            range: "E2 to F4".to_string(),
            usually_plays: "Bass".to_string(),
            colour_name: "Mint".to_string(),
            music_title: "Trombone, the slide".to_string(),
            music_text: "Smears up into notes, oom-pah struts, low fanfares.".to_string(),
            can_play_bass: true,
            portrait: None,
        },
        _ => FighterCard {
            weapon: "Sample fighter".to_string(),
            text: "This card is sample data.".to_string(),
            abilities: [
                ability("LV2", "First move", "Sample ability"),
                ability("LV3", "Second move", "Sample ability"),
                ability("LOW", "Last stand", "Sample ability, below 25 HP"),
            ],
            range: "C4 to C6".to_string(),
            usually_plays: "Melody".to_string(),
            colour_name: "Sample colour".to_string(),
            music_title: format!("{display}, a sample fighter"),
            music_text: "This card is sample data.".to_string(),
            can_play_bass: false,
            portrait: None,
        },
    }
}

/// The 30 fighters of the interface reference, all available, with the one sample theme.
pub fn roster() -> Roster {
    let fighters = FIGHTERS
        .iter()
        .map(|&(section, display, hex)| {
            let name = display.to_uppercase();
            RosterEntry {
                display: display.to_string(),
                section,
                color: rgb(hex),
                available: true,
                music_feeds_fight: name == "PIANO",
                mask: mask(&name),
                card: Some(card(&name, display)),
                home: None,
                parts: Vec::new(),
                name,
            }
        })
        .collect();
    let chords = |s: &str| s.split(' ').map(str::to_string).collect::<Vec<_>>();
    Roster {
        fighters,
        themes: vec![ThemeEntry {
            name: "big_top".to_string(),
            display: "Big top".to_string(),
            text: "A sinister circus in a minor key. Leaves the bass and the top line to the fighters.".to_string(),
            key: "E".to_string(),
            scale: "harmonic minor".to_string(),
            chord_loops: vec![chords("Em Am B7 Em C Am B7 B7"), chords("Em C Am B7 Em C Am B7")],
            loop_keys: vec![("E".to_string(), "harmonic minor".to_string()); 2],
            bars: (24, 42),
            refuses: Vec::new(),
            options: Vec::new(),
        }],
        vs: mask("VS"),
        start: ["FLUTE".to_string(), "TROMBONE".to_string()],
        app_version: "0.1.0".to_string(),
        kit_pairs: Vec::new(),
    }
}

/// What the sample engine says about a draft's music: its theme and first chord loop, the fighter on bass plays
/// the bass and the other the melody, and no theme options.
pub fn pair_music(draft: &Draft) -> PairMusic {
    let r = roster();
    let theme = r.themes.iter().find(|t| t.name == draft.theme);
    PairMusic {
        theme: draft.theme.clone(),
        chord_loop: theme.and_then(|t| t.chord_loops.first().cloned()).unwrap_or_default(),
        key: theme.map_or_else(String::new, |t| t.key.clone()),
        scale: theme.map_or_else(String::new, |t| t.scale.clone()),
        roles: [0, 1].map(|side| if draft.bass == Some(side) { "bass" } else { "melody" }.to_string()),
        options: Vec::new(),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The mock-ups' fights
// ---------------------------------------------------------------------------------------------------------------

/// The mock-ups' 14 seeds, in the order their script lists them.
const SEEDS: [u64; 14] = [
    20418, 7731, 15290, 3306, 11874, 26051, 902, 18433, 5627, 23980, 14402, 8175, 29766, 1248,
];

/// When FIGHT! is called, in seconds.
const T_FIGHT: f64 = 3.6;

/// The first uses of the abilities, the same in every sample fight: (seconds, fighter, name), in time order.
const FIRSTS: [(f32, usize, &str); 5] = [
    (11.96, 0, "Flutter tongue"),
    (17.02, 1, "Sad trombone"),
    (25.99, 1, "Power slide"),
    (30.62, 0, "Snake dance"),
    (41.0, 1, "Pedal tone"),
];

/// The mock-ups' random numbers (mulberry32), each in 0 to 1.
fn rng(seed: u32) -> impl FnMut() -> f64 {
    let mut a = seed;
    move || {
        a = a.wrapping_add(0x6D2B_79F5);
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

/// One seed's fight as the mock-ups' script makes it up: the K.O. time, the winner, the HP left, a random walk of
/// the lead bent to end at the winner's HP, and a score.
pub fn story(seed: u64) -> StoryView {
    let mut r = rng(seed as u32);
    let ko = 49.0 + r() * 4.0;
    let winner = if r() < 0.47 { 0 } else { 1 };
    let left = (4.0 + r() * 25.0).round();
    let mut l = [0.0f64; 30];
    for i in 1..30 {
        l[i] = l[i - 1] + (r() - 0.5) * 22.0;
    }
    let target = if winner == 0 { left } else { -left };
    let d = target - l[29];
    for (i, v) in l.iter_mut().enumerate() {
        *v += d * (i as f64 / 29.0).powi(3);
    }
    let score = 6.2 + r() * 1.7;
    let lead = l
        .iter()
        .enumerate()
        .map(|(i, &v)| ((T_FIGHT + (ko - T_FIGHT) * i as f64 / 29.0) as f32, v as f32))
        .collect();
    StoryView {
        seed,
        winner: Some(winner),
        t_ko: Some(ko),
        winner_hp: left,
        lead_changes: lead_changes(&l),
        score,
        fits: true,
        misses: Vec::new(),
        t_fight: T_FIGHT,
        lead,
        firsts: FIRSTS
            .iter()
            .map(|&(t, who, name)| AbilityMark {
                t,
                who,
                name: name.to_string(),
            })
            .collect(),
    }
}

/// How often the lead changes hands: sign flips of the lead, counting only points outside the band -5 to 5.
fn lead_changes(l: &[f64]) -> u32 {
    let mut changes = 0;
    let mut side: Option<bool> = None;
    for &v in l {
        if v.abs() <= 5.0 {
            continue;
        }
        let ahead = v > 0.0;
        if side.is_some_and(|s| s != ahead) {
            changes += 1;
        }
        side = Some(ahead);
    }
    changes
}

/// The mock-ups' 14 sample fights, best score first.
pub fn stories() -> Vec<StoryView> {
    let mut all: Vec<StoryView> = SEEDS.iter().map(|&s| story(s)).collect();
    all.sort_by(|a, b| b.score.total_cmp(&a.score));
    all
}

// ---------------------------------------------------------------------------------------------------------------
// Balance
// ---------------------------------------------------------------------------------------------------------------

/// The Balance mock-up's numbers (interface reference, section 11).
pub fn stats() -> BalanceStats {
    BalanceStats {
        fights: 3_000,
        win_share: 0.47,
        ko_median: 50.6,
        ko_p10: 44.2,
        ko_p90: 56.9,
        no_ko_share: 0.04,
        lead_changes_mean: 2.6,
        fit_rate: 0.083,
        ko_hist: vec![
            2, 3, 3, 5, 6, 8, 10, 13, 17, 22, 28, 36, 45, 56, 68, 80, 91, 98, 100, 96, 88, 76, 62, 47, 33, 21, 12, 6,
        ],
        seconds: 1.4,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Pictures
// ---------------------------------------------------------------------------------------------------------------

/// Width and height of a video frame.
const W: u32 = 360;
const H: u32 = 640;
const GROUND: [u8; 3] = [0x1e, 0x1e, 0x1e];
const PANEL: [u8; 3] = [0x28, 0x28, 0x28];
const LINE: [u8; 3] = [0x44, 0x44, 0x44];
const WHITE: [u8; 3] = [0xff, 0xff, 0xff];

/// A picture being drawn: RGB, row-major.
struct Canvas {
    w: u32,
    h: u32,
    rgb: Vec<u8>,
}

impl Canvas {
    fn new(w: u32, h: u32, fill: [u8; 3]) -> Canvas {
        Canvas {
            w,
            h,
            rgb: fill.repeat((w * h) as usize),
        }
    }

    fn set(&mut self, x: i64, y: i64, c: [u8; 3]) {
        if (0..self.w as i64).contains(&x) && (0..self.h as i64).contains(&y) {
            let i = (y as usize * self.w as usize + x as usize) * 3;
            self.rgb[i..i + 3].copy_from_slice(&c);
        }
    }

    fn rect(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, c: [u8; 3]) {
        for y in y0..y1 {
            for x in x0..x1 {
                self.set(x, y, c);
            }
        }
    }

    fn disc(&mut self, cx: i64, cy: i64, r: i64, c: [u8; 3]) {
        for y in cy - r..=cy + r {
            for x in cx - r..=cx + r {
                if (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r * r {
                    self.set(x, y, c);
                }
            }
        }
    }

    /// Stamps `m` at whole-number `scale`, centred on `cx`, top at `y`.
    fn stamp(&mut self, m: &PixelMask, scale: i64, cx: i64, y: i64, c: [u8; 3]) {
        let x0 = cx - (m.w as i64 * scale) / 2;
        for my in 0..m.h {
            for mx in 0..m.w {
                if m.on[my * m.w + mx] {
                    let (px, py) = (x0 + mx as i64 * scale, y + my as i64 * scale);
                    self.rect(px, py, px + scale, py + scale, c);
                }
            }
        }
    }

    fn frame(self) -> Frame {
        Frame {
            w: self.w,
            h: self.h,
            rgb: self.rgb.into(),
        }
    }
}

/// The colour mixed 22 % over `panel`, as large fighter fills are (interface reference, section 2).
fn large_fill(c: [u8; 3]) -> [u8; 3] {
    c.map(|v| (40 + (22 * (i32::from(v) - 40) + 50).div_euclid(100)) as u8)
}

/// A triangle wave: 0 up to `span` and back, period `2 * span`.
fn bounce(t: i64, span: i64) -> i64 {
    let p = t.rem_euclid(2 * span);
    if p < span { p } else { 2 * span - p }
}

/// The sample take's frames: two balls in the fighters' colours bouncing in a ring, and a bar along the bottom that
/// grows with the frame number, so every frame is a different picture.
struct SampleFrames {
    colors: [[u8; 3]; 2],
}

impl FrameSource for SampleFrames {
    fn frames(&self) -> u32 {
        NF
    }

    fn draw(&self, f: u32) -> Frame {
        let f = i64::from(f);
        let mut c = Canvas::new(W, H, GROUND);
        // The ring.
        c.rect(20, 96, 340, 500, PANEL);
        for (x0, y0, x1, y1) in [
            (20, 96, 340, 97),
            (20, 499, 340, 500),
            (20, 96, 21, 500),
            (339, 96, 340, 500),
        ] {
            c.rect(x0, y0, x1, y1, LINE);
        }
        // The balls, radius 22, moving 3 and 2 px a frame.
        let r = 22;
        let span_x = 320 - 2 * r - 2;
        let span_y = 404 - 2 * r - 2;
        c.disc(
            21 + r + bounce(f * 3, span_x),
            97 + r + bounce(f * 2, span_y),
            r,
            self.colors[0],
        );
        c.disc(
            21 + r + bounce(f * 2 + span_x, span_x),
            97 + r + bounce(f * 3 + span_y / 2, span_y),
            r,
            self.colors[1],
        );
        // How far through the video, one pixel column per few frames, and a tick for the frame itself.
        let done = (f * i64::from(W)) / i64::from(NF);
        c.rect(0, 620, done, 624, LINE);
        c.rect(f % i64::from(W), 626, f % i64::from(W) + 1, 632, WHITE);
        c.frame()
    }
}

/// The sample take's frames, in the two fighters' colours (left, right).
pub fn frames(colors: [[u8; 3]; 2]) -> Arc<dyn FrameSource> {
    Arc::new(SampleFrames { colors })
}

/// The VS title card: the left fighter's name on its large fill in the top half, the right fighter's in the bottom
/// half, each in its own colour, and "VS" between them.
pub fn title_card(colors: [[u8; 3]; 2], masks: [&PixelMask; 2]) -> Frame {
    let mut c = Canvas::new(W, H, PANEL);
    let half = i64::from(H) / 2;
    c.rect(0, 0, i64::from(W), half, large_fill(colors[0]));
    c.rect(0, half, i64::from(W), i64::from(H), large_fill(colors[1]));
    let cx = i64::from(W) / 2;
    for (i, m) in masks.iter().enumerate() {
        let scale = (320 / m.w.max(1) as i64).clamp(1, 6);
        let top = if i == 0 { 200 } else { 420 } - (m.h as i64 * scale) / 2;
        c.stamp(m, scale, cx, top, colors[i]);
    }
    let vs = mask("VS");
    c.rect(0, half - 22, i64::from(W), half + 22, GROUND);
    c.stamp(&vs, 4, cx, half - 14, WHITE);
    c.frame()
}

// ---------------------------------------------------------------------------------------------------------------
// Sound
// ---------------------------------------------------------------------------------------------------------------

/// Beats in 61 s: 34 bars of four, 133.77 bpm.
const BEATS: u64 = 34 * 4;
/// A blip lasts 10 ms.
const BLIP: u64 = SR as u64 / 100;
/// A 1 kHz tone has 48 samples to a cycle.
const CYCLE: u64 = SR as u64 / 1_000;
const PEAK: f32 = 0.03;

/// The sample soundtrack: silence with a 10 ms 1 kHz blip (a triangle wave, peak 0.03) on every beat at
/// 133.77 bpm, `NA` samples long.
pub fn audio() -> AudioTake {
    let mut samples = vec![[0.0f32; 2]; NA as usize];
    let mut beat = 0;
    loop {
        // Beat `beat` starts at `beat * 61 s / 136` (whole samples), the tempo the draft's 34 bars give.
        let start = beat * 61 * u64::from(SR) / BEATS;
        if start >= NA {
            break;
        }
        for k in 0..BLIP.min(NA - start) {
            // Triangle: 0 at the start of a cycle, PEAK a quarter in, -PEAK three quarters in.
            let p = (k % CYCLE) as i64;
            let q = CYCLE as i64 / 4;
            let v = if p < q {
                p
            } else if p < 3 * q {
                2 * q - p
            } else {
                p - 4 * q
            };
            let s = PEAK * v as f32 / q as f32;
            samples[(start + k) as usize] = [s, s];
        }
        beat += 1;
    }
    AudioTake {
        samples: samples.into(),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Music
// ---------------------------------------------------------------------------------------------------------------

/// Seconds at `p` percent of the 61 s timeline.
fn at(p: f32) -> f32 {
    p * 0.61
}

fn clips(spans: &[(f32, f32)]) -> Vec<(f32, f32)> {
    spans.iter().map(|&(a, b)| (at(a), at(b))).collect()
}

/// The Music mock-up's arrangement (interface reference, section 11): seven lanes, seven markers and three
/// loudness lines that pass.
pub fn music() -> MusicView {
    let fight = [(5.9, 32.5), (32.8, 53.8), (54.1, 83.7)];
    let lane = |group, name: &str, color, spans: Vec<(f32, f32)>| Lane {
        group,
        name: name.to_string(),
        color,
        clips: clips(&spans),
    };
    let hits = [19.6, 27.9, 42.6, 50.2, 67.2].iter().map(|&p| (p, p + 0.5)).collect();
    MusicView {
        checks: vec![
            CheckLine {
                title: "Loudness".to_string(),
                pass: true,
                note: "The opening is 0.6 dB quieter than the ending, and the limit is 1.5 dB.".to_string(),
            },
            CheckLine {
                title: "Perceived loudness".to_string(),
                pass: true,
                note: "The title card is 0.4 LU under the fight.".to_string(),
            },
            CheckLine {
                title: "Fighters above the music".to_string(),
                pass: true,
                note: "Both fighters sit above the band in their own range.".to_string(),
            },
        ],
        lanes: vec![
            lane(
                LaneGroup::Fighters,
                "Flute",
                Some(FLUTE),
                [&[(0.0, 2.6)], &fight[..], &[(86.0, 94.8)]].concat(),
            ),
            lane(
                LaneGroup::Fighters,
                "Trombone",
                Some(TROMBONE),
                [&[(3.0, 5.6)], &fight[..], &[(95.1, 100.0)]].concat(),
            ),
            lane(
                LaneGroup::Band,
                "Drums",
                None,
                vec![(0.0, 5.6), (5.9, 66.9), (72.1, 83.7), (86.0, 94.8), (95.1, 100.0)],
            ),
            lane(LaneGroup::Band, "Calliope", None, vec![(5.9, 66.9), (72.1, 83.7)]),
            lane(LaneGroup::Band, "Organ", None, vec![(54.1, 66.9), (72.1, 83.7)]),
            lane(LaneGroup::Band, "Glockenspiel", None, vec![(72.1, 83.7), (86.0, 94.8)]),
            lane(LaneGroup::Band, "Band hits", None, hits),
        ],
        markers: [
            (0.0, "Title card"),
            (5.9, "Fight"),
            (32.8, "Lead change"),
            (54.1, "Lead change"),
            (67.2, "Desperation"),
            (84.0, "K.O."),
            (95.1, "Reset"),
        ]
        .iter()
        .map(|&(p, name)| (at(p), name.to_string()))
        .collect(),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Captions
// ---------------------------------------------------------------------------------------------------------------

/// The Render mock-up's caption for each platform (interface reference, section 11): the caption, a line break and
/// the tags. The caption line carries emoji as data; nothing that draws a test picture shows it.
pub fn captions() -> CaptionTexts {
    let line = "The front row vs the back row \u{1fa88}\u{1f3ba} Comment the next match-up \u{1f447}";
    CaptionTexts {
        instagram: format!(
            "{line}\n#ballfight #flute #trombone #bandkids #marchingband #musicmemes #musician #pixelart #lowpoly \
             #gamedev #satisfying #music"
        ),
        tiktok: format!(
            "{line}\n#fyp #ballfight #flute #trombone #bandkid #musictok #pixelart #satisfying #bandtok #music"
        ),
        shorts: "Flute vs Trombone\nFlute vs Trombone, every hit in key. Comment the next match-up\n#shorts \
                 #ballfight #flute #trombone #music #pixelart"
            .to_string(),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The Library
// ---------------------------------------------------------------------------------------------------------------

/// The sample output folder: `Videos\BALLFIGHT`.
fn videos() -> PathBuf {
    PathBuf::from("Videos").join("BALLFIGHT")
}

/// 12:00 UTC on `day` November 2026, in seconds since 1970.
fn noon_in_november_2026(day: u64) -> u64 {
    // 2026-11-01 00:00 UTC is day 20,758 since 1970.
    (20_758 + day - 1) * 86_400 + 43_200
}

/// A cover: the two fighters' colours split on a diagonal, 180 x 320, the left colour in the top-left half.
fn cover(colors: [[u8; 3]; 2]) -> Frame {
    let (w, h) = (180u32, 320u32);
    let mut c = Canvas::new(w, h, colors[1]);
    for y in 0..h {
        for x in 0..w {
            // Above the line from the top-right corner to the bottom-left one.
            if u64::from(x) * u64::from(h) + u64::from(y) * u64::from(w) < u64::from(w) * u64::from(h) {
                c.set(i64::from(x), i64::from(y), colors[0]);
            }
        }
    }
    c.frame()
}

/// Six past videos, newest first, with their drawn covers (interface reference, section 11).
pub fn library() -> Vec<(LibraryEntry, Frame)> {
    let color = |display: &str| rgb(FIGHTERS.iter().find(|f| f.1 == display).unwrap().2);
    [
        ("Flute", "Trombone", 18433, 10, None),
        ("Tuba", "Bass drum", 9120, 9, None),
        ("Cello", "Contrabass", 4471, 8, None),
        (
            "Alto sax",
            "Tenor sax",
            15007,
            7,
            Some("Made by a newer version of the app."),
        ),
        ("Oboe", "Bassoon", 662, 6, None),
        ("Trumpet", "Horn", 3306, 5, None),
    ]
    .iter()
    .map(|&(a, b, seed, day, problem)| {
        let colors = [color(a), color(b)];
        let stem = format!("{a}_vs_{b}").to_lowercase().replace(' ', "_");
        let folder = videos().join(&stem);
        let entry = LibraryEntry {
            fight_path: Some(folder.join(format!("{stem}.toml"))),
            folder,
            title: format!("{a} vs {b}"),
            colors,
            seed: Some(seed),
            modified: noon_in_november_2026(day),
            problem: problem.map(str::to_string),
        };
        (entry, cover(colors))
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLORS: [[u8; 3]; 2] = [[0xde, 0xe8, 0xf6], [0x78, 0xf0, 0xb8]];

    #[test]
    fn the_sample_roster_is_the_series() {
        let r = roster();
        assert_eq!(r.fighters.len(), 30);
        let order = [
            (Section::Woodwinds, 9),
            (Section::Brass, 6),
            (Section::Strings, 8),
            (Section::Keys, 5),
            (Section::Percussion, 2),
        ];
        let mut at = 0;
        for (section, n) in order {
            for f in &r.fighters[at..at + n] {
                assert_eq!(f.section, section, "{}", f.name);
            }
            at += n;
        }
        assert!(r.fighters.iter().all(|f| f.name != "GUITAR" && f.name != "SAX"));
        assert!(r.fighters.iter().all(|f| f.available));
        let fighter = |name: &str| r.fighters.iter().find(|f| f.name == name).unwrap();
        assert_eq!(fighter("FLUTE").color, [0xde, 0xe8, 0xf6]);
        assert_eq!(fighter("TROMBONE").color, [0x78, 0xf0, 0xb8]);
        assert_eq!(fighter("TUBA").color, [0x68, 0x5c, 0xec]);
        assert_eq!(fighter("BASS DRUM").color, [0x80, 0x80, 0x8c]);
        let flute = fighter("FLUTE").card.as_ref().unwrap();
        assert_eq!(flute.abilities[0].name, "Flutter tongue");
        assert_eq!(flute.abilities[0].key, "FLUTTER TONGUE");
        assert!(!flute.can_play_bass);
        let trombone = fighter("TROMBONE").card.as_ref().unwrap();
        assert!(trombone.can_play_bass);
        assert_eq!(trombone.weapon, "Slide lance");
        let tuba = fighter("TUBA").card.as_ref().unwrap();
        assert_eq!(tuba.weapon, "Sample fighter");
        assert_eq!(tuba.text, "This card is sample data.");
        for f in &r.fighters {
            assert_eq!(f.music_feeds_fight, f.name == "PIANO", "{}", f.name);
            assert_eq!(f.mask, mask(&f.name));
            for a in &f.card.as_ref().unwrap().abilities {
                assert_eq!(a.key, a.name.to_uppercase());
            }
        }
        assert_eq!(r.start, ["FLUTE".to_string(), "TROMBONE".to_string()]);
        assert_eq!(r.app_version, "0.1.0");
        assert_eq!(r.vs, mask("VS"));
        assert_eq!(r.themes.len(), 1);
        let theme = &r.themes[0];
        assert_eq!((theme.name.as_str(), theme.display.as_str()), ("big_top", "Big top"));
        assert_eq!((theme.key.as_str(), theme.scale.as_str()), ("E", "harmonic minor"));
        assert_eq!(theme.chord_loops[0].join(" "), "Em Am B7 Em C Am B7 B7");
        assert_eq!(theme.chord_loops[1].join(" "), "Em C Am B7 Em C Am B7");
        assert_eq!(theme.bars, (24, 42));
    }

    #[test]
    fn masks_use_the_series_font() {
        let m = mask("FLUTE");
        assert_eq!((m.w, m.h), (29, 7));
        let f = mask("F");
        let rows: Vec<String> = (0..7)
            .map(|y| (0..5).map(|x| if f.on[y * f.w + x] { '#' } else { '.' }).collect())
            .collect();
        assert_eq!(rows, ["#####", "#....", "#....", "####.", "#....", "#....", "#...."]);
        assert_eq!(mask("VS").w, 11);
        // A space is blank, and lower case reads as upper case.
        assert_eq!(mask("bass drum"), mask("BASS DRUM"));
        assert!(mask(" ").on.iter().all(|on| !on));
    }

    #[test]
    fn the_sample_stories_are_the_mock_ups() {
        let s = stories();
        assert_eq!(s.len(), 14);
        assert!(s.windows(2).all(|w| w[0].score >= w[1].score));
        let first = &s[0];
        assert_eq!(first.seed, 18433);
        assert_eq!(first.winner, Some(1));
        assert!((first.t_ko.unwrap() - 49.1).abs() < 0.05);
        assert_eq!(first.winner_hp, 16.0);
        assert_eq!(format!("{:.2}", first.score), "7.84");
        assert_eq!(s[11].seed, 20418);
        assert_eq!(s[11].lead_changes, 4);
        assert_eq!(s.iter().filter(|v| v.winner == Some(1)).count(), 7);
        assert_eq!(s.iter().filter(|v| v.winner_hp <= 12.0).count(), 6);
        for v in &s {
            let t_ko = v.t_ko.unwrap();
            assert!((49.0..=53.0).contains(&t_ko), "{}: {t_ko}", v.seed);
            assert_eq!(v.lead.len(), 30);
            assert_eq!(v.lead[0].0, 3.6);
            assert!(v.fits);
            assert_eq!(v.t_fight, 3.6);
        }
    }

    #[test]
    fn the_shortlist_order_is_the_references() {
        let s = stories();
        let seeds: Vec<u64> = s.iter().map(|v| v.seed).collect();
        assert_eq!(
            seeds,
            [
                18433, 5627, 8175, 23980, 11874, 29766, 7731, 15290, 1248, 26051, 3306, 20418, 14402, 902
            ]
        );
        // (score, winner, K.O., HP left, lead changes) as section 11 states them.
        let row = |v: &StoryView| {
            (
                format!("{:.2}", v.score),
                v.winner,
                format!("{:.1}", v.t_ko.unwrap()),
                v.winner_hp,
                v.lead_changes,
            )
        };
        assert_eq!(row(&s[0]), ("7.84".into(), Some(1), "49.1".into(), 16.0, 0));
        assert_eq!(row(&s[1]), ("7.75".into(), Some(0), "50.3".into(), 26.0, 2));
        assert_eq!(row(&s[2]), ("7.73".into(), Some(1), "52.7".into(), 4.0, 3));
        assert_eq!(row(&s[11]), ("6.55".into(), Some(1), "52.2".into(), 10.0, 4));
        assert_eq!(s.iter().filter(|v| v.lead_changes >= 2).count(), 5);
        for v in &s {
            // The lead ends where the winner's HP says.
            let end = f64::from(v.lead[29].1);
            let want = if v.winner == Some(0) { v.winner_hp } else { -v.winner_hp };
            assert!((end - want).abs() < 1e-3, "{}", v.seed);
            assert_eq!(story(v.seed), *v);
        }
        let marks: Vec<(f32, usize, &str)> = s[0].firsts.iter().map(|m| (m.t, m.who, m.name.as_str())).collect();
        assert_eq!(
            marks,
            [
                (11.96, 0, "Flutter tongue"),
                (17.02, 1, "Sad trombone"),
                (25.99, 1, "Power slide"),
                (30.62, 0, "Snake dance"),
                (41.0, 1, "Pedal tone"),
            ]
        );
    }

    #[test]
    fn the_sample_values_are_the_references() {
        let st = stats();
        assert_eq!(st.fights, 3_000);
        assert_eq!(st.win_share, 0.47);
        assert_eq!(st.ko_hist.len(), 28);
        assert_eq!(st.ko_hist[18], 100);
        assert_eq!(st.fit_rate, 0.083);

        let m = music();
        assert_eq!(m.lanes.len(), 7);
        assert_eq!(m.lanes.iter().filter(|l| l.group == LaneGroup::Fighters).count(), 2);
        assert_eq!(m.lanes.iter().filter(|l| l.group == LaneGroup::Band).count(), 5);
        assert_eq!(m.lanes[0].color, Some(COLORS[0]));
        assert_eq!(m.lanes[6].name, "Band hits");
        assert_eq!(m.markers.len(), 7);
        assert_eq!(m.markers[5].1, "K.O.");
        assert_eq!(m.checks.len(), 3);
        assert!(m.checks.iter().all(|c| c.pass));

        let c = captions();
        assert!(c.instagram.starts_with("The front row vs the back row"));
        assert!(c.instagram.ends_with("#lowpoly #gamedev #satisfying #music"));
        assert!(c.tiktok.ends_with("#bandtok #music"));
        assert!(c.shorts.starts_with("Flute vs Trombone\n"));

        let lib = library();
        assert_eq!(lib.len(), 6);
        for ((entry, cover), day) in lib.iter().zip([10, 9, 8, 7, 6, 5]) {
            let noon = chrono::NaiveDate::from_ymd_opt(2026, 11, day)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp();
            assert_eq!(entry.modified as i64, noon, "{}", entry.title);
            assert_eq!((cover.w, cover.h), (180, 320));
            assert_eq!(cover.rgb.len(), 180 * 320 * 3);
        }
        assert_eq!(lib[0].0.title, "Flute vs Trombone");
        assert_eq!(lib[0].0.seed, Some(18433));
        assert_eq!(lib[0].1.rgb[..3], COLORS[0]);
        let problems: Vec<&str> = lib.iter().filter_map(|(e, _)| e.problem.as_deref()).collect();
        assert_eq!(problems, ["Made by a newer version of the app."]);
    }

    #[test]
    fn sample_frames_differ() {
        let source = frames(COLORS);
        assert_eq!(source.frames(), crate::model::NF);
        let (a, b) = (source.draw(0), source.draw(1));
        assert_eq!((a.w, a.h), (360, 640));
        assert_eq!((b.w, b.h), (360, 640));
        assert_eq!(a.rgb.len(), 360 * 640 * 3);
        assert_ne!(a.rgb, b.rgb);
        assert_eq!(source.draw(7), source.draw(7));
    }

    #[test]
    fn the_title_card_and_the_beat_blips() {
        let (left, right) = (mask("FLUTE"), mask("TROMBONE"));
        let card = title_card(COLORS, [&left, &right]);
        assert_eq!((card.w, card.h), (360, 640));
        assert_eq!(card.rgb.len(), 360 * 640 * 3);
        let has = |c: [u8; 3]| card.rgb.chunks(3).any(|p| p == c);
        assert!(has(COLORS[0]) && has(COLORS[1]));

        let take = audio();
        assert_eq!(take.samples.len() as u64, crate::model::NA);
        let peak = take
            .samples
            .iter()
            .map(|s| s[0].abs().max(s[1].abs()))
            .fold(0.0f32, f32::max);
        assert!((peak - 0.03).abs() < 1e-6, "{peak}");
        // One blip per beat: 34 bars of 4 beats fill 61 s, and the 137th beat lands on 61 s itself.
        let mut blips = 0;
        let mut last: Option<usize> = None;
        for (i, s) in take.samples.iter().enumerate() {
            if s[0] != 0.0 {
                if last.is_none_or(|l| i - l > 1_000) {
                    blips += 1;
                }
                last = Some(i);
            }
        }
        assert_eq!(blips, 137);
    }
}
