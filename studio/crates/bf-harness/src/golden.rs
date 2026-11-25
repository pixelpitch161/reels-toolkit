//! The Classic lock (sound upgrade, 2026-11-25): one Classic fight per theme, and enough more that every fighter
//! plays in one, each pinned by the SHA-256 of its whole soundtrack in `tests/golden/classic.txt`. The sound upgrade
//! changes the Composed arrangement only; this is the proof that Classic did not move by one sample.
//!
//! The list is made from the content itself ([`fights`]), so a new theme or fighter joins it when the file is
//! written again: `cargo run --release -p bf-harness --example golden` (it renders, it is not a test run).

use std::collections::BTreeSet;

use bf_content::themes::{CLASSIC, PORTED_THEMES, theme};
use bf_content::{build_soundtrack, roster, starter, themes_for};
use bf_core::fightfile::FightFile;

use crate::refs::{hex, sha};

/// One pinned fight: the theme and its two fighters, left corner first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoldenFight {
    pub theme: String,
    pub a: String,
    pub b: String,
}

/// What a pinned fight rendered to: the seed of its first fight with a K.O., the hash of every sample of the
/// master and the stems as they are (`exact`), and of the master as 16-bit PCM (`pcm`; it tells a one-digit
/// difference between two machines from a real change).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoldenHash {
    pub seed: u64,
    pub exact: String,
    pub pcm: String,
}

/// The fighters a fight can name, in roster order (the developer fighters among them where a starter exists).
fn names() -> Vec<&'static str> {
    roster().iter().map(|f| f.name.as_str()).collect()
}

/// The pinned fights: every theme with the pair of its own first preset (a theme without one takes the first
/// roster pair it is offered for), then each fighter no such fight holds, with the next roster fighter it shares a
/// theme with, on the first such theme.
pub fn fights() -> Vec<GoldenFight> {
    let names = names();
    let offered = |a: &str, b: &str| {
        if starter(a, b).is_ok() {
            themes_for(a, b)
        } else {
            Vec::new()
        }
    };
    let mut out: Vec<GoldenFight> = Vec::new();
    for name in PORTED_THEMES {
        let preset = theme(name).and_then(|t| t.card.presets.first()).map(|p| p.fighters);
        let pair = preset.filter(|[a, b]| offered(a, b).contains(&name)).or_else(|| {
            names.iter().enumerate().find_map(|(i, a)| {
                names[i + 1..]
                    .iter()
                    .find(|b| offered(a, b).contains(&name))
                    .map(|b| [*a, *b])
            })
        });
        if let Some([a, b]) = pair {
            out.push(GoldenFight {
                theme: name.to_string(),
                a: a.to_string(),
                b: b.to_string(),
            });
        }
    }
    let mut seen: BTreeSet<String> = out.iter().flat_map(|g| [g.a.clone(), g.b.clone()]).collect();
    for (i, a) in names.iter().enumerate() {
        if seen.contains(*a) {
            continue;
        }
        let partner = (1..names.len()).map(|k| names[(i + k) % names.len()]).find_map(|b| {
            let themes = offered(a, b);
            themes.first().map(|t| (b, *t))
        });
        if let Some((b, t)) = partner {
            seen.insert(a.to_string());
            seen.insert(b.to_string());
            out.push(GoldenFight {
                theme: t.to_string(),
                a: a.to_string(),
                b: b.to_string(),
            });
        }
    }
    out
}

/// The fight file of a pinned fight: the pair's starter on the theme, Classic, with the theme's own tempo, key and
/// chords where the pair is its preset's.
pub fn fight_file(g: &GoldenFight) -> Result<FightFile, String> {
    let mut ff = starter(&g.a, &g.b).map_err(|e| e.to_string())?;
    ff.music.theme = g.theme.clone();
    ff.music.arrangement = CLASSIC.to_string();
    ff.music.theme_options.clear();
    ff.music.mix.clear();
    ff.music.lines.clear();
    ff.music.solos.clear();
    let preset = theme(&g.theme)
        .and_then(|t| t.card.presets.first())
        .filter(|p| p.fighters == [g.a.as_str(), g.b.as_str()]);
    if let Some(p) = preset {
        ff.tempo_bpm = p.bpm;
        ff.music.key = p.key.to_string();
        ff.music.scale = p.scale.to_string();
        ff.music.chords_intro = p.intro.iter().map(|c| c.to_string()).collect();
        ff.music.chord_loop = p.chord_loop.iter().map(|c| c.to_string()).collect();
        ff.music.chords_tail = p.tail.iter().map(|c| c.to_string()).collect();
    }
    Ok(ff)
}

/// Renders a pinned fight (the first of seeds 1 to 12 with a K.O.) and hashes its soundtrack.
pub fn hash(g: &GoldenFight) -> Result<GoldenHash, String> {
    let ff = fight_file(g)?;
    let (seed, rec) = (1..=12u64)
        .find_map(|seed| {
            let setup = bf_sim::setup_from(&ff, roster(), seed).ok()?;
            let rec = bf_sim::run(&setup, true).ok()?;
            rec.t_ko.is_some().then_some((seed, rec))
        })
        .ok_or("no K.O. in 12 seeds")?;
    let st = build_soundtrack(&rec, &ff).map_err(|e| e.to_string())?;
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    let mut feed = |x: &[[f64; 2]]| {
        let mut bytes = Vec::with_capacity(x.len() * 16);
        for v in x.iter().flatten() {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        h.update(&bytes);
    };
    feed(&st.master);
    for (name, stem) in &st.stems {
        feed(&[[name.len() as f64, 0.0]]);
        feed(stem);
    }
    let mut pcm = Vec::with_capacity(st.master.len() * 4);
    for v in st.master.iter().flatten() {
        pcm.extend_from_slice(&((v.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes());
    }
    Ok(GoldenHash {
        seed,
        exact: hex(&h.finalize()),
        pcm: sha(&pcm),
    })
}

/// One line of `classic.txt`: `theme|a|b|seed|exact|pcm` (fighter names hold spaces).
pub fn line(g: &GoldenFight, h: &GoldenHash) -> String {
    format!("{}|{}|{}|{}|{}|{}", g.theme, g.a, g.b, h.seed, h.exact, h.pcm)
}

/// Reads a line [`line`] wrote.
pub fn parse(line: &str) -> Option<(GoldenFight, GoldenHash)> {
    let f: Vec<&str> = line.split('|').collect();
    let [theme, a, b, seed, exact, pcm] = f[..] else {
        return None;
    };
    Some((
        GoldenFight {
            theme: theme.to_string(),
            a: a.to_string(),
            b: b.to_string(),
        },
        GoldenHash {
            seed: seed.parse().ok()?,
            exact: exact.to_string(),
            pcm: pcm.to_string(),
        },
    ))
}

/// Hashes `fights` on `threads` threads, in the list's order.
pub fn hash_all(fights: &[GoldenFight], threads: usize) -> Vec<Result<GoldenHash, String>> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out = std::sync::Mutex::new(vec![None; fights.len()]);
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(g) = fights.get(i) else { break };
                    let h = hash(g);
                    out.lock().expect("the results")[i] = Some(h);
                }
            });
        }
    });
    out.into_inner()
        .expect("the results")
        .into_iter()
        .map(|h| h.expect("every fight hashed"))
        .collect()
}
