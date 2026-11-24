//! `ballfight moves <fight.toml> --out <dir> [--effects bold|classic] [--fighter 0|1] [--kinds a,b] [--nth N]
//! [--at t,t] [--seed N | --rec F]`: contact sheets of a fight's moves, for looking at how they read.
//!
//! A dev tool for the people and agents who draw the fighters' moves: it finds the first time each kind of
//! event happens (each ability by its name, each hit by how it landed, every other kind as it is) and draws the
//! video's frames a little before, at and after it, eight to a sheet with a label on each, so a move's wind-up,
//! contact and aftermath can be seen at a glance without rendering a video. `--effects` draws the fight with
//! that look whatever the fight file says. The sheets and a `moments.json` (what each tile shows) go into a new
//! folder.

use std::path::PathBuf;
use std::sync::Arc;

use bf_core::event::{Event, Value};
use bf_core::fightfile::EFFECTS;
use bf_export::output::png_bytes;
use bf_picture::config::{FPS, NF};
use bf_picture::font;
use bf_picture::img::Img;
use serde_json::json;

use crate::{Ctx, Done, Fail, load_fight, need_ko, say, say_json, stopped, the_fight, write_new};

/// Tiles across a sheet and down it.
const COLS: usize = 4;
const ROWS: usize = 2;
/// The most sheets one run writes.
const MAX_SHEETS: usize = 12;

#[derive(clap::Args)]
pub struct Args {
    /// The fight file.
    fight: PathBuf,
    /// The folder to write the sheets into (a new folder).
    #[arg(long, value_name = "DIR")]
    out: PathBuf,
    /// Draw with these effects instead of the fight file's: classic or bold.
    #[arg(long, value_name = "NAME", value_parser = ["classic", "bold"])]
    effects: Option<String>,
    /// Only the events of this fighter (0 the left, 1 the right); default: both.
    #[arg(long, value_name = "K", value_parser = clap::value_parser!(u64).range(0..2))]
    fighter: Option<u64>,
    /// Only these kinds, as the labels name them (for example ability:WAH-WAH,hit:plunger,block), each also
    /// matching as a prefix.
    #[arg(long, value_name = "LIST", value_delimiter = ',')]
    kinds: Vec<String>,
    /// Which time each kind happens to show: 1 the first (the default), 2 the second, and so on (the last one
    /// when there are fewer).
    #[arg(long, value_name = "N", default_value_t = 1, value_parser = clap::value_parser!(u64).range(1..1000))]
    nth: u64,
    /// The offsets from the event, in seconds, of the frames drawn for each kind.
    #[arg(
        long,
        value_name = "LIST",
        value_delimiter = ',',
        allow_hyphen_values = true,
        default_value = "-0.2,0.04,0.3,0.8"
    )]
    offsets: Vec<f64>,
    /// Also these times of the video, in seconds.
    #[arg(long, value_name = "LIST", value_delimiter = ',')]
    at: Vec<f64>,
    /// Draw only the ring (rows 110 to 470 of the frame) at twice the size, four tiles to a sheet: for looking
    /// closely at a move.
    #[arg(long)]
    zoom: bool,
    /// Only the times of `--at`, no events.
    #[arg(long)]
    only_at: bool,
    /// The seed to fight (default: the fight file's).
    #[arg(long, value_name = "N", conflicts_with = "rec")]
    seed: Option<u64>,
    /// Read this recording instead of fighting the seed.
    #[arg(long, value_name = "FILE")]
    rec: Option<PathBuf>,
}

/// The label of an event's kind: `ability:NAME`, `hit:how`, or the kind itself.
fn label_of(e: &Event) -> String {
    let text = |i: usize| match e.fields.get(i) {
        Some(Value::Str(s)) => s.as_str(),
        _ => "",
    };
    match e.kind.as_str() {
        "ability" => format!("ability:{}", text(1)),
        "hit" => format!("hit:{}", text(5)),
        k => k.to_string(),
    }
}

/// The ring's part of a frame (rows 110 to 470), every pixel doubled.
fn ring_twice(img: &Img<u8>) -> Img<u8> {
    let (y0, y1) = (110.min(img.h), 470.min(img.h));
    let mut out: Img<u8> = Img::new(img.w * 2, (y1 - y0) * 2, 3);
    for y in y0..y1 {
        let line: Vec<u8> = img.row(y).chunks(3).flat_map(|px| [px, px].concat()).collect();
        for k in 0..2 {
            out.row_mut((y - y0) * 2 + k).copy_from_slice(&line);
        }
    }
    out
}

pub fn run(a: &Args, ctx: &Ctx) -> Done {
    let mut ff = load_fight(&a.fight)?;
    if let Some(effects) = &a.effects {
        ff.picture.effects = EFFECTS
            .iter()
            .find(|e| *e == effects)
            .map_or_else(|| EFFECTS[0].to_string(), |e| (*e).to_string());
    }
    if std::fs::symlink_metadata(&a.out).is_ok() {
        return Err(Fail(format!(
            "{} already exists, and ballfight never replaces a file; choose another name or delete it first.",
            a.out.display()
        )));
    }
    let rec = the_fight(&ff, &a.fight, a.rec.as_deref(), a.seed, true)?;
    need_ko(&rec)?;
    stopped(ctx, "the sheets")?;
    let seed = rec.seed;

    // the moments: (label, who, the event's time), in the order they first happen
    let mut moments: Vec<(String, Option<usize>, f64)> = Vec::new();
    if !a.only_at {
        let mut seen: Vec<(String, Option<usize>, u64, f64)> = Vec::new();
        for e in &rec.events {
            let who = e.who();
            if matches!((a.fighter, who), (Some(k), Some(w)) if k as usize != w) {
                continue;
            }
            let label = label_of(e);
            if !a.kinds.is_empty() && !a.kinds.iter().any(|k| label.starts_with(k.as_str())) {
                continue;
            }
            match seen.iter_mut().find(|s| s.0 == label && s.1 == who) {
                Some(s) if s.2 < a.nth => {
                    s.2 += 1;
                    s.3 = e.t;
                }
                Some(_) => {}
                None => seen.push((label, who, 1, e.t)),
            }
        }
        seen.sort_by(|x, y| x.3.total_cmp(&y.3));
        moments.extend(seen.into_iter().map(|s| (s.0, s.1, s.3)));
    }

    // the tiles: (label, frame)
    let frame_of = |t: f64| ((t * f64::from(FPS)).round().max(0.0) as usize).min(NF - 1);
    let mut tiles: Vec<(String, usize)> = Vec::new();
    for (label, who, t) in &moments {
        let who = who.map_or(String::new(), |w| format!("{w} "));
        for off in &a.offsets {
            let f = frame_of(t + off);
            let name = format!("{who}{label} {off:+.2}");
            if tiles
                .last()
                .is_none_or(|l| l.1 != f || !l.0.starts_with(&format!("{who}{label}")))
            {
                tiles.push((name, f));
            }
        }
    }
    for t in &a.at {
        tiles.push((format!("at {t:.2}"), frame_of(*t)));
    }
    if tiles.is_empty() {
        return Err(Fail("nothing to draw: no event of the fight matches.".to_string()));
    }
    let (cols, rows) = if a.zoom { (2, 2) } else { (COLS, ROWS) };
    let dropped = tiles.len().saturating_sub(cols * rows * MAX_SHEETS);
    tiles.truncate(cols * rows * MAX_SHEETS);

    let pic =
        bf_content::build_picture(Arc::new(rec), &ff).map_err(|e| Fail(format!("{}: {e}.", a.fight.display())))?;
    std::fs::create_dir_all(&a.out)
        .map_err(|e| Fail(format!("the folder {} cannot be made ({e}).", a.out.display())))?;
    let mut listed = Vec::new();
    let mut sheets = 0;
    for (n, chunk) in tiles.chunks(cols * rows).enumerate() {
        stopped(ctx, "the sheets")?;
        let mut sheet: Option<Img<u8>> = None;
        for (i, (label, f)) in chunk.iter().enumerate() {
            let (full, _, _) = pic.frame_reported(*f);
            let mut img = if a.zoom { ring_twice(&full) } else { full };
            let text = format!("{label} f{f}").to_uppercase();
            let mask = font::text_mask(&text, 1);
            font::stamp(&mut img, &mask, 4, 4, [255, 255, 0], Some(font::OUTLINE), false, 1.0);
            let sh = sheet.get_or_insert_with(|| Img::new(img.w * cols, img.h * rows, 3));
            let (ox, oy) = ((i % cols) * img.w, (i / cols) * img.h);
            for y in 0..img.h {
                let at = ((oy + y) * sh.w + ox) * 3;
                sh.data[at..at + img.w * 3].copy_from_slice(img.row(y));
            }
            listed.push(
                json!({"sheet": n + 1, "tile": i + 1, "label": label, "frame": f, "t": *f as f64 / f64::from(FPS)}),
            );
        }
        if let Some(sh) = sheet {
            write_new(&a.out.join(format!("sheet_{:02}.png", n + 1)), &png_bytes(&sh))?;
            sheets += 1;
        }
    }
    let report =
        json!({"seed": seed, "effects": ff.picture.effects, "sheets": sheets, "dropped": dropped, "tiles": listed});
    write_new(&a.out.join("moments.json"), crate::json_text(&report).as_bytes())?;
    if ctx.json {
        say_json(&report);
    } else {
        say(&format!(
            "seed {seed}: {} tiles on {sheets} sheets in {}{}",
            listed.len(),
            a.out.display(),
            if dropped > 0 {
                format!(" ({dropped} more were left out)")
            } else {
                String::new()
            }
        ));
    }
    Ok(())
}
