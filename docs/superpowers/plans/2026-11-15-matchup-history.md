# Matchup History Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** BALLFIGHT Studio keeps a permanent history of every distinct fight it makes (seeded with the pre-app episodes), lists it in a new Matchups tab with an Uploaded tick, and shows uploaded-fight counts on the Match-up roster chips for the Left fighter.

**Architecture:** A new `History` model (`model/history.rs`) owned by the view model, saved as `config/history.toml` through the existing background `Writer`, like settings and the queue. A finished render (`JobUpdate::Done`, now carrying the winner and the finish time) upserts a row. A new `View::Matchups` screen reads and edits it through actions; `roster_chip` gains an optional count.

**Tech Stack:** Rust, egui/eframe, serde + toml, egui_kittest snapshots (blessed on Windows), existing `store` safe-write helpers.

**Spec:** `docs/superpowers/specs/2026-11-15-matchup-history-design.md`

## Global Constraints

- Windows only; do not run or add Linux checks. Tests run on Aedan's PC in `F:\!Ai Stuff\InstrumentBattleMachine` (see the build thread's setup); share one Rust target folder and clear it when done.
- Ball Studio look: neutral greys, IBM Plex, colour only belongs to fighters, never to states. Use the existing `theme` tokens (`TEXT`, `MUTED`, `FAINT`, `LINE_SOFT`, `small`, `body`, `section`) and Library's header/footer sizes (header 40, footer 46, side pad 16).
- Retired fighters: names exactly in `bf_core::fightfile::RETIRED` (GUITAR, SAX) never appear as fighters anywhere the app writes or shows. **The import leaves out the two episodes that use them** (#27 Guitar vs Piano, #29 Trumpet vs Sax); this narrows the design's "keep them in the list" to respect that standing rule (flagged to Aedan with the plan).
- Never edit `projects/`, `reelkit/` or `series/`. Fights are never edited.
- Files written whole via `store::write_atomic`; an unreadable file is moved aside with `store::move_aside(path, "corrupt")`, never deleted.
- Row identity: same `fighters` (in order, left then right) and same `seed` is the same fight. Pairing counts ignore sides.
- Chip counts count `uploaded == true` rows only; the tab lists every row.

## Review Focus

1. **An old `queue.json`** (jobs saved before this change, without `fighters`) must still load: new `JobSpec`/`JobState` fields are `#[serde(default)]`; a job without fighters records nothing when it finishes. Test in Task 3.
2. **Re-rendering a fight already in the history** (same fighters and seed, already ticked uploaded) updates `made` and `winner` and keeps `uploaded` true; it never adds a second row. Test in Task 1.
3. **A damaged `history.toml`** is moved aside to `history.corrupt-1.toml`, the app starts from the starter rows, and a notice names the kept copy. Test in Task 2.
4. **A history row naming a fighter the roster does not have** (a later rename, a hand-edited file) shows its name title-cased in the tab, with a grey swatch, and never lights a chip or panics. Test in Task 4.
5. **Swapped sides:** Trombone (Left) vs Flute, uploaded, makes Trombone's chip count for Flute and Flute's count for Trombone both 1. Test in Task 1.

---

### Task 1: The history model

**Files:**
- Create: `studio/crates/bf-app/src/model/history.rs`
- Modify: `studio/crates/bf-app/src/model/mod.rs` (add `pub mod history;`)

**Interfaces:**
- Produces:
  - `#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] pub struct FightRecord { pub id: u64, pub fighters: [String; 2], pub seed: Option<u64>, pub winner: Option<String>, pub made: u64, pub uploaded: bool }` (`fighters` and `winner` are roster names such as `"BASS CLARINET"`; `made` is seconds since 1970 UTC).
  - `#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)] pub struct History { pub fights: Vec<FightRecord>, pub next_id: u64 }`
  - `impl History`:
    - `pub fn record(&mut self, fighters: [String; 2], seed: u64, winner: Option<String>, made: u64) -> u64` — upsert, returns the row id.
    - `pub fn set_uploaded(&mut self, id: u64, uploaded: bool) -> bool` (false when no such row)
    - `pub fn delete(&mut self, id: u64) -> bool`
    - `pub fn newest_first(&self) -> Vec<&FightRecord>` (by `made` descending, then `id` descending)
    - `pub fn uploaded_counts(&self, fighter: &str) -> BTreeMap<String, u32>` — for every opponent of `fighter` (either side), how many uploaded rows.
    - `pub fn summary(&self) -> (usize, usize, usize)` — (fights, distinct pairings ignoring sides, not uploaded).

- [ ] **Step 1: Write the failing tests** in `model/history.rs` `#[cfg(test)] mod tests`:
  - `recording_a_new_fight_adds_a_row`: empty history, `record(["FLUTE","TROMBONE"], 2210, Some("FLUTE"), 100)` → 1 row, id 0, `uploaded == false`, `next_id == 1`.
  - `the_same_fighters_and_seed_update_the_row` (Review Focus 2): record, `set_uploaded(id, true)`, record again with `made` 200 and winner `TROMBONE` → still 1 row, `made == 200`, winner `TROMBONE`, `uploaded == true`, same id.
  - `a_new_seed_or_swapped_sides_is_a_new_row`: seed 2211 → 2 rows; `["TROMBONE","FLUTE"]` seed 2210 → 3 rows.
  - `counts_use_uploaded_fights_only_and_ignore_sides` (Review Focus 5): rows FLUTE–TROMBONE (uploaded), TROMBONE–FLUTE (uploaded), FLUTE–TUBA (not uploaded) → `uploaded_counts("FLUTE") == {"TROMBONE": 2}`, `uploaded_counts("TROMBONE") == {"FLUTE": 2}`, `uploaded_counts("TUBA")` empty.
  - `summary_counts_fights_pairings_and_not_uploaded`: the three rows above → `(3, 2, 1)`.
  - `newest_first_orders_by_made_then_id` and `delete_and_set_uploaded_report_missing_ids` (both false for id 99).
- [ ] **Step 2: Run** `cargo test -p bf-app --lib model::history` — expect FAIL (module missing).
- [ ] **Step 3: Implement** the types and methods above. `record` matches on `fighters` and `Some(seed)` exactly.
- [ ] **Step 4: Run** the same command — expect PASS.
- [ ] **Step 5: Commit** `feat(history): the fight history model`.

### Task 2: Saving, loading and the starter rows

**Files:**
- Create: `studio/crates/bf-app/src/store/history.rs`
- Create: `studio/crates/bf-app/assets/history_start.toml` (generated, committed)
- Create: `studio/scripts/history_from_tracker.py` (one-time converter, committed for the record)
- Modify: `studio/crates/bf-app/src/store/mod.rs` (`pub mod history;`, `Paths::history()`, `Writer::history(&History)`, the writer thread writes it like the queue)

**Interfaces:**
- Consumes: `History`, `FightRecord` (Task 1).
- Produces:
  - `Paths::history(&self) -> PathBuf` = `config/history.toml`.
  - `pub fn starter() -> History` — parses `include_str!("../../assets/history_start.toml")`; panics in tests only if the bundled file is bad (a test pins it).
  - `pub fn load_history(path: &Path) -> (History, Option<Notice>)` — missing → `starter()` and no notice; unreadable or not valid → `move_aside(path, "corrupt")`, `starter()`, and a notice `"The matchup history could not be read, so it starts again from the earlier episodes. The old file was kept as {name}."`.
  - `pub fn save_history(path: &Path, h: &History) -> io::Result<()>` via `write_atomic`.
  - `Writer::history(&self, h: &History)` — same pattern as `Writer::queue`.

- [ ] **Step 1: Generate the starter file.** Write `history_from_tracker.py` reading sheet `Episodes` of `ballfight-matchup-tracker.xlsx` (path as argument; columns: B Left, C Right, E Winner, F Seed, H Date made). Skip rows whose Left or Right is in `{"GUITAR","SAX"}`. Each row: `id` in sheet order from 0, `made` = the date at 12:00 UTC, `uploaded = true`. Emit TOML matching `History`'s serde form with `next_id` = row count. Run it with `python -I` on `/mnt/project-files/matchups/ballfight-matchup-tracker.xlsx` → expect 27 rows (episodes #28 and #30–#56, with Horn vs Trombone at seeds 140 and 630).
- [ ] **Step 2: Write the failing tests** in `store/history.rs`:
  - `the_starter_holds_the_27_earlier_episodes`: `starter().fights.len() == 27`, all `uploaded`, no fighter in `bf_core::fightfile::RETIRED`, every fighter name is in `bf_content::roster()`, and the row with seed 1442 is `["BAGPIPES","ACCORDION"]` winner `ACCORDION`.
  - `a_missing_file_starts_from_the_starter`: temp dir → `(starter(), None)`.
  - `a_saved_history_loads_back`: save then load → equal, no notice.
  - `a_damaged_file_is_kept_aside_and_the_starter_used` (Review Focus 3): write `"not toml ["` → returns `starter()`, notice text contains `history.corrupt-1.toml`, `files_in(dir)` holds `history.corrupt-1.toml` and no `history.toml` until saved.
  - In `store/mod.rs` tests: `paths.history() == root.join("config").join("history.toml")`, and a writer asked for the history writes it (mirror the existing queue writer test).
- [ ] **Step 3: Run** `cargo test -p bf-app --lib store::history store::tests` — expect FAIL.
- [ ] **Step 4: Implement** the functions, `Paths::history`, and the writer slot (`State.history: Option<History>`, taken and written like `queue`).
- [ ] **Step 5: Run** the same command — expect PASS.
- [ ] **Step 6: Commit** `feat(history): save and load the history, starting from the earlier episodes`.

### Task 3: A finished render records the fight

**Files:**
- Modify: `studio/crates/bf-app/src/model/queue.rs` (`JobSpec.fighters`, `JobUpdate::Done`/`JobState::Done` new fields)
- Modify: `studio/crates/bf-app/src/engine/jobs.rs` (`enqueue` fills `fighters`; `render` fills `winner` from `rec.winner` and `finished_at` from `SystemTime::now()`)
- Modify: `studio/crates/bf-app/src/seam/fake.rs` (sample engine fills them; `finished_at` a fixed `1_763_208_000`, fighters from the draft)
- Modify: `studio/crates/bf-app/src/state/mod.rs` (`ViewModel.history: History`, `Effect::SaveHistory(History)`, boot takes the history)
- Modify: `studio/crates/bf-app/src/state/queue_run.rs` (`on_job`)
- Modify: `studio/crates/bf-app/src/app.rs` (`load_history` at boot beside `load_queue`, its notice into `notices`, `Boot.history`, `carry_out` sends `SaveHistory` to `writer.history`)
- Test: `studio/crates/bf-app/src/state/queue_run_tests.rs`, `model/queue.rs` tests

**Interfaces:**
- Consumes: `History::record`, `Writer::history`, `load_history` (Tasks 1–2).
- Produces:
  - `JobSpec { …, #[serde(default)] pub fighters: Option<[String; 2]> }`
  - `JobUpdate::Done { folder, seconds, winner: Option<usize>, finished_at: u64 }`; `JobState::Done { folder, seconds, #[serde(default)] winner: Option<usize>, #[serde(default)] finished_at: u64 }` (`apply` copies them).
  - `ViewModel.history: History`; `Effect::SaveHistory(History)`.

- [ ] **Step 1: Write the failing tests:**
  - `model/queue.rs`: `a_queue_saved_before_fighters_were_kept_still_loads` (Review Focus 1): a JSON queue text without `fighters`, `winner`, `finished_at` deserializes, `fighters == None`.
  - `queue_run_tests.rs`: `a_finished_render_adds_the_fight_to_the_history` — queue the sample draft, drive the job to `Done { winner: Some(0), finished_at: 1_763_208_000, .. }` → `vm.history` has a row with the draft's fighters, its seed, winner = left fighter's name, `made == 1_763_208_000`, `uploaded == false`, and `fx` holds `Effect::SaveHistory`.
  - `a_failed_or_cancelled_render_records_nothing` — history unchanged, no `SaveHistory`.
  - `a_job_without_fighters_records_nothing` (Review Focus 1).
- [ ] **Step 2: Run** `cargo test -p bf-app --lib queue_run_tests model::queue` — expect FAIL (fields missing).
- [ ] **Step 3: Implement.** In `on_job`, when `is_done` and the job's `spec.fighters` is `Some` (read before `trim`), call `history.record(fighters, spec.seed, winner.map(|i| fighters[i].clone()), finished_at)` and push `Effect::SaveHistory(self.vm.history.clone())`. Update every other `JobUpdate::Done`/`JobState::Done` construction and match (`seam/fake.rs`, `engine/jobs.rs` tests, `fixtures/render.rs`) with the new fields.
- [ ] **Step 4: Run** the Step 2 command, then `cargo test -p bf-app --lib` — expect PASS.
- [ ] **Step 5: Commit** `feat(history): a finished render records the fight`.

### Task 4: The Matchups tab

**Files:**
- Create: `studio/crates/bf-app/src/screens/matchups.rs`
- Create: `studio/crates/bf-app/src/fixtures/matchups.rs`; Modify `fixtures/mod.rs`
- Modify: `studio/crates/bf-app/src/model/mod.rs` (`View::Matchups`), `screens/mod.rs`, `screens/shell.rs` (toolbar toggle "Matchups" just left of "Library"), `app.rs` (`View::Matchups => matchups::show(ui, vm, out)`), `state/mod.rs` (actions, `MatchupsView`), `state/queue_run.rs` or a new `state/history.rs` for the handlers
- Test: `studio/crates/bf-app/tests/ui/matchups.rs` (+ `mod matchups;` in `tests/ui/main.rs`), snapshots `matchups_1440x900.png`, `matchups_1280x800.png`; re-bless `shell_*` snapshots that show the toolbar

**Interfaces:**
- Consumes: `History` methods (Task 1), `ViewModel.history`, `Effect::SaveHistory` (Task 3), `store::library::local_date`, `wording::display_name`.
- Produces:
  - `View::Matchups`.
  - `#[derive(Clone, Debug, PartialEq, Default)] pub struct MatchupsView { pub selected: Option<u64>, pub asking_delete: bool }`, field `ViewModel.matchups: MatchupsView`.
  - `Action::SelectFight(u64)`, `Action::SetUploaded { id: u64, uploaded: bool }`, `Action::AskDeleteFight`, `Action::KeepFight`, `Action::DeleteFight(u64)`. `SetUploaded` and `DeleteFight` push `Effect::SaveHistory`; `DeleteFight` clears the selection and `asking_delete`; selecting another row clears `asking_delete`.
  - `pub fn show(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>)` in `screens/matchups.rs`.

**Layout (copy fixed here):**
- Header 40: "Matchups" (`section`), then `"{fights} fights · {pairings} pairings · {n} not uploaded"` (`small`, `MUTED`; "not uploaded" part dropped when 0; "1 fight"/"1 pairing" singular).
- Table, scrolls, rows 28 tall with a 1 px `LINE_SOFT` line between, column header row in `small` `MUTED`: Made (`local_date(made)`), Left, Right (each a 12 px colour swatch from the roster, grey `[0x80;3]` when unknown, then `display_name`), Winner (`display_name` or "—"), Seed (or "—"), Uploaded (a checkbox; click pushes `SetUploaded`). Clicking elsewhere in a row pushes `SelectFight`; the selected row gets the `SELECTED` fill.
- Footer 46: nothing selected → `"Pick a fight to delete it."` (`MUTED`); selected → `"{Left} vs {Right}, seed {seed}"` and a "Delete" button (`AskDeleteFight`); asking → `"Delete {Left} vs {Right}, seed {seed}? This cannot be undone."` with "Delete" (`DeleteFight(id)`) and "Keep it" (`KeepFight`).
- Empty history: one centred line `"No fights yet. Render one and it appears here."`.
- Accessible names: the checkbox `"Uploaded, {Left} vs {Right}, seed {seed}"`.

- [ ] **Step 1: Write the failing state tests** (`state/tests.rs` or a new `state/history_tests.rs`): ticking pushes `SaveHistory` with `uploaded == true`; `AskDeleteFight` then `DeleteFight` removes the row and clears the selection; `KeepFight` keeps it; `SetUploaded` for an unknown id changes nothing and saves nothing.
- [ ] **Step 2: Write the failing UI tests** in `tests/ui/matchups.rs` using the fixture `fixtures::matchups::vm()` (the starter history plus one new unticked Flute vs Trombone row selected): the header reads `"28 fights · 26 pairings · 1 not uploaded"` (Horn vs Trombone and Flute vs Trombone are already pairings in the starter); clicking the checkbox named `"Uploaded, Flute vs Trombone, seed 2210"` pushes `SetUploaded { uploaded: true, .. }`; the footer's "Delete" then shows the question; the fixture `fixtures::matchups::unknown_fighter()` (one row with fighter `"KAZOO"`, Review Focus 4) shows "Kazoo" with a grey swatch and no panic; `fixtures::matchups::empty()` shows the empty line; the toolbar's "Matchups" pushes `Go(View::Matchups)`; snapshots at 1440x900 and 1280x800.
- [ ] **Step 3: Run** `cargo test -p bf-app --test ui matchups` and the state tests — expect FAIL.
- [ ] **Step 4: Implement** the view, actions, handlers, toolbar toggle and fixtures. Follow `screens/library.rs` for header/footer structure and scroll style.
- [ ] **Step 5: Bless** `UPDATE_SNAPSHOTS=true cargo test -p bf-app --test ui matchups::` and `shell::`; look at each new PNG before committing.
- [ ] **Step 6: Run** `cargo test -p bf-app` — expect PASS.
- [ ] **Step 7: Commit** `feat(history): the Matchups tab`.

### Task 5: Counts on the Match-up roster chips

**Files:**
- Modify: `studio/crates/bf-app/src/widgets/chips.rs` (`roster_chip` gains `count: Option<u32>`)
- Modify: `studio/crates/bf-app/src/screens/matchup.rs` (`chip_grid` computes `vm.history.uploaded_counts(&vm.draft.fighters[0])` once per frame and passes each chip its count)
- Modify: every other `roster_chip` caller (pass `None`)
- Modify: `studio/crates/bf-app/src/fixtures/matchup.rs` (history with uploaded fights for the fixture's Left fighter)
- Test: `studio/crates/bf-app/tests/ui/matchup.rs`; re-bless `matchup_*` snapshots

**Interfaces:**
- Consumes: `History::uploaded_counts` (Task 1), `ViewModel.history` (Task 3).
- Produces: `pub fn roster_chip(ui: &mut Ui, display: &str, color: [u8; 3], side: Option<usize>, count: Option<u32>, available: bool, width: f32) -> ChipOut`.

**Behaviour:** `count` `Some(n)` with n ≥ 1 and `side` `None` → draws `"×{n}"` at the chip's right edge in `small` `MUTED` (where "Left"/"Right" would sit); "Left"/"Right" wins when `side` is set. Accessible name gains `", fought {n} time"`/`" times"`. The name's ellipsis room accounts for the count. Counts only for the Left fighter; nothing when the Left corner is empty.

- [ ] **Step 1: Write the failing UI tests:** with the fixture's Left fighter having 2 uploaded fights with Trombone and 1 unticked with Tuba, the Trombone chip's accessible name is `"Trombone, fought 2 times"`, Tuba's is `"Tuba"`; after `SwapSides` in the state the counts follow the new Left fighter (state test: counts computed from `draft.fighters[0]`).
- [ ] **Step 2: Run** `cargo test -p bf-app --test ui matchup::` — expect FAIL.
- [ ] **Step 3: Implement** the parameter, drawing, accessible name and the screen's lookup.
- [ ] **Step 4: Bless** `UPDATE_SNAPSHOTS=true cargo test -p bf-app --test ui matchup::`; look at the PNGs.
- [ ] **Step 5: Run** `cargo test -p bf-app` and `cargo clippy -p bf-app --all-targets -- -D warnings` — expect PASS, no warnings.
- [ ] **Step 6: Commit** `feat(history): uploaded-fight counts on the roster chips`.

### Task 6: Ship it

- [ ] **Step 1:** Run `cargo test -p bf-app --release --test retired` — expect PASS (no retired name leaks through the history).
- [ ] **Step 2:** Bump the app version the way the build thread does (`studio/crates/bf-app/Cargo.toml`, `version.rs` line), push the branch, and install the new build on Aedan's PC with the app's own update path.
- [ ] **Step 3:** Launch it on the PC, open Matchups, check the 27 earlier episodes are listed and ticked, and that the Match-up screen shows counts for the Left fighter. Clear the shared target folder.
