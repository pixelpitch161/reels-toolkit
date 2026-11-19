//! The automatic level fix: when a soundtrack would fail the [`levels`](super::levels) check, its loudness is
//! evened out where the problem is, so the video renders instead of stopping at the check.
//!
//! Each 2 s window of the check gets a gain (in dB) that brings it to the level of the ending (the mean of the
//! last 10 windows, what the series is mixed to). The gains are joined smoothly from the middle of one window to
//! the middle of the next, wrapping round from the end to the start so the loop point gets no step. Nearly
//! silent windows are left alone, and no window moves by more than [`MOST_DB`]. A peak limiter then holds every
//! sample under the master's ceiling so a raised stretch never clips. This repeats (each round moving every
//! window the rest of the way) until the check passes, for at most [`ROUNDS`] rounds.
//!
//! A soundtrack that already passes is never touched, so every passing fight sounds exactly as before.

use bf_core::dmath;
use bf_sound::master::MASTER_CEILING;

use super::levels::{WIN, levels, profile};
use super::{SRF, as_wav16, mono};

/// No window is raised or lowered by more than this, in dB.
pub const MOST_DB: f64 = 12.0;
/// Windows quieter than this (dBFS) are left as they are: there is nothing there to raise.
const QUIET_DB: f64 = -60.0;
/// The rounds of evening out tried before giving up.
pub const ROUNDS: usize = 6;
/// The limiter's block: 5 ms.
const BLOCK: usize = 240;

/// What [`even_out`] did.
#[derive(Clone, Debug, PartialEq)]
pub struct Evened {
    /// The evened master.
    pub master: Vec<[f64; 2]>,
    /// The largest gain it gave a stretch, in dB (negative for a cut).
    pub most_up_db: f64,
    /// The largest cut it gave a stretch, in dB (as a positive number).
    pub most_down_db: f64,
}

impl Evened {
    /// A plain sentence for the check list, saying what was changed.
    pub fn note(&self) -> String {
        let mut parts = Vec::new();
        if self.most_up_db >= 0.05 {
            parts.push(format!("quiet stretches raised by up to {:.1} dB", self.most_up_db));
        }
        if self.most_down_db >= 0.05 {
            parts.push(format!("loud stretches lowered by up to {:.1} dB", self.most_down_db));
        }
        if parts.is_empty() {
            parts.push("the peaks held under the ceiling".to_string());
        }
        format!(
            "The soundtrack's loudness was evened out automatically before the render ({}).",
            parts.join(", ")
        )
    }
}

/// The master evened out so it passes the levels check, or `None` when it passes already (or cannot be
/// measured: the check then says why). When even [`ROUNDS`] rounds do not make it pass, the best attempt is still
/// given, since it is closer than the original; the check then says what is left.
pub fn even_out(master: &[[f64; 2]]) -> Option<Evened> {
    let first = levels(&as_wav16(master));
    if first.pass || !first.numbers.contains_key("ending") {
        return None;
    }
    let len = master.len();
    let mut total = vec![0.0f64; window_count(len)];
    let mut out = master.to_vec();
    for _ in 0..ROUNDS {
        let rows = profile(&mono(&as_wav16(&out)), window_count(len) as f64 * WIN);
        if rows.len() < 2 {
            return None;
        }
        let r: Vec<f64> = rows.iter().map(|row| row.dbfs).collect();
        let tail = &r[r.len().saturating_sub(10)..];
        let ending = tail.iter().sum::<f64>() / tail.len() as f64;
        for (w, &db) in r.iter().enumerate() {
            if db > QUIET_DB {
                total[w] = (total[w] + (ending - db)).clamp(-MOST_DB, MOST_DB);
            }
        }
        out = limit(&apply(master, &total), MASTER_CEILING);
        if levels(&as_wav16(&out)).pass {
            break;
        }
    }
    let most_up_db = total.iter().copied().fold(0.0, f64::max);
    let most_down_db = -total.iter().copied().fold(0.0, f64::min);
    Some(Evened {
        master: out,
        most_up_db,
        most_down_db,
    })
}

/// The windows the levels check measures over a master of `len` samples: `round(seconds / 2)`.
fn window_count(len: usize) -> usize {
    bf_core::py::round_half_even(len as f64 / SRF / WIN) as usize
}

/// `x` with the window gains `gains_db` joined smoothly between window middles, wrapping round at the ends.
fn apply(x: &[[f64; 2]], gains_db: &[f64]) -> Vec<[f64; 2]> {
    let n = gains_db.len();
    if n == 0 || x.is_empty() {
        return x.to_vec();
    }
    let len = x.len() as f64;
    // the middle of each window, in samples (the last window may be short)
    let mids: Vec<f64> = (0..n)
        .map(|w| {
            let a = w as f64 * WIN * SRF;
            let b = ((w + 1) as f64 * WIN * SRF).min(len);
            (a + b.max(a)) / 2.0
        })
        .collect();
    let gain = |w: usize| dmath::pow(10.0, gains_db[w] / 20.0);
    x.iter()
        .enumerate()
        .map(|(i, f)| {
            let t = i as f64;
            // the windows whose middles lie either side of t, wrapping round the loop
            let k = mids.iter().position(|&m| m > t).unwrap_or(n);
            let (wa, wb, ta, tb) = if k == 0 {
                (n - 1, 0, mids[n - 1] - len, mids[0])
            } else if k == n {
                (n - 1, 0, mids[n - 1], mids[0] + len)
            } else {
                (k - 1, k, mids[k - 1], mids[k])
            };
            let u = if tb > ta {
                ((t - ta) / (tb - ta)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            // a raised-cosine join, so the gain eases from one window to the next
            let s = 0.5 - 0.5 * dmath::cos(std::f64::consts::PI * u);
            let g = gain(wa) * (1.0 - s) + gain(wb) * s;
            [f[0] * g, f[1] * g]
        })
        .collect()
}

/// `x` with its peaks held at or under `ceiling`: each 5 ms block gets the gain its loudest sample needs, the
/// lowest of it and its neighbours is taken, and the gains are joined in straight lines from block to block, so
/// no sample is ever over the ceiling and the gain never jumps.
pub(crate) fn limit(x: &[[f64; 2]], ceiling: f64) -> Vec<[f64; 2]> {
    let blocks = x.len().div_ceil(BLOCK);
    if blocks == 0 {
        return Vec::new();
    }
    let need: Vec<f64> = x
        .chunks(BLOCK)
        .map(|c| {
            let peak = c.iter().fold(0.0f64, |m, f| m.max(f[0].abs()).max(f[1].abs()));
            if peak > ceiling { ceiling / peak } else { 1.0 }
        })
        .collect();
    if need.iter().all(|&g| g >= 1.0) {
        return x.to_vec();
    }
    let held: Vec<f64> = (0..blocks)
        .map(|b| {
            let lo = b.saturating_sub(1);
            let hi = (b + 1).min(blocks - 1);
            need[lo..=hi].iter().copied().fold(1.0, f64::min)
        })
        .collect();
    x.iter()
        .enumerate()
        .map(|(i, f)| {
            // the middle of block b is at b * BLOCK + BLOCK / 2
            let pos = (i as f64 - BLOCK as f64 / 2.0) / BLOCK as f64;
            let g = if pos <= 0.0 {
                held[0]
            } else {
                let b = pos.floor() as usize;
                if b + 1 >= blocks {
                    held[blocks - 1]
                } else {
                    let u = pos - b as f64;
                    held[b] * (1.0 - u) + held[b + 1] * u
                }
            };
            [f[0] * g, f[1] * g]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(seconds: f64, amp: f64) -> Vec<[f64; 2]> {
        (0..(seconds * SRF) as usize)
            .map(|i| {
                let v = amp * dmath::sin(i as f64 * 0.05);
                [v, v]
            })
            .collect()
    }

    #[test]
    fn a_passing_soundtrack_is_left_alone() {
        assert!(even_out(&tone(61.0, 0.3)).is_none());
        assert!(even_out(&[]).is_none());
    }

    #[test]
    fn a_quiet_start_is_raised() {
        let mut x = tone(20.0, 0.1);
        x.extend(tone(41.0, 0.3));
        assert!(!levels(&as_wav16(&x)).pass);
        let e = even_out(&x).expect("evened");
        let r = levels(&as_wav16(&e.master));
        assert!(r.pass, "{}", r.summary);
        assert!(e.most_up_db > 5.0, "{e:?}");
        assert!(e.master.iter().all(|f| f[0].abs() <= MASTER_CEILING + 1e-9));
        assert!(e.note().contains("raised"));
    }

    #[test]
    fn a_loud_middle_is_lowered_without_clipping() {
        let mut x = tone(61.0, 0.2);
        for f in &mut x[20 * 48_000..30 * 48_000] {
            *f = [f[0] * 4.4, f[1] * 4.4];
        }
        assert!(!levels(&as_wav16(&x)).pass);
        let e = even_out(&x).expect("evened");
        let r = levels(&as_wav16(&e.master));
        assert!(r.pass, "{}", r.summary);
        assert!(e.most_down_db > 5.0, "{e:?}");
        assert!(e.master.iter().all(|f| f[0].abs() <= MASTER_CEILING + 1e-9));
    }

    #[test]
    fn the_limiter_never_passes_the_ceiling() {
        let x: Vec<[f64; 2]> = (0..48_000)
            .map(|i| if i % 997 == 0 { [1.5, -1.2] } else { [0.3, 0.3] })
            .collect();
        let y = limit(&x, 0.89);
        assert!(y.iter().all(|f| f[0].abs() <= 0.89 + 1e-9 && f[1].abs() <= 0.89 + 1e-9));
    }
}
