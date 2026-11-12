//! Between the interface's draft and the engine's fight file, and from engine errors to words.
//!
//! A draft is a thin layer of choices over a fight file: [`fight_file`] lays the draft's choices over the file it
//! was opened from (its `base`), or over the pair's written fight for a new match-up, so everything the draft does
//! not hold (the hand-written lines, the solos, the title hits, an old episode's tags and seeds) is kept.
//! [`draft_of`] is the way back. [`plain`] is the one place an engine error becomes a sentence for the screen.
//!
//! Two edge cases of the captions are accepted: a Shorts text that ends in a line break reopens with one line more
//! (its last text is empty), and an opened file whose `caption` holds a line break moves its second line into the
//! Instagram tags when it is saved again (the Instagram text's first line is the caption).

use bf_core::fightfile::{Captions, FightFile};

use crate::model::draft::{CaptionTexts, Draft};
use crate::wording;

/// The role of the fighter that plays bass, and of every other fighter.
const BASS: &str = "bass";
const MELODY: &str = "melody";

/// Why a draft has no fight file.
pub(crate) enum Unusable {
    /// A new match-up whose music is not written yet (no base, no starter).
    NoMusic,
    /// Anything else, already in words.
    Other(String),
}

/// The fight file of a draft, as [`fight_file`] makes it, keeping apart the pair that has no music.
pub(crate) fn build(draft: &Draft) -> Result<FightFile, Unusable> {
    let [left, right] = [draft.fighters[0].as_str(), draft.fighters[1].as_str()];
    let base = match &draft.base {
        Some(text) => Some(FightFile::parse(text).map_err(|e| unusable(&e))?),
        None => None,
    };
    // The base is used only while it is a fight of these two fighters (either way round): its lines, solos and
    // title hits are written for them by name.
    let same_pair = |ff: &FightFile| {
        let mut a: Vec<&str> = ff.fighters.iter().map(String::as_str).collect();
        let mut b = vec![left, right];
        a.sort_unstable();
        b.sort_unstable();
        a == b
    };
    let mut ff = match base.filter(same_pair) {
        Some(ff) => ff,
        None => bf_content::starter(left, right).map_err(|e| match e {
            bf_content::ContentError::NoMusic { .. } => Unusable::NoMusic,
            other => unusable(&other),
        })?,
    };
    ff.fighters = vec![left.to_string(), right.to_string()];
    ff.handicap = draft.handicap.to_vec();
    ff.tempo_bpm = draft.tempo_bpm();
    ff.pair_rules = bf_content::pair_rules(left, right);
    ff.seed = draft.seed;
    ff.music.theme = draft.theme.clone();
    if let Some(chords) = bf_content::theme_card(&draft.theme).and_then(|t| t.chord_loops.get(draft.chord_loop)) {
        ff.music.chord_loop = chords.iter().map(|c| c.to_string()).collect();
    }
    let bass = draft
        .bass
        .filter(|&i| i < 2 && bf_content::card(&draft.fighters[i]).is_some_and(|c| c.can_play_bass));
    ff.music.roles.clear();
    for (i, name) in draft.fighters.iter().enumerate() {
        let role = if bass == Some(i) { BASS } else { MELODY };
        ff.music.roles.insert(name.clone(), role.to_string());
    }
    ff.music.music_seed = draft.music_seed;
    ff.captions = captions(&draft.captions);
    ff.app_version = env!("CARGO_PKG_VERSION").to_string();
    bf_content::validate(&ff).map_err(|e| unusable(&e))?;
    Ok(ff)
}

/// A fight file this app refuses, in words.
fn unusable(e: &dyn std::fmt::Display) -> Unusable {
    Unusable::Other(plain(&format!("This fight cannot be used: {e}")))
}

/// The fight file of a draft: the file it was opened from (or, for a new match-up, the pair's written fight) with
/// the draft's fighters in its order, handicap, tempo, the pair's rules, seed, theme, chord loop, who plays bass,
/// music seed, captions and this app's version, checked against what this app has.
///
/// # Errors
/// Why the draft cannot be a fight, as a sentence for the screen ("This fight cannot be used: The left fighter's
/// handicap must be above 0, found 0.0.", or "No music is written for Flute vs Piano yet.").
pub fn fight_file(draft: &Draft) -> Result<FightFile, String> {
    build(draft).map_err(|why| match why {
        Unusable::NoMusic => plain(&no_music_sentence(&draft.fighters)),
        Unusable::Other(text) => text,
    })
}

/// "No music is written for Flute vs Piano yet.", from the roster names.
fn no_music_sentence(fighters: &[String; 2]) -> String {
    format!("No music is written for {} yet.", wording::versus(fighters))
}

/// The longer sentence the Match-up screen shows for a pair without music.
pub(crate) fn no_music_problem(fighters: &[String; 2]) -> String {
    format!(
        "No music is written for {} yet, so it cannot be balanced or rendered. Pick another pair.",
        wording::versus(fighters)
    )
}

/// The draft of a fight file read from `toml`: its fighters, theme, tempo as bars, handicap, seed, chord loop (its
/// place among the theme's loops, 0 when it is none of them), who plays bass, music seed and captions; the
/// captions count as the owner's own, and the text is kept as the draft's base.
pub fn draft_of(ff: &FightFile, toml: &str) -> Draft {
    let name = |i: usize| ff.fighters.get(i).cloned().unwrap_or_default();
    let fighters = [name(0), name(1)];
    let handicap = |i: usize| ff.handicap.get(i).copied().unwrap_or(1.0);
    let chord_loop = bf_content::theme_card(&ff.music.theme)
        .and_then(|t| {
            t.chord_loops.iter().position(|l| {
                l.len() == ff.music.chord_loop.len() && l.iter().zip(&ff.music.chord_loop).all(|(a, b)| a == b)
            })
        })
        .unwrap_or(0);
    let bass = fighters
        .iter()
        .position(|f| ff.music.roles.get(f).is_some_and(|r| r == BASS));
    Draft {
        theme: ff.music.theme.clone(),
        bars: u32::try_from(ff.grid().n_bars).unwrap_or(u32::MAX),
        handicap: [handicap(0), handicap(1)],
        seed: ff.seed,
        chord_loop,
        bass,
        music_seed: ff.music.music_seed,
        captions: caption_texts(&ff.captions),
        captions_edited: true,
        base: Some(toml.to_string()),
        fighters,
    }
}

/// The caption fields of a fight file as the three texts the Render screen edits: Instagram's is the caption, a
/// line break and the Instagram tags; TikTok's the caption, a line break and the TikTok tags; Shorts' its texts
/// one under the other.
pub fn caption_texts(c: &Captions) -> CaptionTexts {
    CaptionTexts {
        instagram: format!("{}\n{}", c.caption, c.instagram),
        tiktok: format!("{}\n{}", c.caption, c.tiktok),
        shorts: c.shorts.join("\n"),
    }
}

/// The caption fields of the three texts, the way back from [`caption_texts`]: the caption is the Instagram text's
/// first line and the Instagram tags the rest; the TikTok tags are the TikTok text after its first line; Shorts'
/// texts are the title (first line), the description (second line) and the tags (the rest), and an empty text
/// gives none. A Shorts text of one or two lines gets empty texts for the missing ones, so the file holds three.
///
/// Windows line breaks (`\r\n`, as a pasted text may have) count as line breaks: no `\r` reaches the fight file.
pub fn captions(t: &CaptionTexts) -> Captions {
    let [ig, tt, sh] = [&t.instagram, &t.tiktok, &t.shorts].map(|s| s.replace("\r\n", "\n"));
    let (caption, instagram) = ig.split_once('\n').unwrap_or((&ig, ""));
    let tiktok = tt.split_once('\n').map_or("", |(_, rest)| rest);
    let mut shorts: Vec<String> = if sh.is_empty() {
        Vec::new()
    } else {
        sh.splitn(3, '\n').map(str::to_string).collect()
    };
    if !shorts.is_empty() {
        shorts.resize(3, String::new());
    }
    Captions {
        caption: caption.to_string(),
        instagram: instagram.to_string(),
        tiktok: tiktok.to_string(),
        shorts,
    }
}

/// A fight file's title: "Flute vs Trombone, seed 20418", or "Flute vs Trombone" before a seed is chosen.
pub fn title_of(ff: &FightFile) -> String {
    let name = |i: usize| ff.fighters.get(i).cloned().unwrap_or_default();
    let pair = wording::versus(&[name(0), name(1)]);
    match ff.seed {
        Some(seed) => format!("{pair}, seed {seed}"),
        None => pair,
    }
}

/// An engine error as a sentence for the screen: its text with a capital first letter and a full stop at the end.
/// Every error the engine sends to the interface goes through here.
///
/// A fight file's field path is put in words: where the text (or a part of it after ": ") starts with a path such
/// as `handicap[0]` followed by ": ", the path and its colon become the thing's name, and the words start with a
/// capital, as a reason after a colon does everywhere in the app. A complaint that starts with "must" follows the
/// name ("The left fighter's handicap must be above 0, found 0.0"); a fighter that is already the other one, retired
/// or not in the roster says so ("Flute is already the left fighter, so it cannot also be the right one."); any
/// other follows the name and a colon ("The Shorts text: 3 texts needed (title, description, tags), found 1"). A
/// quoted roster name is written as the screen writes it, and a path inside the complaint is named too.
pub fn plain(e: &dyn std::fmt::Display) -> String {
    let text = in_words(e.to_string().trim());
    let text = text.trim();
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return "Something went wrong in the engine.".to_string();
    };
    let mut out: String = first.to_uppercase().chain(chars).collect();
    if !out.ends_with(['.', '!', '?']) {
        out.push('.');
    }
    out
}

/// `text` with its first fight-file field path put in words (see [`plain`]); unchanged when it holds none.
fn in_words(text: &str) -> String {
    let starts = std::iter::once(0).chain(text.match_indices(": ").map(|(i, _)| i + 2));
    for at in starts {
        let Some((path, complaint)) = text[at..].split_once(": ") else {
            continue;
        };
        let Some(name) = field_name(path) else {
            continue;
        };
        let complaint = quoted_names(complaint)
            .replace("fighters[0]", "the left fighter")
            .replace("fighters[1]", "the right fighter");
        let complaint = complaint.trim_end_matches('.');
        let fighter = path.starts_with("fighters[");
        let words = if name.is_empty() {
            complaint.to_string()
        } else if let Some(rest) = complaint.strip_prefix("must ") {
            format!("{name} must {rest}")
        } else if fighter && complaint.contains(" is already the ") {
            // "Flute is already the left fighter, so it cannot also be the right one."
            let side = name.trim_start_matches("the ").trim_end_matches(" fighter");
            format!("{complaint}, so it cannot also be the {side} one")
        } else if fighter {
            // "Oboe is retired, so it cannot be the left fighter."
            format!("{complaint}, so it cannot be {name}")
        } else {
            format!("{name}: {complaint}")
        };
        // The words start a sentence of their own after the colon, as every reason the files handler gives.
        return format!("{}{}", &text[..at], capitalised(&words));
    }
    text.to_string()
}

/// `text` with its first letter in capitals.
fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// `text` with each quoted roster name (`"FLUTE"`, `"BASS DRUM"`, a retired one too) written as the screen writes it
/// ("Flute", "Bass drum"), without the quotes. Any other quoted text is left as it is.
fn quoted_names(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('"') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else {
            break;
        };
        let inner = &after[..close];
        out.push_str(&rest[..open]);
        let is_name = !inner.is_empty()
            && (bf_content::roster().iter().any(|f| f.name == inner) || bf_core::fightfile::RETIRED.contains(&inner));
        if is_name {
            out.push_str(&wording::display_name(inner));
        } else {
            out.push('"');
            out.push_str(inner);
            out.push('"');
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

/// The words for a fight file's field path (`handicap[0]`, `music.chord_loop[2]`, `line 3`), or `None` when
/// `path` is not one. Empty for a field whose complaints name it themselves (`format`).
fn field_name(path: &str) -> Option<String> {
    let (head, index) = match path.strip_suffix(']').and_then(|p| p.rsplit_once('[')) {
        Some((head, i)) if !i.is_empty() && i.bytes().all(|b| b.is_ascii_digit()) => (head, Some(i)),
        _ => (path, None),
    };
    let side = |i: &str| match i {
        "0" => Some("left"),
        "1" => Some("right"),
        _ => None,
    };
    let chord = |of: &str| -> Option<String> {
        let n = index?.parse::<usize>().ok()?;
        Some(format!("chord {} of {of}", n.checked_add(1)?))
    };
    let named = |s: &str| Some(s.to_string());
    match (head, index) {
        ("handicap", Some(i)) => Some(format!("the {} fighter's handicap", side(i)?)),
        ("handicap", None) => named("the handicaps"),
        ("fighters", Some(i)) => Some(format!("the {} fighter", side(i)?)),
        ("fighters", None) => named("the fighters"),
        // Its complaints name the format themselves.
        ("format", None) => named(""),
        ("arena", None) => named("the arena"),
        ("dmg_scale", None) => named("the damage scale"),
        ("tempo_bpm", None) => named("the tempo"),
        ("seed", None) => named("the seed"),
        ("music.music_seed", None) => named("the music seed"),
        ("music.theme", None) => named("the theme"),
        ("music.key", None) => named("the key"),
        ("music.scale", None) => named("the scale"),
        ("music.arrangement", None) => named("the arrangement"),
        ("music.chord_loop", None) => named("the chord loop"),
        ("music.chord_loop", Some(_)) => chord("the chord loop"),
        ("music.chords_intro", None) => named("the opening chords"),
        ("music.chords_intro", Some(_)) => chord("the opening chords"),
        ("picture.shard_seed", None) => named("the picture's shard seed"),
        ("picture.fx_seed", None) => named("the picture's particle seed"),
        ("captions.shorts", _) => named("the Shorts text"),
        _ => {
            if let Some(n) = head.strip_prefix("line ")
                && index.is_none()
                && !n.is_empty()
                && n.bytes().all(|b| b.is_ascii_digit())
            {
                return Some(format!("line {n} of the fight file"));
            }
            let parts = [
                ("music.roles.", "part"),
                ("music.lines.", "line"),
                ("music.solos.", "solo"),
                ("music.title_hits.", "title hits"),
            ];
            for (table, what) in parts {
                if let Some(name) = head.strip_prefix(table).filter(|n| !n.is_empty()) {
                    return Some(format!("{}'s {what}", wording::display_name(name)));
                }
            }
            if let Some(rule) = head.strip_prefix("pair_rules.").filter(|r| !r.is_empty()) {
                return Some(format!("the pair rule {rule}"));
            }
            if head.starts_with("captions.") {
                return named("the captions");
            }
            if head.starts_with("picture.") {
                return named("the picture settings");
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tests::starting_draft;
    use bf_core::event::Value;

    const EPISODE: &str = include_str!("../../../../fights/flute_vs_trombone.toml");

    fn texts(instagram: &str, tiktok: &str, shorts: &str) -> CaptionTexts {
        CaptionTexts {
            instagram: instagram.to_string(),
            tiktok: tiktok.to_string(),
            shorts: shorts.to_string(),
        }
    }

    #[test]
    fn a_draft_becomes_a_valid_fight_file() {
        let mut d = starting_draft();
        d.seed = Some(7);
        let ff = fight_file(&d).expect("a fight file");
        assert_eq!(bf_content::validate(&ff), Ok(()));
        assert_eq!(ff.tempo_bpm, 34.0 * 4.0 * 60.0 / 61.0);
        assert_eq!(ff.pair_rules.get("swat_cobras"), Some(&Value::Bool(true)));
        assert_eq!(ff.seed, Some(7));
        assert_eq!(ff.fighters, ["FLUTE", "TROMBONE"]);
        assert_eq!(ff.handicap, [1.0, 1.0]);
        assert_eq!(ff.music.theme, "big_top");
        assert_eq!(ff.music.roles.get("TROMBONE").map(String::as_str), Some("bass"));
        assert_eq!(ff.music.roles.get("FLUTE").map(String::as_str), Some("melody"));
        assert_eq!(ff.app_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(title_of(&ff), "Flute vs Trombone, seed 7");
        d.seed = None;
        assert_eq!(title_of(&fight_file(&d).expect("no seed")), "Flute vs Trombone");
    }

    #[test]
    fn swapping_sides_swaps_the_file() {
        let mut d = starting_draft();
        d.handicap = [2.0, 0.4];
        let a = fight_file(&d).expect("as drafted");
        d.fighters.swap(0, 1);
        d.handicap.swap(0, 1);
        d.bass = d.bass.map(|i| 1 - i);
        let b = fight_file(&d).expect("swapped");
        assert_eq!(a.fighters, ["FLUTE", "TROMBONE"]);
        assert_eq!(b.fighters, ["TROMBONE", "FLUTE"]);
        assert_eq!(a.handicap, [2.0, 0.4]);
        assert_eq!(b.handicap, [0.4, 2.0]);
        // the trombone keeps the bass from either corner
        assert_eq!(a.music.roles, b.music.roles);
        assert_ne!(a.search_key(&[false, false]), b.search_key(&[false, false]));
    }

    #[test]
    fn music_and_captions_keep_the_search_key() {
        let d = starting_draft();
        let key = |d: &Draft| fight_file(d).expect("a fight file").search_key(&[false, false]);
        let k = key(&d);
        let mut same = d.clone();
        same.captions.instagram = "Something else\n#tags".to_string();
        same.theme = "big_top".to_string();
        same.music_seed = 99;
        same.seed = Some(5);
        assert_eq!(key(&same), k);
        // the theme field itself does not count either, even when it names a theme this app lacks
        let mut theme = fight_file(&d).expect("a fight file");
        theme.music.theme = "polka".to_string();
        assert_eq!(theme.search_key(&[false, false]), k);
        let mut bars = d.clone();
        bars.bars = 30;
        assert_ne!(key(&bars), k);
        let mut handicap = d.clone();
        handicap.handicap[1] = 1.5;
        assert_ne!(key(&handicap), k);
    }

    #[test]
    fn a_file_round_trips_through_a_draft() {
        let ff = FightFile::parse(EPISODE).expect("the episode");
        let d = draft_of(&ff, EPISODE);
        assert_eq!(d.fighters, ["FLUTE", "TROMBONE"]);
        assert_eq!(d.bars, ff.grid().n_bars as u32);
        assert_eq!(
            (d.handicap, d.seed, d.chord_loop, d.bass),
            ([2.0, 0.4], Some(392), 0, Some(1))
        );
        assert!(d.captions_edited);
        assert_eq!(d.base.as_deref(), Some(EPISODE));
        let back = fight_file(&d).expect("the episode again");
        let mut want = ff.clone();
        want.tempo_bpm = d.tempo_bpm();
        want.app_version = back.app_version.clone();
        assert_eq!(back, want);
        // what the draft does not hold came from the base
        assert_eq!(back.music.lines, ff.music.lines);
        assert_eq!(back.music.solos, ff.music.solos);
        assert_eq!(back.music.title_hits, ff.music.title_hits);
        assert_eq!(back.series_tag.as_deref(), Some("BALLFIGHT #15"));
        assert_eq!(title_of(&back), "Flute vs Trombone, seed 392");
    }

    /// The app neither shows nor edits a solo: a file's solos come back as they were, a chord (ACOUSTIC's strum)
    /// too, and a chord where the kit's part cannot play one is refused in plain words.
    #[test]
    fn a_chord_in_a_solo_is_kept_and_a_wrong_one_named() {
        let text = EPISODE.replace("TROMBONE", "ACOUSTIC").replacen(
            "[0.0, 40, 0.5]",
            "[0.0, [40, 47, 52, 55, 59, 64], 0.5]",
            1,
        );
        let ff = FightFile::parse(&text).expect("a chord in the ACOUSTIC's solo");
        let back = fight_file(&draft_of(&ff, &text)).expect("the fight file again");
        assert_eq!(back.music.solos, ff.music.solos);
        assert!(
            back.to_toml()
                .contains("ACOUSTIC = [\n    [0.0, [40, 47, 52, 55, 59, 64], 0.5],")
        );
        let wrong = EPISODE.replacen("[0.0, 83, 0.5]", "[0.0, [83, 88], 0.5]", 1);
        assert_eq!(
            plain(&FightFile::parse(&wrong).unwrap_err()),
            "Flute's solo: a chord, but FLUTE's solo plays one note at a time (the kit's FLUTE part stops on a chord; \
             only ACOUSTIC can strum one)."
        );
    }

    #[test]
    fn a_file_the_app_wrote_round_trips() {
        let mut d = starting_draft();
        d.fighters.swap(0, 1);
        d.bass = d.bass.map(|i| 1 - i);
        d.handicap = [0.4, 2.0];
        d.seed = Some(7);
        d.captions = caption_texts(&bf_content::draft_captions("TROMBONE", "FLUTE"));
        d.captions.instagram = d
            .captions
            .instagram
            .replacen("Trombone vs Flute", "The back row wins", 1);
        d.captions_edited = true;
        let first = fight_file(&d).expect("written").to_toml();
        let reread = FightFile::parse(&first).expect("read back");
        assert_eq!(
            reread.captions.caption.split(' ').take(4).collect::<Vec<_>>(),
            ["The", "back", "row", "wins"]
        );
        let second = fight_file(&draft_of(&reread, &first)).expect("written again").to_toml();
        assert_eq!(second, first);
    }

    #[test]
    fn captions_map_both_ways() {
        let c = Captions {
            caption: "Flute vs Trombone".to_string(),
            instagram: "#ig #tags".to_string(),
            tiktok: "#tt".to_string(),
            shorts: vec!["Title".to_string(), "Words".to_string(), "#tags".to_string()],
        };
        let t = caption_texts(&c);
        assert_eq!(
            t,
            texts(
                "Flute vs Trombone\n#ig #tags",
                "Flute vs Trombone\n#tt",
                "Title\nWords\n#tags"
            )
        );
        assert_eq!(captions(&t), c);
        // an empty Shorts text is no texts; a short one is padded to three; more lines stay in the tags
        let none = captions(&texts("", "", ""));
        assert_eq!(
            (none.caption.as_str(), none.instagram.as_str(), none.tiktok.as_str()),
            ("", "", "")
        );
        assert!(none.shorts.is_empty());
        assert_eq!(
            captions(&texts("Only a caption", "No tags", "Title")).shorts,
            ["Title", "", ""]
        );
        assert_eq!(
            captions(&texts("Only a caption", "No tags", "Title")).caption,
            "Only a caption"
        );
        assert_eq!(captions(&texts("Only a caption", "No tags", "Title")).tiktok, "");
        let long = captions(&texts("a\nb\nc", "x\ny\nz", "1\n2\n3\n4"));
        assert_eq!(
            (long.caption.as_str(), long.instagram.as_str(), long.tiktok.as_str()),
            ("a", "b\nc", "y\nz")
        );
        assert_eq!(long.shorts, ["1", "2", "3\n4"]);
        assert_eq!(caption_texts(&long).shorts, "1\n2\n3\n4");
        // every edited text is a fight file that validates
        let mut d = starting_draft();
        d.captions = texts("One line", "", "Two\nlines");
        assert!(fight_file(&d).is_ok());
    }

    #[test]
    fn windows_line_breaks_leave_no_carriage_return() {
        let c = captions(&texts(
            "Flute vs Trombone\r\n#ig\r\n#more",
            "Flute vs Trombone\r\n#tt",
            "Title\r\nWords\r\n#tags\r\n#more",
        ));
        assert_eq!(
            (c.caption.as_str(), c.instagram.as_str(), c.tiktok.as_str()),
            ("Flute vs Trombone", "#ig\n#more", "#tt")
        );
        assert_eq!(c.shorts, ["Title", "Words", "#tags\n#more"]);
        let all = [&c.caption, &c.instagram, &c.tiktok].into_iter().chain(&c.shorts);
        for text in all {
            assert!(!text.contains('\r'), "{text:?}");
        }
    }

    #[test]
    fn field_paths_are_put_in_words() {
        let cases = [
            (
                "This fight cannot be used: handicap[1]: must be a finite number, found inf",
                "This fight cannot be used: The right fighter's handicap must be a finite number, found inf.",
            ),
            (
                "This fight cannot be opened: fighters[1]: \"FLUTE\" is already fighters[0]",
                "This fight cannot be opened: Flute is already the left fighter, so it cannot also be the right one.",
            ),
            (
                "fighters[0]: \"BASS DRUM\" is not in the roster",
                "Bass drum is not in the roster, so it cannot be the left fighter.",
            ),
            (
                "tempo_bpm: must be from 60 to 200, found 20.0",
                "The tempo must be from 60 to 200, found 20.0.",
            ),
            (
                "This fight cannot be used: captions.shorts: 3 texts needed (title, description, tags), found 1",
                "This fight cannot be used: The Shorts text: 3 texts needed (title, description, tags), found 1.",
            ),
            (
                "this video's fight file cannot be used: music.chord_loop[2]: big_top has no voicing for \"X\"",
                "This video's fight file cannot be used: Chord 3 of the chord loop: big_top has no voicing for \"X\".",
            ),
            (
                "music.title_hits.TROMBONE[0]: 300 is not a MIDI note (0 to 127)",
                "Trombone's title hits: 300 is not a MIDI note (0 to 127).",
            ),
            (
                "This fight cannot be opened: format: this app reads fight files up to format 2 (this file is format 3)",
                "This fight cannot be opened: This app reads fight files up to format 2 (this file is format 3).",
            ),
            (
                "line 3: expected a table",
                "Line 3 of the fight file: expected a table.",
            ),
        ];
        for (engine, screen) in cases {
            assert_eq!(plain(&engine), screen);
        }
        // Text with no field path is only made a sentence; a word that merely looks like one is left alone.
        assert_eq!(plain(&"the seed: 12 is fine"), "The seed: 12 is fine.");
        assert_eq!(plain(&"handicap[7]: must be above 0"), "Handicap[7]: must be above 0.");
        // No sentence a person can reach still holds a path.
        let mut d = starting_draft();
        d.handicap = [f64::NAN, 1.0];
        let text = fight_file(&d).unwrap_err();
        assert!(!text.contains("handicap[") && !text.contains("]: "), "{text}");
    }

    #[test]
    fn errors_are_sentences() {
        assert_eq!(plain(&"fighters: two needed"), "The fighters: two needed.");
        assert_eq!(plain(&"Already one."), "Already one.");
        assert_eq!(plain(&"  "), "Something went wrong in the engine.");
        let mut d = starting_draft();
        d.handicap[0] = 0.0;
        assert_eq!(
            fight_file(&d).unwrap_err(),
            "This fight cannot be used: The left fighter's handicap must be above 0, found 0.0."
        );
        d.handicap[0] = 1.0;
        d.base = Some("not a fight file".to_string());
        let text = fight_file(&d).unwrap_err();
        assert!(
            text.starts_with("This fight cannot be used: ") && text.ends_with('.'),
            "{text}"
        );
        d.base = None;
        d.fighters[1] = "PIANO".to_string();
        assert_eq!(
            fight_file(&d).unwrap_err(),
            "No music is written for Flute vs Piano yet."
        );
    }
}
