# Porting a kit fighter, theme or pair: the proving harness

This is the manual a port worker follows (milestone 3, Templates F, T and P), and the answer to the spec's "Adding a
fighter after launch" (section 10): a fighter, a theme or a kit episode is in the Studio when the proving harness,
`studio/crates/bf-harness`, passes on it against the Python kit. The harness was proven on FLUTE, TROMBONE, the big
top and the example episode (FLUTE against TROMBONE), whose milestone 1 gates it repeats with the same verdicts.

Commands below run Python from the clone root and cargo from `<clone>\studio`, in PowerShell:

```powershell
$env:PYTHONHASHSEED = "0"; $env:PYTHONPATH = (Get-Location).Path      # once, at the clone root
$py = "F:\!Ai Stuff\InstrumentBattleMachine\.studio-tools\py313\Scripts\python.exe"
& $py -m tools.studio_export <export or command>                        # below: `python -m tools.studio_export`
$env:CARGO_BUILD_JOBS = "3"                                             # in <clone>\studio, once
```

**Build only your own test binary** (`--test fighter_<slug>`, `--test theme_<name>`, `--test episode_<pair>`): each
harness test binary is a few dozen MB, and `cargo test -p bf-harness` alone would build every port's.

## What a port owns, and what it never touches

A port edits **only its own files**:

| What | Where |
| --- | --- |
| its look, voice, part, card, model, state | `bf-content/src/fighters/<slug>/` (`look.rs`, `voice.rs`, `sound.rs`, `card.rs` holds `CARD`: the card's texts, `state.rs`, `mod.rs` with `look()`, `voice()`, `sound()`, `card()`, `MESH`) |
| its model | `bf-content/data/models/<NAME>.json`, written by its export (`fighter_model`), compiled in by `MESH = Some(include_str!(..))` |
| its title pattern | `bf-picture/src/title/patterns/<slug>.rs` (set `PORTED = true`) when the kit's `title.py` `PATTERNS` names it; otherwise its look's `title_pattern_with` |
| its kinds | its stubs in `bf-sim/src/kinds/`, `bf-sim/src/abilities/`, `bf-core/src/roster/kinds/`, `bf-core/src/roster/abilities/` (Task 3) |
| its references | `tools/studio_export/fighter_<slug>.py`, `studio/reference/fighters/<slug>/`, `studio/replay/fighters/<slug>.txt` |
| its record keys | its own section of `studio/docs/recording-format.md`, between its markers |
| its one test file | `studio/crates/bf-harness/tests/fighter_<slug>.rs` (cargo finds it: no `Cargo.toml` line) |

**Never the port's:** `bf_content::registry::PORTED` (the register tasks add a fighter), the pair rules
(`bf-content/src/pair_rules.rs` and `m3refs.PAIR_RULES` in Python), `studio/reference/rulings.toml` (the
controller's), any shared engine or renderer file (plan section 3), anything under `projects/`, `reelkit/`,
`series/`. If a port needs a new pair rule, a helper in a shared file or an exception, it **stops and reports
NEEDS_CONTEXT** with the exact change it needs.

A theme owns `bf-content/src/themes/<name>/`, `tools/studio_export/theme_<name>.py`, `studio/reference/themes/<name>/`
and `bf-harness/tests/theme_<name>.rs`; a pair gate owns `studio/fights/<pair>.toml`,
`tools/studio_export/episode_<pair>.py`, `studio/reference/episodes/<pair>/` and `bf-harness/tests/episode_<pair>.rs`.

## The test files

```rust
//! TUBA against the Python kit: every gate of Template F (studio/docs/porting-a-fighter.md).

bf_harness::fighter_suite!("TUBA");
```

That is the whole file (`tests/fighter_flute.rs` and `tests/fighter_trombone.rs` are exactly this). The macro writes
one `#[test]` per gate, named after it. A port may add tests of its own below the macro line (EUPHONIUM's copy sweep,
a kind's own cases), using `bf_harness::fight`, `look`, `sound`, `voice` and `checks` and the constant `NAME`.

A theme names its voicing functions (one per voicing method of its kit `Bed`; the gate checks that every one is
given); a pair names its folder:

```rust
bf_harness::theme_suite!("heist", [("voice", bf_content::themes::heist::voice)]);
```

```rust
bf_harness::episode_suite!("oboe_vs_bassoon");
```

## The recipe of a fighter port (Template F)

1. **The export.** Write `tools/studio_export/fighter_<slug>.py`:

   ```python
   from . import m3refs
   SPEC = dict(name='TUBA', opponent='FLUTE')
   EXPORTS = {'fighter_tuba': lambda: m3refs.fighter_refs(SPEC)}
   ```

   Only the keys that differ from the defaults (`m3refs.py`'s docstring and `tools/studio_export/README.md`):
   `fights` (handicap, rules, a fixed `left` / `right` seed, `must` coverage items), `scenarios`, `frames` (`left` 64,
   `right` 32, extra `times`), `roles` (the roles its part branches on, Table S3), `voice_notes` (`pitches`,
   `velocities`, `durations`), `sprite_size` (when the look draws its own sprites at another size than FLUTE's
   2 r + 5: `dict(add=7, clip=(12, 19))` is 2 clip(r, 12, 19) + 7, TRUMPET's; a padded sprite adds `pad`:
   `dict(add=7, pad=1.22)` is 2 int(round(r 1.22)) + 7, SOUSAPHONE's), `left_in_cache=True` (ruling M3-R36: only
   when the left recording alone would take the folder over 300 KB, PIANO's; it then lives in the pinned cache with
   its SHA-256 committed, and every gate that reads it needs the cache). A neutral handicap may be the template's
   `DMG_SIDE = (1.0, 1.0)` or `None`: the kit's fights are bit-identical, and the gates read both as the same rules.
   A file of its own beyond the
   template's is made by an `extra` part: `m3refs.fighter_refs(SPEC, extra=<fn>)`, called with `(spec, folder,
   export, module, main, left_seed, left_file)` after the template's parts, in the same run. Never wrap or patch
   the framework or the kit: a `fork` pool the kit asks for maps serially in every part's process
   (`m3parts.serial_fork`), the left recording is on disk for the parts, numpy floats in the kit's events pass the
   replay guard by value, and the left fight is chosen to end after 44 s (the checks need it). Then
   `python -m tools.studio_export fighter_<slug>`: about 7 minutes of fights and
   statistics (3 workers) and 3 to 5 of picture, sound, voice and checks, each part in a fresh process and kit folder
   of its own. It writes `studio/reference/fighters/<slug>/`:

   | File | What it holds | Read by |
   | --- | --- | --- |
   | `fight.json` | the config, the two look fights, every scenario with its full rules, the pooled statistics, the coverage table | `coverage`, `replay`, `left_fight`, `prefix`, `stats` |
   | `left.bfrec` (or a pointer in fight.json; with `left_in_cache`, the pinned cache file `left.bfrec` and its SHA-256 in fight.json, M3-R36) | the fighter left, winning, all three abilities fired, the K.O. after 44 s (the checks run the kit's `music.main()`, which needs it; a `fights.left` that ends earlier stops the export at once, saying so) | every always-run gate |
   | `look.json` | the left fight's frames (SHA-256 of the bytes, the hook calls, the draw log), its hull calls as counts and one SHA-256 (`left.hull`, ruling M3-R29: the calls themselves are in the cache), the VS card at 16 times on either side, the title hooks, the sprites' digest, the roster keys the kit look reads (`cfg_keys`) | `look_frames`, `title`, `sprites`, `setup_as` |
   | `sound.json` | the left soundtrack (real part, the kit's base `Sound` for the opponent): the 16-bit master's SHA-256, the buses, every stem's RMS per 100 ms, the first-time notes, the noise stream's end; the right soundtrack's facts (its RMS is cached); each role (its own bus and the master); the bed hooks its part calls | `sound`, `sound_stems`, `bed_calls` |
   | `voice.json` | the spec's notes (24 by default): length, peak, 10 ms block RMS, the noise state before and after | `voice` |
   | `checks.json` | every check of `kit/checks.py` on the left fight (both real looks and parts); the per-frame rows are cached | `checks` |
   | `exceptions.toml` | empty | `exceptions` |
   | `pins.json`, `manifest.json` | the cache's pins, the folder's manifest shard | `reference_folder` |

   The cache, `studio/reference/.cache/m3/fighters/<slug>/` (git-ignored, pinned): `right.bfrec`, the scenario
   recordings, `stats_rows.json`, `look_right.json`, `hull_left.json` / `hull_right.json` (every hull call of every frame: its points and vertices, ruling M3-R29),
   `sprites.npz` / `.json` (ruling M3-R18), the stem windows `stems_*.npz`, `sound_right_rms.json`,
   `voice_notes.npz`, `checks_rows.json`.
2. **The cache.** `python -m tools.studio_export cache fighters/<slug>`: regenerates every cache file and must match
   every pin and reproduce every committed file byte for byte (7 to 10 minutes). It never changes the tree.
3. **The budget.** `python -m tools.studio_export budget`: the folder at most 300 KB. `python -m tools.studio_export
   verify`: no problems.
4. **Look at it.** Dump six frames (`python -m tools.studio_export fighter_stages <slug> left 300 900 1500` and the
   same for `right`) and confirm they show the fighter: its weapon, an ability, its shards.
5. **The test file** (above). On a port with nothing ported yet the fight, look, sound and voice gates fail, each
   saying what is missing ("the engine has not ported <NAME>'s weapon kind", "<NAME>'s look() is None", ...);
   `reference_folder`, `exceptions` and `looks_take_numbers_from_setup_as` already pass.
6. **Port**, committing at each: the roster config and kinds (with `PORTED = true`), the record keys and their
   `recording-format.md` section, the look, the voice, the part, the card. The engine first:

   ```powershell
   cargo test -p bf-harness --test fighter_<slug> -- --test-threads=3 coverage left_fight record_flag
   ```

   Bless the replay file once the fights agree:

   ```powershell
   $env:BF_BLESS = "1"; cargo test -p bf-harness --test fighter_<slug> -- --exact replay; Remove-Item Env:BF_BLESS
   ```

7. **The gates** (ruling M3-R24: `checks` and the cache gates are ignored and run by name):

   ```powershell
   # always-run gates, debug
   cargo test -p bf-harness --test fighter_<slug> -- --test-threads=3
   # every ignored gate but the stage dumps, by name (release is faster; run both the first time)
   cargo test --release -p bf-harness --test fighter_<slug> -- --test-threads=3 --ignored --skip stages --skip stats
   # the statistics: release only
   cargo test --release -p bf-harness --test fighter_<slug> -- --test-threads=1 stats
   ```

   The ignored gates are `prefix`, `look_frames_cached`, `hull_points`, `scenarios_draw`, `voice_notes`,
   `sound_stems`, `checks` (and `stages`, which needs dumps, step 8). Never run `--include-ignored` without
   `--skip stages` before you have made dumps. Milestone 1's gates stay unchanged: `cargo test -p bf-sim --test
   replay`, `cargo test --release -p bf-content --test golden_frames --test stems`.
8. **When a frame differs**: `python -m tools.studio_export fighter_stages <slug> [left | right] <frame>...` dumps
   every stage of the kit's renderer for those frames (drawn as the gate draws them) into
   `studio/reference/.cache/stages_fighter/<slug>/<fight>/`, and

   ```powershell
   cargo test -p bf-harness --test fighter_<slug> -- --ignored --exact stages
   ```

   names the first stage of each dumped frame that is not the kit's, with the first pixel and the largest
   difference. Fix that stage.
9. **When a number differs in the last bit** (Python's C runtime against `bf_core::dmath`, after real investigation):
   **stop**. Report it to the controller with the frame or sample, the stage, both numbers and the cause. Never widen
   a bar, never write an exception: an exception is the controller's ruling, listed in `studio/reference/rulings.toml`
   (only the controller edits it); the harness fails on any `exceptions.toml` entry without its exact row there, and
   on a ruled entry that excuses nothing any more.
10. **If the spec changes before merge**, `python -m tools.studio_export remake fighters/<slug>` makes the folder
    whole again (committed files, shard and pins). On a merged folder it is the controller's decision (M3-R1).

## The gates of a fighter (`fighter_suite!`) and their minimums

Times on FLUTE and TROMBONE on the reference PC with other lanes busy (the Task 6 report has them). Every gate fails
on an empty input: the minimum column says what it requires.

| Test | Runs | What it proves | Minimum |
| --- | --- | --- | --- |
| `reference_folder` | always | every file the suites read is there and in its manifest shard; the left recording is the one fight.json describes | the 7 files |
| `exceptions` | always | every `exceptions.toml` entry has its exact row in `rulings.toml` | (none needed) |
| `coverage` | always | every coverage row occurs in the Rust run of its scenario (seed and full rules; no cache), at its index, with its fields to 1e-6, as the first of its item; every required item and ability covered | a row per required item, at least 4 |
| `replay` | always | `studio/replay/fighters/<slug>.txt`: both side orders, seeds 1, 2, 3, 500, 1 << 40 | 10 lines |
| `record_flag` | always | events, winner and K.O. equal with the recording on and off | 10 fights |
| `left_fight` | always | the fight file's rules are the recording's (a recorded `dmg_side` of `[1.0, 1.0]` counts as none, which is what the Studio records for a neutral handicap; `fight::rules_text`); the Rust fight agrees with the left fight for the 10 s floor | 1 fight |
| `prefix` | ignored | left, right and every scenario agree for `PREFIX_FLOOR_S` = 10 s of fighting from `t_fight`, or to the end (M3-R5) | 2 + every scenario fight |
| `stats` / `stats_wide` | release | 2,000 / 20,000 Rust fights against the kit's pooled figures, milestone 1's bars; the fits (ruling M3-R32): where the kit has fits the port has at least one, and wherever either has fits the two counts agree under a two-proportion test, \|z\| at most 3 (`fight::fits_present`; it replaces milestone 1's fixed "above 0.002", which refused sampling noise at rare rates) | the seeds |
| `look_frames` | always | the left fight with the real look and `DefaultLook`: frame bytes, hook calls, draw log; hull calls: every call's point count and vertices, as the SHA-256 of `<frame> <points> <v0>,<v1>,...` lines against `left.hull` (M3-R29; FLUTE and TROMBONE, format 1: each row's calls, the point sums within 1e-12 px a point); the build's hooks; a hull fallback STOPS the task (M3-R4) | the spec's frames (64) |
| `look_frames_cached` | ignored | the same for the right fight | the cache's rows |
| `hull_points` | ignored | every hull point of both fights within 1e-12 px and every vertex, from the pinned hull logs; the logs are the ones look.json's `hull` describes (counts, SHA-256). A `look_frames` hull failure is named frame by frame here | the frames and calls of look.json |
| `stages` | ignored (dumps) | step 8 | a dump |
| `scenarios_draw` | ignored | every scenario fight drawn with both real looks (EUPHONIUM's copy sweep, M3-R3): no panic, no hull fallback; only a fight without a K.O. has no video | 1 fight |
| `title` | always | the VS card alone on the fixed noise image (16 frames a side), the close-up, the panel pattern, glass, flare scale, shatter parts | 16 + 16 frames |
| `sprites` | always | every sprite of the kit's sprite cache, drawn and hashed: the digest is the kit's | the case list |
| `setup_as` | always | `cfg_summary` names exactly the kit look's `cfg_keys`; `setup` = `setup_as` with its own numbers; another fighter's numbers differ | the keys |
| `looks_take_numbers_from_setup_as` | always | the source scan (below) | 20 folders |
| `voice` | always | the spec's notes from their noise states: length exact, peak and every 10 ms block RMS within 1e-6, the noise where the kit's stood | the spec's count |
| `voice_notes` | ignored | every sample within 1e-6 | the notes |
| `sound` | always | the left soundtrack and each role: 16-bit master SHA-256, buses, stems, every 100 ms block within 0.01 dB above -90 dBFS, notes, noise end | a first-time note of its part, or, when the kit's own count is 0 (a part that plays only through its own synth, CLARINET), its own bus a stem above -60 dBFS in at least 100 blocks (`sound::part_is_compared`) |
| `sound_stems` | ignored | the right soundtrack likewise (cached RMS), every stem window within -120 dBFS | a window a soundtrack |
| `bed_calls` | always | `Sound::bed_calls()` is every `bed.<hook>(` of the Python part not under `hasattr` | |
| `checks` | ignored (M3-R24) | every check of the left fight: the soundtrack is the kit's `audio.wav`; verdicts and numbers within `checks_parity.rs`'s bars | the cached rows |

Every fight any gate runs goes through `bf_sim::run_traced`: a socket stub reached (a kind whose `PORTED` is true
with an empty socket) fails with the stub's name.

## Looks: the roster numbers come from `setup_as`

EUPHONIUM's copies draw another fighter's look for the euphonium's side with the copied fighter's numbers
(`euphonium.py` 192-201, `Look::setup_as`). So:

- In every file of `bf-content/src/fighters/<slug>/` but `sound.rs` and `voice.rs`, code never reads the picture's
  per-side config or view: no `p.cfg`, `.cfg.`, `.cfg[`, `p.view`, `.view[`, `roster()`, `fighters::entry(`,
  `look_for(`, `mesh_json(`. The one allowed line is the look's `setup`, exactly `self.setup_as(p, k, &p.cfg[k]);`.
  Every roster number comes from the `cfg` that `setup_as(p, k, cfg)` is given. (`p.fighter(k, t)`, a ball's state, is
  fine.)
- A look whose kit class reads `self.cfg` implements `Look::cfg_summary` as `key=value` pairs separated by `; `, one
  per key of `look.json`'s `cfg_keys` (the export lists them: `abilities.low.radius`, `view.2`, ...). FLUTE returns
  `abilities.low.name=...; abilities.low.speed=...; abilities.unlock2.dur_beats=...; view.2=...; weapon.rise=...`,
  TROMBONE `abilities.low.radius=...`.
- A look whose kit class reads no `self.cfg` keeps the default `None`.

## The recipe of a theme (Template T)

`tools/studio_export/theme_<name>.py` with `SPEC = dict(name=..., voicings=(the Bed's voicing methods),
configs=(its reference kit configs, Table S4), options=(one variant per option value: dict(name, text, ff, why)))`
and `EXPORTS = {'theme_<name>': lambda: m3refs.theme_refs(SPEC)}` (`theme_big_top.py`). It writes `theme.json` (each
config's recording and its bed alone: the 16-bit master), `voicings.json` (12 roots x 13 qualities), `bed.json`
(the bed alone's buses, RMS and notes; a second tempo, a second key and each option, with their recordings cached;
the optional hooks the class defines, each called at three times with three gains, vectors cached),
`exceptions.toml` and the cache. Then `cache themes/<name>`, `budget`, and `tests/theme_<name>.rs`.

The bed alone never waits for fighters: a fighter whose voice is not ported yet gets a stand-in voice (it panics in
plain words if the bed ever plays it), and `bed_alone_stand_ins` proves the bed plays no fighter voice at all. A theme
whose bed itself plays a fighter's voice (hot jazz plays the TUBA's) waits for that fighter.

```powershell
cargo test -p bf-harness --test theme_<name> -- --test-threads=3                                 # always run
cargo test --release -p bf-harness --test theme_<name> -- --test-threads=3 --ignored            # by name
```

Always run: `voicings` (every voicing method, 156 chords each), `bed_alone_stand_ins`, `reference_folder`,
`exceptions`, `bed_alone`, `hooks` (`Bed::hooks()` equals the class's methods), `card` (the configs are presets).
Ignored: `bed_variants` (at least 2), `bed_stems` (a window a soundtrack), `hook_vectors` (3 calls a hook).

## The recipe of a pair gate (Template P)

`studio/fights/<pair>.toml` (the kit config, field for field), `tools/studio_export/episode_<pair>.py` with
`SPEC = dict(config='ballfight_<pair>', links=(the pair's own events of Table S5), weapon_hit={fighter: how})` and
`EXPORTS = {'episode_<pair>': lambda: m3refs.episode_refs(SPEC)}` (`episode_flute_vs_trombone.py`). The kit's
recording of the config is pointed at when the theme folder (or milestone 1) holds it. It writes `episode.json`,
`frames.json` (the golden frames, both real looks, and 16 card frames), `sound.json` (the 16-bit master SHA-256 =
`audio.wav`, the master's and the two fighters' RMS; the bed buses' RMS cached), `checks.json` (rows cached),
`stats.json` (4,000 seeds at the config's handicap; rows cached), `pairs.json`, `exceptions.toml`.

```powershell
cargo test -p bf-harness --test episode_<pair> -- --test-threads=3                               # always run
cargo test --release -p bf-harness --test episode_<pair> -- --test-threads=3 --ignored --skip native_render
cargo test --release -p bf-harness --test episode_<pair> -- --test-threads=1 statistics
cargo test --release -p bf-harness --test episode_<pair> -- --ignored --exact native_render      # long, ffmpeg
```

Always run: `reference_folder`, `exceptions`, `fight_file_is_the_config`, `recording`, `golden_frames` (at least 40),
`title_card` (16), `soundtrack`, `pair_links` (a link whenever the pair has a pair rule). Ignored: `checks`,
`stems`, `prefix`, `native_render`; release: `statistics`.
