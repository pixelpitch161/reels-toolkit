//! The Punchy look: the example episode drawn Classic and Punchy. Punchy leaves the title card, the count-in and
//! the loop's last seconds as Classic draws them, changes the fight, and is a pure function of the frame.

mod common;

use std::sync::Arc;

use bf_content::build_picture;
use bf_core::fightfile::FightFile;
use bf_picture::ring::Picture;
use common::EPISODE;

fn picture(look: &str) -> Picture {
    let mut ff: FightFile = EPISODE.ff.clone();
    ff.picture.look = look.to_string();
    build_picture(Arc::new(EPISODE.rec.clone()), &ff).expect("the episode's picture builds")
}

#[test]
fn punchy_keeps_the_edges_and_changes_the_fight() {
    let (classic, punchy) = (picture("classic"), picture("punchy"));
    let ctx = punchy.ctx();
    assert!(ctx.punch.is_some() && classic.ctx().punch.is_none());
    let fps = 30.0;
    let first_hit = ctx.punch.as_ref().unwrap().hits[0].t;
    // before the fight and through the loop's last seconds: the same picture
    for f in [0, 20, (ctx.show.grid.t_fight * fps) as usize - 2, 1745, 1800, 1829] {
        assert!(classic.frame(f).data == punchy.frame(f).data, "frame {f} differs");
    }
    // a frame just after the first hit: different
    let f = (first_hit * fps).ceil() as usize + 2;
    assert!(classic.frame(f).data != punchy.frame(f).data, "frame {f} is the same");
    // the K.O.: different
    let f = (ctx.t_ko * fps) as usize + 6;
    assert!(
        classic.frame(f).data != punchy.frame(f).data,
        "the K.O. frame {f} is the same"
    );
}

#[test]
fn punchy_is_a_pure_function_of_the_frame() {
    let p = picture("punchy");
    let ko = (p.ctx().t_ko * 30.0) as usize;
    for f in [ko - 100, ko + 8] {
        assert!(p.frame(f).data == p.frame(f).data, "frame {f}");
    }
    let q = picture("punchy");
    assert!(p.frame(ko + 8).data == q.frame(ko + 8).data, "two builds agree");
}
