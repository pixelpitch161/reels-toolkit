//! The fight a new match-up begins from, and the caption drafts (Task 26; spec sections 3, 4 and 9).
//!
//! The fight file is the whole recipe, so a match-up starts from a written one: for a pair whose music is written,
//! its example fight with the fighters in the asked order, no handicap, no seed and no episode number.

use bf_core::fightfile::{Captions, FightFile};

use crate::{ContentError, card, pair_rules};

/// The pairs whose music is written, either way round, with the fight file each starts from.
const STARTERS: [(&str, &str, &str); 1] = [(
    "FLUTE",
    "TROMBONE",
    include_str!("../../../fights/flute_vs_trombone.toml"),
)];

/// The tags every Instagram draft ends with.
const INSTAGRAM_TAGS: &str = "#musicmemes #musician #pixelart #lowpoly #gamedev #satisfying #music";

/// The tags every TikTok draft ends with.
const TIKTOK_TAGS: &str = "#bandkid #musictok #pixelart #satisfying #bandtok #music";

/// The tags every Shorts draft ends with.
const SHORTS_TAGS: &str = "#music #pixelart";

/// The fight a new match-up of `left` (the left corner) and `right` begins from: the pair's written fight with the
/// fighters in this order, handicap 1.0 each, no seed, no series tag, the kit's default particle seed, the pair's
/// rules, drafted captions and this app's version.
///
/// # Errors
/// For a pair with no music written yet (every pair but FLUTE and TROMBONE), a `fighters` error whose message is
/// "No music is written for {Left} vs {Right} yet." with both names in sentence case.
pub fn starter(left: &str, right: &str) -> Result<FightFile, ContentError> {
    let text = STARTERS
        .iter()
        .find(|(a, b, _)| (left == *a && right == *b) || (left == *b && right == *a))
        .map(|(_, _, text)| *text)
        .ok_or_else(|| {
            ContentError::field(
                "fighters",
                format!(
                    "No music is written for {} vs {} yet.",
                    sentence_case(left),
                    sentence_case(right)
                ),
            )
        })?;
    let mut ff = FightFile::parse(text)?;
    ff.app_version = env!("CARGO_PKG_VERSION").to_string();
    ff.fighters = vec![left.to_string(), right.to_string()];
    ff.handicap = vec![1.0, 1.0];
    ff.seed = None;
    ff.series_tag = None;
    ff.picture.fx_seed = None;
    ff.pair_rules = pair_rules(left, right);
    ff.captions = draft_captions(left, right);
    Ok(ff)
}

/// The post texts drafted for `left` vs `right` from the fighters' emoji and tags (spec section 9). A fighter
/// without a card adds no emoji and no tags of its own.
pub fn draft_captions(left: &str, right: &str) -> Captions {
    let (l, r) = (sentence_case(left), sentence_case(right));
    let (lt, rt) = (name_tag(left), name_tag(right));
    let emoji: String = [left, right].iter().filter_map(|n| card(n)).map(|c| c.emoji).collect();
    let mut own: Vec<&str> = Vec::new();
    for tag in [left, right]
        .iter()
        .filter_map(|n| card(n))
        .flat_map(|c| c.tags.iter().copied())
    {
        if !own.contains(&tag) {
            own.push(tag);
        }
    }
    let mut instagram = format!("#ballfight {lt} {rt}");
    for tag in own {
        instagram.push(' ');
        instagram.push_str(tag);
    }
    instagram.push(' ');
    instagram.push_str(INSTAGRAM_TAGS);
    let mut caption = format!("{l} vs {r} ");
    if !emoji.is_empty() {
        caption.push_str(&emoji);
        caption.push(' ');
    }
    caption.push_str("Comment the next match-up \u{1F447}");
    Captions {
        caption,
        instagram,
        tiktok: format!("#fyp #ballfight {lt} {rt} {TIKTOK_TAGS}"),
        shorts: vec![
            format!("{l} vs {r}"),
            format!("{l} vs {r}, every hit in key. Comment the next match-up"),
            format!("#shorts #ballfight {lt} {rt} {SHORTS_TAGS}"),
        ],
    }
}

/// A roster name in sentence case: "BASS DRUM" is "Bass drum".
fn sentence_case(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars.flat_map(char::to_lowercase)).collect(),
        None => String::new(),
    }
}

/// A fighter's name tag: "#" and the name in lower case without spaces ("ALTO SAX" is "#altosax").
fn name_tag(name: &str) -> String {
    let mut tag = String::from("#");
    tag.extend(name.chars().filter(|c| !c.is_whitespace()).flat_map(char::to_lowercase));
    tag
}

#[cfg(test)]
mod tests {
    use super::*;
    use bf_core::event::Value;

    #[test]
    fn the_starter_is_a_valid_fight() {
        for (left, right) in [("FLUTE", "TROMBONE"), ("TROMBONE", "FLUTE")] {
            let ff = starter(left, right).unwrap_or_else(|e| panic!("{left} vs {right}: {e}"));
            assert_eq!(crate::validate(&ff), Ok(()), "{left} vs {right}");
            assert_eq!(ff.fighters, [left, right]);
            assert_eq!(ff.handicap, [1.0, 1.0]);
            assert_eq!(ff.seed, None);
            assert_eq!(ff.series_tag, None);
            assert_eq!(ff.picture.fx_seed, None);
            assert_eq!(ff.pair_rules.get("swat_cobras"), Some(&Value::Bool(true)));
            assert_eq!(ff.pair_rules, pair_rules(left, right));
            assert_eq!(ff.captions, draft_captions(left, right));
            assert_eq!(ff.app_version, env!("CARGO_PKG_VERSION"));
            // a written file that reads back as the same fight
            assert_eq!(FightFile::parse(&ff.to_toml()).as_ref(), Ok(&ff));
        }
    }

    #[test]
    fn the_two_orders_are_different_fights() {
        let a = starter("FLUTE", "TROMBONE").expect("flute vs trombone");
        let b = starter("TROMBONE", "FLUTE").expect("trombone vs flute");
        assert_ne!(a.search_key(&[false, false]), b.search_key(&[false, false]));
    }

    #[test]
    fn a_pair_without_music_says_so() {
        let message = |left: &str, right: &str| match starter(left, right) {
            Err(ContentError::FightFile(e)) => e.message,
            other => panic!("{left} vs {right}: {other:?}"),
        };
        assert_eq!(message("FLUTE", "PIANO"), "No music is written for Flute vs Piano yet.");
        assert_eq!(
            message("BASS DRUM", "ALTO SAX"),
            "No music is written for Bass drum vs Alto sax yet."
        );
        assert_eq!(message("FLUTE", "FLUTE"), "No music is written for Flute vs Flute yet.");
        assert_eq!(message("", ""), "No music is written for  vs  yet.");
        assert_eq!(
            message("flute", "trombone"),
            "No music is written for Flute vs Trombone yet."
        );
    }

    #[test]
    fn captions_are_drafted_from_emoji_and_tags() {
        let c = draft_captions("FLUTE", "TROMBONE");
        assert_eq!(
            c.caption,
            "Flute vs Trombone \u{1FA88}\u{1F3BA} Comment the next match-up \u{1F447}"
        );
        assert_eq!(
            c.instagram,
            "#ballfight #flute #trombone #bandkids #marchingband #musicmemes #musician #pixelart #lowpoly #gamedev \
             #satisfying #music"
        );
        assert_eq!(
            c.tiktok,
            "#fyp #ballfight #flute #trombone #bandkid #musictok #pixelart #satisfying #bandtok #music"
        );
        assert_eq!(
            c.shorts,
            [
                "Flute vs Trombone",
                "Flute vs Trombone, every hit in key. Comment the next match-up",
                "#shorts #ballfight #flute #trombone #music #pixelart"
            ]
        );
        // the other order swaps the names and the emoji
        let r = draft_captions("TROMBONE", "FLUTE");
        assert_eq!(
            r.caption,
            "Trombone vs Flute \u{1F3BA}\u{1FA88} Comment the next match-up \u{1F447}"
        );
        assert!(
            r.instagram
                .starts_with("#ballfight #trombone #flute #bandkids #marchingband #musicmemes")
        );
    }

    #[test]
    fn captions_for_fighters_without_cards() {
        let c = draft_captions("ALTO SAX", "BASS DRUM");
        assert_eq!(c.caption, "Alto sax vs Bass drum Comment the next match-up \u{1F447}");
        assert_eq!(
            c.instagram,
            "#ballfight #altosax #bassdrum #musicmemes #musician #pixelart #lowpoly #gamedev #satisfying #music"
        );
        assert_eq!(c.shorts[2], "#shorts #ballfight #altosax #bassdrum #music #pixelart");
    }
}
