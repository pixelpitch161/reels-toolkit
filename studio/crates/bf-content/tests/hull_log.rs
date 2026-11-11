//! The hull log (`bf_picture::hull::log_calls`, `take_log`, `logged`) against the kit's: every
//! `scipy.spatial.ConvexHull` call `kit.picture.render(f)` makes for the episode with both real looks, noted by the
//! export tool's `hull_log` (`studio/reference/picture_title_optins/hull_beam.json`). With FLUTE's and TROMBONE's
//! looks the only hull is the ring's spot beam, one a frame; a look port's test compares its own hulls the same
//! way (Template F: "the hull log has no fallback and equals the kit's call for call").

use std::sync::Arc;

use bf_content::build_picture;
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_picture::hull::{self, HullCall};
use bf_picture::ring::Picture;
use bf_testdata::{hex_arg, reference};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn picture() -> Picture {
    let ff = FightFile::parse(include_str!("../../../fights/flute_vs_trombone.toml")).expect("the episode");
    let rec = Recording::read(&reference("fights/flute_vs_trombone/ep392.bfrec")).expect("ep392");
    build_picture(Arc::new(rec), &ff).expect("the picture")
}

fn index() -> Value {
    let text = std::fs::read_to_string(reference("picture_title_optins/hull_beam.json"))
        .expect("hull_beam.json: run the export hull_log");
    serde_json::from_str(&text).expect("JSON")
}

fn hex(d: &[u8]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// The largest distance between a point of ours and the kit's, in pixels; `None` when the counts differ.
fn largest_gap(ours: &[(f64, f64)], kit: &Value) -> Option<f64> {
    let kit = kit.as_array()?;
    (ours.len() == kit.len()).then(|| {
        ours.iter()
            .zip(kit)
            .map(|(&(x, y), k)| (x - hex_arg(&k[0])).abs().max((y - hex_arg(&k[1])).abs()))
            .fold(0.0, f64::max)
    })
}

#[test]
fn the_hull_log_is_the_kits_call_for_call() {
    let doc = index();
    assert_eq!(doc["tool"], "hull_log");
    let pic = picture();
    let rows = doc["frames"].as_array().expect("frames");
    assert_eq!(rows.len(), 13);
    let (mut frames, mut calls, mut exact, mut worst) = (0, 0, 0, 0.0f64);
    for row in rows {
        let f = row["frame"].as_u64().expect("frame") as usize;
        let ((img, _, report), log): ((_, _, _), Vec<HullCall>) = hull::logged(|| pic.frame_reported(f));
        assert_eq!(
            hex(&Sha256::digest(&img.data)),
            row["sha256"],
            "frame {f}: the kit's picture"
        );
        assert_eq!((report.hull_fallbacks, report.hull_errors), (0, 0), "frame {f}");
        let want = row["calls"].as_array().expect("calls");
        assert_eq!(log.len(), want.len(), "frame {f}: the number of hull calls");
        for ((pts, h), w) in log.iter().zip(want) {
            let gap = largest_gap(pts, &w["points"]).unwrap_or_else(|| panic!("frame {f}: the point counts differ"));
            // the screen points come from the camera's maths (dmath here, the C runtime in Python): equal to
            // rounding, as the renderer kit's notes say
            assert!(gap < 1e-9, "frame {f}: a point {gap} px from the kit's");
            if gap == 0.0 {
                exact += 1;
            }
            worst = worst.max(gap);
            let vertices: Vec<u64> = h.vertices.iter().map(|&v| v as u64).collect();
            assert_eq!(
                Value::from(vertices),
                w["vertices"],
                "frame {f}: the hull's corners in scipy's order"
            );
            assert_eq!(h.fallback, None, "frame {f}: a fallback");
            calls += 1;
        }
        frames += 1;
    }
    println!(
        "hull log: {frames} frames, {calls} calls equal to the kit's ({exact} with every point to the bit, the \
         largest gap {worst:e} px), 0 fallbacks"
    );
    assert_eq!((frames, calls), (13, 13));
}

#[test]
fn the_log_is_the_drawing_threads_own() {
    let pic = picture();
    // off: nothing is kept
    hull::log_calls(false);
    let _ = pic.frame(900);
    assert!(hull::take_log().is_empty());
    // two frames on two threads at once, each logging: each thread holds its own frame's one call
    let (a, b) = std::thread::scope(|s| {
        let one = s.spawn(|| {
            hull::log_calls(true);
            let _ = pic.frame(300);
            let calls = hull::take_log();
            hull::log_calls(false);
            calls
        });
        let two = s.spawn(|| {
            hull::log_calls(true);
            let _ = pic.frame(1200);
            let calls = hull::take_log();
            hull::log_calls(false);
            calls
        });
        (one.join().expect("thread one"), two.join().expect("thread two"))
    });
    assert_eq!((a.len(), b.len()), (1, 1));
    let (_, alone_a) = hull::logged(|| pic.frame(300));
    let (_, alone_b) = hull::logged(|| pic.frame(1200));
    assert_eq!((a, b), (alone_a, alone_b));
    // a thread that never turned its log on keeps nothing, while another thread logs
    std::thread::scope(|s| {
        s.spawn(|| {
            hull::log_calls(true);
            let other = s.spawn(|| {
                let _ = pic.frame(600);
                hull::take_log()
            });
            assert!(other.join().expect("the other thread").is_empty());
            assert!(hull::take_log().is_empty(), "nothing was drawn on this thread");
            hull::log_calls(false);
        });
    });
}
