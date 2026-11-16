# CORNET fighter Implementation Plan

> **For agentic workers:** executed natively in its thread (Aedan: once the design is approved, build to merge without
> waiting). Steps use checkbox syntax.

**Goal:** add CORNET (PLUNGER MUTE, WA-WA, DERBY HAT, STOP-TIME) to BALLFIGHT Studio, offered in the app.

**Architecture:** one new engine module `bf-sim/src/kinds/plunger.rs` holds the weapon and the two new abilities
(`derby`, `stoptime`), as `wrecking.rs` holds SKANK; typed settings in `bf-core/src/roster/kinds/plunger.rs` and
`bf-core/src/roster/abilities/{derby,stoptime}.rs`; one new ball timer `muffle` read by `stunned()`. The fighter folder
`bf-content/src/fighters/cornet/` holds look, voice, part, card, personality. Model made on Aedan's PC from web photos.

**Tech Stack:** Rust workspace `studio/` (bf-core, bf-sim, bf-content, bf-picture, bf-sound).

**Spec:** `docs/superpowers/specs/2026-11-16-cornet-fighter-design.md`

## Global Constraints

- No existing fight changes: `cargo test -p bf-sim --test replay` stays green.
- Never edit `projects/`, `reelkit/`, `series/`. Windows only; no Linux CI. bf-app type-checked with `--target x86_64-pc-windows-gnu`.
- Colour (232, 120, 140), dark (138, 52, 70); unique on the build branch at merge time.
- Lenient balance: fights end with a K.O. most of the time, CORNET wins 25 to 75 % against a few opponents.

## Review Focus

- CORNET against itself (both sides plunger, both muffled): no panic, fights end.
- A copied DERBY HAT / STOP-TIME (EUPHONIUM's IDENTITY CRISIS) on a ball without a plunger: runs, no panic.
- A plunger stuck on an enemy that goes untouchable (swan, dive, rewind, shield): no damage, pops cleanly.
- Fight ends (K.O.) while the cup is out or the hat is in the air: record and look don't panic.
- A muffled enemy still moves (only attacks stop).

---

### Task 1: Engine: roster types, plunger kind, derby and stoptime, muffle

**Files:** create `bf-core/src/roster/kinds/plunger.rs`, `bf-core/src/roster/abilities/{derby,stoptime}.rs`,
`bf-sim/src/kinds/plunger.rs`, `bf-sim/tests/cornet_quick.rs`; modify the dispatch tables in `bf-core/src/roster/{mod,kinds/mod,abilities}.rs`,
`bf-sim/src/{fight,ball,schedule,record}.rs`, `bf-sim/src/kinds/mod.rs`, `bf-sim/src/abilities/mod.rs`; roster entry in
`bf-content/data/roster.json`.

**Interfaces (produces):** `WeaponCfg::Plunger(PlungerCfg)`; `AbilityKind::{Derby, Stoptime}`; `Ball::muffle: f64`;
events `plunger` (fire), `plunger_stick`, `wah`, `plunger_pop`, `derby`, `derby_land`, `muffle`, `stoptime`, `stop`, `break`;
record key `PLG` (frames, 2, 6): cup x, y, state (0 none, 1 out, 2 stuck, 3 back), reach, derby x, derby y (NaN none).

- [ ] Test `cornet_quick`: CORNET against 6 opponents from both corners, seeds 1 to 20: no panic, every fight has a
  `plunger_stick` and a `wah`, every one where CORNET's slots fire has `derby` and `stoptime` events, K.O. in most.
- [ ] Test: muffled ball keeps moving (`muffle > 0`, its position changes, `stunned()` true).
- [ ] Implement; run `cargo test -p bf-core -p bf-sim`; replay test unchanged. Commit.

### Task 2: Content: look, voice, part, card, personality, registry

**Files:** `bf-content/src/fighters/cornet/*` (shape of `bari_sax`), `bf-content/src/fighters/mod.rs`, `registry.rs`
(`PORTED`), `starter.rs`, `fit.rs`, the score order, `bf-picture` title pattern if required, `docs/ballfight/RULES.md` colour table.

- [ ] Look draws cord, cup, wah rings, WA-WA ring, derby, STOP-TIME streaks from `PLG` and events; `MESH = None` until Task 3.
- [ ] Voice: mellow brass with a wah filter; part plays its line, a wah on each `wah` event; personality with invented motifs.
- [ ] Card texts; PORTED; starter pairing and home theme; fit traits. Workspace tests that build on Linux pass;
  `cargo check -p bf-app --target x86_64-pc-windows-gnu`. Commit, push, draft PR into the build branch.

### Task 3: Model on Aedan's PC from web photos

- [ ] Remote Control session on the PC (own clone/branch, shared build folder): download cornet photos, build
  `bf-content/data/models/CORNET.json` (format 1 faces, unit sphere), set `MESH`, render a preview, run bf-content
  and bf-app tests, push.

### Task 4: Merge

- [ ] Rebase/merge the latest build branch, resolve roster/registry conflicts, recheck colour uniqueness, full tests once,
  merge into `claude/project-thread-h102zt`, push.
