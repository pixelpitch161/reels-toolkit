//! Unit tests of the title card. The numbers marked "Python" were printed by `projects/ballfight/title.py` on the
//! reference PC for the example episode's timeline (132 BPM); the whole cards are compared in
//! `tests/title_frames.rs`.

use bf_core::grid::Grid;
use bf_core::show::Show;
use bf_testdata::{hex_arg, parse_hex_f64, reference};

use super::layout::{self, Panel};
use super::{CardLook, GLASS, Look, OPEN_PX, TitleCard, fx, panels, slam_k, words};
use crate::config::{H, W};
use crate::img::{Img, Rgb8, RgbF};
use crate::np;
use crate::sprite::Mesh;
use crate::ui::DrawLog;

/// A show with these names at this tempo; the card reads nothing else of it.
fn show(a: &str, b: &str, bpm: f64) -> Show {
    Show {
        grid: Grid::new(bpm),
        names: [a.to_string(), b.to_string()],
        win: 0,
        lose: 1,
        t_ko: 50.0,
        t_hs: 50.35,
        t_solo: 52.0,
        caps_ability: [Vec::new(), Vec::new()],
        cap_ko: Vec::new(),
        cap_wins: Vec::new(),
        cap_comment: Vec::new(),
        captions: Vec::new(),
    }
}

/// A look with every default.
struct Plain;

impl Look for Plain {}

const FLUTE: ([f64; 3], [f64; 3]) = ([222.0, 232.0, 246.0], [120.0, 134.0, 162.0]);
const TROMBONE: ([f64; 3], [f64; 3]) = ([120.0, 240.0, 184.0], [44.0, 150.0, 104.0]);

/// A card with plain looks and a model without faces (an empty close-up) in each ball.
fn card(a: &str, b: &str, bpm: f64, tag: Option<&str>) -> TitleCard {
    let mesh = Mesh { faces: Vec::new() };
    let looks = [0, 1].map(|_| CardLook {
        look: &Plain,
        mesh: &mesh,
        yaw: 0.0,
        roll: 0.0,
    });
    TitleCard::new(
        &show(a, b, bpm),
        [FLUTE.0, TROMBONE.0],
        [FLUTE.1, TROMBONE.1],
        looks,
        tag,
    )
    .expect("a card of these names")
}

fn grey() -> Rgb8 {
    Img::filled(W, H, &[128, 128, 128])
}

/// Within a few units in the last place: for values that pass through a sine or an exponential.
#[track_caller]
fn near(got: f64, want: f64) {
    assert!(
        (got - want).abs() <= 4.0 * f64::EPSILON * want.abs().max(1.0),
        "got {got:?}, Python gave {want:?}"
    );
}

// ---------------------------------------------------------------- layout
#[test]
fn the_divider_is_pythons() {
    // Python: title.DD, title.N
    let (dd, n) = (layout::dd(), layout::normal());
    assert_eq!(dd[0].to_bits(), parse_hex_f64("0x1.89544cafec8b7p-1").to_bits());
    assert_eq!(dd[1].to_bits(), parse_hex_f64("-0x1.47c63fe7efc98p-1").to_bits());
    assert_eq!(n, [dd[1], -dd[0]]);
    // Python: title.SIDE[y, x]
    let side = layout::side();
    assert_eq!((side.w, side.h, side.c), (W, H, 1));
    for (x, y, want) in [
        (0, 0, "0x1.691062657e240p+8"),
        (359, 639, "-0x1.67a7d51f3235dp+8"),
        (0, 470, "-0x0.0p+0"),
        (359, 170, "0x1.47c63fe7efd00p-1"),
        (180, 320, "0x0.0p+0"),
        (200, 100, "0x1.3868f4e9108c1p+7"),
    ] {
        assert_eq!(
            side.px(x, y)[0].to_bits(),
            parse_hex_f64(want).to_bits(),
            "SIDE[{y}, {x}]"
        );
    }
}

#[test]
fn names_take_the_biggest_scale_that_fits() {
    // Python: title._layout() for these names: (scale, cx, cy) of the left name and of the right name.
    type Spot = (u32, i32, i32);
    let cases: [(&str, &str, Spot, Spot); 9] = [
        ("FLUTE", "TROMBONE", (7, 180, 138), (5, 170, 468)),
        ("TROMBONE", "FLUTE", (6, 180, 134), (7, 180, 468)),
        // the two the Python's own docstring names
        ("BAGPIPES", "ACCORDION", (6, 180, 134), (5, 160, 468)),
        ("BASS DRUM", "SOUSAPHONE", (5, 180, 134), (4, 180, 468)),
        ("BASS CLARINET", "BASS CLARINET X", (4, 180, 134), (3, 160, 468)),
        ("OX", "ALTO SAX", (7, 180, 138), (5, 170, 468)),
        ("", "TENOR SAX", (7, 180, 138), (5, 160, 468)),
        ("PIANO", "CONTRABASS", (7, 180, 138), (4, 180, 468)),
        // names no scale fits are drawn at 2, the right one moved left
        (
            "A VERY LONG FIGHTER NAME INDEED",
            "ANOTHER VERY LONG FIGHTER NAME",
            (2, 180, 134),
            (2, 110, 468),
        ),
    ];
    for (a, b, left, right) in cases {
        let w = layout::words(a, b);
        assert_eq!((w[0].text.as_str(), w[0].panel), (a, Panel::Left));
        assert_eq!(
            (w[0].scale, w[0].cx, w[0].cy),
            (left.0, f64::from(left.1), f64::from(left.2)),
            "{a}"
        );
        assert_eq!(
            (w[1].text.as_str(), w[1].scale, w[1].cx, w[1].cy, w[1].panel),
            ("VS", 12, 180.0, 300.0, Panel::Both)
        );
        assert_eq!((w[2].text.as_str(), w[2].panel), (b, Panel::Right));
        assert_eq!(
            (w[2].scale, w[2].cx, w[2].cy),
            (right.0, f64::from(right.1), f64::from(right.2)),
            "{b}"
        );
    }
    assert_eq!(layout::fit_scale("TROMBONE", 280), 5);
    assert_eq!(layout::fit_scale("TROMBONE", 320), 6);
    assert_eq!([2, 3, 4, 5, 6, 7, 12].map(layout::outline), [2, 2, 2, 3, 4, 4, 7]);
    assert!(Panel::Both.is_on(0) && Panel::Both.is_on(1));
    assert!(Panel::Left.is_on(0) && !Panel::Left.is_on(1) && Panel::Right.is_on(1) && !Panel::Right.is_on(0));
}

// ---------------------------------------------------------------- timing
#[test]
fn a_word_slams_in_from_two_and_a_quarter() {
    // Python: title.slam_k(d, 0.0)
    assert_eq!(slam_k(-0.01, 0.0), None);
    assert_eq!(slam_k(0.0, 0.0), Some(2.25));
    assert_eq!(slam_k(0.05, 0.0), Some(1.5908203125));
    assert_eq!(slam_k(0.1, 0.0), Some(1.17578125));
    assert_eq!(slam_k(0.16, 0.0), Some(1.0));
    near(slam_k(0.2, 0.0).unwrap(), 0.9227259338968745);
    near(slam_k(0.25, 0.0).unwrap(), 0.9843927742387097);
    assert_eq!(slam_k(0.256, 0.0), Some(1.0));
    assert_eq!(slam_k(0.3, 0.0), Some(1.0));
}

#[test]
fn the_doors_open_once_and_shut_on_the_last_frame() {
    let c = card("FLUTE", "TROMBONE", 132.0, None);
    // Python: title.door_offset(t); where it returns None there is no card, and this gives OPEN_PX.
    for (t, want) in [
        (0.0, Some(0.0)),
        (1.0, Some(0.0)),
        (1.4455882352941176, Some(0.0)),
        (1.5, Some(2.791372800364669)),
        (1.6, Some(63.7941077137021)),
        (1.7, Some(285.3323278342)),
        (1.7641176470588233, None),
        (1.77, None),
        (30.0, None),
        (60.69, None),
        (60.7, Some(560.0)),
        (60.8, Some(136.71875000001398)),
        (60.9, Some(8.750000000001679)),
        (1829.0 / 30.0, Some(0.0)),
        (61.0, Some(0.0)),
    ] {
        match want {
            Some(want) => near(c.door_offset(t), want),
            None => assert_eq!(c.door_offset(t), OPEN_PX, "{t}"),
        }
    }
    // exactly shut where Python is exactly shut
    assert_eq!(c.door_offset(1.4455882352941176), 0.0);
    assert_eq!(c.door_offset(1829.0 / 30.0), 0.0);
    // the doors only ever move outward while they open
    let mut last = 0.0;
    for f in 0..=53 {
        let off = c.door_offset(f64::from(f) / 30.0);
        assert!(off >= last && off <= OPEN_PX, "frame {f}: {off}");
        last = off;
    }
    assert_eq!(last, OPEN_PX);
}

#[test]
fn words_come_one_a_beat_and_name_a_returns_as_a_ghost() {
    let c = card("FLUTE", "TROMBONE", 132.0, None);
    // Python: title.word_state(t, j)
    assert_eq!(c.word_state(0.0, 0), Some((2.25, 1.0)));
    assert_eq!(c.word_state(0.0, 1), None);
    assert_eq!(c.word_state(0.1, 0), Some((1.17578125, 1.0)));
    for (t, j, k) in [
        (0.5, 1, 1.5751285750973179),
        // VS swells on beat 4 (from 0.07 s before it: beat 4 is at 1.3456 s), until 0.05 s after the doors
        // begin to open (at 1.4456 s)
        (1.28, 1, 1.0215174519863213),
        (1.3, 1, 1.0338341290656499),
        (1.4, 1, 1.0317721980707044),
        (1.4333333333333333, 1, 1.02406633209108),
        (1.49, 1, 1.0150081236756634),
        (1.0, 2, 1.158970182742214),
    ] {
        let (got, a) = c.word_state(t, j).unwrap();
        near(got, k);
        assert_eq!(a, 1.0);
    }
    assert_eq!(c.word_state(1.27, 1), Some((1.0, 1.0)));
    assert_eq!(c.word_state(1.5, 1), Some((1.0, 1.0)));
    assert_eq!(c.word_state(30.0, 0), Some((1.0, 1.0)));
    // every word stands until the doors begin to shut, 0.3 s before the end
    assert_eq!(c.word_state(60.69, 0), Some((1.0, 1.0)));
    assert_eq!(c.word_state(60.69, 1), Some((1.0, 1.0)));
    // the closing doors carry no words, then name A falls in over the last 0.12 s: on the last frame at frame
    // 0's size, its opacity a hair under 1 (Python's arithmetic)
    assert_eq!(c.word_state(60.7, 0), None);
    assert_eq!(c.word_state(60.75, 0), None);
    assert_eq!(c.word_state(60.75, 1), None);
    assert_eq!(c.word_state(60.875, 0), None);
    assert_eq!(c.word_state(60.885, 0), Some((2.815384615384647, 0.05769230769225522)));
    assert_eq!(c.word_state(60.89, 0), Some((2.7807692307692444, 0.11538461538459242)));
    assert_eq!(c.word_state(60.89, 1), None);
    assert_eq!(c.word_state(60.9, 2), None);
    assert_eq!(c.word_state(60.9, 0), Some((2.711538461538489, 0.23076923076918485)));
    assert_eq!(
        c.word_state(1828.0 / 30.0, 0),
        Some((2.4807692307692712, 0.6153846153845476))
    );
    assert_eq!(c.word_state(1829.0 / 30.0, 0), Some((2.25, 0.9999999999999923)));
    assert_eq!(c.word_state(1829.0 / 30.0, 1), None);
}

#[test]
fn the_card_shakes_after_a_slam_but_not_on_it() {
    let c = card("FLUTE", "TROMBONE", 132.0, None);
    // Python: title.shake(t)
    assert_eq!(c.shake(0.0), (0.0, 0.0));
    assert_eq!(c.shake(1.0 / 30.0), (0.0, 0.0));
    assert_eq!(c.shake(1.4), (0.0, 0.0));
    for (t, x, y) in [
        (2.0 / 30.0, -3.4334801836581628, 1.7176460464799606),
        (0.5, 2.8459136817319983, -3.6775709951513353),
        (0.52, -3.1958464293929034, 3.192994753070411),
        (1.0, -1.3675867687180963, 1.4515838528792324),
    ] {
        let (sx, sy) = c.shake(t);
        near(sx, x);
        near(sy, y);
    }
}

#[test]
fn the_cover_frame_is_shut_and_still_at_every_tempo() {
    // Python: `ceil(T_DOORS[0] * 30) - 1` stepped back while `title.shake(f / 30) != (0, 0)`, on the kit's
    // timeline for every whole tempo from 60 to 198 (neighbouring tempos snap to the same number of bars).
    #[rustfmt::skip]
    let python: [usize; 139] = [
        94, 88, 88, 88, 88, 83, 83, 83, 83, 79, 79, 79, 79, 75, 75, 75, 75, 71, 71, 71, 71, 68, 68, 68, 68, 65, 65,
        65, 65, 62, 62, 62, 62, 60, 60, 60, 60, 57, 57, 57, 57, 55, 55, 55, 55, 53, 53, 53, 53, 52, 52, 52, 52, 50,
        50, 50, 50, 48, 48, 48, 48, 47, 47, 47, 45, 45, 45, 45, 44, 44, 44, 44, 43, 43, 43, 43, 42, 42, 42, 42, 41,
        41, 41, 41, 40, 40, 40, 40, 39, 39, 39, 39, 38, 38, 38, 38, 37, 37, 37, 37, 36, 36, 36, 36, 35, 35, 35, 35,
        34, 34, 34, 34, 34, 34, 34, 34, 33, 33, 33, 33, 32, 32, 32, 32, 32, 32, 32, 31, 31, 31, 31, 31, 31, 31, 31,
        30, 30, 30, 30,
    ];
    for (bpm, want) in (60..=198).zip(python) {
        let g = Grid::new(f64::from(bpm));
        let f = super::cover_frame(&g);
        assert_eq!(f, want, "{bpm} BPM");
        let t = f as f64 / 30.0;
        assert_eq!(super::door_offset(&g, t), 0.0, "{bpm} BPM, frame {f}");
        assert_eq!(super::shake(&g, t), (0.0, 0.0), "{bpm} BPM, frame {f}");
        // the hold: the right name landed 0.26 s ago or more, and on the next frame the doors move
        assert!(t >= g.title[2] + 0.26 && t < g.t_doors.0, "{bpm} BPM, frame {f}");
        assert!(super::door_offset(&g, (f + 1) as f64 / 30.0) > 0.0, "{bpm} BPM");
    }
    // Up to 198 BPM the rule is the one before ruling R36 (no bound; the doors shut and no shake), which the
    // Python list above is.
    let before_r36 = |g: &Grid| {
        let still = |f: usize| {
            let t = f as f64 / 30.0;
            super::door_offset(g, t) == 0.0 && super::shake(g, t) == (0.0, 0.0)
        };
        let mut f = (g.t_doors.0 * 30.0).ceil() as usize - 1;
        while f > 0 && !still(f) {
            f -= 1;
        }
        f
    };
    for bpm in 30..=198 {
        let g = Grid::new(f64::from(bpm));
        assert_eq!(super::cover_frame(&g), before_r36(&g), "{bpm} BPM");
    }
    // At every tempo a fight file may ask for, the cover has the doors shut and all three words settled: the
    // names at rest, VS opaque and within the 6 % the card files a settled word at (it swells on beat 4).
    let mut fast = Vec::new();
    for bpm in 30..=300 {
        let g = Grid::new(f64::from(bpm));
        let f = super::cover_frame(&g);
        let t = f as f64 / 30.0;
        assert_eq!(super::door_offset(&g, t), 0.0, "{bpm} BPM, frame {f}");
        assert_eq!(super::word_state(&g, t, 0), Some((1.0, 1.0)), "{bpm} BPM, frame {f}");
        assert_eq!(super::word_state(&g, t, 2), Some((1.0, 1.0)), "{bpm} BPM, frame {f}");
        let (k, a) = super::word_state(&g, t, 1).expect("VS is up");
        assert!((k - 1.0).abs() < 0.06 && a == 1.0, "{bpm} BPM, frame {f}: VS at {k}");
        if bpm <= 198 {
            assert!(t >= g.title[2] + 0.26 && t < g.t_doors.0, "{bpm} BPM");
        } else {
            // from 199 BPM (200.7 once snapped to whole bars) no frame after the right name settles is still:
            // the last shut frame, shaking
            assert_eq!(f, (g.t_doors.0 * 30.0).ceil() as usize - 1, "{bpm} BPM");
            fast.push(super::shake(&g, t) != (0.0, 0.0));
        }
    }
    assert_eq!((fast.len(), fast.iter().filter(|&&s| s).count()), (102, 102));
    let c = card("FLUTE", "TROMBONE", 132.0, None);
    assert_eq!((c.cover_frame(), super::cover_frame(&Grid::new(132.0))), (43, 43));
}

// ---------------------------------------------------------------- panels
#[test]
fn the_plain_panel_and_the_slide_tubes_are_pythons() {
    // Python: title._panel_rgb(COL[0], DARK[0], 0, None) with the FLUTE's colours (no pattern), and
    // title._panel_rgb(COL[1], DARK[1], 1, 'TROMBONE'): [y, x] values, the sum of all values, the bands used.
    let plain = panels::panel_rgb(FLUTE.0, FLUTE.1, |_| {});
    let tubes = panels::panel_rgb(TROMBONE.0, TROMBONE.1, |base| {
        panels::builtin_pattern("TROMBONE").unwrap().unwrap()(base, &layout::Layout::plain("A", "B"))
    });
    for (x, y, p, t) in [
        (0, 0, [102.0, 127.5, 153.0], [51.0, 153.0, 102.0]),
        (1, 1, [127.5, 127.5, 153.0], [51.0, 178.5, 127.5]),
        (100, 200, [153.0, 153.0, 178.5], [51.0, 178.5, 127.5]),
        (180, 320, [204.0, 229.5, 255.0], [102.0, 229.5, 178.5]),
        (359, 639, [127.5, 127.5, 153.0], [51.0, 178.5, 127.5]),
        (3, 470, [204.0, 229.5, 255.0], [102.0, 255.0, 178.5]),
        (77, 333, [178.5, 204.0, 229.5], [76.5, 204.0, 153.0]),
    ] {
        assert_eq!(plain.px(x, y), &p, "plain[{y}, {x}]");
        assert_eq!(tubes.px(x, y), &t, "tubes[{y}, {x}]");
    }
    // Every value is a multiple of 25.5 = 51 / 2, so the sums are exact whatever the order.
    assert_eq!(plain.data.iter().sum::<f64>(), 127477203.0);
    assert_eq!(tubes.data.iter().sum::<f64>(), 95500560.0);
    let mut bands: Vec<i64> = plain.data.iter().map(|v| (v / 25.5).round() as i64).collect();
    bands.sort_unstable();
    bands.dedup();
    assert_eq!(bands, [4, 5, 6, 7, 8, 9, 10]);
    assert!(plain.data.iter().all(|v| (v / 25.5).fract() == 0.0));
}

/// A look whose pattern blacks the panel out.
struct Blackout;

impl Look for Blackout {
    fn title_pattern(&self, base: &mut RgbF) {
        base.data.fill(0.0);
    }
}

#[test]
fn the_cards_own_pattern_wins_over_the_looks() {
    let mesh = Mesh { faces: Vec::new() };
    let both = |a: &str, b: &str| {
        let looks = [0, 1].map(|_| CardLook {
            look: &Blackout,
            mesh: &mesh,
            yaw: 0.0,
            roll: 0.0,
        });
        TitleCard::new(
            &show(a, b, 132.0),
            [FLUTE.0, TROMBONE.0],
            [FLUTE.1, TROMBONE.1],
            looks,
            None,
        )
        .expect("both patterns are ported or absent")
    };
    // PATTERNS.setdefault(name, look.title_pattern): TROMBONE is in the card's table already, TUBA is not.
    let c = both("TUBA", "TROMBONE");
    let tubes = panels::panel_rgb(TROMBONE.0, TROMBONE.1, |b| panels::slide_tubes(b, &c.layout));
    // far from the divider's bright edge the panels are as built
    assert_eq!(c.panels[0].px(10, 10), &[0.0, 0.0, 0.0]);
    assert_eq!(c.panels[1].px(350, 630), tubes.px(350, 630));
    assert!(matches!(panels::builtin_pattern("TROMBONE"), Ok(Some(_))));
    assert!(
        matches!(panels::builtin_pattern("TUBA"), Ok(None)) && matches!(panels::builtin_pattern("FLUTE"), Ok(None))
    );
    // the divider's edge is bright on both panels (np.clip(COL * 0.5 + 150, 0, 255)) and opaque; each panel is
    // 94 % opaque on its own half only
    assert_eq!(c.panels[0].px(180, 320), &[255.0, 255.0, 255.0]);
    assert_eq!(c.panels[1].px(180, 320), &[210.0, 255.0, 242.0]);
    assert_eq!((c.alpha[0].px(180, 320)[0], c.alpha[1].px(180, 320)[0]), (1.0, 1.0));
    assert_eq!((c.alpha[0].px(10, 10)[0], c.alpha[1].px(10, 10)[0]), (0.94, 0.0));
    assert_eq!((c.alpha[0].px(350, 630)[0], c.alpha[1].px(350, 630)[0]), (0.0, 0.94));
}

#[test]
fn the_defaults_of_a_look() {
    let mut base: RgbF = Img::filled(2, 2, &[1.0, 2.0, 3.0]);
    Plain.title_pattern(&mut base);
    assert_eq!(base, Img::filled(2, 2, &[1.0, 2.0, 3.0]));
    assert!(Plain.title_closeup(44).is_none());
    assert_eq!((Plain.glass(), GLASS), ([236.0, 232.0, 226.0], [236.0, 232.0, 226.0]));
    // the card calls title_pattern_with, whose default is the look's title_pattern
    let mut black: RgbF = Img::filled(2, 2, &[1.0, 2.0, 3.0]);
    Blackout.title_pattern_with(&mut black, &layout::Layout::plain("A", "B"));
    assert_eq!(black, Img::filled(2, 2, &[0.0, 0.0, 0.0]));
}

/// A look whose pattern reads the card's balls, as TUBA's does (`title.BALLS[k]`): it blacks out the left ball's
/// centre.
struct BallMark;

impl Look for BallMark {
    fn title_pattern_with(&self, base: &mut RgbF, card: &layout::Layout) {
        let b = card.balls[0];
        base.px_mut(b.cx as usize, b.cy as usize)
            .copy_from_slice(&[0.0, 0.0, 0.0]);
    }
}

#[test]
fn a_looks_pattern_sees_the_cards_balls() {
    let mesh = Mesh { faces: Vec::new() };
    let looks = [0, 1].map(|_| CardLook {
        look: &BallMark,
        mesh: &mesh,
        yaw: 0.0,
        roll: 0.0,
    });
    let mut lay = layout::Layout::plain("TUBA", "PIANO");
    lay.balls[0].cx = 100.0;
    lay.balls[0].cy = 120.0;
    let s = show("TUBA", "PIANO", 132.0);
    let c = TitleCard::with_layout(&s, [FLUTE.0, TROMBONE.0], [FLUTE.1, TROMBONE.1], looks, None, lay)
        .expect("no card patterns for these names");
    assert_eq!(c.layout().balls[0].cx, 100.0);
    // the black mark survives the dither (the lowest band), unlike the gradient around it
    assert_eq!(c.panels[0].px(100, 120), &[0.0, 0.0, 0.0]);
    assert_ne!(c.panels[0].px(101, 120), &[0.0, 0.0, 0.0]);
}

/// A look whose close-up is not square.
struct Oblong;

impl Look for Oblong {
    fn title_closeup(&self, r: u32) -> Option<Img<u8>> {
        Some(Img::new(2 * r as usize, 2 * r as usize + 1, 4))
    }
}

#[test]
#[should_panic(expected = "the card wants a square RGBA sprite")]
fn a_close_up_must_be_square() {
    let mesh = Mesh { faces: Vec::new() };
    let looks = [0, 1].map(|_| CardLook {
        look: &Oblong,
        mesh: &mesh,
        yaw: 0.0,
        roll: 0.0,
    });
    let _ = TitleCard::new(
        &show("A", "B", 132.0),
        [FLUTE.0, TROMBONE.0],
        [FLUTE.1, TROMBONE.1],
        looks,
        None,
    );
}

#[test]
fn a_pattern_not_ported_yet_is_an_error() {
    // Python: sorted(title.PATTERNS): the card's own sixteen
    let python = [
        "ACCORDION",
        "ACOUSTIC",
        "BAGPIPES",
        "BARI SAX",
        "BASS CLARINET",
        "BASSOON",
        "CLARINET",
        "EUPHONIUM",
        "HARP",
        "HORN",
        "OBOE",
        "ORGAN",
        "TROMBONE",
        "UKULELE",
        "VIOLA",
        "VIOLIN",
    ];
    let mut all = super::patterns::names().to_vec();
    all.sort_unstable();
    assert_eq!(all, python);
    // each name is refused exactly when its file says it is not ported; TROMBONE's (milestone 1) is
    let unported = super::patterns::unported();
    assert!(!unported.contains(&"TROMBONE"));
    for name in super::patterns::names() {
        let refused = unported.contains(&name);
        match panels::builtin_pattern(name) {
            Ok(Some(_)) => assert!(!refused, "{name}"),
            Ok(None) => panic!("{name} is in the table and has no pattern"),
            Err(err) => {
                assert!(refused, "{name}");
                assert_eq!(err.to_string(), format!("the title pattern {name} is not ported yet"));
            }
        }
    }
    // the error's words, for a made-up name
    assert_eq!(
        super::TitleError::PatternNotPorted("KAZOO".into()).to_string(),
        "the title pattern KAZOO is not ported yet"
    );
    // the card's constructor says so, on either side, whatever the look offers (for a pattern still unported,
    // found at run time; once all sixteen are ported there is none to refuse)
    let mesh = Mesh { faces: Vec::new() };
    let looks = [0, 1].map(|_| CardLook {
        look: &Blackout,
        mesh: &mesh,
        yaw: 0.0,
        roll: 0.0,
    });
    let new = |a: &str, b: &str| {
        TitleCard::new(
            &show(a, b, 132.0),
            [FLUTE.0, TROMBONE.0],
            [FLUTE.1, TROMBONE.1],
            looks,
            None,
        )
    };
    if let Some(&name) = unported.first() {
        let err = new(name, "TROMBONE").expect_err("not ported");
        assert_eq!(err, super::TitleError::PatternNotPorted(name.into()));
        let err = new("FLUTE", name).expect_err("not ported");
        assert_eq!(err.to_string(), format!("the title pattern {name} is not ported yet"));
    }
    // names the table does not hold are fine: the look's pattern, or a plain panel
    assert!(new("KAZOO", "NOT A FIGHTER").is_ok());
}

#[test]
fn a_ball_is_opaque_to_its_ink_edge_and_stays_in_its_box() {
    let spr: Img<u8> = Img::new(95, 95, 4);
    let mut layer: RgbF = Img::filled(W, H, &[7.0, 7.0, 7.0]);
    let mut alpha: Img<f64> = Img::new(W, H, 1);
    panels::draw_big_ball(&mut layer, &mut alpha, &spr, GLASS, 78.0, 222.5, 56.0, 0.25, FLUTE.0);
    for y in 0..H {
        for x in 0..W {
            let (ex, ey) = (x as f64 + 0.5 - 78.0, y as f64 + 0.5 - 222.5);
            let d = (ex * ex + ey * ey).sqrt();
            let a = alpha.px(x, y)[0];
            if d <= 58.4 {
                assert_eq!(a, 1.0, "({x}, {y})");
            } else if d >= 58.6 {
                assert_eq!((a, layer.px(x, y)), (0.0, &[7.0, 7.0, 7.0][..]), "({x}, {y})");
            }
        }
    }
    // the ink edge, the rim in the fighter's colour, the small highlight up and to the left
    assert_eq!(layer.px(78 + 57, 222), &super::INK);
    let rim = FLUTE.0.map(|c| (c * (0.9 + 0.5 * 0.25) + 50.0 * 0.25).min(255.0));
    assert_eq!(layer.px(78 + 55, 222), &rim);
    assert_eq!(layer.px(78 - 21, 222 - 28), &[255.0, 252.0, 244.0]);
    // a ball off the frame draws nothing
    let (mut l2, mut a2) = (layer.clone(), alpha.clone());
    panels::draw_big_ball(&mut l2, &mut a2, &spr, GLASS, -200.0, 222.5, 56.0, 0.25, FLUTE.0);
    panels::draw_big_ball(&mut l2, &mut a2, &spr, GLASS, 78.0, 900.0, 56.0, 0.25, FLUTE.0);
    assert!(l2 == layer && a2 == alpha);
}

// ---------------------------------------------------------------- words
#[test]
fn word_masks_are_pythons() {
    // Python: title.word_masks(text, scale, k): the masks' (height, width, set pixels), and r.
    type Size = (usize, usize, usize);
    type Row = (&'static str, u32, f64, Size, Size, usize, i32);
    let cases: [Row; 7] = [
        ("VS", 12, 2.25, (189, 297, 20412), (221, 329, 54702), 6561, 16),
        ("VS", 12, 1.0, (84, 132, 4032), (98, 146, 10704), 1296, 7),
        // within a thousandth of 1 the word is not resized; just beyond it is, to the same size here
        ("VS", 12, 1.0005, (84, 132, 4032), (98, 146, 10704), 1296, 7),
        ("VS", 12, 1.002, (84, 132, 4032), (98, 146, 10704), 1296, 7),
        ("FLUTE", 7, 0.93, (46, 189, 2888), (54, 197, 7131), 990, 4),
        ("TROMBONE", 5, 1.5, (52, 352, 7724), (60, 360, 16920), 2486, 4),
        ("A", 2, 0.1, (1, 1, 0), (5, 5, 0), 0, 2),
    ];
    let count = |m: &Img<u8>| m.data.iter().filter(|&&v| v != 0).count();
    for (text, scale, k, m, o, hi, r) in cases {
        let got = words::word_masks(text, scale, k);
        assert_eq!((got.fill.h, got.fill.w, count(&got.fill)), m, "{text} x {k}");
        assert_eq!((got.outline.h, got.outline.w, count(&got.outline)), o, "{text} x {k}");
        assert_eq!((count(&got.hi), got.r), (hi, r), "{text} x {k}");
        assert_eq!((got.hi.w, got.hi.h), (got.fill.w, got.fill.h));
    }
}

#[test]
fn a_stamped_word_is_centred_and_files_its_outline_box() {
    // The settled FLUTE of the example episode; the kit's safe-zone check recorded this box for it.
    let s = words::stamp_word(W, H, "FLUTE", 7, 1.0, 180.0, 138.0, [255.0, 0.0, 0.0], 1.0);
    assert_eq!(s.noted, [(74, 110, 285, 167)]);
    // the layer reaches from the outline's corner to its drop shadow's, 3 px right and 4 px down
    let l = &s.layer;
    assert_eq!((l.x, l.y, l.w, l.h), (74, 110, 211 + 3, 57 + 4));
    let at = |x: usize, y: usize| {
        let i = (y - l.y) * l.w + (x - l.x);
        (&l.rgb[i * 3..i * 3 + 3], l.alpha[i])
    };
    // the F's stem: the highlight on the top two font rows (the colour mixed with white), the colour below
    assert_eq!(at(79, 115), (&[255.0, 255.0 * 0.45, 255.0 * 0.45][..], 1.0));
    assert_eq!(at(79, 130), (&[255.0, 0.0, 0.0][..], 1.0));
    // the outline, 4 px thick; outside it nothing; under its lower right corner the half-ink shadow
    assert_eq!(at(74, 110), (&super::INK[..], 1.0));
    assert_eq!(at(74 + 212, 110 + 30).1, 0.0);
    assert_eq!(at(285 + 2, 167 + 3), (&[8.0, 5.0, 13.0][..], 1.0));
    // at opacity a every stamp mixes into what is there, and the layer's opacity is a
    let half = words::stamp_word(W, H, "FLUTE", 7, 1.0, 180.0, 138.0, [200.0, 100.0, 50.0], 0.5);
    let i = (130 - half.layer.y) * half.layer.w + (79 - half.layer.x);
    assert_eq!(half.layer.alpha[i], 0.5);
    // the shadow, the outline, then the letters: ((0 * 0.5 + 8 * 0.5) * 0.5 + 16 * 0.5) * 0.5 + 200 * 0.5
    assert_eq!(
        half.layer.rgb[i * 3],
        ((8.0 * 0.5) * 0.5 + 16.0 * 0.5) * 0.5 + 200.0 * 0.5
    );
    // a word slammed in at 2.25 is larger than the frame and is cut at its edges
    let big = words::stamp_word(W, H, "TROMBONE", 5, 2.25, 170.0, 468.0, [1.0, 2.0, 3.0], 1.0);
    assert_eq!((big.layer.x, big.layer.w), (0, W));
    assert!(big.layer.y + big.layer.h <= H);
    // and one that is wholly off the frame leaves an empty layer
    let off = words::stamp_word(W, H, "VS", 12, 1.0, 5000.0, 300.0, [1.0, 2.0, 3.0], 1.0);
    assert!(off.layer.rgb.is_empty() && off.layer.alpha.is_empty());
}

// ---------------------------------------------------------------- the crack
/// The crack's points against the ones Python handed to Pillow on frame 41 (the whole crack):
/// `pillow/as_used.json`, the `crack` case of that frame.
#[test]
fn the_crack_is_pythons() {
    let text = std::fs::read_to_string(reference("pillow/as_used.json")).expect("as_used.json");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    let case = doc["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["args"]["kind"] == "crack" && c["args"]["frame"] == 41)
        .expect("the crack of frame 41");
    let ops = case["args"]["ops"].as_array().expect("ops");
    let crack = fx::crack_pts();
    assert_eq!(crack.len(), ops.len());
    assert_eq!(
        crack.iter().map(Vec::len).collect::<Vec<_>>(),
        [4, 4, 4, 4, 4, 18, 4, 4, 4, 17]
    );
    for (line, op) in crack.iter().zip(ops) {
        let xy = op["xy"].as_array().expect("xy");
        assert_eq!(line.len(), xy.len());
        for (p, q) in line.iter().zip(xy) {
            assert_eq!(p.0.to_bits(), hex_arg(&q[0]).to_bits());
            assert_eq!(p.1.to_bits(), hex_arg(&q[1]).to_bits());
        }
    }
    // both lines start at the centre of the divider and end just past the frame's sides
    assert_eq!((crack[5][0], crack[9][0]), ((180.0, 320.0), (180.0, 320.0)));
    assert!(crack[5].last().unwrap().0 > W as f64 && crack[9].last().unwrap().0 < 0.0);
}

#[test]
fn the_crack_grows_from_the_centre_and_cools() {
    let g = Grid::new(132.0);
    let crack = fx::crack_pts();
    let drawn = |t: f64| {
        let mut layer: RgbF = Img::new(W, H, 3);
        let mut alpha: Img<f64> = Img::new(W, H, 1);
        fx::draw_crack(&mut layer, &mut alpha, &g, t, &crack);
        (layer, alpha)
    };
    // nothing before VS slams in
    let (_, a) = drawn(g.title[1] - 0.001);
    assert!(a.data.iter().all(|&v| v == 0.0));
    // on the slam: every polyline's first segment, in gold
    let (l0, a0) = drawn(g.title[1]);
    let n0 = a0.data.iter().filter(|&&v| v == 1.0).count();
    assert!(n0 > 0 && a0.px(180, 320)[0] == 1.0);
    assert_eq!(l0.px(180, 320), &super::GOLD);
    // 0.12 s later the whole crack, cooled by 1 - 1 / e of the way to ink: red 16 + (255 - 16) / e = 103.9
    let (l1, a1) = drawn(g.title[1] + 0.12);
    let n1 = a1.data.iter().filter(|&&v| v == 1.0).count();
    assert!(n1 > 3 * n0);
    let c = l1.px(180, 320);
    assert!((c[0] - 103.92).abs() < 0.01 && c[1] < 226.0 && c[2] < 120.0, "{c:?}");
    assert!(a1.data.iter().all(|&v| v == 0.0 || v == 1.0));
    // and it stays: long after, the same pixels in ink
    let (l2, a2) = drawn(g.title[1] + 5.0);
    assert!(a2 == a1);
    assert!((l2.px(180, 320)[0] - 16.0).abs() < 1e-9);
}

// ---------------------------------------------------------------- the card
#[test]
fn the_series_tag_is_drawn_only_when_given() {
    let t = 41.0 / 30.0;
    let draw = |tag: Option<&str>| {
        let mut img = grey();
        card("FLUTE", "TROMBONE", 132.0, tag).draw(&mut img, t);
        img
    };
    let (none, empty, tagged) = (draw(None), draw(Some("")), draw(Some("BALLFIGHT #15")));
    assert!(none == empty && none != tagged);
    // 13 characters at scale 1 are 77 px wide: columns 142..219 at rows 100..107, and a 1 px outline around
    for y in 0..H {
        for x in 0..W {
            if none.px(x, y) != tagged.px(x, y) {
                assert!((141..220).contains(&x) && (99..108).contains(&y), "({x}, {y})");
            }
        }
    }
    // the B's stem in the tag's pale gold, the outline's corner in ink
    assert_eq!(tagged.px(142, 100), &[255, 214, 96]);
    assert_eq!(tagged.px(141, 99), &[16, 10, 26]);
    // A tag wider than the frame (Python raises for one): the part on the frame is drawn, nothing panics.
    let long = "X".repeat(70);
    let wide = draw(Some(&long));
    // 419 px of text from column -30: an X begins on column 0, and the frame's last column is the gap after one
    assert_eq!(wide.px(0, 100), &[255, 214, 96]);
    assert_eq!(wide.px(358, 106), &[255, 214, 96]);
    assert_eq!(wide.px(359, 106), &[16, 10, 26]);
}

#[test]
fn the_shut_card_covers_the_frame_and_notes_its_settled_words() {
    let c = card("FLUTE", "TROMBONE", 132.0, None);
    let boxes = |log: &DrawLog| -> Vec<(i32, i32, i32, i32, String)> {
        log.boxes
            .iter()
            .map(|b| (b.x0, b.y0, b.x1, b.y1, b.info.clone()))
            .collect()
    };
    // Frame 38: all three words at rest (the kit's safe-zone check recorded these boxes for it).
    let mut img = grey();
    let mut log = DrawLog::new();
    c.draw_logged(&mut img, &mut log, 38.0 / 30.0);
    // every pixel is the dimmed frame under a 94 % panel, or something opaque: none is left as it was
    assert!(img.data.as_chunks::<3>().0.iter().all(|p| p != &[128, 128, 128]));
    let rest = [
        (74, 110, 285, 167, "FLUTE".to_string()),
        (107, 251, 253, 349, "VS".to_string()),
        (49, 447, 290, 488, "TROMBONE".to_string()),
    ];
    assert_eq!(boxes(&log), rest);
    // Frame 41: VS swells on beat 4, by less than 6 %, so it still counts as settled and its box swells with it.
    let mut log = DrawLog::new();
    c.draw_logged(&mut grey(), &mut log, 41.0 / 30.0);
    let swollen = boxes(&log);
    assert_eq!((&swollen[0], &swollen[2]), (&rest[0], &rest[2]));
    assert_eq!(swollen[1], (103, 248, 257, 352, "VS".to_string()));
    // while a word is still slamming in, it is drawn but not noted
    let mut log = DrawLog::new();
    c.draw_logged(&mut grey(), &mut log, 0.0);
    assert!(log.boxes.is_empty());
    // drawing is a pure function of the time, and a card may be shared between the threads that draw frames
    let mut again = grey();
    c.draw(&mut again, 38.0 / 30.0);
    assert!(again == img);
    fn shared<T: Send + Sync>(_: &T) {}
    shared(&c);
}

#[test]
#[should_panic(expected = "the title card is drawn on a 360 x 640 RGB frame")]
fn a_frame_of_another_size_is_refused() {
    let mut small: Rgb8 = Img::new(90, 160, 3);
    card("A", "B", 132.0, None).draw(&mut small, 0.0);
}

// ---------------------------------------------------------------- what the 18 cards cannot tell apart
#[test]
fn a_mix_keeps_pythons_order() {
    // Python: (v * 0.55) * (1 - a) + src * a for a dimmed byte v, where the other form of the same mix,
    // dst + (src - dst) * a, gives the neighbouring double and with it another byte.
    let cases = [
        (94.0, 148.7, 0.9, 138.99999999999997),
        (68.0, 242.2, 0.75, 190.99999999999997),
        (184.0, 107.6, 0.75, 105.99999999999999),
        (226.0, 117.3, 0.9, 117.99999999999999),
        (178.0, 184.7, 0.75, 162.99999999999997),
        (14.0, 144.1, 0.75, 109.99999999999999),
    ];
    for (v, src, a, want) in cases {
        let dst = v * 0.55;
        assert_eq!(super::over(dst, src, a), want, "{v} under {src} at {a}");
        assert_ne!(dst + (src - dst) * a, want, "{v} under {src} at {a}");
    }
    // A door laid over the picture is that mix, moved: 3 px right and 2 px up here.
    let (v, src, a, want) = cases[0];
    let mut out: RgbF = Img::filled(W, H, &[v * 0.55; 3]);
    let layer: RgbF = Img::filled(W, H, &[src; 3]);
    let alpha: Img<f64> = Img::filled(W, H, &[a]);
    super::lay_over(&mut out, &layer, &alpha, 3, -2);
    assert_eq!(out.px(3, 0), &[want; 3]);
    assert_eq!(out.px(359, 637), &[want; 3]);
    // what the move uncovers keeps the picture: the three columns on the left, the two rows at the bottom
    assert_eq!(out.px(2, 0), &[v * 0.55; 3]);
    assert_eq!(out.px(359, 638), &[v * 0.55; 3]);
    // a door moved off the frame changes nothing
    let before = out.clone();
    super::lay_over(&mut out, &layer, &alpha, 360, 0);
    super::lay_over(&mut out, &layer, &alpha, 0, -640);
    assert!(out == before);
}

#[test]
fn the_gradient_before_the_dither_is_pythons() {
    // Python: the array title._panel_rgb hands to PATTERNS[name] (the gradient, not yet dithered) for the FLUTE's
    // and for the TROMBONE's colours, at [y, x].
    //
    // The gradient's `k ** 1.3` is the C runtime's pow in Python, and `dmath::pow` is that to the bit (ruling
    // M3-R38), so every pixel here is Python's to the bit. The pixel (311, 17) is one of the 21,097 of the frame's
    // 230,400 (9 %) where the old pow (musl's) was one unit in the last place away.
    const OTHER_POW: (usize, usize) = (311, 17);
    #[rustfmt::skip]
    let cases: [(usize, usize, [&str; 3], [&str; 3]); 10] = [
        (0, 0, ["0x1.e86ae5017e2b3p+6", "0x1.1010cd381621ap+7", "0x1.478a3f3d3a0b6p+7"],
               ["0x1.6c54c40362833p+5", "0x1.2fc3e9cd4f397p+7", "0x1.a6a009b7a62c3p+6"]),
        (311, 17, ["0x1.3d9f698d7dbd0p+7", "0x1.595da7cec0f32p+7", "0x1.8d567c65497d7p+7"],
                  ["0x1.1623f1331ed77p+6", "0x1.77106a6dea66dp+7", "0x1.0e5a5f525edeap+7"]),
        (100, 200, ["0x1.40d0c702619aep+7", "0x1.5c985c3b3a477p+7", "0x1.908368f0f578bp+7"],
                   ["0x1.1a1cff742f50fp+6", "0x1.7a433eec37882p+7", "0x1.10f14fa59592ap+7"]),
        (180, 320, ["0x1.b09999999999ap+7", "0x1.ce66666666666p+7", "0x1.011999999999ap+8"],
                   ["0x1.a19999999999ap+6", "0x1.ec00000000000p+7", "0x1.6c00000000000p+7"]),
        (359, 639, ["0x1.e8d75c7b92181p+6", "0x1.1045736eb6e1fp+7", "0x1.47b8be23f0553p+7"],
                   ["0x1.6cf121d73118bp+5", "0x1.2ff50bd6456cfp+7", "0x1.a6f5aeee1ecb9p+6"]),
        (3, 470, ["0x1.aeb5eec8fc9b2p+7", "0x1.cc77ecb6c5506p+7", "0x1.00202967f31efp+8"],
                 ["0x1.9f5969370b44ap+6", "0x1.ea0f027bfcb38p+7", "0x1.6a74ec3792c62p+7"]),
        (77, 333, ["0x1.7e1a7e90e88efp+7", "0x1.9ada1ee74bba2p+7", "0x1.ce601dc19c57cp+7"],
                  ["0x1.6512010f3fe5fp+6", "0x1.b84b42d1c2efap+7", "0x1.42cb118a24bd7p+7"]),
        (20, 600, ["0x1.5409e70b7b251p+7", "0x1.701299e089ad1p+7", "0x1.a3bfccfabef3bp+7"],
                  ["0x1.31dd4510cd6eap+6", "0x1.8d9a24caf1432p+7", "0x1.208d5f518c9fap+7"]),
        (250, 50, ["0x1.35ffe0e3af2dbp+7", "0x1.51a9cdafca39dp+7", "0x1.85c83f30d5bbcp+7"],
                  ["0x1.0c9e0259935b2p+6", "0x1.6f720e8bd0cd3p+7", "0x1.082c9793d3aa6p+7"]),
        (199, 411, ["0x1.69a31c0732f5ep+7", "0x1.8603e2b95917cp+7", "0x1.b98f0d95afe4bp+7"],
                   ["0x1.4c446eb87baccp+6", "0x1.a377fb37418cep+7", "0x1.321f16e17f2ecp+7"]),
    ];
    let gradient = |colour: [f64; 3], dark: [f64; 3]| {
        let mut seen = None;
        panels::panel_rgb(colour, dark, |base| seen = Some(base.clone()));
        seen.expect("the pattern is handed the gradient")
    };
    let (flute, trombone) = (gradient(FLUTE.0, FLUTE.1), gradient(TROMBONE.0, TROMBONE.1));
    for (x, y, a, b) in cases {
        for c in 0..3 {
            for (name, got, want) in [("FLUTE", &flute, a[c]), ("TROMBONE", &trombone, b[c])] {
                let (got, want) = (got.px(x, y)[c], parse_hex_f64(want));
                assert_eq!(got.to_bits(), want.to_bits(), "{name} [{y}, {x}, {c}]");
            }
        }
    }
    assert!(cases.iter().any(|c| (c.0, c.1) == OTHER_POW));
    // and there, too, the finished panel is Python's: title._panel_rgb(...)[17, 311] with the FLUTE's colours
    let plain = panels::panel_rgb(FLUTE.0, FLUTE.1, |_| {});
    assert_eq!(plain.px(OTHER_POW.0, OTHER_POW.1), &[153.0, 178.5, 204.0]);
}

#[test]
fn while_the_doors_move_there_are_no_slam_effects_and_no_boxes() {
    let c = card("FLUTE", "TROMBONE", 132.0, None);
    // Frame 44: the doors have just begun to open, every word is at rest, and the right name's dust (0.6 s from
    // the frame after its slam) would still be in the air on a shut card.
    let t = 44.0 / 30.0;
    let off = c.door_offset(t);
    assert!(off > 0.0 && off < 1.0, "{off}");
    assert_eq!(c.word_state(t, 2), Some((1.0, 1.0)));
    let dimmed: RgbF = Img::filled(W, H, &[128.0 * 0.55; 3]);
    let mut dusty = dimmed.clone();
    c.slam_effects(&mut dusty, t, 0.0, 0.0);
    assert!(dusty != dimmed, "an effect would still be running");
    // The card as drawn is the dimmed frame under the two doors and nothing else, and no word is filed, as in
    // Python (`if off == 0.0 and t < T.DUR - 1` for the effects, `settled = ... and off == 0.0` for the boxes).
    let mut log = DrawLog::new();
    let mut want = dimmed.clone();
    let (n, (sx, sy)) = (layout::normal(), c.shake(t));
    for (side, sgn) in [(0, 1.0), (1, -1.0)] {
        let (layer, alpha) = c.door(side, t, off, &mut log);
        let dx = crate::ui::round_i32(n[0] * off * sgn + sx);
        let dy = crate::ui::round_i32(n[1] * off * sgn + sy);
        super::lay_over(&mut want, &layer, &alpha, dx, dy);
    }
    let mut img = grey();
    c.draw_logged(&mut img, &mut log, t);
    assert!(img.data.iter().zip(&want.data).all(|(&p, &v)| p == np::clip_u8(v)));
    assert!(log.boxes.is_empty());
    // nor is anything filed on other frames of the moving doors
    let mut log = DrawLog::new();
    for f in [46, 52, 1822, 1826, 1829] {
        c.draw_logged(&mut grey(), &mut log, f64::from(f) / 30.0);
    }
    assert!(log.boxes.is_empty());
}

#[test]
fn each_effect_has_its_window() {
    // title.py: the light for 0.5 s from the slam, the streaks for 0.25 s and the dust for 0.6 s from the frame
    // after it, the sparks for 0.45 s from the slam of VS.
    let g = Grid::new(132.0);
    let (t0, f) = (g.title[1], 1.0 / 30.0);
    let flat: RgbF = Img::filled(W, H, &[50.0, 60.0, 70.0]);
    let touched = |draw: &dyn Fn(&mut RgbF)| {
        let mut img = flat.clone();
        draw(&mut img);
        img != flat
    };
    let light = |t: f64| touched(&|img| fx::light(img, &g, t, 1, 180.0, 300.0, super::GOLD));
    let streaks = |t: f64| touched(&|img| fx::streaks(img, &g, t, 1, 180.0, 300.0, 144, 84, super::GOLD));
    let dust = |t: f64| touched(&|img| fx::dust(img, &g, t, 1, 180.0, 300.0, 144, 84));
    let sparks = |t: f64| touched(&|img| fx::sparks(img, &g, t));
    // (on the slam frame itself the light is on and adds nothing: it rises from zero)
    assert_eq!(
        [-0.01, 0.0, 0.03, 0.49, 0.51].map(|d| light(t0 + d)),
        [false, false, true, true, false]
    );
    assert_eq!(
        [0.0, f - 0.005, f + 0.005, f + 0.24, f + 0.26].map(|d| streaks(t0 + d)),
        [false, false, true, true, false]
    );
    assert_eq!(
        [0.0, f - 0.005, f + 0.005, f + 0.59, f + 0.61].map(|d| dust(t0 + d)),
        [false, false, true, true, false]
    );
    assert_eq!(
        [-0.01, 0.01, 0.44, 0.46].map(|d| sparks(t0 + d)),
        [false, true, true, false]
    );
    // each word's effects count from its own slam: at the slam of VS the right name's have not begun
    assert!(!touched(&|img| fx::light(
        img,
        &g,
        t0 + 0.03,
        2,
        170.0,
        468.0,
        TROMBONE.0
    )));
    assert!(!touched(&|img| fx::dust(img, &g, t0 + 0.1, 2, 170.0, 468.0, 240, 35)));
}

#[test]
fn the_streaks_are_pythons() {
    // Python: title.streaks(img, 16 / 30, 1, 180.0, 300.0, 144, 84, GOLD) on a float picture that is (50, 60, 70)
    // everywhere: 681 pixels change, all to one value, on 14 rows.
    let g = Grid::new(132.0);
    let flat: RgbF = Img::filled(W, H, &[50.0, 60.0, 70.0]);
    let mut img = flat.clone();
    fx::streaks(&mut img, &g, 16.0 / 30.0, 1, 180.0, 300.0, 144, 84, super::GOLD);
    // The value to the bit: a streak is plain arithmetic (no sine, no exponential). Written with its last
    // product the other way round, `hot * 0.65 * (1 - u)`, the green sample is the neighbouring double.
    let want =
        ["0x1.37a1e1e1e1e1dp+7", "0x1.3f3e7e7e7e7e7p+7", "0x1.1234b4b4b4b4bp+7"].map(|s| parse_hex_f64(s).to_bits());
    let mut spans: Vec<(usize, usize, usize)> = Vec::new();
    let mut count = 0;
    for y in 0..H {
        let hit: Vec<usize> = (0..W).filter(|&x| img.px(x, y) != flat.px(x, y)).collect();
        for &x in &hit {
            assert_eq!(img.px(x, y).iter().map(|v| v.to_bits()).collect::<Vec<_>>(), want);
        }
        if let (Some(&a), Some(&b)) = (hit.first(), hit.last()) {
            assert_eq!(hit.len(), b - a + 1, "row {y} is one run");
            spans.push((y, a, b));
            count += hit.len();
        }
    }
    assert_eq!(count, 681);
    assert_eq!(
        spans.iter().map(|s| s.0).collect::<Vec<_>>(),
        [256, 266, 268, 272, 284, 285, 292, 296, 298, 322, 329, 342, 345, 348]
    );
    // the first four rows: streaks to the left of the word and to its right, each from its own inner end
    assert_eq!(
        spans[..4],
        [(256, 44, 94), (266, 35, 94), (268, 264, 324), (272, 264, 319)]
    );
}

#[test]
fn the_kits_messages_quote_as_python_does() {
    // Python 3.13: repr('REMATCH') 'REMATCH'; repr("IT'S ON") "IT'S ON"; repr('SAY "HI"') 'SAY "HI"';
    // repr('IT\'S "ON"') 'IT\'S "ON"'; repr(r'A\B') 'A\\B'
    use super::panels::py_repr;
    assert_eq!(py_repr("REMATCH"), "'REMATCH'");
    assert_eq!(py_repr("IT'S ON"), r#""IT'S ON""#);
    assert_eq!(py_repr(r#"SAY "HI""#), r#"'SAY "HI"'"#);
    assert_eq!(py_repr(r#"IT'S "ON""#), r#"'IT\'S "ON"'"#);
    assert_eq!(py_repr(r"A\B"), r"'A\\B'");
}
