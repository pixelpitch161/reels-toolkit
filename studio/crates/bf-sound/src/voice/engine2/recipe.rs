//! The recipe of a recorded instrument: for each recorded note, how loud each of its overtones is at each moment of
//! the note, how far off its pitch each one sits, and how loud the noise between them is in 24 bands. The player
//! ([`super::player`]) rebuilds a note from it; the `recipes` tool in `bf-harness` measures it from the recordings.
//!
//! A recipe file (`bf-content/data/voices/<NAME>.ve2`) holds one instrument, little-endian:
//!
//! ```text
//! "VE2R"  u16 version (1)  name (u8 length + UTF-8)
//! grid    u16 count, f32 seconds each                 the moments every note is measured at (`grid`)
//! layers  u8 count; each: name, f32 strength, f32 gain dB
//! notes   u16 count; each:
//!         u8 layer, f32 midi, f32 f0, f32 b, f32 release, f32 top dB
//!         u16 partials; each: f32 ratio, u16 first point, u16 points,
//!                             points x u8 level, points x i8 cents
//!         u16 noise points (from the first), then points x 24 u8 levels
//! ```
//!
//! A level byte `v` is `top - v / 2` dB (255 is silence), so levels keep half a decibel over 127 dB under the
//! note's loudest; a cents byte is a quarter of a cent (up to about 32 cents either way). A partial is kept only
//! from its first to its last audible moment, and the noise only until it has died away, which is what keeps a
//! piano of 132 recorded notes to about a megabyte.

use std::fmt;

/// The first four bytes of a recipe file.
pub const MAGIC: [u8; 4] = *b"VE2R";
/// The recipe format's version.
pub const VERSION: u16 = 1;
/// How many noise bands a note has.
pub const BANDS: usize = 24;
/// The lowest and highest noise band edges, Hz.
const BAND_LO: f64 = 60.0;
const BAND_HI: f64 = 16_000.0;
/// A level byte that means silence.
const SILENT: u8 = 255;

/// The noise bands' edges, Hz: 25 edges spaced evenly in pitch from 60 Hz to 16 kHz.
pub fn band_edges() -> [f64; BANDS + 1] {
    let mut e = [0.0; BANDS + 1];
    for (i, v) in e.iter_mut().enumerate() {
        *v = BAND_LO * bf_core::dmath::pow(BAND_HI / BAND_LO, i as f64 / BANDS as f64);
    }
    e
}

/// The moments a note is measured at, seconds from its start: every 2.5 ms up to 0.1 s (a note's start changes
/// fast), then each step 6% longer than the one before and never longer than 0.1 s, until a point at or past `end`.
pub fn grid(end: f64) -> Vec<f64> {
    let mut g: Vec<f64> = (0..=40).map(|i| f64::from(i) * 0.0025).collect();
    let mut step: f64 = 0.0025;
    while g[g.len() - 1] < end {
        step = (step * 1.06).min(0.1);
        let last = g[g.len() - 1];
        g.push(last + step);
    }
    g
}

/// One strength the instrument was recorded at.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    /// Its name in the recording set (`vl3`, `loud`).
    pub name: String,
    /// How hard it is played, 0 to 1: the strength a note asks for picks between the layers by this.
    pub strength: f64,
    /// How loud it is meant to sound against the others, dB, added to its notes' levels (a set evened out to one
    /// loudness gets its steps back here).
    pub gain_db: f64,
}

/// One overtone of a recorded note over time.
#[derive(Clone, Debug, PartialEq)]
pub struct Partial {
    /// Its frequency over the note's measured `f0`.
    pub ratio: f64,
    /// The grid point its first value is at; before it, and after its last, it is silent.
    pub first: usize,
    /// Its level at each grid point from `first`, dB (full scale); `f32::NEG_INFINITY` is silence.
    pub level: Vec<f32>,
    /// How far it sits off `ratio x f0` at each of those points, cents.
    pub cents: Vec<f32>,
}

impl Partial {
    /// Its level at grid point `i`, dB, silence outside what it keeps.
    pub fn level_at(&self, i: usize) -> f32 {
        i.checked_sub(self.first)
            .and_then(|j| self.level.get(j))
            .copied()
            .unwrap_or(f32::NEG_INFINITY)
    }

    /// Its pitch offset at grid point `i`, cents: the nearest kept value outside what it keeps.
    pub fn cents_at(&self, i: usize) -> f32 {
        if self.cents.is_empty() {
            return 0.0;
        }
        let j = i.saturating_sub(self.first).min(self.cents.len() - 1);
        self.cents[j]
    }

    /// The grid point after its last kept value.
    pub fn end(&self) -> usize {
        self.first + self.level.len()
    }
}

/// One recorded note.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    /// Which of the set's layers it belongs to.
    pub layer: usize,
    /// Its pitch as measured, MIDI (not rounded).
    pub midi: f64,
    /// Its fundamental as measured, Hz.
    pub f0: f64,
    /// Its strings' stiffness: partial k sits near `k f0 sqrt(1 + b k^2)` (0 for a pipe or a reed).
    pub b: f64,
    /// How fast it dies when the key comes up, the time constant in seconds (0 where unmeasured).
    pub release: f64,
    /// Its overtones.
    pub partials: Vec<Partial>,
    /// The noise between its overtones at each grid point from the first, dB per band; past the last row, silence.
    pub noise: Vec<[f32; BANDS]>,
}

/// A whole instrument's recipe.
#[derive(Clone, Debug, PartialEq)]
pub struct RecipeSet {
    /// The instrument (`PIANO`).
    pub name: String,
    /// The moments every note is measured at ([`grid`]).
    pub grid: Vec<f64>,
    /// The strengths it was recorded at, softest first.
    pub layers: Vec<Layer>,
    /// The recorded notes.
    pub notes: Vec<Note>,
}

/// Why a recipe file could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeError(pub String);

impl fmt::Display for RecipeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "recipe file: {}", self.0)
    }
}

impl std::error::Error for RecipeError {}

/// Up to four recorded notes and how much of each a note takes: the two nearest pitches on each of the two nearest
/// layers. The weights add up to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corners {
    /// How many of `at` are used.
    pub n: usize,
    /// `(note index, weight)`.
    pub at: [(usize, f64); 4],
}

impl Corners {
    /// The corners in use.
    pub fn iter(&self) -> impl Iterator<Item = (usize, f64)> + '_ {
        self.at[..self.n].iter().copied()
    }

    fn push(&mut self, note: usize, w: f64) {
        if w <= 0.0 {
            return;
        }
        if let Some(c) = self.at[..self.n].iter_mut().find(|c| c.0 == note) {
            c.1 += w;
            return;
        }
        self.at[self.n] = (note, w);
        self.n += 1;
    }
}

fn quant_level(db: f32, top: f32) -> u8 {
    if !db.is_finite() {
        return SILENT;
    }
    let v = ((top - db) * 2.0).round();
    if v >= f32::from(SILENT) {
        SILENT
    } else {
        v.max(0.0) as u8
    }
}

fn unquant_level(v: u8, top: f32) -> f32 {
    if v == SILENT {
        f32::NEG_INFINITY
    } else {
        top - f32::from(v) / 2.0
    }
}

fn quant_cents(c: f32) -> i8 {
    (c * 4.0).round().clamp(-127.0, 127.0) as i8
}

impl RecipeSet {
    /// The set as a recipe file.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&MAGIC);
        b.extend_from_slice(&VERSION.to_le_bytes());
        put_name(&mut b, &self.name);
        b.extend_from_slice(&(self.grid.len() as u16).to_le_bytes());
        for &t in &self.grid {
            put_f32(&mut b, t);
        }
        b.push(self.layers.len() as u8);
        for l in &self.layers {
            put_name(&mut b, &l.name);
            put_f32(&mut b, l.strength);
            put_f32(&mut b, l.gain_db);
        }
        b.extend_from_slice(&(self.notes.len() as u16).to_le_bytes());
        for n in &self.notes {
            let top = n
                .partials
                .iter()
                .flat_map(|p| p.level.iter())
                .chain(n.noise.iter().flat_map(|r| r.iter()))
                .copied()
                .filter(|v| v.is_finite())
                .fold(-200.0_f32, f32::max);
            b.push(n.layer as u8);
            for v in [n.midi, n.f0, n.b, n.release, f64::from(top)] {
                put_f32(&mut b, v);
            }
            b.extend_from_slice(&(n.partials.len() as u16).to_le_bytes());
            for p in &n.partials {
                put_f32(&mut b, p.ratio);
                b.extend_from_slice(&(p.first as u16).to_le_bytes());
                b.extend_from_slice(&(p.level.len() as u16).to_le_bytes());
                b.extend(p.level.iter().map(|&v| quant_level(v, top)));
                for j in 0..p.level.len() {
                    b.push(quant_cents(p.cents.get(j).copied().unwrap_or(0.0)) as u8);
                }
            }
            b.extend_from_slice(&(n.noise.len() as u16).to_le_bytes());
            for row in &n.noise {
                b.extend(row.iter().map(|&v| quant_level(v, top)));
            }
        }
        b
    }

    /// Reads a recipe file.
    ///
    /// # Errors
    /// A file that is not a recipe, of another version, cut short, or whose numbers do not fit together.
    pub fn parse(bytes: &[u8]) -> Result<RecipeSet, RecipeError> {
        let mut r = Reader { b: bytes, at: 0 };
        if r.take(4)? != MAGIC {
            return Err(RecipeError("not a recipe (its first bytes are not VE2R)".into()));
        }
        let version = r.u16()?;
        if version != VERSION {
            return Err(RecipeError(format!(
                "version {version}, this build reads version {VERSION}"
            )));
        }
        let name = r.name()?;
        let grid: Vec<f64> = (0..r.u16()?).map(|_| r.f32()).collect::<Result<_, _>>()?;
        let layers: Vec<Layer> = (0..r.u8()?)
            .map(|_| {
                Ok(Layer {
                    name: r.name()?,
                    strength: r.f32()?,
                    gain_db: r.f32()?,
                })
            })
            .collect::<Result<_, RecipeError>>()?;
        let count = r.u16()?;
        let mut notes = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            let layer = usize::from(r.u8()?);
            if layer >= layers.len() {
                return Err(RecipeError(format!("a note on layer {layer} of {}", layers.len())));
            }
            let (midi, f0, b, release, top) = (r.f32()?, r.f32()?, r.f32()?, r.f32()?, r.f32()? as f32);
            let mut partials = Vec::new();
            for _ in 0..r.u16()? {
                let ratio = r.f32()?;
                let first = usize::from(r.u16()?);
                let len = usize::from(r.u16()?);
                if first + len > grid.len() {
                    return Err(RecipeError(format!(
                        "a partial runs past the grid's {} points",
                        grid.len()
                    )));
                }
                let level = r.take(len)?.iter().map(|&v| unquant_level(v, top)).collect();
                let cents = r.take(len)?.iter().map(|&v| f32::from(v as i8) / 4.0).collect();
                partials.push(Partial {
                    ratio,
                    first,
                    level,
                    cents,
                });
            }
            let rows = usize::from(r.u16()?);
            if rows > grid.len() {
                return Err(RecipeError(format!("noise runs past the grid's {} points", grid.len())));
            }
            let mut noise = Vec::with_capacity(rows);
            for _ in 0..rows {
                let raw = r.take(BANDS)?;
                let mut row = [0.0; BANDS];
                for (o, &v) in row.iter_mut().zip(raw) {
                    *o = unquant_level(v, top);
                }
                noise.push(row);
            }
            notes.push(Note {
                layer,
                midi,
                f0,
                b,
                release,
                partials,
                noise,
            });
        }
        if r.at != bytes.len() {
            return Err(RecipeError(format!(
                "{} bytes left over after the last note",
                bytes.len() - r.at
            )));
        }
        Ok(RecipeSet {
            name,
            grid,
            layers,
            notes,
        })
    }

    /// The recorded notes a note at `midi` played at `strength` is made from: on each of the two layers nearest
    /// `strength` (or the end layer past them), the two recorded pitches either side of `midi` (or the end pitch past
    /// them), weighted by how near each is.
    pub fn nearest(&self, midi: f64, strength: f64) -> Corners {
        let mut c = Corners {
            n: 0,
            at: [(0, 0.0); 4],
        };
        for (layer, wl) in pick_two(self.layers.iter().map(|l| l.strength).enumerate(), strength) {
            let pitches = self
                .notes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.layer == layer)
                .map(|(i, n)| (i, n.midi));
            for (note, wp) in pick_two(pitches, midi) {
                c.push(note, wl * wp);
            }
        }
        if c.n == 0 && !self.notes.is_empty() {
            c.push(0, 1.0);
        }
        c
    }
}

/// Of `(id, position)` pairs, the nearest one at or below `x` and the nearest at or above, weighted linearly; the end
/// one alone past either end.
fn pick_two(items: impl Iterator<Item = (usize, f64)>, x: f64) -> Vec<(usize, f64)> {
    let mut lo: Option<(usize, f64)> = None;
    let mut hi: Option<(usize, f64)> = None;
    for (id, p) in items {
        if p <= x && lo.is_none_or(|l| p > l.1) {
            lo = Some((id, p));
        }
        if p >= x && hi.is_none_or(|h| p < h.1) {
            hi = Some((id, p));
        }
    }
    match (lo, hi) {
        (Some(l), Some(h)) if h.1 > l.1 => {
            let t = (x - l.1) / (h.1 - l.1);
            vec![(l.0, 1.0 - t), (h.0, t)]
        }
        (Some(l), _) => vec![(l.0, 1.0)],
        (None, Some(h)) => vec![(h.0, 1.0)],
        (None, None) => vec![],
    }
}

fn put_f32(b: &mut Vec<u8>, v: f64) {
    b.extend_from_slice(&(v as f32).to_le_bytes());
}

fn put_name(b: &mut Vec<u8>, s: &str) {
    let s = &s.as_bytes()[..s.len().min(255)];
    b.push(s.len() as u8);
    b.extend_from_slice(s);
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], RecipeError> {
        let end = self.at + n;
        if end > self.b.len() {
            return Err(RecipeError(format!("cut short at byte {} of {}", self.b.len(), end)));
        }
        let s = &self.b[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, RecipeError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, RecipeError> {
        let s = self.take(2)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }
    fn f32(&mut self) -> Result<f64, RecipeError> {
        let s = self.take(4)?;
        Ok(f64::from(f32::from_le_bytes([s[0], s[1], s[2], s[3]])))
    }
    fn name(&mut self) -> Result<String, RecipeError> {
        let n = usize::from(self.u8()?);
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| RecipeError("a name that is not UTF-8".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> RecipeSet {
        let grid = grid(0.3);
        let n = grid.len();
        let note = |layer: usize, midi: f64, p2: f32| Note {
            layer,
            midi,
            f0: 440.0 * bf_core::dmath::pow(2.0, (midi - 69.0) / 12.0),
            b: 0.0003,
            release: 0.07,
            partials: vec![
                Partial {
                    ratio: 1.0,
                    first: 0,
                    level: vec![-12.0; n],
                    cents: vec![0.5; n],
                },
                Partial {
                    ratio: 2.001,
                    first: 3,
                    level: (0..n - 5)
                        .map(|i| if i == 2 { f32::NEG_INFINITY } else { p2 - i as f32 * 0.1 })
                        .collect(),
                    cents: (0..n - 5).map(|i| i as f32 * 0.25 - 3.0).collect(),
                },
            ],
            noise: vec![[-60.0; BANDS]; 10],
        };
        RecipeSet {
            name: "TEST".into(),
            grid,
            layers: vec![
                Layer {
                    name: "soft".into(),
                    strength: 0.5,
                    gain_db: -6.0,
                },
                Layer {
                    name: "loud".into(),
                    strength: 1.0,
                    gain_db: 0.0,
                },
            ],
            notes: vec![note(0, 48.0, -18.0), note(0, 54.0, -6.0), note(1, 48.0, -20.0)],
        }
    }

    #[test]
    fn a_set_survives_the_round_trip() {
        let s = tiny();
        let back = RecipeSet::parse(&s.to_bytes()).expect("parses");
        assert_eq!(back.name, s.name);
        assert_eq!(back.layers, s.layers);
        assert_eq!(back.grid.len(), s.grid.len());
        for (a, b) in s.notes.iter().zip(&back.notes) {
            assert_eq!((a.layer, a.midi), (b.layer, b.midi));
            for (p, q) in a.partials.iter().zip(&b.partials) {
                assert_eq!((p.first, p.level.len()), (q.first, q.level.len()));
                for (x, y) in p.level.iter().zip(&q.level) {
                    assert!(x == y || (x - y).abs() <= 0.25 + 1e-4, "{x} {y}");
                }
                for (x, y) in p.cents.iter().zip(&q.cents) {
                    assert!((x - y).abs() <= 0.125 + 1e-4, "{x} {y}");
                }
            }
            assert_eq!(a.noise.len(), b.noise.len());
            assert!((b.noise[0][0] + 60.0).abs() <= 0.25);
        }
        assert!(back.notes[0].partials[1].level[2].is_infinite());
    }

    #[test]
    fn the_grid_is_fine_early_and_coarse_late() {
        let g = grid(6.0);
        assert_eq!(&g[..3], &[0.0, 0.0025, 0.005]);
        assert_eq!(g.iter().filter(|&&t| t <= 0.1 + 1e-12).count(), 41);
        assert!(g.windows(2).all(|w| w[1] - w[0] <= 0.1 + 1e-12));
        assert!(*g.last().unwrap() >= 6.0);
        assert!(g.len() < 150, "{}", g.len());
    }

    #[test]
    fn a_wrong_file_is_refused() {
        let good = tiny().to_bytes();
        let mut magic = good.clone();
        magic[0] = b'X';
        assert!(RecipeSet::parse(&magic).unwrap_err().0.contains("VE2R"));
        let mut version = good.clone();
        version[4] = 2;
        assert!(RecipeSet::parse(&version).unwrap_err().0.contains("version 2"));
        assert!(
            RecipeSet::parse(&good[..good.len() - 7])
                .unwrap_err()
                .0
                .contains("cut short")
        );
    }

    #[test]
    fn the_nearest_corners_interpolate() {
        let mut s = tiny();
        s.notes.push(Note {
            layer: 1,
            midi: 54.0,
            ..s.notes[1].clone()
        });
        let c = s.nearest(50.0, 0.75);
        let w: Vec<(usize, f64)> = c.iter().collect();
        assert_eq!(w.len(), 4);
        let want = [
            (0, 2.0 / 3.0 * 0.5),
            (1, 1.0 / 3.0 * 0.5),
            (2, 2.0 / 3.0 * 0.5),
            (3, 1.0 / 3.0 * 0.5),
        ];
        for (n, x) in want {
            let got = w.iter().find(|c| c.0 == n).expect("corner").1;
            assert!((got - x).abs() < 1e-12, "{n}: {got} {x}");
        }
        let c = s.nearest(40.0, 2.0);
        assert_eq!(c.iter().collect::<Vec<_>>(), vec![(2, 1.0)]);
    }
}
