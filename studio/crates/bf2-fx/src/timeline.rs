//! The fight's effects: every record, and which are live at a time.

use std::sync::Mutex;

use crate::record::{FxError, FxRecord};

/// Records sorted by start, for finding the live ones by binary search.
#[derive(Default)]
struct Index {
    /// (t0, record) by t0, then by record.
    by_t0: Vec<(f64, u32)>,
    /// The longest life.
    max_life: f64,
    /// How many records it was built from.
    built_from: usize,
}

/// Every effect of a fight. Built once from the recording (and by the looks); read every frame.
#[derive(Default)]
pub struct FxTimeline {
    /// Add with [`FxTimeline::push`], which checks the record.
    pub records: Vec<FxRecord>,
    index: Mutex<Index>,
}

impl Clone for FxTimeline {
    fn clone(&self) -> Self {
        FxTimeline {
            records: self.records.clone(),
            index: Mutex::new(Index::default()),
        }
    }
}

impl std::fmt::Debug for FxTimeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FxTimeline")
            .field("records", &self.records.len())
            .finish()
    }
}

impl FxTimeline {
    pub fn new() -> FxTimeline {
        FxTimeline::default()
    }

    /// Adds `r`, refusing a life of 0 or less or a start that is not a number.
    pub fn push(&mut self, r: FxRecord) -> Result<(), FxError> {
        r.check()?;
        self.records.push(r);
        Ok(())
    }

    /// Adds every record of `rs`, stopping at the first refused.
    pub fn extend(&mut self, rs: impl IntoIterator<Item = FxRecord>) -> Result<(), FxError> {
        for r in rs {
            self.push(r)?;
        }
        Ok(())
    }

    /// The records live at `t` (`t0 <= t < t0 + life`) into `out` (cleared first), oldest start first.
    pub fn active(&self, t: f64, out: &mut Vec<usize>) {
        out.clear();
        let mut ix = self.index.lock().unwrap_or_else(|e| e.into_inner());
        if ix.built_from != self.records.len() {
            let mut by_t0: Vec<(f64, u32)> = self.records.iter().enumerate().map(|(i, r)| (r.t0, i as u32)).collect();
            by_t0.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            *ix = Index {
                by_t0,
                max_life: self.records.iter().map(|r| f64::from(r.life)).fold(0.0, f64::max),
                built_from: self.records.len(),
            };
        }
        let from = t - ix.max_life;
        let start = ix.by_t0.partition_point(|&(t0, _)| t0 <= from);
        for &(t0, i) in &ix.by_t0[start..] {
            if t0 > t {
                break;
            }
            if self.records[i as usize].live_at(t) {
                out.push(i as usize);
            }
        }
    }

    /// Checks every record again (for records changed in place).
    pub fn check_all(&self) -> Result<(), FxError> {
        for (i, r) in self.records.iter().enumerate() {
            r.check().map_err(|e| FxError(format!("effect {i}: {e}")))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lights::LightDef;
    use crate::record::{Anchor, Env, Exit, FxKind};

    fn rec(t0: f64, life: f32) -> FxRecord {
        FxRecord {
            t0,
            life,
            exit: Exit::Fade,
            env: Env::HIT,
            seed: 1,
            anchor: Anchor::World([0.0; 3]),
            kind: FxKind::Light(LightDef::default()),
        }
    }

    #[test]
    fn push_refuses_zero_life() {
        let mut tl = FxTimeline::new();
        assert!(tl.push(rec(1.0, 0.0)).is_err());
        assert!(tl.push(rec(1.0, -2.0)).is_err());
        assert!(tl.push(rec(1.0, f32::NAN)).is_err());
        assert!(tl.push(rec(f64::NAN, 1.0)).is_err());
        assert!(tl.push(rec(f64::INFINITY, 1.0)).is_err());
        assert!(tl.push(rec(1.0, 0.5)).is_ok());
        assert_eq!(tl.records.len(), 1);
        assert!(tl.check_all().is_ok());
    }

    #[test]
    fn active_finds_overlapping_records() {
        let mut state = 0x2545_F491_4F6C_DD1Du64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut tl = FxTimeline::new();
        for _ in 0..100_000 {
            let t0 = next() * 61.0;
            let life = (next() * next() * 8.0 + 0.001) as f32;
            tl.push(rec(t0, life)).unwrap();
        }
        let mut got = Vec::new();
        for _ in 0..200 {
            let t = next() * 63.0 - 1.0;
            tl.active(t, &mut got);
            let mut want: Vec<usize> = (0..tl.records.len()).filter(|&i| tl.records[i].live_at(t)).collect();
            let mut sorted = got.clone();
            sorted.sort_unstable();
            want.sort_unstable();
            assert_eq!(sorted, want, "at {t}");
            assert!(got.windows(2).all(|w| tl.records[w[0]].t0 <= tl.records[w[1]].t0));
        }
        // Edges: live from t0, gone at t0 + life.
        let mut tl = FxTimeline::new();
        tl.push(rec(2.0, 1.0)).unwrap();
        tl.active(2.0, &mut got);
        assert_eq!(got, vec![0]);
        tl.active(3.0, &mut got);
        assert!(got.is_empty());
    }
}
