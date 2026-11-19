//! `kit/checks.py` `band()`: each fighter's band sits above the bed while it plays.
//!
//! Every stem is read as its 16-bit file holds it (mono, divided by 32,768) and band-passed with the fighter's
//! own band (`butter(4, [lo, hi], 'band', fs=48000)` as sections). Over each section of the fight (FIGHT! to
//! 10 s, 10 to 20, 20 to 32, 32 to 44, 44 s to the K.O.) two numbers are taken, in dB: the fighter's level less
//! the bed's (the bed buses added up) over the whole section, and the same over the samples where the fighter
//! sounds (its 20 ms moving RMS above a tenth of its largest in the section). The check passes when every
//! "while it sounds" number is above 0.
//!
//! Where the kit would stop with a Python error the port decides, in the kit's spirit: a section that does not
//! exist (a K.O. before 44 s) or is shorter than the 20 ms window is left out; a section in which the fighter
//! never sounds is left out as in the kit (its number is NaN there), but a fighter that sounds in no section at
//! all fails the check.

use bf_core::show::Show;
use bf_sound::dsp::{self, FilterKind};
use bf_sound::master::Soundtrack;
use bf_sound::wav::sample_i16;

use super::{CheckResult, SRF, level_db, sample_at};

const NAME: &str = "band";
const TITLE: &str = "Band";

/// `_stem(k)`: a stem as its 16-bit file holds it, mono: `x.astype(float).mean(1) / 32768`.
fn stem(x: &[[f64; 2]]) -> Vec<f64> {
    x.iter()
        .map(|f| (f64::from(sample_i16(f[0])) + f64::from(sample_i16(f[1]))) / 2.0 / 32768.0)
        .collect()
}

/// `db(x)`: `20 * np.log10(np.sqrt((x ** 2).mean()) + 1e-9)`.
fn db(x: &[f64]) -> f64 {
    level_db(x, 1e-9)
}

/// `gated(f, bed, sr)`: the fighter's level less the bed's over the samples where the fighter's 20 ms moving RMS
/// is above a tenth of its largest; `None` when it never sounds (the kit's NaN). `f` is at least `w` long.
fn gated(f: &[f64], bed: &[f64], w: usize) -> Option<f64> {
    // e = np.sqrt(np.convolve(f ** 2, np.ones(w) / w, 'same')); on = e > e.max() * 0.1
    let squares: Vec<f64> = f.iter().map(|v| v * v).collect();
    let window = vec![1.0 / w as f64; w];
    let e: Vec<f64> = dsp::convolve_same(&squares, &window)
        .into_iter()
        .map(f64::sqrt)
        .collect();
    let gate = e.iter().copied().fold(f64::NEG_INFINITY, f64::max) * 0.1;
    let (mut mine, mut theirs) = (Vec::new(), Vec::new());
    for ((&level, &a), &b) in e.iter().zip(f).zip(bed) {
        if level > gate {
            mine.push(a);
            theirs.push(b);
        }
    }
    // db(f[on]) - db(bed[on]) if on.any() else nan
    (!mine.is_empty()).then(|| db(&mine) - db(&theirs))
}

/// The band check. `sounds` gives, per fighter (left corner first, as `show.names`), its bus and its band in
/// Hz; `bed_buses` the buses it must sit above (the bed's own and `fx`; one the soundtrack does not have is
/// skipped, as the kit skips a stem file that is not there).
///
/// Numbers, per fighter and section (1 to 5): `<bus> section <n>` and `<bus> while it sounds <n>`, in dB; a
/// section that was left out has none.
pub fn band(st: &Soundtrack, show: &Show, sounds: [(&str, (f64, f64)); 2], bed_buses: &[&str]) -> CheckResult {
    let fail = |text: String| CheckResult::failed(NAME, format!("{TITLE}: {text}"));
    // what the kit's reads would raise on
    let n = st.master.len();
    if n == 0 {
        return fail("the soundtrack is empty, so there is nothing to measure; build the soundtrack again.".into());
    }
    for (k, &(bus, (lo, hi))) in sounds.iter().enumerate() {
        let name = &show.names[k];
        if !(lo.is_finite() && hi.is_finite() && 0.0 < lo && lo < hi && hi < SRF / 2.0) {
            return fail(format!(
                "the {name}'s band, {lo} to {hi} Hz, is not a band below 24000 Hz, so it cannot be measured; \
                 this is a fault in the fighter's sound, not in your fight."
            ));
        }
        if !st.stems.contains_key(bus) {
            return fail(format!(
                "the soundtrack has no part for the {name} (no bus \"{bus}\"), so its band cannot be measured; \
                 build the soundtrack again."
            ));
        }
    }
    let bed: Vec<&str> = bed_buses
        .iter()
        .copied()
        .filter(|b| st.stems.contains_key(*b))
        .collect();
    if bed.is_empty() {
        return fail(
            "the soundtrack has none of the bed's buses, so there is nothing to compare the fighters with; \
             build the soundtrack again."
                .into(),
        );
    }
    for bus in sounds.iter().map(|s| s.0).chain(bed.iter().copied()) {
        let x = &st.stems[bus];
        if x.len() != n || x.iter().any(|f| !(f[0].is_finite() && f[1].is_finite())) {
            return fail(format!(
                "the bus \"{bus}\" of the soundtrack is not {n} samples of numbers, so it cannot be measured; \
                 build the soundtrack again."
            ));
        }
    }

    let w = (0.02 * SRF) as usize;
    let t = &show.grid;
    // spans = [(T.T_FIGHT, 10), (10, 20), (20, 32), (32, 44), (44, M.T_KO)]
    let spans = [
        (t.t_fight, 10.0),
        (10.0, 20.0),
        (20.0, 32.0),
        (32.0, 44.0),
        (44.0, show.t_ko),
    ];
    let bed_stems: Vec<Vec<f64>> = bed.iter().map(|b| stem(&st.stems[*b])).collect();
    let mut result = CheckResult::new(NAME, true, String::new(), &[]);
    let mut faults = Vec::new();
    let mut ranges = Vec::new();
    for (k, &(bus, (lo, hi))) in sounds.iter().enumerate() {
        let name = &show.names[k];
        // f = bp(x, sr, lo, hi); bedx = sum(bp(_stem(k)[1], sr, lo, hi) for k in bed_buses)
        let sos = dsp::butter_sos(FilterKind::Bp, &[lo, hi], 4, SRF);
        let f = dsp::sosfilt(&sos, &stem(&st.stems[bus]));
        let mut bedx = vec![0.0; n];
        for x in &bed_stems {
            for (sum, v) in bedx.iter_mut().zip(dsp::sosfilt(&sos, x)) {
                *sum += v;
            }
        }
        let (mut least, mut most): (Option<(f64, usize)>, f64) = (None, f64::NEG_INFINITY);
        for (i, &(a, b)) in spans.iter().enumerate() {
            // sl = slice(int(a * sr), int(b * sr))
            let (i0, i1) = (sample_at(a, n), sample_at(b, n));
            if i1 < i0 + w {
                continue;
            }
            let (fs, bs) = (&f[i0..i1], &bedx[i0..i1]);
            result.put(&format!("{bus} section {}", i + 1), db(fs) - db(bs));
            if let Some(g) = gated(fs, bs, w) {
                result.put(&format!("{bus} while it sounds {}", i + 1), g);
                if least.is_none_or(|(l, _)| g < l) {
                    least = Some((g, i));
                }
                most = most.max(g);
            }
        }
        match least {
            None => faults.push(format!(
                "the {name} is silent in its band ({lo:.0} to {hi:.0} Hz) for the whole fight, so check that its \
                 part plays"
            )),
            // ok &= all(g > 0 for _, g in vals if not math.isnan(g))
            Some((g, i)) if g <= 0.0 || g.is_nan() => faults.push(format!(
                "the {name} is {:.1} dB under the bed in its band ({lo:.0} to {hi:.0} Hz) between {:.0} and {:.0} s \
                 while it plays, so turn the {name} up or thin the bed there",
                -g, spans[i].0, spans[i].1
            )),
            Some((g, _)) => ranges.push(format!("the {name} by {g:.1} to {most:.1} dB")),
        }
    }
    result.pass = faults.is_empty();
    result.summary = if result.pass {
        format!(
            "{TITLE}: each fighter is louder than the bed in its own band in every section of the fight while it \
             plays: {}.",
            ranges.join(", ")
        )
    } else {
        format!("{TITLE}: {}.", faults.join("; and "))
    };
    result
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bf_core::dmath;
    use bf_core::fightfile::FightFile;
    use bf_core::recording::Recording;

    use super::*;

    fn show() -> Show {
        let rec = Recording::read(&bf_testdata::reference("fights/flute_vs_trombone/ep392.bfrec")).unwrap();
        let path = bf_testdata::repo_root().join("studio/fights/flute_vs_trombone.toml");
        let ff = FightFile::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
        Show::new(&rec, &ff).unwrap()
    }

    fn tone(n: usize, hz: f64, amp: f64) -> Vec<[f64; 2]> {
        (0..n)
            .map(|i| {
                let v = amp * dmath::sin(std::f64::consts::TAU * hz * i as f64 / SRF);
                [v, v]
            })
            .collect()
    }

    fn soundtrack(n: usize, flute: f64, trombone: f64, bed: f64) -> Soundtrack {
        let mut stems = BTreeMap::new();
        stems.insert("flute".to_string(), tone(n, 1000.0, flute));
        stems.insert("trombone".to_string(), tone(n, 200.0, trombone));
        // the bed plays in both bands
        let both: Vec<[f64; 2]> = tone(n, 1100.0, bed)
            .iter()
            .zip(tone(n, 220.0, bed))
            .map(|(a, b)| [a[0] + b[0], a[1] + b[1]])
            .collect();
        stems.insert("drums".to_string(), both);
        Soundtrack {
            master: vec![[0.0, 0.0]; n],
            stems,
        }
    }

    const SOUNDS: [(&str, (f64, f64)); 2] = [("flute", (600.0, 3200.0)), ("trombone", (70.0, 700.0))];

    #[test]
    fn louder_than_the_bed_passes_and_quieter_fails() {
        let show = show();
        let n = 12 * 48_000;
        // 12 s: FIGHT! to 10 s and 2 s of the next section; the later sections do not exist and are left out
        let r = band(&soundtrack(n, 0.2, 0.2, 0.1), &show, SOUNDS, &["drums", "fx"]);
        assert!(r.pass, "{}", r.summary);
        assert_eq!(r.numbers.len(), 8);
        assert!(
            (r.numbers["flute while it sounds 1"] - 6.02).abs() < 0.05,
            "{:?}",
            r.numbers
        );
        let r = band(&soundtrack(n, 0.2, 0.05, 0.1), &show, SOUNDS, &["drums", "fx"]);
        assert!(!r.pass);
        assert!(
            r.summary.contains("the TROMBONE is 6.") && r.summary.contains("dB under the bed"),
            "{}",
            r.summary
        );
        assert!(!r.summary.contains("FLUTE"), "{}", r.summary);
        // a fighter that never plays
        let r = band(&soundtrack(n, 0.0, 0.2, 0.1), &show, SOUNDS, &["drums"]);
        assert!(!r.pass && r.summary.contains("the FLUTE is silent"), "{}", r.summary);
    }

    #[test]
    fn unusual_input_fails_in_words() {
        let show = show();
        let good = soundtrack(48_000 * 6, 0.2, 0.2, 0.1);
        type Case = (
            Soundtrack,
            [(&'static str, (f64, f64)); 2],
            &'static [&'static str],
            &'static str,
        );
        let cases: [Case; 6] = [
            (
                Soundtrack {
                    master: vec![],
                    stems: BTreeMap::new(),
                },
                SOUNDS,
                &["drums"],
                "empty",
            ),
            (
                good.clone(),
                [("oboe", (600.0, 3200.0)), SOUNDS[1]],
                &["drums"],
                "no part for the FLUTE",
            ),
            (good.clone(), SOUNDS, &["strings"], "none of the bed's buses"),
            (
                good.clone(),
                [("flute", (3200.0, 600.0)), SOUNDS[1]],
                &["drums"],
                "is not a band",
            ),
            (
                good.clone(),
                [("flute", (600.0, 30_000.0)), SOUNDS[1]],
                &["drums"],
                "is not a band",
            ),
            (
                {
                    let mut st = good.clone();
                    st.stems.get_mut("drums").unwrap()[5][0] = f64::NAN;
                    st
                },
                SOUNDS,
                &["drums"],
                "\"drums\"",
            ),
        ];
        for (st, sounds, bed, word) in cases {
            let r = band(&st, &show, sounds, bed);
            assert!(!r.pass && r.summary.contains(word), "{}", r.summary);
            assert!(r.summary.starts_with("Band: ") && r.summary.ends_with('.'));
        }
        // shorter than FIGHT!: no section exists, both fighters count as silent
        let r = band(&soundtrack(48_000, 0.2, 0.2, 0.1), &show, SOUNDS, &["drums"]);
        assert!(!r.pass && r.summary.contains("silent"), "{}", r.summary);
    }
}
