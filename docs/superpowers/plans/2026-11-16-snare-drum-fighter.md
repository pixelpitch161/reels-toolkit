# SNARE DRUM fighter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add SNARE DRUM (weapon PARADIDDLE, FLAM, RIMSHOT, BUZZ ROLL) as the Studio's next from-scratch fighter.

**Architecture:** Same shape as MARIMBA (the nearest template; read its commits 52801ff8, d592d375, 4a67f453,
873ab951, 98ae1980): a new weapon kind `sticks` with its three abilities in one engine module, a roster entry in
`roster_new.json`, a `fighters/snare_drum/` content module, and the model JSON generated from web photos on Aedan's PC.

**Tech Stack:** Rust workspace in `studio/` (bf-core, bf-sim, bf-content, bf-app); Python stdlib for the model script.

**Spec:** `docs/superpowers/specs/2026-11-16-snare-drum-fighter-design.md`

## Global Constraints

- Work on branch `claude/project-thread-bx9f03`, based on `claude/project-thread-h102zt`; merge into it at the end.
- Never edit `projects/`, `reelkit/` or `series/`. Never edit an existing fighter's numbers or a fight; only add.
- Windows-only app: no Linux CI. bf-app only builds on Windows (type-check in the cloud with
  `--target x86_64-pc-windows-gnu`, run its tests on the PC).
- Engine maths through `bf_core::dmath` (no `f64::sin` etc.; the `no_std_maths` test enforces it).
- Lenient balance check: a handful of quick fights, no long searches. Do not install the app.
- On the PC: share the one Rust build folder other threads use and clear what this thread adds when done; don't touch
  other threads' working copies.

## Review Focus

- A jab while the ball is stunned or in BUZZ ROLL: no stroke starts (stunned balls start no attack; BUZZ ROLL pauses the weapon).
- RIMSHOT armed but nothing lands for `dur_beats`: it disarms quietly (no stun later).
- FLAM and RIMSHOT on the same jab: both apply (flam hit after the rimshot hit).
- SNARE DRUM vs SNARE DRUM mirror match and vs a copier (IDENTITY CRISIS copies abilities): no side assumptions.
- Determinism: the same seed gives the same recording twice (the quick-fight test asserts it).

---

### Task 1: Engine kind `sticks` with FLAM, RIMSHOT, BUZZ ROLL (cloud)

**Files:**
- Create: `studio/crates/bf-core/src/roster/kinds/sticks.rs`, `studio/crates/bf-core/src/roster/abilities/{flam,rimshot,buzz_roll}.rs`
- Create: `studio/crates/bf-sim/src/kinds/sticks.rs`, `studio/crates/bf-sim/tests/snare_drum_fights.rs`
- Modify: the same registration points MARIMBA touched in 52801ff8 (`roster/mod.rs`, `roster/abilities.rs`,
  `kinds/mod.rs`, `bf-sim/src/{abilities/mod.rs,fight.rs,record.rs,schedule.rs}`, `tests/sockets*.rs`,
  `sockets_order.txt`), and `studio/crates/bf-content/data/roster_new.json` (append SNARE DRUM with the spec's
  starting numbers, `corner` "red", `pp` "SNARE DRUM", `idiom` "rudiment").

**Interfaces:**
- Produces: `StickworkCfg`-style typed settings (fields as the spec names them); record rows `STICKS` per frame
  (per stick: side 0 = left / 1 = right, extension 0..1, tip x, y); events `stroke`, `flam`, `rimshot`, `buzz_roll`;
  hit kinds `stick`, `flam`, `buzz`.

- [ ] Write `snare_drum_fights.rs`: quick fights of SNARE DRUM vs DRUMS, MARIMBA and itself (a few seeds) assert: the
  fight ends, both sides land hits, `stroke` events follow R L R R L R L L, a `rimshot` event is followed by the enemy
  stunned, every ability name appears at least once across the seeds, and the same seed twice gives equal recordings.
- [ ] Run `cargo test -p bf-sim --test snare_drum_fights`: fails (kind unknown).
- [ ] Implement the kind and abilities as the spec says; mirror `mallets.rs` for structure.
- [ ] Run `cargo test -p bf-sim -p bf-core`: passes (sockets tests updated for the new rows).
- [ ] Commit.

### Task 2: Model from web photos (Aedan's PC)

**Files:**
- Create: `studio/scripts/models/snare_drum_model.py`, `studio/crates/bf-content/data/models/SNARE DRUM.json`,
  `studio/reference/models/snare_drum/{NOTES.md,photos/*,render_*.png}`

- [ ] Download 3 to 5 snare drum photos from Wikimedia Commons (marching and concert snares: shell, hoops, lugs,
  snare strainer, head), note sources and what each shows in NOTES.md, like MARIMBA's.
- [ ] Write the generator in MARIMBA's style (stdlib only, same JSON format, centred and scaled to a unit sphere,
  sparkle cherry shell rgb(255, 48, 128) with a darker sparkle, chrome hoops and lugs, white coated head), run it,
  render previews the way MARIMBA's were rendered and compare against the photos; refine once.
- [ ] Commit and push.

### Task 3: Content module and wiring (cloud)

**Files:**
- Create: `studio/crates/bf-content/src/fighters/snare_drum/{mod,look,sound,voice,card,state,personality}.rs`
- Modify: the points MARIMBA touched in 4a67f453 / 873ab951 / 98ae1980 (`fighters/mod.rs`, `registry.rs`,
  `cards.rs`, `composer.rs`, `fit.rs`, `pair_rules.rs`, `personality.rs`, `starter.rs`, `themes/mod.rs` if needed,
  content tests' fighter counts, `bf-app/src/engine/content.rs`).

- [ ] Update the content tests' counts for one more fighter; run `cargo test -p bf-content`: fails.
- [ ] Write the module per the spec's look and sound (state reads `STICKS`; voice as specified; motifs invented),
  `MESH` from Task 2, fit traits: energy 3, crisp/marching/percussive.
- [ ] Run `cargo test -p bf-content` and `cargo check -p bf-app --target x86_64-pc-windows-gnu`: pass.
- [ ] Commit and push.

### Task 4: App tests on the PC, merge

- [ ] On the PC, fetch the branch; run `cargo test -p bf-app`; update roster-count tests and the snapshots that show
  the fighter count, as CONTRABASS CLARINET did in ed6c230c; render one short SNARE DRUM fight preview and look at it.
- [ ] Fetch the latest `claude/project-thread-h102zt`, merge it into this branch, resolve `roster_new.json`, NEW
  lists, counts and registry conflicts by keeping every fighter; recheck the colour against the siblings; re-run the
  failed/affected tests, then the full suite once.
- [ ] Merge into `claude/project-thread-h102zt` and push. No install.
