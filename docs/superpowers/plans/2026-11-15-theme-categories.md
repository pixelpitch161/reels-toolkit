# Theme Categories and Recommendations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Group Studio's theme picker into categories, recommend three themes for the pair from the fighters' traits, and offer a recommended tempo.

**Architecture:** The trait data lives in one table module in `bf-content` (`fit.rs`), not on the 56 card statics. The app copies it into its own roster model (`RosterEntry`, `ThemeEntry`), where the scoring lives, so the fixtures can test it without the engine. The grouped dropdown is a new widget in `buttons.rs`, and both screens use it through a single `theme_picker` helper.

**Tech Stack:** Rust, egui (kittest UI tests), the existing `bf-content` / `bf-app` crates.

**Spec:** `docs/superpowers/specs/2026-11-15-theme-categories-design.md`

## Global Constraints

- Recommendations never hide or remove a theme. `Roster::themes_for` (ruling M3-R2) stays the only filter.
- Picking a theme or a pair never changes the tempo. Only the "Use" button does.
- No colour for the recommended state: headings are `MUTED` text. No ★ glyph either, because the app font (IBM Plex Sans) has none and egui panics on a missing glyph.
- The trait vocabulary is closed: brassy, reedy, airy, bowed, plucked, keys, electric, percussive, high, low, folk, jazz, classical, rock, club, sinister.
- Categories, in order: Jazz & Latin, Folk & World, Cinematic & Dark, Rock & Punk, Electronic & Club.
- Windows is the only target. UI tests run on the PC, and the content and model unit tests also run anywhere.

## Review Focus

- A pair where both fighters share no traits with any theme: still exactly three recommended (ties go to category order), never zero.
- A pair whose playable list is under three themes (refusals): recommend all of them, never more than are offered.
- Fixture rosters with a theme missing from the fit table (empty category): it shows under a trailing "Other" heading and stays pickable.
- Each theme is listed once (recommended ones only under "Recommended for this pair"), so tests picking "Dark tango" by label find exactly one option.
- A theme whose tempo range, once snapped, would leave the theme's `bars` range: the recommended bars are clamped into `bars`.

---

### Task 1: Fit table in bf-content

**Files:**
- Create: `studio/crates/bf-content/src/fit.rs`
- Modify: `studio/crates/bf-content/src/lib.rs` (add `pub mod fit;`)

**Interfaces:**
- Produces: `pub const TRAITS: [&str; 16]`, `pub const CATEGORIES: [&str; 5]`, `pub struct FighterFit { pub traits: &'static [&'static str], pub energy: u8 }`, `pub struct ThemeFit { pub category: &'static str, pub likes: &'static [(&'static str, u8)], pub tempo: (f64, f64) }`, `pub fn fighter_fit(name: &str) -> Option<&'static FighterFit>`, `pub fn theme_fit(name: &str) -> Option<&'static ThemeFit>`.

- [ ] Tests in `fit.rs`: `every_roster_fighter_has_a_fit` (each `shown_order()` name: 3 to 5 traits, all in `TRAITS`, energy 1 to 3); `every_theme_has_a_fit` (each `theme_names()` entry: category in `CATEGORIES`, likes in `TRAITS` with weight 1 or 2, `tempo.0 < tempo.1`, and both ends inside the bpm the card's `bars` give: `bars * 240 / 61`).
- [ ] Write the tables (the builder's draft, close enough for the eye test). Run `cargo test -p bf-content fit`, which should pass. Commit.

### Task 2: Roster model and scoring

**Files:**
- Modify: `studio/crates/bf-app/src/model/roster.rs`, `src/engine/content.rs`, `src/seam/sample.rs`, `src/state/tests.rs`

**Interfaces:**
- Consumes: Task 1.
- Produces: `RosterEntry { traits: Vec<String>, energy: u8, .. }`, `ThemeEntry { category: String, likes: Vec<(String, u8)>, tempo: (f64, f64), .. }`, `Roster::recommended(&self, fighters: &[String; 2]) -> Vec<&ThemeEntry>` (at most 3, from `themes_for`, best first), `Roster::grouped(&self, fighters) -> Vec<(String, Vec<&ThemeEntry>)>` (categories in `CATEGORIES` order, then "Other"; empty groups left out), `Roster::recommended_bpm(&self, fighters, theme: &str) -> Option<f64>`.

Scoring: fit = the sum of the theme's like weights for the traits the fighter has. Score = 2·min + max, plus 100 for each fighter whose `home.theme` is this theme. Ties go to group order. Tempo = `lo + (avg_energy − 1) / 2 · (hi − lo)`.

- [ ] Tests in `roster.rs`: three are recommended, all playable; a home theme is always among them; fewer playable themes than three gives all of them; a pair with no shared traits still gets three, in group order; every theme appears in exactly one group; `recommended_bpm` with energy 1+1 gives `tempo.0` and with 3+3 gives `tempo.1`.
- [ ] Fill the new fields in `engine/content.rs` from `bf_content::fit`, and give `sample.rs` and `state/tests.rs` empty or plain values. Run `cargo test -p bf-app --lib`, which should pass. Commit.

### Task 3: Grouped theme picker on both screens

**Files:**
- Modify: `studio/crates/bf-app/src/widgets/buttons.rs` (new `dropdown_grouped`), `src/screens/matchup.rs`, `src/screens/music.rs`
- Create: `studio/crates/bf-app/src/screens/theme_picker.rs` (registered in `screens/mod.rs`)
- Test: `studio/crates/bf-app/tests/ui/buttons.rs`

**Interfaces:**
- Produces: `pub enum MenuEntry { Heading(String), Item { index: usize, label: String } }`, `pub fn dropdown_grouped(ui, name: &str, entries: &[MenuEntry], value: &str, chosen: usize, width: f32) -> Option<usize>` (returns the picked item's `index`; headings are 20 tall, `MUTED` small text, and not clickable), `pub fn theme_picker(ui, roster: &Roster, draft: &Draft, width: f32) -> Option<String>` (a "Recommended for this pair" heading with its items, then each group without those).

- [ ] UI test `grouped_dropdown_skips_headings`: headings are not list options, and clicking an item returns its index.
- [ ] Swap both screens' dropdown calls for `theme_picker`. The existing music tests (`choosing_a_theme_pushes`, `a_theme_the_pair_cannot_play_is_not_offered`) must keep passing unchanged. Commit.

### Task 4: Recommended tempo

**Files:**
- Modify: `studio/crates/bf-app/src/screens/matchup.rs` (`tempo_controls`)

- [ ] When `recommended_bpm` snaps (`nearest_bars`) to bars other than `draft.bars`, the note cell shows a flat button "Use {bpm:.0} bpm" (pushing `SetBars`), and otherwise the note shows "{bars} bars fit in 61 seconds". UI test `use_recommended_tempo_pushes_set_bars`. Commit.

### Task 5: Verify and ship

- [ ] Run `cargo test -p bf-content -p bf-app --lib`, `cargo clippy`, and `cargo fmt --check` here. Run the full UI suite on the PC through the build thread. Push, and update draft PR #4.
