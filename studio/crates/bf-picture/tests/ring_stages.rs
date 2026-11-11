//! The ring renderer against the Python kit's stage dumps, and its purity.
//!
//! The dumps are local files, not reference files: `python -m tools.studio_export stages 150 600 1500` writes
//! `studio/reference/.cache/stages/flute_vs_trombone/f<frame>.npz`, the buffers of `kit.picture.render` after
//! each stage (`tools/studio_export/stages.py`). A test that needs them prints that it was skipped when the
//! folder is absent.
//!
//! `bf-picture` cannot see the fighters' looks (they live in `bf-content`), so two things are compared:
//!
//! - against the episode's own dumps, the stages before the fighters' layers must be Python's, every bit:
//!   `floor`, `arena` and `skirt`. Those stages hold nothing a look draws, but they are not independent of the
//!   looks: the camera shakes with each look's `shake(e)`, and the attack flares each look adds while the effects
//!   are built light the arena floor and the crowd. So the picture for this test has stand-in looks that are the
//!   default look plus exactly those hooks, written out below from `kit/fighters/flute.py` and `trombone.py`
//!   ([`FluteLights`], [`TromboneLights`]). With the bare default look these stages differ from the episode's
//!   (a test holds that too).
//! - against dumps the kit made with its base-class look for both fighters
//!   (`tools/studio_export/unit_checks/ring_default_looks.py` writes them beside the others, under
//!   `stages_default`), every stage of the pipeline is compared with [`DefaultLook`] for both.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use bf_core::event::Event;
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::roster::{FighterCfg, parse_roster};
use bf_core::show::Show;
use bf_picture::look::{DefaultLook, Fx, Look};
use bf_picture::ring::effects::up;
use bf_picture::ring::{Picture, PictureCtx, Stage, StageBuffers, StagePic};
use bf_picture::sprite::Mesh;
use bf_testdata::{NpArray, NpData, Npz, reference, reference_dir, repo_root};

const FRAMES: [usize; 3] = [150, 600, 1500];
/// `S.R_BALL`.
const R: f64 = 0.6;

fn fighter(name: &str) -> FighterCfg {
    let path = repo_root().join("studio/crates/bf-content/data/roster.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let roster = parse_roster(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    roster
        .into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{name} is not in the roster"))
}

fn mesh(pp: &str) -> Mesh {
    let path = repo_root().join(format!("studio/crates/bf-content/data/models/{pp}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    Mesh::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// `e[7]` of a hit: what hit.
fn how(e: &Event) -> &str {
    if e.kind == "hit" { e.s(7) } else { "" }
}

/// The FLUTE's look as far as the camera and the lights go (`kit/fighters/flute.py`, class `Look`): its
/// `flare_scale`, its `shake` and the `P.FL.add` calls of its `fx_setup` and `fx_event`. Everything it draws is
/// left out.
#[derive(Default)]
struct FluteLights {
    k: usize,
}

impl Look for FluteLights {
    fn flare_scale(&self) -> f64 {
        0.85
    }

    fn setup(&mut self, _p: &PictureCtx, k: usize) {
        self.k = k;
    }

    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match how(e) {
            "wave" => Some(2.0),
            "fang" => Some(0.35),
            _ => (e.kind == "slam").then_some(1.6),
        }
    }

    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        // for t_, att, vic, bid, x, z in self.SWAT_EV: a lance swats one of its cobras flat
        for e in p
            .rec
            .events
            .iter()
            .filter(|e| e.kind == "swat" && e.i(3) == self.k as i64)
        {
            if e.t > p.t_ko {
                break;
            }
            let att = e.i(2) as usize;
            fx.fl
                .add(e.t, up(e.f(5), e.f(6), 0.6), p.fl_col[att], 1.2, 0.45, 0.03, 0.2, 0.0);
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if how(e) == "fang" {
            let att = e.i(2) as usize;
            fx.fl
                .add(e.t, up(e.f(5), e.f(6), R), p.fl_col[att], 1.2, 0.4, 0.03, 0.2, 0.0);
        } else if e.kind == "ability" && e.s(3) == p.cfg[self.k].low.name {
            // PICCOLO: p = P.RUN['P'][P.sim_i(t_), k]; SILV * 0.85
            let at = p.run.p(p.sim_i(e.t), e.i(2) as usize);
            let silv = [226.0, 234.0, 248.0].map(|c: f64| c * 0.85);
            fx.fl.add(e.t, up(at[0], at[1], 1.0), silv, 3.0, 0.8, 0.03, 0.6, 0.0);
        }
    }
}

/// The TROMBONE's look as far as the camera and the lights go (`kit/fighters/trombone.py`, class `Look`): its
/// `shake` and the `P.FL.add` calls of its `fx_event` and `fx_after`.
#[derive(Default)]
struct TromboneLights {
    k: usize,
}

impl Look for TromboneLights {
    fn setup(&mut self, _p: &PictureCtx, k: usize) {
        self.k = k;
    }

    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match how(e) {
            "blade" => Some(0.55),
            "slam" => Some(2.5),
            "shockwave" => Some(1.2),
            _ if e.kind == "swat" => Some(0.4),
            _ => (e.kind == "slam").then_some(1.6),
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        let col = p.fl_col[e.i(2) as usize];
        match how(e) {
            "blade" => fx.fl.add(e.t, up(e.f(5), e.f(6), R), col, 1.4, 0.6, 0.03, 0.22, 0.0),
            "shockwave" => fx.fl.add(e.t, up(e.f(5), e.f(6), R), col, 1.6, 0.8, 0.03, 0.3, 0.0),
            // PEDAL TONE: the bell stomps (low: under the bell's rim)
            _ if e.kind == "slam" => fx.fl.add(e.t, up(e.f(3), e.f(4), 0.15), col, 2.4, 0.9, 0.03, 0.5, 0.0),
            _ => {}
        }
    }

    fn fx_after(&self, p: &PictureCtx, fx: &mut Fx) {
        // PEDAL TONE's bell hovers mint-lit
        let ev = &p.rec.events;
        for e in ev {
            if e.kind == "ability" && e.s(4) == "slam" && e.i(2) == self.k as i64 && e.t < p.t_ko {
                let first = ev.iter().find(|q| q.kind == "slam" && q.i(2) == e.i(2) && q.t >= e.t);
                if let Some(sl) = first {
                    // decay=max(0.3, sl[0][0] - e[0])
                    let decay = if sl.t - e.t > 0.3 { sl.t - e.t } else { 0.3 };
                    let pos = up(sl.f(3), sl.f(4), 2.4);
                    fx.fl
                        .add(e.t + 0.15, pos, p.fl_col[e.i(2) as usize], 2.6, 0.65, 0.03, decay, 0.0);
                }
            }
        }
    }
}

/// Makes a fighter's look from its name.
type MakeLook = fn(&str) -> Box<dyn Look>;

/// The example episode with the two looks `make` gives.
fn build(make: MakeLook) -> Picture {
    let rec = Recording::read(&reference("fights/flute_vs_trombone/ep392.bfrec")).expect("the recording reads");
    let path = repo_root().join("studio/fights/flute_vs_trombone.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let ff = FightFile::parse(&text).expect("the fight file parses");
    let show = Show::new(&rec, &ff).expect("the episode has a K.O.");
    let cfg = [fighter(&show.names[0]), fighter(&show.names[1])];
    let meshes = [mesh(&cfg[0].pp), mesh(&cfg[1].pp)];
    let looks = [make(&show.names[0]), make(&show.names[1])];
    Picture::new(Arc::new(rec), &ff, show, [&cfg[0], &cfg[1]], meshes, looks).expect("the picture builds")
}

/// The example episode drawn with the default look for both fighters, built once for the whole test binary.
fn picture() -> &'static Picture {
    static PICTURE: OnceLock<Picture> = OnceLock::new();
    PICTURE.get_or_init(|| build(|_| Box::new(DefaultLook)))
}

/// The example episode with the stand-in looks: the default look plus each fighter's shake and attack lights.
fn picture_with_lights() -> &'static Picture {
    static PICTURE: OnceLock<Picture> = OnceLock::new();
    PICTURE.get_or_init(|| {
        build(|name| match name {
            "FLUTE" => Box::new(FluteLights::default()),
            "TROMBONE" => Box::new(TromboneLights::default()),
            other => panic!("no look for {other} in this test"),
        })
    })
}

/// The stage dumps of `kind` (`stages` or `stages_default`), or `None` with a note when they are not there.
fn dumps(kind: &str) -> Option<PathBuf> {
    let dir = reference_dir().join(".cache").join(kind).join("flute_vs_trombone");
    if FRAMES.iter().all(|f| dir.join(format!("f{f:04}.npz")).is_file()) {
        Some(dir)
    } else {
        eprintln!(
            "skipped: no stage dumps in {} (see the top of tests/ring_stages.rs for the command)",
            dir.display()
        );
        None
    }
}

/// How one buffer compares with Python's.
#[derive(Debug, Default)]
struct Diff {
    /// Samples that are not equal to the bit.
    differing: usize,
    total: usize,
    /// The largest difference.
    worst: f64,
    /// The first sample that differs: `(x, y, channel, got, want)`.
    first: Option<(usize, usize, usize, f64, f64)>,
}

fn diff_of<T: Copy + Into<f64>>(got: &[T], want: &[T], same: impl Fn(T, T) -> bool, w: usize, c: usize) -> Diff {
    assert_eq!(got.len(), want.len(), "the buffers differ in size");
    let mut d = Diff {
        total: got.len(),
        ..Diff::default()
    };
    for (i, (&g, &p)) in got.iter().zip(want).enumerate() {
        if !same(g, p) {
            d.differing += 1;
            let (gf, pf): (f64, f64) = (g.into(), p.into());
            let delta = (gf - pf).abs();
            // (a NaN on one side only counts as the largest difference there is)
            d.worst = if delta.is_nan() {
                f64::INFINITY
            } else {
                d.worst.max(delta)
            };
            if d.first.is_none() {
                d.first = Some((i / c % w, i / c / w, i % c, gf, pf));
            }
        }
    }
    d
}

/// Compares a colour buffer with Python's array: the same dtype, the same shape, and how the samples differ.
fn diff_rgb(got: &StagePic, want: &NpArray) -> Diff {
    let (w, h, c, dtype) = match got {
        StagePic::U8(a) => (a.w, a.h, a.c, "|u1"),
        StagePic::F32(a) => (a.w, a.h, a.c, "<f4"),
        StagePic::F64(a) => (a.w, a.h, a.c, "<f8"),
    };
    let shape: Vec<usize> = if c == 1 { vec![h, w] } else { vec![h, w, c] };
    assert_eq!(want.shape, shape, "the shapes differ");
    assert_eq!(want.dtype(), dtype, "Python holds this buffer as {}", want.dtype());
    match (got, &want.data) {
        (StagePic::U8(a), NpData::U8(p)) => diff_of(&a.data, p, |x, y| x == y, w, c),
        (StagePic::F32(a), NpData::F32(p)) => diff_of(&a.data, p, |x, y| x.to_bits() == y.to_bits(), w, c),
        (StagePic::F64(a), NpData::F64(p)) => diff_of(&a.data, p, |x, y| x.to_bits() == y.to_bits(), w, c),
        _ => unreachable!("the dtypes were compared"),
    }
}

fn diff_depth(got: &StageBuffers, want: &NpArray) -> Diff {
    let depth = got.depth.as_ref().expect("the stage has a depth buffer");
    assert_eq!(want.shape, [depth.h, depth.w]);
    diff_of(&depth.data, want.f32(), |x, y| x.to_bits() == y.to_bits(), depth.w, 1)
}

/// The three stages of `pic` against the episode's dumps: one line per buffer that differs.
fn early_stages(pic: &Picture, dir: &Path) -> Vec<String> {
    let mut failures = Vec::new();
    for f in FRAMES {
        let z = Npz::open(&dir.join(format!("f{f:04}.npz")));
        for stage in [Stage::Floor, Stage::Arena, Stage::Skirt] {
            let got = pic.stage_buffers(f, stage);
            let name = stage.name();
            let rgb = diff_rgb(&got.rgb, z.get(&format!("{name}__rgb")));
            let depth = diff_depth(&got, z.get(&format!("{name}__depth")));
            for (what, d) in [("rgb", rgb), ("depth", depth)] {
                if d.differing != 0 {
                    failures.push(format!(
                        "frame {f} {name} {what}: {} of {} samples differ, worst {:e}, first (x, y, c, got, want) {:?}",
                        d.differing, d.total, d.worst, d.first
                    ));
                }
            }
        }
    }
    failures
}

/// The stages before the fighters' layers equal Python's, every bit, in the episode's own frames: the colour
/// and the depth buffers of `floor`, `arena` and `skirt`.
#[test]
fn floor_arena_and_skirt_match_python() {
    let Some(dir) = dumps("stages") else { return };
    let failures = early_stages(picture_with_lights(), &dir);
    println!(
        "floor, arena, skirt of frames {FRAMES:?}: {} of 18 buffers differ",
        failures.len()
    );
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// Those stages do depend on the looks: with the bare default look for both fighters (no shake of theirs, no
/// attack lights) the floor of every one of the three frames differs from the episode's.
#[test]
fn the_early_stages_need_the_looks_shake_and_lights() {
    let Some(dir) = dumps("stages") else { return };
    let failures = early_stages(picture(), &dir);
    for f in FRAMES {
        assert!(
            failures
                .iter()
                .any(|line| line.starts_with(&format!("frame {f} floor rgb"))),
            "frame {f}: the floor is the episode's without the looks' lights:\n{}",
            failures.join("\n")
        );
    }
}

/// Every stage against the kit drawn with its base-class look for both fighters.
///
/// The drawing stages (bytes, and the `float32` buffers before the lens) must be Python's to the bit, and so
/// must the frame from the cut to bytes on. The float stages between the lens and the cut to bytes pass through
/// the FFT, where Python's last bits are numpy's and the C runtime's (the renderer kit's finding): they are held
/// to `1e-9` of a level.
#[test]
fn every_stage_matches_python_with_default_looks() {
    let Some(dir) = dumps("stages_default") else { return };
    let pic = picture();
    let mut failures = Vec::new();
    for f in FRAMES {
        let z = Npz::open(&dir.join(format!("f{f:04}.npz")));
        for stage in Stage::ALL {
            let got = pic.stage_buffers(f, stage);
            let name = stage.name();
            let key = if stage == Stage::Coc {
                format!("{name}__map")
            } else {
                format!("{name}__rgb")
            };
            let rgb = diff_rgb(&got.rgb, z.get(&key));
            let float = (Stage::Coc..=Stage::PostFx).contains(&stage);
            println!(
                "frame {f} {name}: {}/{} samples differ, worst {:e}",
                rgb.differing, rgb.total, rgb.worst
            );
            let ok = if float { rgb.worst <= 1e-9 } else { rgb.differing == 0 };
            if !ok {
                failures.push(format!(
                    "frame {f} {name}: {} of {} samples differ, worst {:e}, first (x, y, c, got, want) {:?}",
                    rgb.differing, rgb.total, rgb.worst, rgb.first
                ));
            }
            let depth_key = format!("{name}__depth");
            if z.has(&depth_key) {
                let d = diff_depth(&got, z.get(&depth_key));
                if d.differing != 0 {
                    failures.push(format!(
                        "frame {f} {name} depth: {} of {} samples differ, worst {:e}, first {:?}",
                        d.differing, d.total, d.worst, d.first
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// A frame is a pure function of its number: frames 300 and 301 are the same bytes in either order and on one
/// thread or eight.
#[test]
fn frame_is_pure() {
    let pic = picture();
    let (a300, a301) = (pic.frame(300), pic.frame(301));
    let (b301, b300) = (pic.frame(301), pic.frame(300));
    assert_eq!(a300, b300, "frame 300 depends on what was drawn before it");
    assert_eq!(a301, b301, "frame 301 depends on what was drawn before it");
    assert_ne!(a300, a301, "frames 300 and 301 are the same picture");
    let threaded: Vec<_> = std::thread::scope(|s| {
        let jobs: Vec<_> = (0..8).map(|j| s.spawn(move || pic.frame(300 + j % 2))).collect();
        jobs.into_iter().map(|j| j.join().expect("a frame panicked")).collect()
    });
    for (j, img) in threaded.iter().enumerate() {
        let want = if j % 2 == 0 { &a300 } else { &a301 };
        assert_eq!(img, want, "frame {} drawn on thread {j} differs", 300 + j % 2);
    }
    // the stages are the frame's own: the last one is the frame
    assert_eq!(pic.stage_buffers(300, Stage::Title).rgb, StagePic::U8(a300));
}

/// Every 15th frame renders, at the frame's size, with the beam's hull in scipy's order; a frame past the end is
/// the last one.
#[test]
fn no_panic_sweep() {
    for pic in [picture(), picture_with_lights()] {
        let frames: Vec<usize> = (0..pic.nf()).step_by(15).chain([pic.nf() - 1]).collect();
        let next = AtomicUsize::new(0);
        let reports: Vec<_> = std::thread::scope(|s| {
            let jobs: Vec<_> = (0..8)
                .map(|_| {
                    s.spawn(|| {
                        let mut out = Vec::new();
                        while let Some(&f) = frames.get(next.fetch_add(1, Ordering::Relaxed)) {
                            let (img, _, report) = pic.frame_reported(f);
                            assert_eq!((img.w, img.h, img.c), (360, 640, 3), "frame {f}");
                            out.push((f, report));
                        }
                        out
                    })
                })
                .collect();
            jobs.into_iter()
                .flat_map(|j| j.join().expect("a frame panicked"))
                .collect()
        });
        assert_eq!(reports.len(), frames.len());
        for (f, report) in reports {
            assert_eq!(
                (report.hull_fallbacks, report.hull_errors),
                (0, 0),
                "frame {f}: the beam's hull is not in scipy's order"
            );
        }
        assert_eq!(pic.frame(pic.nf() + 7), pic.frame(pic.nf() - 1));
    }
}

/// Not a check: prints the time of a frame and of each stage (the time to reach a stage less the time to reach
/// the one before), best of three, one thread. Run it in a release build:
/// `cargo test --release -p bf-picture --test ring_stages timings -- --ignored --nocapture`.
#[test]
#[ignore = "a measurement, not a check"]
fn timings() {
    let best = |job: &dyn Fn()| -> f64 {
        (0..3)
            .map(|_| {
                let t0 = std::time::Instant::now();
                job();
                t0.elapsed().as_secs_f64() * 1e3
            })
            .fold(f64::INFINITY, f64::min)
    };
    for (what, pic) in [
        ("the default looks", picture()),
        ("the looks' lights", picture_with_lights()),
    ] {
        pic.frame(600); // (the sprites and the lens's tables are made on first use)
        for f in [20, 150, 600, 1500, 1520, 1700] {
            let whole = best(&|| drop(pic.frame(f)));
            let mut line = format!("{what}, frame {f}: {whole:.1} ms;");
            let mut before = 0.0;
            for stage in Stage::ALL {
                let upto = best(&|| drop(pic.stage_buffers(f, stage)));
                line += &format!(" {} {:.1}", stage.name(), (upto - before).max(0.0));
                before = upto;
            }
            println!("{line}");
        }
    }
    let t0 = std::time::Instant::now();
    let pic = build(|_| Box::new(DefaultLook));
    println!("Picture::new: {:.1} ms", t0.elapsed().as_secs_f64() * 1e3);
    let t0 = std::time::Instant::now();
    let mut n = 0;
    for f in (0..pic.nf()).step_by(10) {
        drop(pic.frame(f));
        n += 1;
    }
    println!(
        "every 10th frame, one thread: {:.1} ms a frame over {n} frames",
        t0.elapsed().as_secs_f64() * 1e3 / n as f64
    );
}

/// Other fights render too: an early and a late K.O., another seed, and the fighters in swapped corners (every
/// 61st frame and the frames round the K.O.), with the bare default look and with the stand-in lights.
#[test]
fn other_fights_render() {
    let path = repo_root().join("studio/fights/flute_vs_trombone.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    for (file, swapped) in [
        ("seed1", false),
        ("seed500", false),
        ("scenarios/early_ko_s1", false),
        ("scenarios/swapped_s1", true),
    ] {
        let rec = Recording::read(&reference(&format!("fights/flute_vs_trombone/{file}.bfrec"))).expect("it reads");
        let mut ff = FightFile::parse(&text).expect("the fight file parses");
        if swapped {
            ff.fighters.swap(0, 1);
        }
        let rec = Arc::new(rec);
        let makes: [MakeLook; 2] = [
            |_| Box::new(DefaultLook),
            |name| match name {
                "FLUTE" => Box::new(FluteLights::default()),
                _ => Box::new(TromboneLights::default()),
            },
        ];
        for make in makes {
            let show = Show::new(&rec, &ff).unwrap_or_else(|e| panic!("{file}: {e}"));
            let ko = (show.t_ko * 30.0) as usize;
            let cfg = [fighter(&show.names[0]), fighter(&show.names[1])];
            let meshes = [mesh(&cfg[0].pp), mesh(&cfg[1].pp)];
            let looks = [make(&show.names[0]), make(&show.names[1])];
            let pic = Picture::new(rec.clone(), &ff, show, [&cfg[0], &cfg[1]], meshes, looks)
                .unwrap_or_else(|e| panic!("{file}: {e}"));
            let frames: Vec<usize> = (0..pic.nf()).step_by(61).chain(ko - 1..ko + 14).collect();
            let next = AtomicUsize::new(0);
            std::thread::scope(|s| {
                for _ in 0..8 {
                    s.spawn(|| {
                        while let Some(&f) = frames.get(next.fetch_add(1, Ordering::Relaxed)) {
                            let (img, _, report) = pic.frame_reported(f);
                            assert_eq!((img.w, img.h, img.c), (360, 640, 3), "{file}, frame {f}");
                            assert_eq!((report.hull_fallbacks, report.hull_errors), (0, 0), "{file}, frame {f}");
                            assert!(
                                pic.focus_points(f).iter().all(|q| q.0 == 0.0),
                                "{file}, frame {f}: focus"
                            );
                        }
                    });
                }
            });
        }
    }
}
