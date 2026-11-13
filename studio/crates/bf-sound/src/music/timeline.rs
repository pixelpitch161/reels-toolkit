//! What the fighters' parts read of the timeline and of the run beyond `kit/music.py`'s own helpers:
//! `T.line_at` (with the override windows of #53), `T.SOLO` (the fight file's solo, else the kit's sketch) and
//! the recorded per-frame values (`RUN[key][P.sim_i(t), k]`).

use bf_core::fightfile::{SoloNote, solo_of_notes};
use bf_core::harmony::HarmonyError;
use bf_core::lines::DEFAULT_LINE_PER_BAR;
use bf_core::recording::Array;

use super::MusicCtx;
use super::error::MusicError;

impl MusicCtx<'_> {
    /// The side of the fighter `name`, whose roster entry a default line or solo is made from (Python looks
    /// `name` up in the whole roster; a soundtrack has the two fighters' entries).
    fn side(&self, helper: &'static str, name: &str) -> Result<usize, MusicError> {
        self.show.names.iter().position(|n| n == name).ok_or_else(|| {
            MusicError::new(
                helper,
                format!(
                    "{name:?} is not a fighter of this fight ({:?} and {:?} are)",
                    self.show.names[0], self.show.names[1]
                ),
            )
        })
    }

    /// `T.line_at(name, t)`: the note of fighter `name`'s line at `t`: the fight file's line for it (8 notes a
    /// bar are 8ths, 4 are beats), else the kit's sketch (`default_line`, from the chords' tones in the
    /// fighter's register); inside a window registered with [`MusicCtx::line_override`] (#53, opt-in), the
    /// override line's note instead. `None` is a rest (a fight file's line may hold rests; a window's rest lets
    /// the line itself through).
    ///
    /// # Errors
    /// Where Python raises: `name` is not a fighter of this fight and the fight file has no line for it, the
    /// line has fewer notes than the loop has bars, or the fighter's register holds no tone of a chord of the
    /// loop. And when a voice panicked earlier.
    pub fn line_at(&self, name: &str, t: f64) -> Result<Option<i32>, MusicError> {
        const H: &str = "line_at";
        let bad = |e: HarmonyError| MusicError::new(H, format!("{name}'s line: {e}"));
        let beat = self.show.grid.beat;
        for w in self.line_overrides(name)? {
            if w.t0 - 1e-6 <= t && t < w.t1 {
                let mut cur = None;
                for &(bt, m, _) in &w.notes {
                    if w.tz + bt * beat <= t + 1e-6 {
                        cur = m;
                    }
                }
                if cur.is_some() {
                    return Ok(cur);
                }
            }
        }
        match self.ff.music.lines.get(name) {
            Some(line) => self.harmony.line_note(line, t).map_err(bad),
            None => {
                let k = self.side(H, name)?;
                let line = self
                    .harmony
                    .default_line(self.cfg[k].register, DEFAULT_LINE_PER_BAR)
                    .map_err(bad)?;
                self.harmony.line_note(&line, t).map(Some).map_err(bad)
            }
        }
    }

    /// `T.SOLO[name]` for a part that plays one note at a time: fighter `name`'s victory solo as `(beat, midi
    /// note, beats)`: the fight file's, else the kit's sketch (`default_solo`: a 2-bar motif on the first bar's
    /// chord in the fighter's register). A part that can strum a chord reads [`MusicCtx::solo_entries`].
    ///
    /// # Errors
    /// Where Python raises: `name` is not a fighter of this fight, or its register holds no tone of the first
    /// bar's chord; or the solo holds a chord (the kit's single-note parts stop on a tuple of notes; a fight file
    /// that passed `validate` has a chord only in the solo of a fighter of
    /// [`bf_core::fightfile::SOLO_CHORDS`]).
    pub fn solo(&self, name: &str) -> Result<Vec<(f64, i32, f64)>, MusicError> {
        const H: &str = "solo";
        self.solo_entries(name)?
            .into_iter()
            .enumerate()
            .map(|(i, (b, n, d))| match n.single() {
                Some(n) => Ok((b, n, d)),
                None => Err(MusicError::new(
                    H,
                    format!("{name}'s solo: entry {i} is a chord, and {name}'s part plays one note at a time"),
                )),
            })
            .collect()
    }

    /// `T.SOLO[name]` as it is: each entry `(beat, note or chord, beats)`: the fight file's, else the kit's sketch
    /// (single notes).
    ///
    /// # Errors
    /// Where Python raises: `name` is not a fighter of this fight, or its register holds no tone of the first
    /// bar's chord.
    pub fn solo_entries(&self, name: &str) -> Result<Vec<(f64, SoloNote, f64)>, MusicError> {
        const H: &str = "solo";
        let k = self.side(H, name)?;
        match self.ff.music.solos.get(name) {
            Some(solo) => Ok(solo.clone()),
            None => self
                .harmony
                .default_solo(self.cfg[k].register)
                .map(|s| solo_of_notes(&s))
                .map_err(|e| MusicError::new(H, format!("{name}'s solo: {e}"))),
        }
    }

    /// `float(RUN[key][P.sim_i(t), k])`: the recorded value `key` of fighter `k` at the frame `t` falls in (`HA`:
    /// the seconds of haste it has left).
    ///
    /// # Errors
    /// Where Python raises: the recording has no array `key` of two numbers a frame that reaches that frame, or
    /// `k` is not 0 or 1.
    pub fn run_at(&self, key: &str, t: f64, k: usize) -> Result<f64, MusicError> {
        let at = self.sim_i(t) * 2 + k;
        let value = match self.rec.arrays.get(key) {
            _ if k > 1 => None,
            Some(Array::F64 { shape, data }) if shape.len() == 2 && shape[1] == 2 => data.get(at).copied(),
            Some(Array::I64 { shape, data }) if shape.len() == 2 && shape[1] == 2 => data.get(at).map(|&v| v as f64),
            _ => None,
        };
        value.ok_or_else(|| {
            MusicError::new(
                "run_at",
                format!("the recording has no {key} of fighter {k} at {t} s (two numbers at every frame)"),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use bf_core::event::Value;
    use bf_core::fightfile::{SoloNote, solo_of_notes};
    use bf_core::recording::Array;

    use crate::music::MusicCtx;
    use crate::music::testkit::{context, episode};

    /// As the kit printed `T.line_at` for the episode (see `bf_core::lines` for the plain lines): the FLUTE's
    /// own line, the TROMBONE's sketch, and the FLUTE's with a window registered by
    /// `music.line_override(0, 12.0, 20.0, 12.5, [(0, 90, 1), (1, None, 1), (2.5, 91, 0.5), (4, 92, 2)])`.
    #[test]
    fn line_at_matches_python() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        let ts = [0.0, 3.588, 3.82, 12.34, 31.4, 50.02, 60.99];
        let flute = [83, 76, 79, 84, 78, 76, 76];
        let trombone = [51, 52, 55, 52, 57, 52, 52];
        for (i, t) in ts.into_iter().enumerate() {
            assert_eq!(ctx.line_at("FLUTE", t), Ok(Some(flute[i])), "FLUTE at {t}");
            assert_eq!(ctx.line_at("TROMBONE", t), Ok(Some(trombone[i])), "TROMBONE at {t}");
        }
        ctx.line_override(
            0,
            12.0,
            20.0,
            12.5,
            &[
                (0.0, Some(90), 1.0),
                (1.0, None, 1.0),
                (2.5, Some(91), 0.5),
                (4.0, Some(92), 2.0),
            ],
        )
        .unwrap();
        // The window opens a millionth of a second early; before its first note, and while its rest lasts, the
        // line itself plays.
        let cases = [
            (11.9, 76),
            (11.9999995, 79),
            (12.0, 79),
            (12.4, 84),
            (12.5, 90),
            (12.9, 90),
            (12.95, 72),
            (13.0, 72),
            (13.62, 84),
            (13.63, 91),
            (14.29, 91),
            (14.3, 92),
            (19.99, 92),
            (20.0, 84),
        ];
        for (t, want) in cases {
            assert_eq!(ctx.line_at("FLUTE", t), Ok(Some(want)), "at {t}");
        }
        // The other fighter's line is its own.
        assert_eq!(ctx.line_at("TROMBONE", 12.5), Ok(Some(48)));
    }

    #[test]
    fn a_line_that_cannot_be_read_is_an_error() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        let (cfg, voices) = (ctx.cfg, ctx.voices.clone());
        assert_eq!(
            ctx.line_at("PIANO", 4.0).unwrap_err().to_string(),
            "line_at: \"PIANO\" is not a fighter of this fight (\"FLUTE\" and \"TROMBONE\" are)"
        );
        drop(ctx);
        // A line with rests gives them; one shorter than the loop is refused.
        let mut ff = ep.ff.clone();
        ff.music.lines.insert("TROMBONE".into(), vec![Some(40), None, Some(43)]);
        let ctx = MusicCtx::new(&ep.show, &ep.rec, &ff, cfg, voices.clone(), Vec::new()).unwrap();
        assert_eq!(
            ctx.line_at("TROMBONE", 4.0).unwrap_err().to_string(),
            "line_at: TROMBONE's line: a line of 3 note(s) is shorter than the loop's 8 bars (it needs at least \
             one a bar)"
        );
        drop(ctx);
        ff.music
            .lines
            .insert("TROMBONE".into(), (0..8).map(|i| (i != 1).then_some(40 + i)).collect());
        let ctx = MusicCtx::new(&ep.show, &ep.rec, &ff, cfg, voices, Vec::new()).unwrap();
        let bar = ep.show.grid.bar;
        assert_eq!(ctx.line_at("TROMBONE", 2.0 * bar), Ok(Some(40)));
        assert_eq!(ctx.line_at("TROMBONE", 3.0 * bar), Ok(None));
    }

    #[test]
    fn solo_is_the_fight_files_or_the_kits_sketch() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        let (cfg, voices) = (ctx.cfg, ctx.voices.clone());
        assert_eq!(solo_of_notes(&ctx.solo("FLUTE").unwrap()), ep.ff.music.solos["FLUTE"]);
        assert_eq!(ctx.solo_entries("FLUTE").unwrap(), ep.ff.music.solos["FLUTE"]);
        assert_eq!(ctx.solo("TROMBONE").unwrap().len(), 22);
        assert_eq!(
            ctx.solo("PIANO").unwrap_err().to_string(),
            "solo: \"PIANO\" is not a fighter of this fight (\"FLUTE\" and \"TROMBONE\" are)"
        );
        drop(ctx);
        // Without one in the fight file: `T.default_solo(name)`, as the kit printed it for the episode.
        let mut ff = ep.ff.clone();
        ff.music.solos.clear();
        let ctx = MusicCtx::new(&ep.show, &ep.rec, &ff, cfg, voices.clone(), Vec::new()).unwrap();
        let notes = |solo: Vec<(f64, i32, f64)>| solo.iter().map(|n| n.1).collect::<Vec<_>>();
        assert_eq!(
            notes(ctx.solo("FLUTE").unwrap()),
            [79, 83, 88, 91, 88, 83, 91, 88, 83, 79, 79]
        );
        assert_eq!(
            notes(ctx.solo("TROMBONE").unwrap()),
            [47, 52, 55, 59, 55, 52, 59, 55, 52, 47, 47]
        );
        assert_eq!(ctx.solo("FLUTE").unwrap()[6], (3.0, 91, 1.0));
        drop(ctx);
        // A chord: `solo_entries` has it as written, `solo` (the single-note parts) stops on it as the kit does.
        let mut ff = ep.ff.clone();
        ff.music.solos.get_mut("TROMBONE").unwrap()[2].1 = SoloNote::Chord(vec![40, 47, 52]);
        let ctx = MusicCtx::new(&ep.show, &ep.rec, &ff, cfg, voices, Vec::new()).unwrap();
        assert_eq!(
            ctx.solo_entries("TROMBONE").unwrap()[2],
            (1.0, SoloNote::Chord(vec![40, 47, 52]), 1.0)
        );
        assert_eq!(
            ctx.solo("TROMBONE").unwrap_err().to_string(),
            "solo: TROMBONE's solo: entry 2 is a chord, and TROMBONE's part plays one note at a time"
        );
    }

    #[test]
    fn run_at_reads_the_recorded_frame() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        let (cfg, voices) = (ctx.cfg, ctx.voices.clone());
        // FLUTTER TONGUE starts at 11.21 s: the FLUTE is hasted a moment later, the TROMBONE is not.
        assert!(ctx.run_at("HA", 11.5, 0).unwrap() > 0.0);
        assert_eq!(ctx.run_at("HA", 11.5, 1), Ok(0.0));
        assert_eq!(ctx.run_at("HA", 5.0, 0), Ok(0.0));
        // Before FIGHT! it is the first frame's value, after the K.O. the last one's.
        assert_eq!(ctx.run_at("HP", 0.0, 0), Ok(100.0));
        assert!(ctx.run_at("HP", 60.0, 0).unwrap() <= 0.0);
        let missing = |key: &str, k: usize| ctx.run_at(key, 11.5, k).unwrap_err().to_string();
        assert_eq!(
            missing("NOPE", 0),
            "run_at: the recording has no NOPE of fighter 0 at 11.5 s (two numbers at every frame)"
        );
        // An array of another shape (`P` holds two coordinates for each fighter), a ragged one, a third fighter.
        assert!(missing("P", 0).contains("no P of fighter 0"));
        assert!(missing("COB", 1).contains("no COB of fighter 1"));
        assert!(missing("HA", 2).contains("no HA of fighter 2"));
        drop(ctx);
        // Whole numbers are read as numbers; an array shorter than the fight is an error at the frames it lacks.
        let mut rec = ep.rec.clone();
        rec.arrays.insert(
            "HA".into(),
            Array::I64 {
                shape: vec![3, 2],
                data: vec![0, 1, 2, 3, 4, 5],
            },
        );
        rec.arrays.insert("X".into(), Array::Nested(Value::List(Vec::new())));
        let ctx = MusicCtx::new(&ep.show, &rec, &ep.ff, cfg, voices, Vec::new()).unwrap();
        let t_fight = ep.show.grid.t_fight;
        assert_eq!(ctx.run_at("HA", t_fight + 2.0 / 30.0, 1), Ok(5.0));
        assert!(ctx.run_at("HA", t_fight + 0.1, 1).is_err());
        assert!(ctx.run_at("X", t_fight, 0).is_err());
    }
}
