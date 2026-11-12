# BALLFIGHT Studio, milestone 3: the kit roster (Implementation Plan)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the other 18 kit fighters and the other 13 kit themes, each proven against the Python kit the way
FLUTE, TROMBONE and the big top were in milestone 1, so that every kit pair can be balanced, searched, played and
rendered in the Studio, and the app offers all 20 kit fighters and all 14 kit themes.

**Architecture:** Nothing new at the top: the recording stays the seam, `bf-sim` gains one module per weapon kind and
ability kind, `bf-content` one folder per fighter and per theme. Three things change underneath, all in foundation
tasks that land before any port: (1) where reference data lives (a small committed set plus a regenerated, pinned
cache), (2) the theme model and the fight file (format 2: any kit chord list, mix overrides, theme options, title
opt-ins, optional bed hooks), and (3) "sockets": every place in the shared engine files where a later kind attaches
is wired once, so 18 ports can run in six clones without touching each other's files.

**Tech stack:** as milestones 0 to 2 (`studio/rust-toolchain.toml`, the pinned workspace; Python 3.13.18 with numpy
2.5.3, scipy 1.18.1, Pillow 12.3.0 for the export tool only). No new crates.

**Spec:** `docs/studio/2026-11-10-ballfight-studio-design.md` (sections 5 to 10 and 12 to 14). Section numbers (§)
below refer to it. The milestone 0 and 1 plan (`2026-11-10-ballfight-studio-plan-m0-m1.md`) is the template: its
Tasks 11 to 14 (one fighter in the fight engine), 22 to 24 (the looks) and 25 to 27 (voices, parts, the theme,
`validate`) are what each port here repeats. Its rulings R1 to R42 (in `docs/studio/m1-measurements.md` §11 and the
build's ledger) still hold.

**Scope:** spec milestone 3 only. Not here: the ten older fighters and their twelve themes (milestone 4), the music
system (milestone 5: layered arrangements, personalities, the composer; every theme here gets only its classic
arrangement, exactly as the kit plays it), packaging (milestone 6). Milestone 2's last tasks (32 preview speed, 35 the
real app end to end) must be merged before Task 60, which is the only task here that edits `bf-app`.

---

## 1. What success looks like

Milestone 3 is done when all of these hold, each proven by a named test or command:

1. **Every kit fighter is offered.** `bf_content::available()` returns the 20 kit fighters of §10 and
   `bf_content::theme_names()` the 14 kit themes. The Match-up screen shows no "not available yet" chip for any
   roster fighter (the older ten are not in the roster until milestone 4). Tasks 13, 20, 27, 41, 60.
2. **Each fighter is the kit's fighter.** For each of the 18: its fights agree with the kit's to 1e-6 for at least
   `PREFIX_FLOOR_S = 10.0` s of fighting in every scenario; every event, hit and ability kind it has fires inside
   that window at the kit's time; its statistics against FLUTE over 2,000 seeds are inside milestone 1's bars; its
   look draws the kit's bytes on at least 96 frames of two fights and its title card; its voice and its part match
   the kit's (the soundtrack's bytes). Template F, Tasks 7 to 26.
3. **Each theme is the kit's bed.** For each of the 13: its voicings equal the kit's for every chord the harmony
   parses; its classic bed alone, under silent fighters, gives the kit's 16-bit soundtrack bytes on its reference
   episodes, at a second tempo and key, and for every theme option the kit has. Template T, Tasks 28 to 40.
4. **Each kit episode is the kit's video.** For each of the 17 kit episode configs not yet proven (Table S4): the
   kit's own recording of that episode draws the kit's frames (at least 40, plus 16 title-card frames), its
   soundtrack is the kit's `audio.wav` byte for byte, every check gives the kit's verdict and numbers, the Studio's
   statistics at the episode's handicap match the kit's over 4,000 seeds, and a Studio-native fight of the pair
   renders from the command line to an mp4 that passes all 15 checks. Template P, Tasks 42 to 58, 61.
5. **Every pair of kit fighters can start.** `bf_content::starter(a, b)` returns a valid fight file for all 380
   ordered pairs of two different kit fighters; the 17 kit configs start from the kit's own music and frozen
   handicap; a theme a pair's fighters cannot play under is not offered and is refused by `validate`. Task 59.
6. **Nothing old moved.** Milestone 1's gates (`golden_frames`, `flute_look`, `trombone_look`, `stems`,
   `checks_parity`, `prefix`, `coverage`, `replay`, `stats_parity`, `render_is_reproducible`) pass unchanged;
   `studio/replay/hashes.txt` and `render_hashes.txt` are byte-identical; `python tools/ballfight_replay_hash.py`
   prints what it printed before; `git status --short projects reelkit series` is empty. Task 63.
7. **The reference set is inside its budget**: at most 147 MB tracked (`budget` export and `bf-testdata`'s
   `budget` test), every cache file pinned. Tasks 1, 63.

## 2. Global Rules

Every task's requirements include this section. Milestone 0 and 1's Global Constraints still apply where they are
not restated; where they differ, this section wins (in particular, ruling R1: this Windows PC is the reference
platform and nothing runs on Linux).

- **Nothing in `projects/`, `reelkit/` or `series/` is edited.** The export tool imports the kit read-only (run-time
  patches allowed, as `pp_bridge` and the look exports already do). The kit's replay guard stays green (on this PC
  its kit-episode lines are checked by running the child commands one by one, as `m1-measurements.md` §12 says).
- **Retired fighters.** They are never offered and never ported; their names come only from
  `bf_core::fightfile::RETIRED`. Two Pixel Pitch *voice ids* carry a retired fighter's name: ELECTRIC's roster `pp`
  (`RETIRED[0]`, the `electric60_lowpoly` voice) and the alto voice the mambo bed plays (`RETIRED[1]`,
  `altosax60_lowpoly`). Under ruling R14 these stay internal ids in code and data; no screen, fight file, caption,
  folder name or log line the Studio writes may contain either name (Task 60 tests this). ALTO SAX, TENOR SAX and
  BARI SAX are different fighters and are fine.
- **Honest fights.** Fights are never edited; the user chooses seeds; the damage handicap (and the shared damage
  scale) is the only balance lever; pair rules come only from the series' table (Task 3). Reach, timing and
  abilities never change per pair.
- **Determinism.** As milestone 1: `f64` in Python's order, no `mul_add`, every transcendental through
  `bf_core::dmath` (the source scan in `bf-core` enforces it), `float32` stays `f32`, Python scalar maths ported as
  scalar maths and numpy array maths as numpy computes it (pairwise sums, the shortcuts of `**`, `np.round`), no
  result that depends on hash-map order or thread count, every random draw in the kit's order and number, including
  draws whose value is unused. Ruling R25 stands: no fused dot product, even though numpy's is fused on this PC.
- **The bars, never loosened.** Fight: positions, HP and events to 1e-6 over the prefix window, floor
  `PREFIX_FLOOR_S = 10.0` s of fighting. Frames: identical RGB; an exception goes in the fighter's or episode's
  `exceptions.toml` with a reason, at most 2 levels on at most 0.5 % of pixels, reviewed by eye. Soundtrack: the
  16-bit master is the kit's bytes; stems within -120 dBFS, documented exceptions to -80 dBFS. Statistics: the bars of
  `m1-measurements.md` §5 (win share 3 points, median K.O. 0.7 s, mean lead changes 0.25 or 0.06 in release, fit rate
  1.5 or 0.5 points in release, 10th percentile K.O. 1 s, no K.O. 2 points). A last-bit maths gap (Python's C
  runtime against `dmath`) is reported with the frame or sample, the stage, the numbers and the cause; the controller
  decides. A break before the prefix floor is a bug until the events around it show otherwise.
- **The kit is the arbiter.** Port from the Python at the pinned versions, not from this plan's summaries. Where this
  plan and the kit disagree, the kit wins and the report says where.
- **Reference data** follows Section 4 (committed set, pinned cache, budgets). No test may skip silently: a
  gate that needs the cache fails with the export command to run when a pinned file is missing or differs.
- **Ownership** follows Section 3: a port task only creates or edits the files its entry lists; shared files are
  changed only by the foundation tasks and the register tasks.
- **Tests first** (write the failing test, see it fail for the right reason, implement, see it pass), `cargo fmt`
  and `cargo clippy --workspace --all-targets -- -D warnings` clean for the crates touched, test output free of
  warnings.
- **One PC, six lanes.** Each lane builds with `CARGO_BUILD_JOBS=3`, runs tests with `--test-threads=3` and gives the
  export tool's process pools 3 workers, so six lanes share the 16 threads. Every M3 test checks exact results, never
  timing; timings are measured only in Task 62, with the other lanes idle.
- **Commits** as `env.md` and ruling R7 say. No model names in commits, code or docs. Each task's commit
  subjects are given in its entry.

## 3. How six lanes work without merge conflicts

**Lanes.** Six clones, `F:\!Ai Stuff\InstrumentBattleMachine\.lanes\m3-1` to `m3-6`, on branches `lane/m3-1` to
`lane/m3-6`, used exactly as `lanes.md` describes (merge the build branch that `lanes.md` names before starting and
before the last test run, push the lane branch, the controller reviews and merges). The scratch folder for briefs,
reports and lane notes is `.superpowers/sdd/2026-11-11-ballfight-studio-plan-m3-kit-roster/`.

**The ownership rule.** Every file in the workspace is in exactly one of three classes:

1. **Owned by one port task**: a fighter's folder and test files, a weapon or ability kind's module, a theme's folder,
   a pair's fight file and test file, a title pattern's file, the export module and the reference folder of that
   fighter, theme or pair. Foundation Tasks 3 and 5 create every one of these as a stub before the first port starts,
   so a port only edits files that already exist and are its own. The full list is in each template's "Files".
2. **Shared, append-only through register tasks**: `bf_content::registry::PORTED`, `themes::PORTED_THEMES`,
   `starter::STARTERS`, `studio/README.md`'s roster line, `docs/studio/HANDOFF.md`. Only the register tasks (13, 20,
   27, 41) and the integration tasks (59 to 63) edit them, one commit each.
3. **Shared engine files** (`bf-sim/src/{fight,schedule,record,ball}.rs`, `bf-sim/src/{kinds,abilities}/mod.rs`,
   `bf-core/src/roster/mod.rs`, `bf-sound/src/{traits,score,mix}.rs`, `bf-picture/src/{look.rs,ring/*,title/*}`,
   `bf-content/src/{lib,fighters/mod,themes/mod,cards,picture,soundtrack}.rs`, `tools/studio_export/{__main__,
   manifest,common,m3refs}.py`): changed only by the foundation tasks. A port that finds a socket's signature wrong
   may change that one call line and its stub in its own file, says so in its report, and the controller merges it
   by hand. A port that needs anything else in a shared file (a renderer helper the context lacks, a new trait
   method) stops with NEEDS_CONTEXT naming it; the controller adds it in a small "helper" commit on the build branch,
   which every lane then merges.

**Reference manifests are sharded** (Task 1): each `studio/reference/fighters/<slug>/`, `themes/<name>/` and
`episodes/<pair>/` folder has its own `manifest.json`, so no two lanes write the same manifest. The top-level manifest
keeps milestone 0 and 1's files.

**Reviews.** Every task is reviewed before it is merged (implementer report, reviewer, at most five fix rounds, then
the controller merges), as in milestones 1 and 2. The foundation tasks get the most careful review because every
port builds on them; before Phase 1 starts the controller runs a pre-flight scan of the sockets against `fight.py`
(Section 7). Register tasks are small and are checked by one reviewer reading the diff.

## 4. Storage: where reference data lives

**Decision: a small committed reference set for the gates that always run, plus a cache that the export tool
regenerates on this PC and whose every file is pinned by SHA-256 in the committed set. Not Git LFS.**

Why: the set is at 133.5 MB of the 150 MB the spec allows (`m1-measurements.md` §10). Milestone 1's fighter pair
alone holds about 90 MB of stems, voices, DSP vectors and frames. Eighteen fighters and thirteen themes at that rate
would be several hundred MB. Git LFS would hold them, but: every lane clone's `origin` is the main folder on this PC,
so LFS objects would need a second transfer path between local clones (not tried on this PC); a private GitHub repo
has an LFS storage and bandwidth quota; milestone 6's Update button (`git pull`) would pull them to the owner's PC for
nothing. The kit is deterministic on this PC with the pinned libraries, so a file the export tool regenerates is the
same file; pinning its SHA-256 proves that it is. The looks of milestone 1 were already proven this way (frame and
hook-log hashes, ruling R39 and R40), and it worked.

**What is committed** (always-run gates, readable without Python):

- **Hashes of frames, title-card frames, title-hook outputs, sprites, hook logs and draw logs** Why it can be small: the
  Studio's frames must be the kit's bytes, so a SHA-256 decides.
- **The SHA-256 of the 16-bit master, and per-bus RMS of every 100 ms block** Why it can be small: milestone 1's master
  was the kit's file byte for byte; RMS catches a wrong bus at 0.01 dB.
- **Coverage tables (`kind, scenario, seed, rules, time, fields`)** Why it can be small: the Rust run of a scenario
  needs only its seed and rules.
- **Pooled statistics (a dozen numbers per pairing)** Why it can be small: the statistics test compares pooled figures.
- **Voice fingerprints (per note: length, peak, RMS of each 10 ms block, as hex floats)** Why it can be small: float64
  voices agree to about -260 dBFS, so their block RMS agrees far inside 1e-9 dB.
- **Voicing vectors, check numbers, roll rows, title-hook vectors** Why it can be small: a few KB each.
- **One Python recording per fighter and one per kit episode config (130 to 160 KB each)** Why it can be small: the
  picture and sound gates draw from the kit's own run.
- **`pins.json` per folder: path under the cache, SHA-256, bytes, the export that makes it** Why it can be small: proves
  a regenerated cache is the pinned one.

**What is cached** (`studio/reference/.cache/m3/`, git-ignored, regenerated by `python -m tools.studio_export cache
<folder>`): scenario recordings for prefix agreement, statistics rows, the second look fight per fighter, stems and
premaster windows, voice note vectors, stage dumps. Cache-backed tests are `#[ignore]` by default, run by name in
every review and in Task 63's closing run, and fail (never skip) with the command to run when a pinned file is
absent or differs. Re-pinning (`cache --repin`) is a controller decision, recorded in the ledger with the reason.

**Budgets** (enforced by the `budget` export and `bf-testdata`'s `budget` test, Task 1):

- **`fighters/<slug>/`** Holds: one recording; `fight`, `look`, `sound`, `voice`, `checks`, `pins`, `manifest`
  (`.json`); `exceptions.toml`; Budget: 300 KB.
- **`themes/<name>/`** Holds: its reference configs' recordings (one or two); `theme`, `voicings`, `pins`, `manifest`;
  Budget: 120 KB + 160 KB a recording.
- **`episodes/<pair>/`** Holds: `episode`, `frames`, `sound`, `checks`, `stats`, `pairs`, `pins`, `manifest`;
  `exceptions.toml`; Budget: 100 KB (260 KB for Tasks 45, 46).
- **everything tracked under `studio/reference/`** Budget: 147 MB.
- **`picture_title_optins/`** (Task 5: the title opt-in cards and the beam's hull log, about 110 KB, top-level
  manifest) has no budget of its own: it is counted in the row above, the whole set.

If every folder used its whole budget the set would end at 133.5 + 18 x 0.3 + 13 x 0.12 + 15 x 0.16 + 17 x 0.1 +
2 x 0.16 = about 145 MB (FLUTE's and TROMBONE's folders point at milestone 1's recordings instead of copying them,
about 60 KB each); the 147 MB cap leaves 2 MB for what this estimate misses. Models
(`bf-content/data/models/*.json`, content compiled into the app, about 50 to 300 KB each) are not reference data and
are outside this budget; Task 62 reports their total. **Milestone 4 cannot add to the committed
set**: its plan moves the per-fighter recordings to the cache too (the recommended answer to Q12 in Section 8).

---

## 5. Survey: the 18 fighters

Measured from the code in this clone: `projects/ballfight/fight.py` (5,982 lines), `kit/fighters/*.py` (lines with
blank lines; "Look / Sound / module" are the lines of the two classes and of the module-level helpers),
`data/roster.json`, `series/instrument-history/*_lowpoly.py` and `projects/which_instrument_wins/pp_bridge.py`.
"Ported" below means present in the Studio after milestone 1: weapon kinds `striker`, `blade` (thrust); ability kinds
`haste`, `dance`, `wave`, `shockwave` (with `slow`), `slam`.

### Table S1: the fight engine

- **PIANO** Weapon kind: `fight.py` lines: `roll` 5164-5283; projectile flight and hits in `move` 3235-3278; `spawn_key`
  370; New ability kinds (trigger at): `sustain` 2941, `arc` 2852 (emitted 3099-3113), `runoff` 2944; State others read,
  records: keys `K`, `ROLL`, `HOLD`; its line feeds the fight (`kit/story.py roll_rows`); Size: L.
- **TRUMPET** Weapon kind: `fight.py` lines: `combo` 898-966; New ability kinds (trigger at): `beam` 2870
  (`ab_beam_charge` 2990, `ab_beam_fire` 3001, `beams_step` 857-897); State others read, records: `HA`, `WAVES`; Size:
  L.
- **ORGAN** Weapon kind: `fight.py` lines: `line` (hazards 1323-1485: `hazard_beat`, `fire_line`, `hazards_step`); New
  ability kinds (trigger at): `shield` 2847; Size: L.
- **CLARINET** Weapon kind: `fight.py` lines: `jump` 2338-2424; New ability kinds (trigger at): `stun` 2889 (hit in
  `effects_step` 3072-3074), `rhapsody` 2930; State others read, records: `stun` (every grid's `_stunned`); Size: M.
- **OBOE** Weapon kind: `fight.py` lines: `tune` 2425-2525; New ability kinds (trigger at): `snip` 2526-2567
  (`snip_ready` gate), `endless` 2884, `wave` option `detune` 2845 / 3069; State others read, records: `detune_t`
  (damage 345), `DET`, `TUNE`, `SNIP`; Size: M.
- **SOUSAPHONE** Weapon kind: `fight.py` lines: `drill` 4758-5005 (tramples an enemy's baskets: FLUTE's); New ability
  kinds (trigger at): `flash_spin` 2977, `company_front` 2980; State others read, records: `march`, `DRILL`; Size: L.
- **TENOR SAX** Weapon kind: `fight.py` lines: `crowd` 4513-4757 (with `runway`, `stagedive` steps); New ability kinds
  (trigger at): `runway` 2966, `subtone` 2969, `stagedive` 2973; State others read, records: `strut`, `dive` (no hit,
  knock or clash), `CROWD`, `THROWS`; Size: L.
- **ALTO SAX** Weapon kind: `fight.py` lines: `spotlight` 4369-4512; New ability kinds (trigger at): `octave` 2958,
  `altissimo` 2961, `encore` 2964; State others read, records: `SPOT`; Size: M.
- **BASS CLARINET** Weapon kind: `fight.py` lines: `anchor` 2568-2658; New ability kinds (trigger at): `twin` 2897,
  `lights_out` 2767-2800; `snip` (OBOE's); State others read, records: `anc`, `shadow` (intangible), `ANC`, `LO`; Size:
  M.
- **TUBA** Weapon kind: `fight.py` lines: `swallow` 1045-1130 (`swallow_rollers` rule 1100); New ability kinds (trigger
  at): `shield`; State others read, records: `held` (rides inside, no clash); Size: M.
- **CONTRABASS** Weapon kind: `fight.py` lines: `walk` 3965-4040 (walker branch of the clash 3190-3231); New ability
  kinds (trigger at): `slap` 4202-4219 (`slap_ready`), `stampede` 4302-4368; State others read, records: `walking`,
  `walker`, `WALK`; Size: M.
- **BASSOON** Weapon kind: `fight.py` lines: `tiptoe` 3280-3429; New ability kinds (trigger at): `hook` 3430-3476
  (`hook_ready`; a hopper's follow-up 3470), `mountain` 3423; State others read, records: `air`, `hopper`, `hop`, `HOP`,
  `HOOK`; Size: L.
- **HORN** Weapon kind: `fight.py` lines: `echo` 2196-2281; New ability kinds (trigger at): `shield`, `flick` 2282-2337;
  State others read, records: `ECHO_G`, `ECHO_N`, `ECHO_R`, `FLICK`; Size: L.
- **ACOUSTIC** Weapon kind: `fight.py` lines: `rake` (`fire_rake` 528, `aim_blend`, `rake_lateral`, `sweep_front`; the
  `dc` 16th counter 5751, 5964-5974; the rake branches of `dash_cone_step` 457 and `surf_rake_step` 585-700); New
  ability kinds (trigger at): `whip`, `sweep` (`ab_x33` 566); State others read, records: shared code with the older
  surf / spout kinds; Size: L.
- **ELECTRIC** Weapon kind: `fight.py` lines: `lightning` 3845-3922; New ability kinds (trigger at): `fork` 2933,
  `tapping` 2936, `smash` 3923-3964 (`splinters_step`); State others read, records: `smashed` (still, intangible,
  `rebuild` event in the timers); Size: M.
- **CELLO** Weapon kind: `fight.py` lines: `grapple` 4041-4201 (a walker slips the grab 4147); New ability kinds
  (trigger at): `vibrato` 2955, `prelude` 4220-4243, `swan` 4244-4301; State others read, records: `grabbed`, `pinned`,
  `swan` (airborne), `GRAB`; Size: L.
- **BASS DRUM** Weapon kind: `fight.py` lines: `roll_charge` 5284-5431 (not while `held`, 5411); New ability kinds
  (trigger at): `heartbeat`, `drum_roll`, `cannons` 5432-5494 (slams with `how = 'cannon'`); State others read, records:
  `rolling`, `RCHG`; Size: L.
- **EUPHONIUM** Weapon kind: `fight.py` lines: `mortar` 2659-2766 (`predict` round an anchored enemy, `shells_step`);
  New ability kinds (trigger at): `copy` 2801-2822 (fires the enemy's last ability on its own ball), `barrage` 2721;
  State others read, records: `SHELLS`; `swat_shells` rule (TROMBONE's lance, 1243-1271); Size: L.

Kinds several fighters share, and so go to a foundation task or to one owner: `shield` (TUBA, HORN, ORGAN: trivial,
Task 3); `snip` (OBOE owns it, BASS CLARINET and a copied snip use it); `_stunned`, the per-ball states of `move`,
`damage` and `knock`, and the "ready" gates of `abilities_step` (Task 3's sockets). Kinds of the older fighters that
`fight.py` also holds (`volley`, `pulse`, `beam` weapon, `orbit`, `combo`'s sibling `flail`, `inflate`, `bite`,
`tether`, `pack`, `rattle`, `wrecking`, `rests`, `surf`, `spout`, `quake`, `offbeat`, `snap`, `joke`, `melody`,
`double_stop`, `lash`, `rewind`, `clamp`, `charge`) stay `SLOT` comments for milestone 4.

### Table S2: the picture

- **PIANO** Look / module lines: 407 / 50; What is new for the renderer: `shadows_free`, `items_free`, `canvas_after`,
  `glows`; needs `P.shade` (Task 5); VS-card panel: plain (no pattern anywhere).
- **TRUMPET** Look / module lines: 489 / 301; What is new for the renderer: the largest look;
  `scipy.ndimage.gaussian_filter` on its beam masks (Task 4); own sprite and atlas; `ball`; VS-card panel: its look's
  `title_pattern`.
- **ORGAN** Look / module lines: 290 / 254; What is new for the renderer: `ConvexHull` at two sites (a pipe's outline,
  the CATHEDRAL dome): Qhull risk; `light`, `fx_end`, `block_spark`; VS-card panel: `title.py` `_organ_pipes`.
- **CLARINET** Look / module lines: 182 / 118; What is new for the renderer: `teleports`, `shimmer`, `after_grade`;
  `_reeds.py` helpers (Task 4); VS-card panel: `_clarinet_keys`.
- **OBOE** Look / module lines: 244 / 115; What is new for the renderer: `sprite_rolls` and `ball` (its debuffs tilt and
  sour the enemy's ball); `quiet_hits`; VS-card panel: `_oboe_reeds`.
- **SOUSAPHONE** Look / module lines: 311 / 79; What is new for the renderer: own sprite and `title_closeup` through
  CONTRABASS's `padded_sprite` (Task 4); a ghost sprite; VS-card panel: look's `title_pattern`.
- **TENOR SAX** Look / module lines: 352 / 66; What is new for the renderer: `hides_hp`; moves its ball's `p` and `lift`
  during the stage dive; VS-card panel: look's `title_pattern`.
- **ALTO SAX** Look / module lines: 208 / 47; What is new for the renderer: `ConvexHull` of the lamp cone (Qhull risk);
  `light`, `glows`; VS-card panel: look's `title_pattern`.
- **BASS CLARINET** Look / module lines: 279 / 184; What is new for the renderer: `sink` state; `after_grade` dims the
  ring except what other looks keep (`dim_keep`: no kit look has one); `st['anc']` is a tuple (recompute it, do not
  widen `BallState`); VS-card panel: `_bassclarinet`; `TITLE_LINES`.
- **TUBA** Look / module lines: 308 / 169; What is new for the renderer: `canvas_first` (light under both fighters'
  decals), `hides_hp`, `items_ball`; VS-card panel: plain.
- **CONTRABASS** Look / module lines: 281 / 83; What is new for the renderer: own sprite and `title_closeup`
  (`padded_sprite`); VS-card panel: look's `title_pattern`.
- **BASSOON** Look / module lines: 317 / 219; What is new for the renderer: `speed_lift`; the `sx`, `sy`, `air_bias`,
  `paint_h` state keys (the ring reads them already); VS-card panel: `_bassoon`.
- **HORN** Look / module lines: 384 / 301; What is new for the renderer: `canvas_overlay` (the copper thread),
  `block_burst`, `shadows_free`; VS-card panel: `_horn_coils`.
- **ACOUSTIC** Look / module lines: 271 / 221; What is new for the renderer: `sprite_k` 0.86, rakes on the canvas;
  VS-card panel: `_woodgrain`.
- **ELECTRIC** Look / module lines: 419 / 112; What is new for the renderer: `hides_hp`; rope sparks through
  `P.rope_points` and `P.side_ends` (Task 5); VS-card panel: look's `title_pattern`.
- **CELLO** Look / module lines: 336 / 107; What is new for the renderer: `quiet_hits`; `lift` while SWAN SONG flies;
  VS-card panel: look's `title_pattern`.
- **BASS DRUM** Look / module lines: 318 / 98; What is new for the renderer: own sprite, `title_closeup`, `ball_sprite`
  from variant builds of `bassdrum_lowpoly` (the drum on its side); VS-card panel: look's `title_pattern`.
- **EUPHONIUM** Look / module lines: 283 / 169; What is new for the renderer: IDENTITY CRISIS: the enemy's own look,
  made for the euphonium's side with the enemy's numbers (`dup`, Task 5's `setup_as`), draws the copied kinds;
  `ball_sprite`, `extra_sprites` (the enemy's model, tinted); VS-card panel: `_euphonium`.

Title patterns: of `title.py`'s 15 unported `PATTERNS`, eight are kit fighters' (ORGAN, HORN, ACOUSTIC, CLARINET,
OBOE, BASS CLARINET, EUPHONIUM, BASSOON) and arrive with them; the other seven belong to milestone 4 and stay
`PatternNotPorted`. Eight fighters bring their own `title_pattern` in their look (as FLUTE did). PIANO and TUBA have
neither and draw the plain panel. Qhull: ORGAN and ALTO SAX call `ConvexHull` (plus the ring's beam, milestone 1);
Section 8, Q4, says how a merge is found.

### Table S3: the sound

- **PIANO** Sound lines: 138; Voice: `pp_bridge` entry, module (lines, `synth` at): `piano_lowpoly` `inst_note('grand')`
  (902); What its part calls or reads: `keys`; the roll's rows are its line.
- **TRUMPET** Sound lines: 197; Voice: `pp_bridge` entry, module (lines, `synth` at): `trumpet_lowpoly` (405, 297):
  `brass`; What its part calls or reads: `LINES['TRUMPET']` makes the punches a melody.
- **ORGAN** Sound lines: 170; Voice: `pp_bridge` entry, module (lines, `synth` at): `organ60_lowpoly` (1,240, 952):
  `pipe`; clears `_NOTE` per note; What its part calls or reads: role `bass` = a pedal under each pipe row; `bed.boom`.
- **CLARINET** Sound lines: 130; Voice: `pp_bridge` entry, module (lines, `synth` at): `clarinet60_lowpoly` (416, 264):
  `reed`; What its part calls or reads: `thin_37`.
- **OBOE** Sound lines: 118; Voice: `pp_bridge` entry, module (lines, `synth` at): `oboe_lowpoly` (330, 320):
  `double_reed`; What its part calls or reads: plays a sour twin in the *victim's* voice (`M.SOUNDS[self.enemy]`).
- **SOUSAPHONE** Sound lines: 205; Voice: `pp_bridge` entry, module (lines, `synth` at): `sousaphone_lowpoly` (191,
  180): `sousa_tone`, `trumpet.bend`; What its part calls or reads: `prepare` + `line_override` (ported) hands the enemy
  a line.
- **TENOR SAX** Sound lines: 176; Voice: `pp_bridge` entry, module (lines, `synth` at): `tenorsax_lowpoly` (132, 121):
  `tenor_tone`; What its part calls or reads: `M.roar`.
- **ALTO SAX** Sound lines: 150; Voice: `pp_bridge` entry, module (lines, `synth` at): `altosax_lowpoly` (371, 360):
  `alto_tone` on `altosax60.sax_tone`.
- **BASS CLARINET** Sound lines: 185; Voice: `pp_bridge` entry, module (lines, `synth` at): `bassclarinet_lowpoly` (438,
  427): `bass_reed`; What its part calls or reads: `bed.erupt` when the bed has it (horror trap); role `bass` vs an
  octave up.
- **TUBA** Sound lines: 170; Voice: `pp_bridge` entry, module (lines, `synth` at): `tuba60_lowpoly` (447, 332):
  `trumpet.brass`; What its part calls or reads: `bed.boom`; its voice is also the hot jazz and second line beds' tuba.
- **CONTRABASS** Sound lines: 182; Voice: `pp_bridge` entry, module (lines, `synth` at): `contrabass_lowpoly` (415,
  402): pizz under 0.5 s, else arco; What its part calls or reads: its voice is also the mambo bed's tumbao.
- **BASSOON** Sound lines: 153; Voice: `pp_bridge` entry, module (lines, `synth` at): `bassoon_lowpoly` (368, 357):
  `bassoon_reed` (called directly); What its part calls or reads: `thin_37`.
- **HORN** Sound lines: 161; Voice: `pp_bridge` entry, module (lines, `synth` at): `horn60_lowpoly` (414, 298):
  `trumpet.brass`; What its part calls or reads: `LINES['HORN']`, the extra line `LINES['HORN THEME']`;
  `bed.trailer_hit`, else `bed.boom`.
- **ACOUSTIC** Sound lines: 157; Voice: `pp_bridge` entry, module (lines, `synth` at): `acoustic_lowpoly` (230, 110):
  `piano.ks_pluck`; What its part calls or reads: its voice is also the stadium rock bed's unplugged strums (`u_gtr`).
- **ELECTRIC** Sound lines: 192; Voice: `pp_bridge` entry, module (lines, `synth` at): `electric60_lowpoly` (173, 114),
  voice id `RETIRED[0]`: `ks_pluck`.
- **CELLO** Sound lines: 228; Voice: `pp_bridge` entry, module (lines, `synth` at): `cello60_lowpoly` (295, 208):
  `violin.bowed`; What its part calls or reads: `stops()`: its rope slams stop the tango band.
- **BASS DRUM** Sound lines: 228; Voice: `pp_bridge` entry, module (lines, `synth` at): `bassdrum_lowpoly` (238, 222):
  velocity >= 0.9 is the boom, MIDI < 24 a roll; What its part calls or reads: reads its look's cannon list (recompute
  it in the part).
- **EUPHONIUM** Sound lines: 243; Voice: `pp_bridge` entry, module (lines, `synth` at): `euphonium_lowpoly` (293, 282);
  What its part calls or reads: a ghost of the enemy in the enemy's voice; `bed.boom`; `bed.press_roll` when present.

Voice blocks more than one voice calls (Task 4): `trumpet_lowpoly.brass`, `bend`, `fingering` (TRUMPET, TUBA, HORN,
SOUSAPHONE), `piano_lowpoly.ks_pluck` and `additive` (ACOUSTIC, ELECTRIC, PIANO), `altosax60_lowpoly.sax_tone`
(ALTO SAX, the mambo bed's alto). Sounds that read their own look's state (BASS CLARINET 537, BASS DRUM 500 and 554,
SOUSAPHONE 492) recompute it from the recording in the fighter's folder, as FLUTE's `Charmer` does.

### Table S4: the 13 themes and the kit configs

- **`baroque_storm`** Lines: 246; Buses: strings, harpsi, drums; New in the bed: `HARP_RH` / `VLN` tables with the
  chord-tone rule as fallback; runs offset from D; Reference configs: `examples/clarinet_vs_oboe` (140, Dm, 8-chord
  loop); Size: S.
- **`bossa_noir`** Lines: 408; Buses: drums, bass, plucked strings, keys; New in the bed: jazz chords
  (m7, m6, maj7, m7b5); `lfilter`; `hit`, `boom`; Reference configs: `ballfight_flute_vs_clarinet` (132, Dm harmonic);
  Size: M.
- **`brass_trailer`** Lines: 337; Buses: strings, taiko, braam, tremolo, pad (+ the sequel's); New in the bed: option
  `TRAILER.sequel`; `trailer_hit`, `boom`; Reference configs: `examples/trombone_vs_horn` (4-chord loop);
  `ballfight_horn_vs_trombone_rematch` (sequel, `MIDDLE`); Size: M.
- **`dark_disco`** Lines: 280; Buses: drums, bass, strings, pad, shimmer; New in the bed: **no `boom`**; Reference
  configs: `ballfight_altosax_vs_tenorsax` (118, Am); Size: S.
- **`dark_drumline`** Lines: 298; Buses: drums, pit; New in the bed: options `DRUMLINE.bass_drums`, `after_ko`;
  `press_roll`, `boom`; reads the story; Reference configs: `ballfight_flute_vs_sousaphone`;
  `ballfight_bassdrum_vs_tuba` (options on); Size: M.
- **`dark_tango`** Lines: 369; Buses: bandoneon, chopped strings, perc; New in the bed: `stop_times` / `apply_stops`:
  fighters' `stops()` cut the bed's buses; reads `FIGHTERS`, `LINES`, `ROLES`; Reference configs:
  `ballfight_cello_vs_contrabass` (118, Dm harmonic); Size: L.
- **`heist`** Lines: 307; Buses: drums, bass, pizz, vibes, tension; New in the bed: **no `boom`**; Reference configs:
  `ballfight_oboe_vs_bassoon` (118, Dm); Size: S.
- **`horror_trap`** Lines: 373; Buses: 808, drums, musicbox, strings, swell; New in the bed: reads LIGHTS OUT windows
  from the events; `erupt`, `boom`; Reference configs: `ballfight_bassclarinet_vs_bassoon` (140, Em); Size: M.
- **`prog_duel`** Lines: 271; Buses: drums, bass; Reference configs: `ballfight_piano_vs_organ` (132, Am); Size: S.
- **`hot_jazz`** Lines: 353; Buses: drums, tuba, chopped strings; New in the bed: `lfilter`; Plays voices: TUBA;
  Reference configs: `ballfight_trumpet_vs_clarinet` (140, Dm harmonic); Size: M.
- **`second_line`** Lines: 311; Buses: drums, sousa, perc; New in the bed: `press_roll`, `boom`; Plays voices: TUBA;
  Reference configs: `ballfight_euphonium_vs_trombone` (122, Em harmonic); Size: M.
- **`mambo`** Lines: 364; Buses: perc, bass, piano, sax; New in the bed: ability fills from the events; Plays voices:
  CONTRABASS, PIANO, `RETIRED[1]` (alto); Reference configs: `ballfight_trumpet_vs_trombone` (132, Gm harmonic); Size:
  M.
- **`stadium_rock`** Lines: 429; Buses: kick and two arrangements (plugged / unplugged); New in the bed: option
  `PLUGGED` (a fighter); lead times with 5 HP hysteresis, `lufs` (K-weighting), a lead envelope that rescales its buses;
  Plays voices: ACOUSTIC; Reference configs: `ballfight_electric_vs_acoustic` (126, Em); Size: L.

Every theme voices every chord: a table where it has one, the chord tones (`T.chord_tones`) otherwise. So
`big_top::has_voicing` (ruling R33) is replaced by voicing vectors (Task 2), not by more stems.

The 17 kit configs a pair gate proves (all those whose two fighters are kit fighters, beside milestone 1's
`flute_vs_trombone`), with what each fixes for the Studio (Task 59 reads this):

- **`ballfight/examples/clarinet_vs_oboe`** Theme: baroque storm; BPM: 140; Key, loop + tail: D minor, 8 + 2; damage
  scale 0.55, handicap none; series tag.
- **`ballfight/examples/trombone_vs_horn`** Theme: brass trailer; BPM: 122; Key, loop + tail: D minor, 4 + 1; damage
  scale 0.72, handicap none; series tag.
- **`ballfight_horn_vs_trombone_rematch`** Theme: brass trailer; BPM: 122; Key, loop + tail: D minor, 8 + 5; damage
  scale 0.72, handicap (1.04, 1.08); `TRAILER.sequel`, `MIDDLE`, `MIX`.
- **`ballfight/examples/trombone_vs_tuba`** Theme: big top; BPM: 126; Key, loop + tail: F minor, 4 + 2; damage scale
  1.0, handicap (0.35, 1.55); series tag.
- **`ballfight/examples/bassclarinet_vs_euphonium`** Theme: big top; BPM: 126; Key, loop + tail: C minor, 4 + 2; damage
  scale 0.68, handicap none; `TITLE_LINES`, `TITLE_BALLS`, series tag.
- **`ballfight_altosax_vs_tenorsax`** Theme: dark disco; BPM: 118; Key, loop + tail: A minor, 8 + 4; damage scale 0.75,
  handicap none.
- **`ballfight_bassclarinet_vs_bassoon`** Theme: horror trap; BPM: 140; Key, loop + tail: E minor, 8 + 2; damage scale
  1.0, handicap (0.58, 0.70); `TITLE_LINES`, `MIX`.
- **`ballfight_bassdrum_vs_tuba`** Theme: dark drumline; BPM: 126; Key, loop + tail: C minor, 4 + 2; damage scale 0.66,
  handicap (0.86, 1.0); `swallow_rollers`, `DRUMLINE`, `MIX`.
- **`ballfight_cello_vs_contrabass`** Theme: dark tango; BPM: 118; Key, loop + tail: D harmonic minor, 8 + 4; damage
  scale 0.85, handicap none.
- **`ballfight_electric_vs_acoustic`** Theme: stadium rock; BPM: 126; Key, loop + tail: E minor, 8 + 6; damage scale
  1.0, handicap (0.92, 1.33); `PLUGGED`.
- **`ballfight_euphonium_vs_trombone`** Theme: second line; BPM: 122; Key, loop + tail: E harmonic minor, 8 + 5; damage
  scale 1.0, handicap (0.44, 0.35); `swat_shells`, `MIX`.
- **`ballfight_flute_vs_clarinet`** Theme: bossa noir; BPM: 132; Key, loop + tail: D harmonic minor, 8 + 0; damage scale
  1.0, handicap (5.3, 0.36); `MIX`.
- **`ballfight_flute_vs_sousaphone`** Theme: dark drumline; BPM: 126; Key, loop + tail: C minor, 4 + 2; damage scale
  0.7, handicap none.
- **`ballfight_oboe_vs_bassoon`** Theme: heist; BPM: 118; Key, loop + tail: D minor, 8 + 4; damage scale 1.0, handicap
  (0.6, 0.85).
- **`ballfight_piano_vs_organ`** Theme: prog duel; BPM: 132; Key, loop + tail: A minor, 8 + 0; damage scale 1.0,
  handicap (1.0, 0.55).
- **`ballfight_trumpet_vs_clarinet`** Theme: hot jazz; BPM: 140; Key, loop + tail: D harmonic minor, 8 + 2; damage scale
  1.0, handicap (0.34, 0.41); `MIX`.
- **`ballfight_trumpet_vs_trombone`** Theme: mambo; BPM: 132; Key, loop + tail: G harmonic minor, 8 + 0; damage scale
  1.0, handicap (2.8, 0.40); `MIX`.

Every kit config's `CHORDS` is `[intro 2] + LOOP * n + tail`, with a 4- or 8-chord `LOOP` and a tail of 0 to 6 bars
(often a prefix of the loop). Milestone 1's fight file (2 intro chords, an 8-chord loop, no tail) fits only three.

### Table S5: links between fighters (tested at the pair gates)

- **a live lance swats a falling shell (`Rules.swat_shells`)** `fight.py`: 755, 1243-1271; Pairs it changes: TROMBONE,
  EUPHONIUM; Proven at: Task 52.
- **a swallowing bell gulps a rolling drum (`Rules.swallow_rollers`)** `fight.py`: 1100, 5411; Pairs it changes: TUBA,
  BASS DRUM; Proven at: Task 49.
- **a march tramples the enemy's baskets** `fight.py`: 4876-4888, 4972; Pairs it changes: SOUSAPHONE, FLUTE; Proven at:
  Task 54 (and Task 12's FLUTE fights).
- **a walking bass slips the grab, a slap breaks it** `fight.py`: 4147, 4211; Pairs it changes: CELLO, CONTRABASS;
  Proven at: Task 50.
- **shells and the whirl: aimed round the whirl, no knock on an anchored ball** `fight.py`: 2675-2698, 2755; Pairs it
  changes: EUPHONIUM, BASS CLARINET; Proven at: Task 46.
- **IDENTITY CRISIS copies (kinds in `COPYABLE` 2802) and the signature copy (anchor, mortar)** `fight.py`: 2804-2821;
  Pairs it changes: EUPHONIUM with anyone; Proven at: Tasks 26, 46, 52.
- **a hook spins a hopper, which follows up** `fight.py`: 3470; Pairs it changes: BASSOON with OBOE or BASS CLARINET
  (hook owner); Proven at: Tasks 48, 55.
- **the stun, the shadow, the smash, the swan, the dive: no attack, no hit, no clash** `fight.py`: 3131-3150, 3180-3187,
  325-334; Pairs it changes: everyone; Proven at: Task 3 (guards), each fighter's own task.
- **a fighter calls a bed hook the theme lacks (`boom` under dark disco or heist)** where: the parts in
  `kit/fighters/*.py`; Pairs it changes: TROMBONE, ORGAN, TUBA, EUPHONIUM, HORN under 2 themes; Proven at: Tasks 2, 59.

**Totals.** About 14,600 lines of kit Python for the fighters (their modules 11,600, their kinds in `fight.py` about
3,000), 7,300 lines of voice modules (most of each is model and video code; the voice parts are 50 to 300 lines), and
4,300 lines of beds. Sizes: S under about 600 lines and no new mechanism; M 600 to 900 lines or one new mechanism; L
over 900 lines, or two new mechanisms, or a link to another fighter's port.

---

## 6. Review Focus

Failure modes the spec implies but states no test for, most likely first. Each has a test in the task named.

1. **A theme that lacks a bed hook a fighter's part calls** (TROMBONE, ORGAN, TUBA, EUPHONIUM and HORN call
   `bed.boom`; `dark_disco` and `heist` have none). Python would raise mid-soundtrack. The Studio refuses the fight
   file with `music.theme: Heist has no boom, which the TROMBONE's part plays on a slam; pick another theme`, and
   the app does not offer the pair that theme. Tasks 2, 59, 60.
2. **A milestone 2 fight file** (format 1, from the Library) opens unchanged in meaning; a format 3 file is refused
   with `this app reads fight files up to format 2`. Task 2.
3. **A line that does not fit its loop** (`LINES` with 8 a bar over a 4-chord loop where the kit expects 32 notes;
   a PIANO line of 6 a bar): one-line error naming `music.lines.PIANO`. Tasks 2, 7.
4. **A copied ability on the copier's ball** (a copied whirl, a copied snip that waits, a copied shadow): the copy
   runs every kind's code for the other ball; a kind written for "its own" ball only fails here. Tasks 26, 46, 52.
5. **A missing or stale cache** gives a failing test with the command to run, never a pass. Tasks 1, 6.
6. **A retired name leaks** through a voice id into a caption, folder name, fight file or screen. Task 60.
7. **PIANO's music changes its fight**: a key, loop or line change invalidates the shortlist for a PIANO pair and
   for no other pair. Tasks 7, 60.
8. **A Qhull merge** in a look's hull is counted, never silently drawn another way. Tasks 5, 9, 15.

---

## 7. Order of work

Phase 0 is the six foundation tasks and must finish first. After it the ports run in up to six lanes; the table says
what each task needs merged first. "Beside" lists what may run at the same time. The controller fills free lanes in
the order of the task numbers among those whose needs are met.

- **1 Storage and the export framework** Can run beside: 2, 3, 5; Notes: Python lane.
- **2 Fight file format 2, the theme model** Can run beside: 1, 3.
- **3 Engine sockets** Can run beside: 1, 2, 4, 5; Notes: the largest foundation; pre-flight scan after it.
- **4 Shared look and voice helpers** Needs merged first: 1, 5; Can run beside: 3; Notes: its vectors come from Task 1's
  framework.
- **5 Picture opt-ins, look setup, hull log** Needs merged first: 2 (the fight file's picture fields); Can run beside:
  1, 3.
- **6 The proving harness** Needs merged first: 1 to 5; Notes: proven on FLUTE and TROMBONE.
- **7 to 12 batch A** Needs merged first: 6; Can run beside: each other; themes 28 to 36; Notes: riskiest kinds first.
- **13 Register A** Needs merged first: 7 to 12; Can run beside: everything; Notes: controller.
- **14 to 19 batch B** Needs merged first: 6; 16 needs 11 (`snip`); Can run beside: each other; themes.
- **20 Register B** Needs merged first: 14 to 19; Can run beside: everything; Notes: controller.
- **21 to 26 batch C** Needs merged first: 6; 26 needs 7 to 19 (its copy sweep); Can run beside: each other; themes;
  Notes: 26's kit-pair copies are gated at 46, 52.
- **27 Register C** Needs merged first: 21 to 26; Can run beside: everything; Notes: controller.
- **28 to 36 themes** Needs merged first: 2, 6; Can run beside: any port; Notes: 36 is small; 33 and 35 read
  fighter-made events only.
- **37, 38 hot jazz, second line** Needs merged first: 2, 6, 17 (the TUBA voice); Can run beside: any port.
- **39 mambo** Needs merged first: 2, 4, 6, 7, 18 (PIANO, CONTRABASS voices); Can run beside: any port.
- **40 stadium rock** Needs merged first: 2, 6, 22 (the ACOUSTIC voice); Can run beside: any port.
- **41 Register themes** Needs merged first: 28 to 40; Can run beside: pair gates; Notes: controller; also registers any
  theme merged before 13, 20 or 27.
- **42 to 58 pair gates** Needs merged first: both fighters' and the theme's tasks, and the register task that offered
  them; Can run beside: each other; Notes: S each; six at once.
- **59 Starters, handicaps, compatibility** Needs merged first: 41, 42 to 58; Can run beside: 61.
- **60 The app** Needs merged first: 59, and milestone 2's Task 35; Can run beside: 61; Notes: the only `bf-app` task.
- **61 Full renders** Needs merged first: 59; Can run beside: 60.
- **62 Measurements** Needs merged first: 60, 61; Notes: other lanes idle.
- **63 Docs, closing run** Needs merged first: 62.

**Suggested lane fill** (about 63 tasks; each line is one round of up to six lanes):

1. 1, 2, 3; then 5 when 2 merges, 4 when 1 and 5 merge; then 6.
2. 7, 8, 9, 10, 11, 12 (batch A). The controller runs 13 when they are merged.
3. 14, 15, 16, 17, 18, 19 (batch B); 20.
4. 28, 29, 30, 31, 32, 33, with pair gates 42 and 53 as soon as their needs are met.
5. 21, 22, 23, 24, 25, 26 (batch C); 27.
6. 34, 35, 36, 37, 38, 39, then 40; 41.
7. The remaining pair gates, six at once.
8. 59; then 60 and 61; then 62 and 63.

**Pre-flight scan** (after Task 3, before Phase 1): the controller compares every socket in `schedule.rs`,
`record.rs`, `fight.rs` and `abilities/mod.rs` with the Python lines it names, and lists in the ledger any socket whose
place, order or arguments disagree. Phase 1 starts when the list is empty.

---

# Foundations

### Task 1: Storage and the export framework

**Files:**
- Create: `tools/studio_export/m3refs.py` (the framework; no `EXPORTS` of its own except `cache` and `budget`, both in
  `TAKES_ARGS` so `all` leaves them out), `tools/studio_export/fighter_flute.py`, `fighter_trombone.py` (the
  framework's first users), `studio/crates/bf-testdata/src/{cache.rs,budget.rs}`,
  `studio/crates/bf-testdata/tests/budget.rs`
- Modify: `tools/studio_export/manifest.py` (shards), `tools/studio_export/models.py` and `sprites.py` (callable for one
  roster name, writing that name's files only), `tools/studio_export/README.md`, `studio/crates/bf-testdata/src/lib.rs`,
  `.gitignore` (nothing new: `studio/reference/.cache/` is already ignored; check it)

**Interfaces:**
- Produces, Python:
  ```python
  # m3refs.py
  REF = 'studio/reference'; CACHE = 'studio/reference/.cache/m3'
  def fighter_refs(spec): ...   # spec: the SPEC dict of a fighter_<slug>.py (below); writes fighters/<slug>/ + cache
  def theme_refs(spec): ...     # themes/<name>/ + cache (Task 2 adds the bed-alone parts)
  def episode_refs(spec): ...   # episodes/<pair>/ + cache (Task 6 adds the pair-gate parts)
  def pin(path): ...            # record a cache file in its folder's pins.json (path, sha256, bytes, export)
  def check_pins(folder): ...   # every pinned file present and equal, else SystemExit naming the command to run
  # `python -m tools.studio_export cache <area>/<name>... | all [--repin]`  regenerate cache files and check pins
  # `python -m tools.studio_export budget`  sizes per folder against Section 4; exit 1 naming each folder over
  ```
  A fighter module is `fighter_<slug>.py` with `SPEC = dict(name=..., opponent='FLUTE', fights=..., scenarios=...,
  frames=..., roles=..., voice_notes=..., hooks=...)` and `EXPORTS = {'fighter_<slug>': lambda:
  m3refs.fighter_refs(SPEC)}`. The fields are fixed by this task (documented in `README.md`) and filled by each port.
- Produces, Rust (`bf_testdata`):
  ```rust
  pub fn cache_file(rel: &str) -> PathBuf;   // studio/reference/.cache/m3/<rel>, after checking its pin; when it
                                             // is absent or changed, panics with "run: python -m tools.studio_export
                                             // cache <folder>"
  pub fn pins(folder: &str) -> Vec<Pin>;     // the folder's pins.json
  pub struct Budget { pub folder: String, pub bytes: u64, pub limit: u64 }
  pub fn budget_report() -> Vec<Budget>;     // Section 4's budgets over the tracked files (git ls-files)
  ```
- Manifest shards: `manifest.add(path)` writes into the nearest `fighters/<slug>/`, `themes/<name>/` or
  `episodes/<pair>/` `manifest.json`, else the top-level one; `verify` walks every shard; entries stay sorted.

- [ ] **Step 1: Write the failing tests.** Python (in `tools/studio_export/unit_checks/m3refs_checks.py`, run by the
  tool as `python -m tools.studio_export.unit_checks.m3refs_checks`): a file added under `fighters/flute/` lands in
  that shard and not the top-level manifest; `verify` finds a changed byte in a shard; `check_pins` fails with the
  command text on a missing and on a changed cache file; `budget` reports a 301 KB fighter folder as over. Rust:
  `budget.rs::the_set_is_inside_its_budget` (every folder within Section 4's numbers, the total at most 147 MB, the
  report printed); `cache.rs` unit tests for the panic text.
- [ ] **Step 2:** Run: FAIL.
- [ ] **Step 3: Implement**, then write `fighter_flute.py` and `fighter_trombone.py` with the opponent set to the
  other milestone 1 fighter. Their committed and cached outputs reproduce what milestone 1 already proved (the same
  recordings by SHA-256 where the seeds are the same), which is the framework's own proof.
- [ ] **Step 4:** Run `python -m tools.studio_export fighter_flute fighter_trombone`, then `cache all`, then `verify`
  and `budget`: PASS. `cargo test -p bf-testdata`: PASS.
- [ ] **Step 5:** Commit `tools/studio_export: sharded manifest, pinned cache, the per-fighter framework`.

### Task 2: Fight file format 2 and the general theme model

**Files:**
- Modify: `studio/crates/bf-core/src/fightfile/{mod,read,write,key,tests}.rs`, `studio/crates/bf-core/src/harmony.rs`
  (`chords_for`), `studio/crates/bf-sound/src/{traits,score,mix}.rs`, `studio/crates/bf-sound/src/music/mod.rs`
  (`MusicCtx`), `studio/crates/bf-content/src/{themes/mod.rs,themes/big_top/{mod,classic}.rs,cards.rs,soundtrack.rs,
  error.rs}`, `studio/fights/flute_vs_trombone.toml` (written as format 2; same fight)
- Create: `tools/studio_export/theme_big_top.py` (voicing vectors and the bed-alone gate, Template T's export for the
  big top), `studio/reference/themes/big_top/`, `studio/crates/bf-content/tests/theme_big_top.rs`,
  `studio/crates/bf-content/src/themes/<name>/{mod.rs,classic.rs}` stubs for the 13 themes (a `THEME` with
  `ported: false`; nothing else)

**Interfaces:**
- Produces:
  ```rust
  // bf_core::fightfile
  pub const FIGHT_FORMAT: u32 = 2;   // reads 1 and 2, writes 2; 3 and up: "this app reads fight files up to format 2"
  pub struct Music { /* M1 fields */ pub chords_tail: Vec<String>,   // bars after the last whole loop (0 to 8)
      pub mix: BTreeMap<String, (f64, f64)>,                         // kit MIX: bus -> (gain, reverb send)
      pub theme_options: BTreeMap<String, Value>,                    // kit TRAILER / DRUMLINE / PLUGGED, by its schema
      /* lines: keys are fighters or the extra lines a fighter declares, such as "HORN THEME" */ }
  pub struct PictureSeeds { /* M1 fields */ pub title_lines: BTreeMap<String, (String, String)>,
      pub title_balls: Option<[[f64; 3]; 2]>, pub middle: Option<(String, Middle)> }
  pub enum Middle { Divider, Degrees(f64) }
  // chord_loop: 4 or 8 chords ("music.chord_loop: 4 or 8 chords needed, found 7")
  // bf_core::harmony
  pub fn chords_for(intro: &[String], loop_: &[String], tail: &[String], n_bars: usize) -> Vec<String>;
      // intro, then the loop repeated (the last repeat may stop part-way), then the tail; a format 1 file has no tail
  // FightFile::search_key: "fight-key 2"; when music feeds the fight, chords_tail counts too; roles still do not
  // (kit/story.py roll_rows reads LINES, LOOP and the chord tones, never ROLES)

  // bf_sound
  pub enum BedHook { Boom, Hit, PressRoll, Erupt, TrailerHit }
  pub trait Bed { /* M1 methods */ fn hooks(&self) -> &'static [BedHook];   // the optional hooks this mood has
      fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, chord: &str, g: f64, ring: f64) -> Played { no_hook() }
      fn press_roll(&self, ctx: &MusicCtx, mix: &mut Mix, t0: f64, t1: f64, g0: f64, g1: f64) -> Played { no_hook() }
      fn erupt(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64) -> Played { no_hook() }
      fn trailer_hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, taiko_g: f64, seed: i64, braam: Option<f64>,
          braam_g: f64) -> Played { no_hook() } }
      // boom stays required by the trait, but a bed without it (dark disco, heist) returns no_hook() and lists no Boom
  pub trait Sound { /* M1 methods */
      fn bed_calls(&self) -> &'static [BedHook] { &[] }      // hooks its part calls without checking (`bed.boom`)
      fn stops(&self, ctx: &MusicCtx) -> Vec<f64> { Vec::new() }   // dark tango's stops (CELLO)
      fn play_with(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix, enemy: &dyn Sound) -> Played {
          self.play(ctx, bed, mix) } }                       // score() calls this; OBOE and EUPHONIUM use `enemy`
  pub struct BaseSound { pub k: usize }                      // kit/fighters/base.py Sound: a silent part with the
                                                             // roster's voice (the opponent in a fighter's sound gate)
  impl Mix { pub fn bus_mut(&mut self, name: &str) -> Option<&mut Vec<[f64; 2]>>; }   // apply_stops, lead_envelope
  // MusicCtx gains `fighter_stops: Vec<f64>`, filled by score() from both parts before bed.fight()

  // bf_content::themes
  pub struct Theme { pub name: &'static str, pub ported: bool, pub card: &'static ThemeCard,
      pub options: &'static [ThemeOption], pub voices: &'static [&'static str],   // pp voices its bed plays
      classic: fn(&ThemeOptions) -> Result<Box<dyn Bed>, ContentError> }
  pub struct ThemeOption { pub key: &'static str, pub kit: &'static str /* "TRAILER.sequel" */, pub kind: OptionKind,
      pub default: Value }
  pub enum OptionKind { Bool, Fighter }
  pub const PORTED_THEMES: [&str; 1] = ["big_top"];          // grows only in register tasks
  pub struct ThemeCard { pub display, pub text, pub key, pub scale,
      pub chord_loops: &'static [&'static [&'static str]],   // 4 or 8 chords each
      pub presets: &'static [Preset], pub bars: (u32, u32) }  // presets: the kit configs it ships with (Table S4)
  // bf_content::validate adds: the theme is ported; options match its schema; every fighter's bed_calls() is in
  // the theme's hooks(); every `lines` key is a fighter or an extra line it declares; every `mix` key is a bus of the
  // theme or of a fighter; title_lines names a fighter of the fight
  ```
  `big_top`'s chord gate (R33) is replaced by the voicing rule; FLUTE's and TROMBONE's parts are untouched (they
  inherit the defaults; TROMBONE's `bed_calls()` returns `[Boom]` in a one-line addition to its `sound.rs`, the only
  edit to a milestone 1 fighter file in this task).

- [ ] **Step 1: Write the failing tests.** `fightfile/tests.rs`: every Table S4 config's chord list rebuilt by
  `chords_for` equals the config's `CHORDS` (the 17 lists and milestone 1's, from a JSON the `theme_big_top` export
  writes from the kit configs); a format 1 file reads with an empty tail, no mix, no options and writes back as
  format 2 with the same `grid()` and chords; `format = 3` is refused; a 7-chord loop, a tail of 9, an option the
  theme does not have, an option of the wrong type, `PLUGGED` naming a fighter not in the fight, a `lines` key that
  is neither a fighter nor its declared extra line, a `mix` bus nobody has, each give a one-line error naming the
  field; `search_key` changes with the tail only when music feeds the fight. `theme_big_top.rs`: the big top's
  `voice` and `organ` voicings for all 12 roots x 13 qualities equal the kit's (`voicings.json`); the bed alone under
  two `BaseSound`s on the episode recording gives the kit's 16-bit master (SHA-256 from the export); the episode's
  full soundtrack still equals `audio.wav` (milestone 1's gate, unchanged). `bf-sound` unit tests: a bed without
  `Boom` refuses `boom` with `no_hook`; `bus_mut` returns the bus `place` made; `score()` calls `play_with`.
  `bf-content`: Review Focus 1 with a stub theme that lacks `Boom` and TROMBONE in the fight.
- [ ] **Step 2:** Run the export (`python -m tools.studio_export theme_big_top`), then the tests: FAIL.
- [ ] **Step 3: Implement.** **Step 4:** PASS, and every milestone 1 and 2 test still passes.
- [ ] **Step 5:** Commit `fight file format 2 and the general theme model`.

### Task 3: Engine sockets

**Files:**
- Modify: `studio/crates/bf-core/src/roster.rs` (split into `roster/{mod.rs,striker.rs,blade.rs,abilities.rs}`),
  `studio/crates/bf-sim/src/{fight,schedule,record,ball,rules,lib}.rs`, `studio/crates/bf-sim/src/kinds/mod.rs`,
  `studio/crates/bf-sim/src/abilities/{mod,wave,slam}.rs`, `studio/crates/bf-sim/src/kinds/blade.rs` (one socket
  line: the `swat_shells` call at `fight.py` 755), `studio/crates/bf-content/src/pair_rules.rs`,
  `studio/docs/recording-format.md`
- Create (stubs, each later owned by one port; see Template F): `studio/crates/bf-core/src/roster/kinds/<kind>.rs` for
  the 18 weapon kinds and `roster/abilities/<kind>.rs` for the 36 new ability kinds of Table S1;
  `studio/crates/bf-sim/src/kinds/<kind>.rs` for the 18 weapon kinds; `studio/crates/bf-sim/src/abilities/<kind>.rs`
  for the new ability kinds (one file per kind, except `x33.rs` for `whip` and `sweep`, `beam.rs`, and the kinds that
  live inside their weapon's file: `rhapsody` in `jump`, `endless` in `tune`, `mountain` in `tiptoe`, `fork` and
  `tapping` in `lightning`, `octave`, `altissimo` and `encore` in `spotlight`, `runway`, `subtone` and `stagedive` in
  `crowd`, `flash_spin` and `company_front` in `drill`, the bass drum's three in `roll_charge`, `barrage` in
  `mortar`); `abilities/shield.rs` (ported here, complete); `studio/crates/bf-sim/tests/sockets.rs`

**Interfaces:**
- Produces:
  ```rust
  // bf_core::roster
  pub enum WeaponCfg { Striker(StrikerCfg), Blade(BladeCfg), Roll(RollCfg), Swallow(SwallowCfg), Echo(EchoCfg),
      Line(LineCfg), Rake(RakeCfg), Combo(ComboCfg), Jump(JumpCfg), Tune(TuneCfg), Anchor(AnchorCfg),
      Mortar(MortarCfg), Tiptoe(TiptoeCfg), Lightning(LightningCfg), Grapple(GrappleCfg), Walk(WalkCfg),
      Spotlight(SpotlightCfg), Crowd(CrowdCfg), Drill(DrillCfg), RollCharge(RollChargeCfg), Unsupported(String) }
  pub enum AbilityKind { Haste, Dance, Wave, Shockwave, Slam, Shield, /* one per Table S1 kind */ Unsupported(String) }
  // each roster/kinds/<kind>.rs: `pub const PORTED: bool = false;` and `pub fn parse(raw) -> Result<..>`; while
  // PORTED is false the kind loads as Unsupported("<kind>"), so nothing a port has not finished can fight
  // bf_sim::kinds::<kind> (stubs, every body empty, every name and argument from fight.py):
  pub struct State { }                                   // its per-fight state, a field `pub <kind>: State` on Fight
  pub fn is(f: &Fight, k: usize) -> bool;                // the weapon is this kind (false while not ported)
  pub fn setup(f: &mut Fight, k: usize);  pub fn beat(f: &mut Fight, k: usize, kb: i64);
  pub fn tick(f: &mut Fight, k: usize, k16: i64);  pub fn step(f: &mut Fight, k: usize, dt: f64);
  pub fn record_keys(f: &Fight) -> &'static [&'static str];  pub fn snap(f: &Fight, out: &mut Frames);
  // bf_sim::abilities::<kind>: pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg);
  //   plus `ready(f, k, ab) -> bool` for snip, hook, slap (abilities_step 3037-3049)
  // Ball gains the per-ball states of Table S1 with Python's defaults: stun, detune_t / detune_w / detune_d, shadow,
  //   held, anc (Option), air, hopper, smashed, walking, walker, grabbed, pinned, swan (Option), strut, dive, march,
  //   rolling, last_ab; Fight gains extra_snip, wclash_t, shells, next_salvo, salvo_landed
  // Wave gains `stun: Option<f64>`, `detune: Option<DetuneCfg>`; Slam gains `how: String` ("slam" unless cannons)
  // Rules gains swat_shells, swat_shell_h (0.85), swat_shell_r (0.2), swallow_rollers, roll_rows, roll_start8
  pub fn stunned(f: &Fight, k: usize) -> bool;           // fight.py _stunned: stun or shadow
  ```
- **What is wired**, each at the Python line it names and in Python's order, calling the stubs: the setups of
  `run()` 5756-5911; the beat triggers 5925-5945; the eighteen 16th counters 5946-6095 (each its own counter, as
  milestone 1's two); the timers 6096-6107 (stun, detune, the smashed timer and its `rebuild` event); `pull_step`,
  `lights_out_step`, `hold_step` 6108-6114; the weapon steps 6118-6136 and the later steps 6137-6202; `trigger`'s
  dispatch 2841-2988 (the `quiet` copy path of 2830-2840 included); `abilities_step`'s gates 3027-3053 and the
  charged-beam check 3013-3016; `effects_step`'s wave-hit branches for `detune`, `stun` and `slow` 3069-3077 and the
  slam's `how` 3097; `move`'s skips 3131-3150, speeds 3153-3160, the clash conditions 3180-3187, the walker branch
  3190-3208 and `wclash_t` 3227-3231; `damage`'s skips 325-334 and the detuned attacker 345-346; `knock`'s skips
  355-360; `snap`'s per-kind keys 5495-5734. `shield` (trigger 2847-2848) is ported in full; it is three lines and
  three fighters have it.
- **Pair rules.** `pair_rules.rs`' table becomes the series' three rows: (FLUTE, TROMBONE, `swat_cobras`),
  (EUPHONIUM, TROMBONE, `swat_shells`), (BASS DRUM, TUBA, `swallow_rollers`). Their words are Task 5's.
- **`recording-format.md`** gets, once, every key and event field each kit kind's `snap()` and `ev()` writes (from
  the Python, with line numbers), one section per kind between `<!-- kind:<name> -->` and `<!-- /kind:<name> -->`
  markers; a port only edits text between its own markers.

- [ ] **Step 1: Write the failing tests.** `sockets.rs`: with a test-only switch that makes every stub log its name
  instead of doing nothing (`Fight::trace_sockets`), a fight of FLUTE against TROMBONE logs the sockets in the order
  `tests/sockets_order.txt` lists, which is transcribed from `fight.py` with the line of each; every milestone 1
  fight is unchanged (the replay hashes, `prefix`, `coverage`, the bags); a roster entry of each of the 18 weapon
  kinds still loads as `Unsupported`; `shield` unit vectors (a hit on a shielded ball is a `block` event with the
  fields of `fight.py` 336) from `unit_checks/shield_unit_vectors.py`; `pair_rules` is symmetric for all three rows
  and empty for any other pair of the 20.
- [ ] **Step 2:** Run: FAIL. **Step 3:** Implement. **Step 4:** PASS, with `studio/replay/hashes.txt` unchanged.
- [ ] **Step 5:** Commit `bf-sim: sockets for every kit kind; the shield; the series' pair rules`.

### Task 4: Shared look and voice helpers

**Files:**
- Create: `studio/crates/bf-content/src/fighters/shared/{mod,reeds,padded}.rs` (`kit/fighters/_reeds.py` whole: masks,
  `blend`, `add`, `ribbon`, `ring_exit`, `beam_lane`, `draw_woodwind_shard`, `tail_theta`, the colours; and
  `contrabass.padded_sprite` with `jag_ring`), `studio/crates/bf-picture/src/ndimage.rs` (`scipy.ndimage.
  gaussian_filter` as TRUMPET calls it: separable `correlate1d`, `truncate = 4.0`, mode `reflect`, the input's
  dtype), `studio/crates/bf-sound/src/voice/pp/{brass,pluck,sax}.rs` (`trumpet_lowpoly.brass`, `brass_fx`, `bend`,
  `fingering`; `piano_lowpoly.ks_pluck`, `additive`; `altosax60_lowpoly.sax_tone`), `tools/studio_export/
  shared_vectors.py`, `studio/reference/shared/` (vectors, about 200 KB, in the top-level manifest), tests
  `bf-content/tests/shared_helpers.rs`, `bf-picture/tests/ndimage.rs`, `bf-sound/tests/pp_blocks.rs`
- Modify: `studio/crates/bf-picture/src/lib.rs`, `studio/crates/bf-sound/src/voice/mod.rs`. (`fighters/shared/mod.rs`
  and its `pub mod` line are created empty by Task 5, so this task needs Task 5 merged.)

**Interfaces:** the Python functions above with the same names and arguments, ported from the Python. TROMBONE's
milestone 1 voice keeps its own copy of the brass helpers; it is not switched.

- [ ] **Step 1:** Export the vectors: each helper on the argument sets the kit's looks and voices use (collected by
  wrapping them while the 18 fighters' default scenario renders and plays, a frame every second and every first
  note), plus edge cases (empty masks, a ribbon of two points, `gaussian_filter` on float32 and float64 at sigma 0.5
  to 6, a pluck of 1 ms). **Step 2:** Write the failing tests: masks and sprites identical, floats within 1e-12
  relative or -120 dBFS for sounds. **Step 3:** FAIL. **Step 4:** Implement; PASS.
- [ ] **Step 5:** Commit `shared helpers: the reeds' masks, the padded sprite, gaussian_filter, the Pixel Pitch blocks`.

### Task 5: Picture opt-ins, look setup for another side, the hull log

**Files:**
- Modify: `studio/crates/bf-picture/src/title/{mod,layout,words,fx,panels}.rs`, `studio/crates/bf-picture/src/look.rs`,
  `studio/crates/bf-picture/src/ring/{state,ropes,mod}.rs`, `studio/crates/bf-picture/src/hull.rs`,
  `studio/crates/bf-content/src/{picture,cards}.rs` (the fighter dispatch in `card()`; `pair_rule_text` for Task 3's
  two new rules: "A live lance knocks a falling shell out of the air." and "The bell can swallow a rolling drum.")
- Create: `studio/crates/bf-picture/src/title/patterns/{mod.rs,<slug>.rs}` for the 15 names of `UNPORTED_PATTERNS`
  (each a stub returning `PatternNotPorted`; the eight kit ones are filled by their fighters' ports, the seven older
  ones stay stubs), `tools/studio_export/title_optins.py`, `studio/reference/picture_title_optins/` (hashes),
  `studio/crates/bf-picture/tests/title_optins.rs`; per-fighter stubs in `studio/crates/bf-content/src/fighters/
  <slug>/{mod,look,sound,voice,card,state}.rs` for the 18 (each `mod.rs` exports `look() -> Option<Box<dyn Look>>`,
  `voice()`, `sound(k, ff)`, `CARD: Option<FighterCard>`, `MESH: Option<&str>`, `MUSIC_FEEDS_FIGHT: bool`,
  `EXTRA_LINES: &[&str]`, all `None` / `false` / empty), and the dispatch in `fighters/mod.rs` and `cards.rs` that
  calls them for all 20; `fighters/shared/mod.rs` (empty, filled by Task 4) with its `pub mod` line

**Interfaces:**
- `TITLE_LINES`, `TITLE_BALLS` and `MIDDLE` (`title.py` 18-26, 61-171, 830-902: `rotate_mask`, `_fit_middle`,
  `_streaks_turned`), read from the fight file's `picture` fields (Task 2).
- `PictureCtx::shade(...)` and `PictureCtx::side_ends(...)` (`kit/picture.py`, the functions PIANO and ELECTRIC call).
- `trait Look { fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) { self.setup(p, k) } }`: a look
  made for side `k` with another fighter's numbers (EUPHONIUM's `dup`, `euphonium.py` 192-201); the ring renderer
  never calls it; a look whose `setup` reads `cfg` takes it from here.
- `hull::log_calls(on: bool)` and `hull::take_log() -> Vec<(Vec<(f64, f64)>, Hull)>`: a test-only record of every
  hull the renderer and the looks asked for, so a look test can compare each call with the kit's (Template F).

- [ ] **Step 1: Write the failing tests.** `title_optins.rs`: the card of the bass clarinet example (two lines,
  moved balls) and of the horn rematch (REMATCH along the divider) at 16 times each equal the kit's hashes; a
  `MIDDLE` word that cannot fit is the kit's error. A unit test for `setup_as`. The hull log records the ring beam's
  calls of milestone 1's episode in order.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS; `title_frames` and `golden_frames` unchanged.
- [ ] **Step 5:** Commit `bf-picture: title opt-ins, pattern files, setup_as, the hull log; fighter stubs`.

### Task 6: The proving harness

**Files:**
- Create: `studio/crates/bf-sim/tests/support/fighter.rs`, `studio/crates/bf-content/tests/common/{fighter,theme}.rs`,
  `studio/crates/bf-export/tests/support/episode.rs`, `studio/crates/bf-sim/tests/fighter_flute.rs`,
  `fighter_trombone.rs`, `studio/crates/bf-content/tests/{look,sound,voice}_flute.rs` and `_trombone.rs` (the
  harness's first users), `studio/replay/fighters/{flute,trombone}.txt`, `studio/docs/porting-a-fighter.md`
- Modify: `tools/studio_export/m3refs.py` (the parts of `fighter_refs`, `theme_refs` and `episode_refs` the suites
  read), `studio/crates/bf-sim/src/parity.rs` (paths only: any pair's recordings; it already compares every key)

**Interfaces:**
- Produces:
  ```rust
  // bf-sim/tests/support/fighter.rs
  pub fn coverage(name: &str);            // every row of fighters/<slug>/fight.json `coverage` occurs in the Rust run
                                          // of its scenario at its time (1e-6) with its fields; always runs
  pub fn replay(name: &str);              // studio/replay/fighters/<slug>.txt (BF_BLESS=1 writes it); always runs
  pub fn prefix(name: &str);              // every cached scenario agrees for PREFIX_FLOOR_S; #[ignore], cache
  pub fn stats(name: &str, seeds: u64);   // Studio statistics against fight.json `stats`: 2,000 or 20,000 seeds
  pub fn record_flag(name: &str);         // events, winner and K.O. equal with the recording on and off
  // bf-content/tests/common/fighter.rs
  pub fn look_frames(name: &str);         // frames of the committed recording with the real look and DefaultLook
                                          // for the opponent: frame, hook-log and draw-log SHA-256, the hull log;
                                          // always runs
  pub fn look_frames_cached(name: &str);  // the same for the second fight; #[ignore], cache
  pub fn title(name: &str);               // the card frames, the close-up, the pattern on the fixed base; always runs
  pub fn sprites(name: &str);             // the sprite cache's hashes; always runs
  pub fn voice(name: &str);               // fingerprints always; whole notes against the cache (-120 dBFS) #[ignore]
  pub fn sound(name: &str);               // its part with BaseSound for the opponent over the big top: the 16-bit
                                          // master's SHA-256, per-bus RMS (0.01 dB), the note order; always runs
  pub fn sound_stems(name: &str);         // stems against the cache at -120 dBFS; #[ignore]
  pub fn checks(name: &str);              // `ballfight check`'s verdicts and numbers against checks.json; always runs
  // bf-content/tests/common/theme.rs, bf-export/tests/support/episode.rs: Template T's and Template P's suites
  ```
- `porting-a-fighter.md`: the manual a port worker follows (the files of Template F, the export spec fields, the
  commands, how to read a failing frame with stage dumps, how to report a last-bit gap), which also answers the
  spec's "Adding a fighter after launch" (§10).

- [ ] **Step 1: Write the failing tests:** the FLUTE and TROMBONE files, each a few lines calling the suites.
- [ ] **Step 2:** Run `python -m tools.studio_export fighter_flute fighter_trombone` and `cache fighters/flute
  fighters/trombone`; the tests FAIL (no suite yet).
- [ ] **Step 3:** Implement the suites. **Step 4:** PASS, including the ignored ones by name; the numbers equal
  milestone 1's where they measure the same thing (`m1-measurements.md` §4 to §7).
- [ ] **Step 5:** Commit `the proving harness, proven on FLUTE and TROMBONE`.

---

# Template F: port fighter X

Written once in full; the entries below say only what differs. `<NAME>` is the roster name, `<slug>` the kit's
(`name.lower().replace(' ', '_')`: `bass_clarinet`), `<kind>` its weapon kind and each new ability kind.

**Files (all owned by the task):**
- `studio/crates/bf-core/src/roster/kinds/<kind>.rs` (typed config; `PORTED = true` once its engine part runs) and
  `roster/abilities/<kind>.rs` for each new ability kind
- `studio/crates/bf-sim/src/kinds/<kind>.rs` (+ `kinds/<kind>/tests.rs`), `studio/crates/bf-sim/src/abilities/
  <kind>.rs` for each new ability kind the entry names, `studio/crates/bf-sim/tests/fighter_<slug>.rs`
- `studio/crates/bf-content/src/fighters/<slug>/{mod,look,sound,voice,card,state}.rs` (state: what the look and the
  part both derive from the recording), `studio/crates/bf-content/data/models/<NAME>.json` (and any variant meshes
  its look builds), `studio/crates/bf-content/tests/{look,sound,voice}_<slug>.rs`
- `studio/crates/bf-picture/src/title/patterns/<slug>.rs` when Table S2 names a `title.py` pattern
- `tools/studio_export/fighter_<slug>.py`, `tools/studio_export/unit_checks/<kind>_unit_vectors.py`
- `studio/reference/fighters/<slug>/`, `studio/replay/fighters/<slug>.txt`
- `studio/docs/recording-format.md`: only between its kinds' markers

**Interfaces:**
- `bf_sim::kinds::<kind>` and `bf_sim::abilities::<kind>`: the stubs of Task 3 filled in, every Python function of
  the kind's section ported with its name, every draw from the fight's generator in Python's order and number.
- `fighters::<slug>::look()` returns its `Look` (all hooks of Table S2), `voice()` its `Voice`, `sound(k, ff)` its
  `Sound` (with `bed_calls()`, and `stops()` or `play_with` where Table S3 says), `CARD` its card (below), `MESH` its
  model, `MUSIC_FEEDS_FIGHT` (true only for PIANO), `EXTRA_LINES` (HORN: `["HORN THEME"]`). Every suite of Task 6
  reaches the fighter through `fighters::<slug>` directly, never through the registry, so a port is proven before it
  is registered.
- **The card** (`card.rs`): texts in the interface's style (sentence case, plain verbs, no jargon): the weapon's
  name and a one or two sentence paragraph of how it fights and grows, one line per ability (what it does, its
  length in beats, "below 25 HP" for the low one), the colour's name from `docs/ballfight/RULES.md`'s table, "usually
  plays" from its role in its kit configs, the Music screen's title and line, whether it has a bass part, its emoji
  and tags. Sources: the fighter's kit module docstring and the roster; `docs/studio/interface-reference.md` gives
  texts only for FLUTE and TROMBONE. The reviewer reads every text.

**The references it exports** (`python -m tools.studio_export fighter_<slug>`; spec in `fighter_<slug>.py`):
- *Fights.* Opponent FLUTE (TROMBONE for FLUTE itself), handicap `[1.0, 1.0]`, dmg scale 1.0, the big top at 132 BPM
  in E harmonic minor with the milestone 1 loop. Two recordings chosen by scanning seeds: `left` (X left, X wins,
  every one of its three abilities fires, its K.O. shatter not shown) and `right` (FLUTE left, X loses: its shards fly).
  `left` is committed; `right` is cached. Scenarios (cached): `early_levels`, `early_low`, `early_ko`, `plain` with
  three seeds each against FLUTE, `swapped` with three, and two `plain` against TROMBONE (a lance against its kinds).
  Statistics: 2,000 seeds of X against FLUTE (rows cached, pooled figures committed). The coverage table: every
  event kind and `hit` sub-kind seen in at least 20 of the 2,000, and every ability of X, each at its first
  occurrence in a scenario within 20 s of FIGHT!; the exporter lowers thresholds or adds seeds until each appears,
  and fails naming any it cannot reach.
- *Picture.* For `left` (committed hashes) at least 64 frames and for `right` (cached hashes) at least 32: a spread
  over the video plus the frames where each layer of its look is on screen (each ability 0.2 s and 0.6 s after its
  first use, a weapon hit, HP below 25, its shards, its victory solo); the frame, hook-log and draw-log SHA-256 as
  `look_flute.py` does; every `ConvexHull` call's points and vertices; the title card at 16 times on both sides; its
  title hooks on the fixed base; its sprite cache.
- *Sound.* The soundtrack of `left` and `right` with X's real part and the kit's base `Sound` for FLUTE (patched at
  run time), X in each role its code branches on (Table S3: ORGAN `bass` and another; BASS CLARINET `bass` and
  `counter`; others their kit role), its sketch line (`default_line`) and solo (`default_solo`): the 16-bit master's
  SHA-256, per-bus RMS, the order of first-time notes; stems cached. Its voice: 4 pitches across its `pp_bridge`
  register x velocities 0.6 and 0.9 x durations 0.12, 0.25, 1.0, each with the shared noise stream's state before
  it; fingerprints committed, notes cached. The kit's `check` numbers for `left` (`checks.json`).

**Steps:**
- [ ] **Step 1: Export.** Write `fighter_<slug>.py`, run it and `cache fighters/<slug>`; `budget` passes for the
  folder (300 KB). Open six of its frames and confirm they show the fighter (its weapon, an ability, its shards).
- [ ] **Step 2: Write the failing tests.**
  - Engine, `fighter_<slug>.rs`: `coverage`, `replay`, `record_flag`, `prefix` (ignored), `stats` (release); unit
    tests per kind function in `kinds/<kind>/tests.rs` with numbers from `<kind>_unit_vectors.py` (each grid, each
    growth rule and cap, each state change, each event's fields); errors: every roster number the kind cannot run is
    refused when the roster is read, naming the fighter and the field (as `roster.rs`' milestone 1 tests), each
    checked against what the Python does with it (a script in `unit_checks/`).
  - Picture, `look_<slug>.rs`: `look_frames`, `look_frames_cached` (ignored), `title`, `sprites`; the hull log has no
    fallback and equals the kit's call for call.
  - Sound, `voice_<slug>.rs` and `sound_<slug>.rs`: `voice`, `sound`, `sound_stems` (ignored), `checks`; its
    `bed_calls()` equals the hooks its Python part calls unguarded (`bed.<hook>(` not under `hasattr`).
  - Card: present, every text non-empty and free of the retired names, range from the roster's register.
- [ ] **Step 3:** Run: FAIL, each for the reason that nothing is ported yet.
- [ ] **Step 4: Implement** in this order, committing at each: roster config and kinds (`bf-sim` tests green), the
  record keys and their `recording-format.md` section, the look (with stage dumps, `python -m tools.studio_export
  stages`, to find the first stage that differs), the voice, the part, the card. `PORTED = true` in its roster kind
  files comes with the first engine commit (a fight needs it); the app still cannot offer the fighter, because only
  the register task adds it to `bf_content::PORTED`.
- [ ] **Step 5: Gates.** `cargo test -p bf-sim --test fighter_<slug>`; the same with `-- --include-ignored`;
  `cargo test --release -p bf-sim --test fighter_<slug> stats`; `cargo test -p bf-content --test look_<slug> --test
  voice_<slug> --test sound_<slug>` and with `--include-ignored`; milestone 1's gates unchanged (`replay`,
  `golden_frames`, `stems`); `cargo fmt`, `cargo clippy`. The report gives the prefix window of every scenario (and
  how each ended), the statistics table against the kit's, the counts of frames and notes proven, the sizes.
- [ ] **Commit subjects:** `bf-sim: <NAME>'s kinds`, `bf-content: <NAME>'s look`, `bf-content: <NAME>'s voice and
  part`, `reference: <NAME>`.

## Batch A: the riskiest new mechanisms

### Task 7: PIANO
**New:** weapon `roll`; the projectile machinery (`spawn_key`, the flight, rope kills and hits of `move` 3235-3278,
the volley growth for `roll` 3267-3268, the `K` rows of `snap`); `arc` emission 3099-3113; abilities `sustain`,
`arc`, `runoff`; **its line feeds its fight**: `bf_core::lines::roll_rows` (port of `kit/story.py` 41-75, in
`bf-core` because `setup_from` needs it) and `setup_from` passing `Rules.roll_rows` and `roll_start8 = 8 *
LOOP_START`; `MUSIC_FEEDS_FIGHT = true`. Look needs `P.shade`; plain title panel. Voice `inst_note('grand')` on the
Task 4 blocks. **Extra tests:** `roll_rows` against the kit for six configs (8 a bar, 4 a bar with rests, a sketch
line, a 4-chord loop, notes above and below the register, `None` rests); a key, loop or line change changes the search
key of a PIANO fight and of no fight without it; TROMBONE's lance parries keys (`parry` events) in the TROMBONE
scenarios; a line that is not 4 or 8 a bar is refused naming `music.lines.PIANO` (Review Focus 3). **Size L.**

### Task 8: TRUMPET
**New:** weapon `combo`; ability `beam` (charge, fire, the ability-beam part of `beams_step`; the `beam` *weapon*'s
`beam_beat` / `beam_fire` stay unported). Look: the largest (489 lines), `gaussian_filter` (Task 4), its own sprite and
atlas, `ball`, its `title_pattern`. Voice `trumpet_lowpoly.brass`. Part: `LINES['TRUMPET']` turns the punches into a
melody (test both with and without it). **Extra:** its look's frame time is reported (the heaviest look; Task 62).
**Size L.**

### Task 9: ORGAN
**New:** weapon `line` (the hazards section; `lay_string` and `strings` belong to the older `trap` kind: port them
only if `line` reaches them, and say which); `shield` (Task 3), `haste`, `slam` ported. Look: two `ConvexHull` sites
(the first port to meet Qhull, Section 8, Q4), `light`, `fx_end`, `block_burst` / `block_spark`; title pattern
`_organ_pipes`. Voice `organ60_lowpoly.pipe` (the largest voice module; `_NOTE` cleared per note as `pp_bridge` does).
Part: role `bass` (a pedal under each pipe row) and `melody`; `bed_calls = [Boom]`. **Size L.**

### Task 10: CLARINET
**New:** weapon `jump`; abilities `stun` (the wave's stun hit, the `stun` timer, `stunned()` already in every grid
from Task 3) and `rhapsody` (inside `jump_tick`). Look: `teleports` (no speed lines across a jump), `after_grade`,
`shimmer`; reeds helpers; pattern `_clarinet_keys`. Voice `clarinet60_lowpoly.reed`. **Extra:** a stunned FLUTE starts
no strike and does not move (scenario with an early SQUEAK). **Size M.**

### Task 11: OBOE
**New:** weapon `tune`; abilities `snip` (owned here: `start_snip`, `snip_step`, `snip_ready`, `SNIP`; BASS CLARINET
and a copied snip reuse it, so it must work for either ball), `endless`, the `detune` option of `wave` (its branch of
`effects_step` and the detuned attacker in `damage`, through Task 3's sockets). Look: `sprite_rolls` and `ball`
(debuffs drawn on the enemy), `quiet_hits`; pattern `_oboe_reeds`. Voice `oboe_lowpoly.double_reed`. Part: the sour
twin in the victim's voice through `play_with`'s `enemy`; `Sound::detune` (milestone 1) now has data. **Size M.**

### Task 12: SOUSAPHONE
**New:** weapon `drill` (march, tramples, the bell blast; `_trample_baskets` reads the enemy's baskets, so its FLUTE
fights exercise it); abilities `flash_spin`, `company_front`. Look: own sprite and `title_closeup` through
`padded_sprite` (Task 4), the ghost sprite, its `title_pattern`. Voice `sousaphone_lowpoly.sousa_tone` with
`trumpet.bend` (Task 4). Part: `prepare` hands FLUTE a line through `line_override` (milestone 1's). **Extra:** a
trampled basket in the coverage table. **Size L.**

### Task 13: Register batch A
**Files:** `studio/crates/bf-content/src/registry.rs` (`PORTED` += the merged fighters of Tasks 7 to 12),
`themes::PORTED_THEMES` (+= any theme task merged by then), the registry's tests (`available_is_*`), `studio/README.md`
(the roster line). **Steps:** run `cargo test --workspace` (with the ignored M3 tests of the batch by name) on the
build branch after the merges; every `every_ported_fighter_has_*` test passes for the new names; commit `register
batch A`. **Done by the controller; one reviewer reads the diff. Size S.**

## Batch B

### Task 14: TENOR SAX
**New:** weapon `crowd` (poses, honks, the crowd's throws from beyond the nearest rope, `THROWS`); abilities `runway`
(the strut: no knock, does not give way in a clash), `subtone`, `stagedive` (airborne: no hit, no knock, no clash).
Look: `hides_hp`, sets the ball's `p` and `lift` during the dive, its `title_pattern`. Voice `tenorsax_lowpoly.
tenor_tone`. Part: `M.roar`. **Size L.**

### Task 15: ALTO SAX
**New:** weapon `spotlight` (the `SPOT` record names which side is the spotlight, `fight.py` 5679); abilities
`octave`, `altissimo`, `encore`. Look: the lamp cone's `ConvexHull` (second Qhull site), `light`, `glows`, its
`title_pattern`. Voice `altosax_lowpoly.alto_tone` on `sax_tone` (Task 4). **Size M.**

### Task 16: BASS CLARINET
**Needs Task 11** (`snip`). **New:** weapon `anchor` (the whirl drives the ball; `ANC`); abilities `twin` (a second
arm), `lights_out` (the `shadow` state: intangible, no clash, steered before `move`; `LO`). Look: the `sink` state,
`after_grade`'s dim with the `dim_keep` loop over the other look (none in the kit defines one: keep the loop, test it
with a stub), `st['anc']` recomputed where `focus_points` needs it; pattern `_bassclarinet`; its starter carries
`TITLE_LINES` (Task 5). Voice `bassclarinet_lowpoly.bass_reed`. Part: roles `bass` and `counter` (an octave up);
`bed.erupt` only when the bed has it (not in `bed_calls`). **Size M.**

### Task 17: TUBA
**New:** weapon `swallow` (the `held` state: the enemy rides inside, no clash); the `swallow_rollers` branch is ported
but proven at Task 49. Look: `canvas_first`, `hides_hp`, `items_ball`; plain title panel. Voice `tuba60_lowpoly` on
`trumpet.brass`; this voice is also what Tasks 37 and 38 play. Part: `bed_calls = [Boom]`. **Size M.**

### Task 18: CONTRABASS
**New:** weapon `walk` (a walker does not give way in a clash and is not knocked: the branch of `move` 3190-3208 and
`wclash_t`, through Task 3's sockets); abilities `slap` (with `slap_ready`: waits for a close enemy, fires at once to
break a grab), `stampede`. Look: own sprite and `title_closeup` through `padded_sprite`, its `title_pattern`. Voice
`contrabass_lowpoly` (pizz under 0.5 s, arco above), also played by Task 39. **Size M.**

### Task 19: BASSOON
**New:** weapon `tiptoe` (hops: `air`, `hopper`, a hopper slides only when knocked; `HOP`); abilities `hook` (with
`hook_ready`; the hopper's follow-up hop), `mountain`. Look: `speed_lift`, the `sx` / `sy` / `air_bias` / `paint_h`
keys, `items_ball`; pattern `_bassoon`. Voice `bassoon_lowpoly.bassoon_reed` (its part calls it directly). **Size L.**

### Task 20: Register batch B
As Task 13 for Tasks 14 to 19 (and any theme merged since). **Size S.**

## Batch C

### Task 21: HORN
**New:** weapon `echo` (`ECHO_G`, `ECHO_N`, `ECHO_R`); ability `flick` (`FLICK`; the rope-hit bonus). Look:
`canvas_overlay` (the copper thread), `block_burst`, `shadows_free`; pattern `_horn_coils`. Voice `horn60_lowpoly` on
`trumpet.brass`. Part: `LINES['HORN']` (a call a beat), `EXTRA_LINES = ["HORN THEME"]` (STOPPED HORN's notes); a heavy
hit is `bed.trailer_hit` when the bed has it, else `bed.boom`, so `bed_calls = [Boom]` unless the theme has
`TrailerHit` (Task 2's compatibility check reads both). **Size L.**

### Task 22: ACOUSTIC
**New:** weapon `rake` and abilities `whip`, `sweep`. They share code with the older `surf` and `spout` kinds: the
`dc` 16th counter, `dash_cone_step` and `surf_rake_step`. Port the rake, whip and sweep branches only, keep each
`surf` / `spout` / `dash` / `cone` branch as a `SLOT` comment at its place, and say in the report which Python lines
each Rust line comes from (the reviewer checks that no older kind's branch was half-ported). Look: `sprite_k` 0.86;
pattern `_woodgrain`. Voice `acoustic_lowpoly` on `ks_pluck`; also played by Task 40. **Size L.**

### Task 23: ELECTRIC
**New:** weapon `lightning` (bolts from the headstock or the nearest post); abilities `fork`, `tapping`, and its
desperation kind `smash` (it lies in pieces: still, intangible, silent; `splinters_step`; the `rebuild` event in the
timers). Look: `hides_hp`, rope sparks through `rope_points` and `side_ends` (Task 5), its `title_pattern`. Voice
`electric60_lowpoly` (`RETIRED[0]` is its voice id: never shown, Global Rules). **Size M.**

### Task 24: CELLO
**New:** weapon `grapple` (rush, grab, swing, throw into the ropes, the rope-slam bonus; `grabbed` and `pinned`;
`GRAB`; a walking enemy slips the grab, proven at Task 50); abilities `vibrato`, `prelude`, `swan` (airborne: no
hit, knock or clash; `lift`). Look: `quiet_hits`, its `title_pattern`. Voice `cello60_lowpoly` on
`violin_lowpoly.bowed` (ported here: only CELLO uses it in this milestone). Part: `stops()` (its rope slams).
**Size L.**

### Task 25: BASS DRUM
**New:** weapon `roll_charge` (tips, rolls along a telegraphed line, unstoppable, the BOOM; not while `held`;
`RCHG`); abilities `heartbeat`, `drum_roll`, `cannons` (slams whose `how` is `cannon`: Task 3's `Slam.how`). Look:
own sprite, `title_closeup` and `ball_sprite` from variant builds of `bassdrum_lowpoly` (export each variant mesh it
builds into `data/models/BASS DRUM.<variant>.json`), `canvas_wave`, its `title_pattern`. Voice `bassdrum_lowpoly`
(velocity >= 0.9 the boom, MIDI < 24 a roll). Part: recomputes the look's cannon list in `state.rs`. **Size L.**

### Task 26: EUPHONIUM
**New:** weapon `mortar` (salvos, `predict`, `shells_step`, `SHELLS`; the `swat_shells` branch in TROMBONE's step is
filled here, in `kinds/mortar.rs`, through the socket Task 3 put in `blade.rs`); abilities `barrage`, `copy` (fires
the enemy's last copyable ability on its own ball through `trigger(quiet)`, `extra_snip`, or one round of the enemy's
signature: an anchor whirl or a mortar salvo). Look: the `dup` (the enemy's look made with `setup_as` for the
euphonium's side and the enemy's numbers, drawing the copied kinds only), `ball_sprite` and `extra_sprites` (the
enemy's model, tinted amber); pattern `_euphonium`. Voice `euphonium_lowpoly`. Part: a ghost of the enemy in its
voice (`play_with`), `bed_calls = [Boom]`, `bed.press_roll` when present. **Extra tests:** against FLUTE the copies are
haste and wave; a copy table over the ported fighters of batches A and B (`copy_sweep`: for each, a scenario where
EUPHONIUM copies each copyable kind that fighter has: `arc`, `stun`, `snip`, `twin`, `lights_out`, `shield`, `slam`,
`shockwave`, `wave`, `haste`) agrees for the prefix floor and draws without a panic. The kit pairs' copies are proven
by frames at Tasks 46 and 52. **Size L.**

### Task 27: Register batch C
As Task 13 for Tasks 21 to 26. **Size S.**

---

# Template T: port theme Y

**Files (owned):** `studio/crates/bf-content/src/themes/<name>/{mod.rs,classic.rs}` (and `parts.rs` when the bed
is over about 600 lines of Rust), `studio/crates/bf-content/tests/theme_<name>.rs`, `tools/studio_export/
theme_<name>.py`, `studio/reference/themes/<name>/` (with the kit recordings of its reference configs, Table S4).

**Interfaces:** `THEME` with `ported`, the card (display name, the line beside the picker, key and scale of its first
preset, its chord loops, its presets, its bar range), its options schema (Table S4), the voices it plays, and the
classic bed: the mood's `Bed(M)` class, every method in the kit's order, every voicing function, every random draw
in the kit's order (`np.random.default_rng(seed)` per call where the kit makes one), `hooks()` listing exactly the
optional methods the class defines.

**The references it exports** (`python -m tools.studio_export theme_<name>`):
- *Voicings:* each voicing function of the bed (Table S4's tables and the chord-tone rule) for the 12 roots x 13
  qualities, as integers.
- *The bed alone:* for each reference config, the kit's own recording of that episode (committed under the theme's
  folder), the soundtrack with both fighters replaced at run time by the kit's base `Sound` (silent): the 16-bit
  master's SHA-256, per-bus RMS, the note order (when the bed plays Pixel Pitch voices); stems cached.
- *Variants:* the same at a second tempo (the config's BPM plus 14, snapped by `loop_bpm`) and a second key with the
  loop of another config of the same length; each theme option at each value the kit has (a `PLUGGED` for each side);
  for `dark_tango`, the bed with a stub part whose `stops()` returns fixed times (so `apply_stops` is proven without
  CELLO); each optional hook (`boom`, `hit`, `press_roll`, `erupt`, `trailer_hit`) called directly at three times
  with three gains, its output written as a cached vector and pinned.

**Steps:**
- [ ] **Step 1:** Write `theme_<name>.py`; export; `cache themes/<name>`; `budget` passes.
- [ ] **Step 2: Write the failing tests:** voicings equal; the bed-alone master's SHA-256 for every reference config
  and variant; per-bus RMS within 0.01 dB; stems and hook vectors against the cache (ignored); `hooks()` equals the
  kit class's methods; the card; options: unknown option, wrong type, `PLUGGED` naming a fighter not in the fight,
  each refused naming the field; `validate` refuses a fight whose part needs a hook the theme lacks.
- [ ] **Step 3:** FAIL. **Step 4:** Implement; PASS; milestone 1's `stems` unchanged.
- [ ] **Commit subject:** `bf-content: <name> theme, classic bed`. It does not add itself to `PORTED_THEMES`.

### Task 28: `baroque_storm`
The `HARP_RH` and `VLN` tables with the rule as fallback; `self.off`, the runs' offset from D; reads the story
(`P.STORY`). Reference `examples/clarinet_vs_oboe`. **Size S.**

### Task 29: `bossa_noir`
Seventh and sixth chords (`m7`, `m6`, `maj7`, `m7b5`) through the voicing vectors; `scipy.signal.lfilter` (ported in
milestone 1); `hit`, `boom`. Reference `ballfight_flute_vs_clarinet`. **Size M.**

### Task 30: `brass_trailer`
Option `sequel` (`TRAILER = dict(sequel=True)`: extra buses `braam`, `taiko` re-weighted and a darker hall);
`trailer_hit` and `boom`. References `examples/trombone_vs_horn` (4-chord loop, plain) and
`ballfight_horn_vs_trombone_rematch` (sequel). **Size M.**

### Task 31: `dark_disco`
No `boom`: `hooks()` is `[Hit]`; `h01` (milestone 1). Reference `ballfight_altosax_vs_tenorsax`. **Size S.**

### Task 32: `dark_drumline`
Options `bass_drums` and `after_ko` (`DRUMLINE`); `press_roll`, `boom`; reads the story. References
`ballfight_flute_vs_sousaphone` (defaults) and `ballfight_bassdrum_vs_tuba` (`bass_drums=False, after_ko=True`).
**Size M.**

### Task 33: `dark_tango`
`stop_times` (from `MusicCtx::fighter_stops`, Task 2) and `apply_stops` (cuts its own buses through `Mix::bus_mut`,
short fades); reads `FIGHTERS`, `LINES` and `ROLES` from the fight file; `theme()`. Reference
`ballfight_cello_vs_contrabass`; the stub-stops variant proves `apply_stops` before CELLO exists. **Size L.**

### Task 34: `heist`
No `boom` (`hooks()` is `[Hit]`). Reference `ballfight_oboe_vs_bassoon`. **Size S.**

### Task 35: `horror_trap`
Reads the LIGHTS OUT windows from the recording's events at construction; `erupt`, `boom`, `hit`. Reference
`ballfight_bassclarinet_vs_bassoon` (its recording has real LIGHTS OUT events, so the bed alone exercises the dark
windows). **Size M.**

### Task 36: `prog_duel`
Reference `ballfight_piano_vs_organ`. **Size S.**

### Task 37: `hot_jazz`
**Needs Task 17** (the TUBA voice). Plays `note('TUBA')`; `lfilter`. Reference `ballfight_trumpet_vs_clarinet`.
**Size M.**

### Task 38: `second_line`
**Needs Task 17.** Plays the TUBA voice; `press_roll`, `boom`, `hit`. Reference `ballfight_euphonium_vs_trombone`.
**Size M.**

### Task 39: `mambo`
**Needs Tasks 4, 7, 18.** Plays the CONTRABASS and PIANO voices and the alto voice whose `pp_bridge` id is
`RETIRED[1]` (`altosax60_lowpoly.synth` on `sax_tone`; ported here as `themes/mambo/alto.rs`, a voice of the theme,
never a fighter); `ability_fills` from the events. Reference `ballfight_trumpet_vs_trombone`. **Size M.**

### Task 40: `stadium_rock`
**Needs Task 22** (the ACOUSTIC voice). Option `plugged` (`PLUGGED`: a fighter of the fight, default the left one);
`lead_times` with the 5 HP hysteresis, `plugged_share`, `lufs` (its own K-weighting `lfilter` pair, ported here: the
loudness check's copy lives in `bf-export`, which `bf-content` cannot depend on), `lead_envelope` (rescales its own
buses through `Mix::bus_mut`); reads the story, `NAMES` and the winner. Reference `ballfight_electric_vs_acoustic`,
with `PLUGGED` on each side. **Size L.**

### Task 41: Register the themes
`PORTED_THEMES` += every theme task merged and not yet registered; the theme cards' tests; `studio/README.md`.
**Size S, controller.**

---

# Template P: pair gate for a kit config

**Files (owned):** `studio/fights/<a>_vs_<b>.toml` (the kit config as a fight file, field for field, the series tag
of an example included), `tools/studio_export/episode_<a>_vs_<b>.py`, `studio/reference/episodes/<a>_vs_<b>/`,
`studio/crates/bf-export/tests/episode_<a>_vs_<b>.rs`. The kit recording it draws from is the theme task's
(`studio/reference/themes/<theme>/<a>_vs_<b>.bfrec`), except for the two big top configs (Tasks 45, 46), whose
theme was ported in milestone 1: those gates export the kit recording into their own folder (`run.bfrec`).

**The references it exports** (`python -m tools.studio_export episode_<a>_vs_<b>`): the config's values
(`episode.json`); the golden frames as milestone 1's `frames.py` chose them (at least 40: the card, the count-in,
each fighter's three abilities at 0.2 s and 0.6 s, a weapon hit each, HP below 25 each, the K.O. and after, the
solo, the comment caption, the reassembly, the glide, the last two frames) as SHA-256 (`frames.json`), plus 16
title-card frames; the master's SHA-256 and per-bus RMS (`sound.json`); the kit's `check` numbers (`checks.json`);
4,000 seeds at the config's handicap (rows cached, pooled `stats.json`); the pair's links of Table S5 in scenario
fights (`pairs.json`, a coverage table of the pair's own events: for example `swat_shell`, a gulp of a rolling
drum, a slipped grab, a copied whirl).

**Steps:**
- [ ] **Step 1:** Export; `cache episodes/<a>_vs_<b>`; `budget`.
- [ ] **Step 2: Write the failing tests:** `fight_file_is_the_config` (every field equals `episode.json`, the chords
  through `chords_for`); `golden_frames` (both real looks; identical, or within the bar and listed in its
  `exceptions.toml`); `title_card`; `soundtrack` (the master's SHA-256 equals the kit's `audio.wav`; RMS; stems
  ignored); `checks` (verdicts and numbers within `checks_parity`'s bars); `statistics` (release: 4,000 Studio seeds
  against the kit's pooled figures, milestone 1's bars); `pair_links` (every row of `pairs.json` in the Rust run of
  its scenario at its time); `prefix` (ignored: the episode recording and the pair scenarios for the floor);
  `native_render` (ignored, long: `ballfight search --stop 10` on the fight file with its seed removed, then `render`
  of the first fit that passes every check; the seed, the K.O. and the render time go in the report).
- [ ] **Step 3:** FAIL. **Step 4:** Fix what fails in the owning fighter's or theme's files only through a fix round
  of that task (the controller reopens it), never in this task. PASS.
- [ ] **Commit subject:** `<a> vs <b>: the kit episode as a fight file, proven`.

- **42** Config: `examples/clarinet_vs_oboe`; Needs: 10, 11, 28; What it adds beyond the template: stun against a tuner;
  detune of a jumper; snip against teleports; Size: S.
- **43** Config: `examples/trombone_vs_horn`; Needs: 21, 30; What it adds beyond the template: a shield against the
  lance; FINAL FLICK into the ropes; Size: S.
- **44** Config: `ballfight_horn_vs_trombone_rematch`; Needs: 21, 30, 5; What it adds beyond the template: the sequel
  bed; `MIDDLE` REMATCH on the card; Size: S.
- **45** Config: `examples/trombone_vs_tuba`; Needs: 17; What it adds beyond the template: the big top in F minor (Task
  2's voicings); the lance against a swallower; Size: S.
- **46** Config: `examples/bassclarinet_vs_euphonium`; Needs: 16, 26, 5; What it adds beyond the template: copies of
  snip, twin, shadow and the whirl, drawn by the bass clarinet's look in amber; shells round a whirl; `TITLE_LINES`,
  `TITLE_BALLS`; Size: M.
- **47** Config: `ballfight_altosax_vs_tenorsax`; Needs: 14, 15, 31; What it adds beyond the template: spotlight against
  the crowd; a theme without `boom`; Size: S.
- **48** Config: `ballfight_bassclarinet_vs_bassoon`; Needs: 16, 19, 35; What it adds beyond the template: the bed's
  `erupt` from the part; the hook on a hopper's enemy; `TITLE_LINES`; `MIX`; Size: S.
- **49** Config: `ballfight_bassdrum_vs_tuba`; Needs: 17, 25, 32; What it adds beyond the template: `swallow_rollers`: a
  gulped drum ends its roll; `DRUMLINE` options; `MIX`; Size: M.
- **50** Config: `ballfight_cello_vs_contrabass`; Needs: 18, 24, 33; What it adds beyond the template: a walker slips
  the grab; a slap breaks it; the tango stops from the cello's rope slams; Size: M.
- **51** Config: `ballfight_electric_vs_acoustic`; Needs: 22, 23, 40; What it adds beyond the template: plugged and
  unplugged with the lead envelope; the smash against the rake; Size: M.
- **52** Config: `ballfight_euphonium_vs_trombone`; Needs: 26, 38; What it adds beyond the template: `swat_shells` (a
  swatted shell recorded with `landed = 3` and its time); SAD TROMBONE and PEDAL TONE copied, drawn by TROMBONE's look
  in amber, played as the wah-wah; Size: M.
- **53** Config: `ballfight_flute_vs_clarinet`; Needs: 10, 29; What it adds beyond the template: jumps past baskets; a
  stunned cobra charmer; Size: S.
- **54** Config: `ballfight_flute_vs_sousaphone`; Needs: 12, 32; What it adds beyond the template: the march tramples
  the baskets; Size: S.
- **55** Config: `ballfight_oboe_vs_bassoon`; Needs: 11, 19, 34; What it adds beyond the template: an out-of-tune
  hopper; a theme without `boom`; Size: S.
- **56** Config: `ballfight_piano_vs_organ`; Needs: 7, 9, 36; What it adds beyond the template: keys against the pipe
  lines and the shield; the roll from the config's own line; Size: S.
- **57** Config: `ballfight_trumpet_vs_clarinet`; Needs: 8, 10, 37; What it adds beyond the template: the HIGH C beam
  against jumps; stun on the punches; Size: S.
- **58** Config: `ballfight_trumpet_vs_trombone`; Needs: 8, 39; What it adds beyond the template: the lance against the
  combo; the trombone's `boom` over the mambo; Size: S.

Each also needs the register task that offered its fighters and theme (13, 20, 27 or 41).

---

# Integration

### Task 59: Starters, starting handicaps, theme compatibility

**Files:** `studio/crates/bf-content/src/starter.rs`, `studio/crates/bf-content/src/soundtrack.rs` (`validate`),
`studio/crates/bf-content/src/themes/mod.rs` (`themes_for(a, b)`), `studio/crates/bf-content/tests/starters.rs`,
`tools/studio_export/crossovers.py`, `studio/reference/episodes/crossovers/` (hashes, within the episodes budget)

**Interfaces:**
```rust
pub fn starter(left: &str, right: &str) -> Result<FightFile, ContentError>;   // any ordered pair of two kit fighters
pub fn themes_for(a: &str, b: &str) -> Vec<&'static str>;     // the ported themes whose hooks cover both parts
```
- **A kit pair** (Table S4, plus `flute_vs_trombone`) starts from its fight file. When the pair has two configs (HORN
  and TROMBONE), the one whose left fighter is the asked left one; the other order swaps the fighters, the handicap
  and the sides of every side-bound field (`PLUGGED`, `title_lines` keep their names). **The starting handicap is the
  kit's frozen `DMG_SIDE` with its `DMG_SCALE`** (Section 8, Q6), swapped with the sides; no handicap means
  `[1.0, 1.0]`.
- **Any other pair** starts from the left fighter's home config (its newest kit config, by the episode number in its
  folder's notes; an example only when it has no other): its theme (or, if the theme lacks a hook a part needs, the
  right fighter's home theme, else the big top), tempo, key, scale, loop, tail and options (a `PLUGGED` that names a
  fighter not in the pair becomes the left fighter); both fighters play the kit's sketch line and solo
  (`default_line`, `default_solo`), no title hits, no `MIX`; roles: the left fighter keeps its home role, the right
  takes its own home role unless that is the same, then `bass` if its card can play bass and the left's role is not
  `bass`, else `melody`, else `line`; handicap `[1.0, 1.0]`, scale 1.0, the pair's rules. This is a config the kit can
  play, so it is proven like any other (below).
- `validate` refuses a theme not in `themes_for` with Review Focus 1's message.

- [ ] **Step 1: Export** `crossovers`: for six crossover starters (one per theme family: PIANO vs TUBA, CELLO vs
  FLUTE, BASS DRUM vs HORN, ALTO SAX vs ORGAN, EUPHONIUM vs CLARINET, ELECTRIC vs BASSOON), the kit config this rule
  makes, the kit's own recording of its first fit, and its master's SHA-256 and 8 frame hashes.
- [ ] **Step 2: Write the failing tests:** all 380 ordered pairs give a starter that `validate` accepts and that
  round-trips through TOML; the 17 kit pairs' starters equal their fight files in both orders (handicap swapped);
  the six crossovers' fight files equal the export's configs, and their soundtracks and frames equal the kit's;
  `themes_for` excludes `dark_disco` and `heist` for any pair with TROMBONE, ORGAN, TUBA, EUPHONIUM or HORN (HORN
  needs `boom` or `trailer_hit`, and those two have neither), and includes every theme for any other pair.
- [ ] **Step 3:** FAIL. **Step 4:** Implement; PASS. **Step 5:** Commit `starters for every kit pair, the series'
  handicaps, theme compatibility`.

### Task 60: The app with the whole kit roster

**Files:** `studio/crates/bf-app/src/engine/{content,convert}.rs`,
`studio/crates/bf-app/src/screens/{matchup,music}.rs`, `studio/crates/bf-app/src/model/roster.rs`, the screens'
snapshot tests, `studio/crates/bf-app/tests/retired.rs`

- [ ] **Step 1: Write the failing tests:** the roster offers the 20 kit fighters, each with its card (portrait,
  weapon, abilities, range, usually plays, colour), and no chip says "not available yet"; the theme picker on Match-up
  lists `themes_for` the chosen pair and nothing else; the Music screen lists every chord loop of the theme (4 or 8
  chords, as one button each), shows the theme's options read-only ("The sequel", "No bass drums", "Plugged:
  Electric") and the roles from the fight file, with "Who plays bass" only when both cards can play bass; a pair rule
  shows its words; for a PIANO pair a key, loop or line change restarts the search and for any other pair it does not
  (Review Focus 7); the status bar reads "20 fighters, 14 themes" (the counts the app really has, as the interface
  reference asks); a 13-letter name fits the corner card at the 21 px size;
  `retired.rs`: a whole flow per kit pair at the fixtures' speed writes fight files, captions, folder names and logs
  that contain neither `RETIRED` name (Review Focus 6). Snapshots of Match-up with every chip offered, Music for
  `cello_vs_contrabass` and `electric_vs_acoustic`, Render for `bassdrum_vs_tuba`.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** PASS; milestone 2's tests pass.
- [ ] **Step 5:** Commit `bf-app: the whole kit roster and its themes`.

### Task 61: Full renders from the command line

- [ ] For each of the 18 kit configs (milestone 1's included) run `ballfight render studio/fights/<pair>.toml --rec
  <its kit recording> --out <scratch>` (the kit's own fight: every check passes, the mp4 decodes, its cover and
  caption are written) and the native render of its pair gate if that task ran it with `--ignored` only by name; and
  for the six crossovers of Task 59, `search --stop 10` then `render` of the first fit that passes every check. Write
  the table (pair, seed, K.O., checks, render time, mp4 SHA-256) into `docs/studio/m3-measurements.md` §2 (Task 62
  creates the file; this task fills that section). A pair whose first ten fits all fail a check is reported with the
  check and the numbers (a fit is not a promise of a video: `m1-measurements.md` §8). Commit `m3: full renders of
  every kit pair`.

### Task 62: Measurements

**Files:** `docs/studio/m3-measurements.md`
- [ ] With the other lanes idle: `ballfight bench` on one fight of each kit pair: fights per second, frame time
  (mean and 95th percentile, one thread) and soundtrack build time, against milestone 1's (`frame_ms_p95` 178 ms,
  `soundtrack_build_s` 4.67 s, `fights_per_second` 6,350); the heaviest looks named. The prefix window of every
  fighter's scenarios and every pair gate (shortest, median, how each ended); the statistics tables; the frame and
  stem exceptions with images; the reference set's size by folder; the models' total size and the exe's size; the
  rulings this milestone took. Commit `m3: measurements`.

### Task 63: Docs and the closing run

**Files:** `docs/studio/HANDOFF.md` (status: the Studio knows the 20 kit fighters and 14 kit themes; what the owner can
make now), `studio/README.md` (the export commands `fighter_*`, `theme_*`, `episode_*`, `cache`, `budget`),
`studio/docs/recording-format.md` (checked complete), `studio/docs/porting-a-fighter.md` (what the ports taught)
- [ ] `cargo test --workspace --release -- --include-ignored` after `python -m tools.studio_export cache all`; the
  Windows CI job green; `verify`; `budget`; `python tools/ballfight_replay_hash.py` and the kit-episode lines one by
  one, unchanged; `git status --short projects reelkit series` empty. The kit README is **not** edited (Section 8,
  Q13). Commit `milestone 3: docs and the closing run`.

---

## 8. Risks and open questions

Each item: what is at stake, what the controller decides, and the recommended answer.

1. **Q1 Storage.** Decided above (committed small set plus pinned cache, not LFS). Risk: a library or CPU change on
   this PC changes a regenerated file. Decide: re-pin or investigate. *Recommended:* investigate first; re-pin only
   when the cause is outside the kit (an OpenBLAS kernel, a numpy patch), with the ledger entry naming it.
2. **Q2 Theme and fighter compatibility.** The kit itself would raise if TROMBONE, ORGAN, TUBA, EUPHONIUM or HORN
   played a slam under `dark_disco` or `heist` (no `boom`). Decide: refuse the pair-theme combination, or give those
   beds a `boom`. *Recommended:* refuse and do not offer it (Tasks 2, 59, 60); adding a sound the kit never made would
   break "a classic arrangement is the kit's".
3. **Q3 IDENTITY CRISIS crosses every port.** A copy runs the other fighter's kind on the euphonium's ball, and its
   look draws through the other fighter's look. Decide: how far to prove it for non-kit opponents. *Recommended:*
   Task 26's `copy_sweep` over every fighter ported by then (fight parity and no-panic drawing), frames only for the
   two kit pairs (Tasks 46, 52); every kind is written for "ball `k`", never "the owner", and Review Focus 4 checks
   it.
4. **Q4 Qhull's merge step** is not ported (R35). ORGAN (two sites) and ALTO SAX (one) call `ConvexHull`. How it is
   found: the look exports log every call's points and vertices; Task 5's hull log lets the look tests compare call
   by call and count fallbacks over all committed and cached frames, plus a 1,000-frame sweep of ten fights each.
   Decide: what to do if one appears. *Recommended:* stop the task, report the call and the points; the controller
   opens a Task 9a to port Qhull 8.0.2's 2-D merge (about a day); no fallback is accepted in a gate.
5. **Q5 Shorter prefix windows.** New kinds have thresholds a last-bit drift can tip (a hop's landing, a grab's
   reach, a stun's end). Decide: what to accept. *Recommended:* milestone 1's rule unchanged: the 10 s floor holds;
   a break before it is a bug until the events around it show a knife-edge with the margin printed; the controller
   rules per case.
6. **Q6 Starting handicap.** Milestone 2's starter starts every pair at `[1.0, 1.0]`. The kit's frozen `DMG_SIDE` and
   `DMG_SCALE` were tuned to 40 to 60 percent for the 17 kit pairs, and Studio statistics match the kit's. Decide:
   start from them or from neutral. *Recommended:* start the kit pairs from the kit's values (shown on Balance as
   "the series' handicap"; Auto-balance still available), neutral for the rest.
7. **Q7 Pairs without a kit config.** 346 of the 380 ordered pairs (all but the 17 kit pairs in both orders) have
   no written music. Decide: offer them with the kit's sketch lines, or only offer the 17. *Recommended:* offer all
   with Task 59's rule (the kit plays the same config, so it is proven the same way); the Music screen says "Lines:
   the kit's sketch" for a sketch line until milestone 5's composer replaces it.
8. **Q8 Roles.** The kit's roles are richer than bass and melody (`riff`, `line`, `rhythm`, `drums`, `counter`), and a
   part's code branches on them. Decide: what the Music screen lets the user change. *Recommended:* in this milestone
   roles come from the fight file and are shown read-only, except the bass swap when both cards can play bass;
   milestone 5 designs the roles control.
9. **Q9 Voice ids with a retired name.** ELECTRIC's `pp` and the mambo's alto are Pixel Pitch voices that
   `pp_bridge` lists under the two retired names. Decide: rename them in the Studio or keep the ids. *Recommended:*
   keep them as internal ids (an extension of R14, written as a ruling), with Task 60's test that they never reach
   anything the user sees or the Studio writes.
10. **Q10 Fight file format 2.** Milestone 2's Library may hold format 1 files on the owner's PC. *Recommended:* read
    format 1 with Task 2's mapping (no tail, no mix, no options), write format 2; the search key's prefix changes,
    so cached shortlists are rebuilt once.
11. **Q11 Render speed.** TRUMPET's blurred beams, ORGAN's hulls and TENOR SAX's crowd add work per frame; the full
    render was 81 s against a 60 s target before any of them. *Recommended:* measure in Task 62 and plan any speed
    work for milestone 6, without changing a bit of a frame; no target changes here.
12. **Q12 The reference budget after this milestone.** The set ends near its 147 MB cap. Milestone 4 adds ten
    fighters and twelve themes. *Recommended:* milestone 4's plan moves every Python recording to the pinned cache (the
    always-run picture and sound gates then run from the cache too, by name), keeping only hashes and fingerprints
    committed; Git LFS stays a fallback if a fresh clone without Python must verify the port.
13. **Q13 The kit README.** The spec (§10) says the kit README is updated once Rust is the source of truth; this
    plan's rule forbids edits under `projects/`. *Recommended:* leave it untouched in milestone 3 (the port is not
    complete until milestone 4); write the note in `HANDOFF.md` and ask the owner at milestone 4's close for a
    one-line edit under `projects/`.
14. **Q14 Six lanes on one PC.** Six cargo targets (each several GB) and the export's process pools share 16 threads
    and 64 GB. *Recommended:* the resource rule in the Global Rules; the controller lowers to four lanes if a
    lane's test run takes more than twice its alone time.
15. **Q15 A wrong socket.** A stub's signature in a shared file may not fit the kind once ported. *Recommended:* the
    one-line change rule of Section 3; two lanes changing the same call line is a merge the controller does by hand.
16. **Q16 Sounds that read another part or another look** (OBOE's and EUPHONIUM's enemy voice, BASS CLARINET's,
    BASS DRUM's and SOUSAPHONE's own look state). *Recommended:* Task 2's `play_with` for the enemy's part; a
    fighter's own look state recomputed in its `state.rs` for both its look and its part, as FLUTE's `Charmer` does.

**What could not be determined from the code** (and how each is settled during the work):
- Whether Git LFS's transfer between local lane clones works on this PC (not tried; one reason LFS was not chosen).
- The size of the 18 new model meshes and of each fighter's recording (estimated from FLUTE's 58 KB and TROMBONE's
  271 KB meshes and milestone 1's 130 to 145 KB recordings; Task 1's budget test settles it per folder).
- Whether any Qhull merge happens in ORGAN's or ALTO SAX's frames (Q4).
- How long the new kinds' prefix windows are (each port's report).
- How cleanly ACOUSTIC's branches separate from the older surf and spout code in `surf_rake_step` and
  `dash_cone_step` (only their headers were read; Task 22 reports it line by line).
- Which renderer attributes beyond `shade` and `side_ends` a new look reads from `kit/picture.py` (the survey caught
  called functions; module attributes such as `P.XX_` are found by each port, NEEDS_CONTEXT rule).
- The final state of milestone 2 (Tasks 32 and 35 were still open in this clone); Task 60 waits for them.

## 9. What milestone 3 proves, mapped to the spec

- **§6 prefix agreement, kind coverage, statistics, self-replay, for every kit fighter** Where it is proven: Template F
  (Tasks 7 to 26), Template P (42 to 58).
- **§6 "two fighters inside, a list outside"; search keys; balance** Where it is proven: Tasks 2, 7, 59, 60.
- **§7 identical frames for every ported fighter, the exceptions file** Where it is proven: Template F, Template P.
- **§8 voices, parts and theme parts against the kit, the classic arrangement** Where it is proven: Tasks 4, Template F,
  Template T.
- **§9 checks, encode and folders for every kit pair** Where it is proven: Template P, Task 61.
- **§10 one folder per fighter and per theme; the registry; the 20 kit fighters and 14 kit themes** Where it is proven:
  Tasks 3, 5, 13, 20, 27, 41.
- **§10 adding a fighter after launch** Where it is proven: Task 6's `porting-a-fighter.md`.
- **§11 the app reads the roster and themes from the registry** Where it is proven: Task 60.
- **§12 reference tools, manifest, the 150 MB rule** Where it is proven: Tasks 1, 63.
- **§13 CI on Windows** Where it is proven: Tasks 6 (replay files under `bf-sim`), 63.
- **§14 milestone 3: "the other 18 kit fighters and 13 kit themes, parallel work, each with its own tests"** Where it is
  proven: this plan.
