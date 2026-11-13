//! From a fight to its picture: the kit's `kit.picture` module for one episode, with the content of this app.
//!
//! [`build_picture`] is what the app's preview and the render job call: it checks the fight file against what
//! this app has ([`validate`]), derives the show from the recording, gathers the two fighters' roster entries,
//! models and looks ([`look_for`]) and builds the [`Picture`]. Everything a fight file or a recording can do
//! wrong comes back as a [`ContentError`]; so does a panic while the picture is built (the looks' `setup` and
//! the effects build run there), which is caught here as [`crate::build_soundtrack`] catches the sound code's.
//!
//! [`title_preview`] draws the VS title card of a fight file alone, before any fight exists.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::roster::FighterCfg;
use bf_core::show::Show;
use bf_picture::Rgb8;
use bf_picture::config::{H, W};
use bf_picture::ring::{Picture, PictureError};
use bf_picture::sprite::Mesh;
use bf_picture::title::layout::Layout;
use bf_picture::title::{CardLook, TitleCard};

use crate::error::ContentError;
use crate::fighters::look_for;
use crate::registry::{name_status, roster};
use crate::soundtrack::validate;

/// The model file of the fighter `name`, as the export tool's `models` wrote it (`data/models/<NAME>.json`), for
/// the fighters whose look is ported: its folder's `MESH`.
fn model_json(name: &str) -> Option<&'static str> {
    crate::fighters::mesh_json(name)
}

/// The roster entry and the model of the fighter `name` on side `k`, or the error that this app has none.
fn fighter(name: &str, k: usize) -> Result<(&'static FighterCfg, Mesh), ContentError> {
    let missing = || ContentError::field(format!("fighters[{k}]"), format!("{name} has no look yet"));
    let cfg = roster().iter().find(|f| f.name == name).ok_or_else(missing)?;
    let json = model_json(&cfg.name).ok_or_else(missing)?;
    let mesh = Mesh::from_json(json)
        .map_err(|e| ContentError::Picture(PictureError::Mesh(format!("{name}'s model cannot be read: {e}"))))?;
    Ok((cfg, mesh))
}

/// The model of the roster fighter `name` (the mesh its sprites are drawn from), or `None` for a fighter whose
/// model is not in this app or a name that is not in the roster. The app draws its corner-card portraits from it.
pub fn mesh(name: &str) -> Option<Mesh> {
    let cfg = roster().iter().find(|f| f.name == name)?;
    Mesh::from_json(model_json(&cfg.name)?).ok()
}

/// What a caught panic said.
fn panic_text(payload: Box<dyn Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(text) => *text,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(text) => (*text).to_string(),
            Err(_) => "the picture code stopped without a message".to_string(),
        },
    }
}

/// The picture of the fight `rec` under the fight file `ff`: every frame of its video, drawn on demand
/// ([`Picture::frame`], [`Picture::frame_reported`]). The picture is `Send + Sync` and a frame is a pure function
/// of its number, so the preview and the render job draw frames in any order on any number of threads and get
/// the same bytes on every PC.
///
/// # Errors
/// - [`ContentError::FightFile`]: what [`validate`] refuses, or a fighter whose look is not in this app.
/// - [`ContentError::Show`]: the fight has no K.O., or the recording is not this fight file's fight.
/// - [`ContentError::Picture`]: the recording lacks something the picture reads, or holds values the picture
///   code cannot use (a panic while the picture is built is caught and reported here too).
pub fn build_picture(rec: Arc<Recording>, ff: &FightFile) -> Result<Picture, ContentError> {
    validate(ff)?;
    let show = Show::new(&rec, ff)?;
    let [(cfg0, mesh0), (cfg1, mesh1)] = [fighter(&show.names[0], 0)?, fighter(&show.names[1], 1)?];
    let looks = [look_for(&show.names[0]), look_for(&show.names[1])];
    let built = catch_unwind(AssertUnwindSafe(|| {
        Picture::new(rec, ff, show, [cfg0, cfg1], [mesh0, mesh1], looks)
    }));
    match built {
        Ok(result) => result.map_err(ContentError::Picture),
        Err(payload) => Err(ContentError::Picture(PictureError::Recording(format!(
            "the picture could not be built: {}",
            panic_text(payload)
        )))),
    }
}

/// The yaw and the roll of the fighter's view (`view[0]` and `view[2]` of its roster entry), as the picture reads
/// them.
fn view(cfg: &FighterCfg) -> Result<(f64, f64), ContentError> {
    let v: Vec<f64> = cfg
        .raw
        .get("view")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_f64()).collect())
        .unwrap_or_default();
    match v[..] {
        [yaw, _, roll] => Ok((yaw, roll)),
        _ => Err(ContentError::Picture(PictureError::Roster(format!(
            "{} has no view (yaw, swing, roll) in the roster",
            cfg.name
        )))),
    }
}

/// The VS title card of the fight file `ff` before any fight exists: the card on its cover frame
/// ([`TitleCard::cover_frame`]) over a black 360 x 640 picture, built from [`Show::placeholder`] with the fighters'
/// roster colours, models and looks ([`look_for`], whose title hooks need nothing from a fight), the file's
/// series tag and its title opt-ins (`picture.title_lines`, `title_balls`, `middle`). The card reads only the grid, the names and the tag, so this is the card the picture of any fight
/// of this file draws on that frame (there over the dimmed arena instead of over black).
///
/// A pure function of the fight file: the same file gives the same bytes on every call and every PC, on any
/// thread. Each call builds the card anew (both panels, both close-up sprites) and draws one frame; nothing is
/// kept between calls.
///
/// # Errors
/// - [`ContentError::FightFile`]: what `FightFile::validate` refuses (with this app's fighter names), or a fighter
///   whose look is not in this app. The theme and the music are not read: [`validate`] checks those.
/// - [`ContentError::Picture`]: a fighter's model or roster view cannot be read, the card's panel pattern for a
///   fighter is not ported, the middle word does not fit or a ball is too big for its close-up
///   ([`PictureError::Title`]), or the card code panicked (caught here).
pub fn title_preview(ff: &FightFile) -> Result<Rgb8, ContentError> {
    ff.validate(&name_status)?;
    let show = Show::placeholder(ff);
    let [(cfg0, mesh0), (cfg1, mesh1)] = [fighter(&show.names[0], 0)?, fighter(&show.names[1], 1)?];
    let views = [view(cfg0)?, view(cfg1)?];
    let looks = [look_for(&show.names[0]), look_for(&show.names[1])];
    let (cfg, meshes) = ([cfg0, cfg1], [mesh0, mesh1]);
    let drawn = catch_unwind(AssertUnwindSafe(|| {
        // the fight file's title opt-ins (two-line names, moved balls, a middle word), as the picture's card has
        let layout = Layout::new(&show.names[0], &show.names[1], &ff.picture)?;
        let card = TitleCard::with_layout(
            &show,
            cfg.map(|c| c.color.map(f64::from)),
            cfg.map(|c| c.dark.map(f64::from)),
            [0, 1].map(|k| CardLook {
                look: &*looks[k],
                mesh: &meshes[k],
                yaw: views[k].0,
                roll: views[k].1,
            }),
            ff.series_tag.as_deref(),
            layout,
        )?;
        let mut img = Rgb8::new(W, H, 3);
        card.draw(&mut img, card.cover_frame() as f64 / f64::from(show.grid.fps));
        Ok(img)
    }));
    match drawn {
        Ok(result) => result.map_err(|e| ContentError::Picture(PictureError::Title(e))),
        Err(payload) => Err(ContentError::Picture(PictureError::Recording(format!(
            "the title card could not be drawn: {}",
            panic_text(payload)
        )))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::starter;

    fn black_share(img: &Rgb8) -> f64 {
        let black = img.data.chunks(3).filter(|p| p.iter().all(|&v| v == 0)).count();
        black as f64 / (img.w * img.h) as f64
    }

    #[test]
    fn the_title_preview_is_the_card() {
        let ff = starter("FLUTE", "TROMBONE").expect("the starter");
        let a = title_preview(&ff).expect("the preview");
        assert_eq!((a.w, a.h, a.c), (360, 640, 3));
        assert_eq!(
            title_preview(&ff).expect("the preview again").data,
            a.data,
            "two calls, the same bytes"
        );
        let swapped = starter("TROMBONE", "FLUTE").expect("the swapped starter");
        let b = title_preview(&swapped).expect("the swapped preview");
        assert_eq!((b.w, b.h, b.c), (360, 640, 3));
        assert_ne!(a.data, b.data, "the swapped order is another card");
        for img in [&a, &b] {
            assert!(black_share(img) < 0.10, "{:.1} % black", black_share(img) * 100.0);
        }
    }

    #[test]
    fn the_title_preview_is_the_pictures_card() {
        // the episode's own picture draws the same card on its cover frame
        let ff = FightFile::parse(include_str!("../../../fights/flute_vs_trombone.toml")).expect("the episode");
        let rec = Recording::read(&bf_testdata::reference("fights/flute_vs_trombone/ep392.bfrec")).expect("ep392");
        let pic = build_picture(Arc::new(rec), &ff).expect("the picture");
        let mut want = Rgb8::new(360, 640, 3);
        let card = pic.card();
        card.draw(&mut want, card.cover_frame() as f64 / 30.0);
        assert_eq!(title_preview(&ff).expect("the preview").data, want.data);
    }

    #[test]
    fn the_ported_fighters_have_models() {
        for name in crate::PORTED {
            let m = mesh(name).unwrap_or_else(|| panic!("{name} has no model"));
            assert_eq!(Some(m), fighter(name, 0).ok().map(|(_, m)| m), "{name}");
        }
        // a roster fighter's model is its folder's; a name outside the roster has none
        for f in roster() {
            assert_eq!(
                mesh(&f.name).is_some(),
                crate::fighters::mesh_json(&f.name).is_some(),
                "{}",
                f.name
            );
        }
        for name in ["KAZOO", "", "flute", "NOT A FIGHTER"] {
            assert!(mesh(name).is_none(), "{name:?}");
        }
    }

    #[test]
    fn a_fighter_without_a_look_has_no_preview() {
        // every roster fighter whose model is not in the app yet (found at run time: none once all are ported)
        // has no preview, in words; a name outside the roster has none either
        let mut base = starter("FLUTE", "TROMBONE").expect("the starter");
        // the second fighter changes below: nothing of the file may name TROMBONE
        base.music.roles.clear();
        base.music.lines.clear();
        base.music.solos.clear();
        base.music.title_hits.clear();
        base.pair_rules.clear();
        let lookless: Vec<&str> = roster()
            .iter()
            .map(|f| f.name.as_str())
            .filter(|n| mesh(n).is_none())
            .collect();
        for name in &lookless {
            let mut ff = base.clone();
            ff.fighters[1] = name.to_string();
            let err = title_preview(&ff).expect_err("no model, no preview");
            assert!(
                err.to_string().contains(&format!("{name} has no look yet")),
                "{name}: {err}"
            );
        }
        let mut ff = base.clone();
        ff.fighters[1] = "KAZOO".to_string();
        assert!(title_preview(&ff).is_err());
        // and every fighter the app offers has one (with FLUTE on the other side)
        for name in crate::PORTED.iter().filter(|&&n| n != "FLUTE") {
            let mut ff = base.clone();
            ff.fighters[1] = name.to_string();
            assert!(title_preview(&ff).is_ok(), "{name}");
        }
    }
}
