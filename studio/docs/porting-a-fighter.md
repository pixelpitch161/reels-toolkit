# Porting a kit fighter, theme or pair: the proving harness

This is the manual a port worker follows (milestone 3, Templates F, T and P), and the answer to the spec's "Adding a
fighter after launch" (section 10): a fighter, a theme or a kit episode is in the Studio when the proving harness,
`studio/crates/bf-harness`, passes on it against the Python kit. The harness was proven on FLUTE, TROMBONE, the big
top and the example episode (FLUTE against TROMBONE), whose milestone 1 gates it repeats with the same verdicts.

## The one rule of ownership

A port edits **only its own files**: its fighter folder `bf-content/src/fighters/<slug>/`, its kinds in `bf-sim` and
`bf-core/src/roster/` (the stubs Task 3 made for it), its title pattern file
`bf-picture/src/title/patterns/<slug>.rs`, its model `bf-content/data/models/<NAME>.json`, its export module
`tools/studio_export/fighter_<slug>.py`, its reference folder `studio/reference/fighters/<slug>/`, its replay file
`studio/replay/fighters/<slug>.txt`, its section of `studio/docs/recording-format.md`, and **one test file**,
`studio/crates/bf-harness/tests/fighter_<slug>.rs`. No shared list, no shared test file, no `Cargo.toml` line: cargo
finds the test file by itself. A theme owns `bf-content/src/themes/<name>/`, `tools/studio_export/theme_<name>.py`,
`studio/reference/themes/<name>/` and `bf-harness/tests/theme_<name>.rs`; a pair gate owns
`studio/fights/<pair>.toml`, `tools/studio_export/episode_<pair>.py`, `studio/reference/episodes/<pair>/` and
`bf-harness/tests/episode_<pair>.rs`.

## What a port's test file contains

```rust
//! TUBA against the Python kit: every gate of Template F (studio/docs/porting-a-fighter.md).

bf_harness::fighter_suite!("TUBA");
```

That is the whole file (`tests/fighter_flute.rs` and `tests/fighter_trombone.rs` are exactly this). The macro writes
one `#[test]` per gate, named after it; a port may add tests of its own below the macro line (EUPHONIUM's copy sweep,
a kind's own cases), using `bf_harness::fight`, `look`, `sound`, `voice` and `checks` and the constant `NAME`.

A theme's file adds its voicing functions (they are its own); a pair's file is one line:

```rust
bf_harness::theme_suite!("heist");

#[test]
fn voicings() {
    use bf_content::themes::heist;
    bf_harness::theme::voicings(NAME, &[("voice", heist::voice)]);
}
```

```rust
bf_harness::episode_suite!("oboe_vs_bassoon");
```

## The recipe of a fighter port (Template F)

Run Python from the clone root with `$env:PYTHONHASHSEED = "0"` and `$env:PYTHONPATH = (Get-Location).Path`, through
`F:\!Ai Stuff\InstrumentBattleMachine\.studio-tools\py313\Scripts\python.exe` (below: `python`). Run cargo from
`<clone>\studio` with `CARGO_BUILD_JOBS=3` and `--test-threads=3`.

1. **The export.** Write `tools/studio_export/fighter_<slug>.py`:

   ```python
   from . import m3refs
   SPEC = dict(name='TUBA', opponent='FLUTE')
   EXPORTS = {'fighter_tuba': lambda: m3refs.fighter_refs(SPEC)}
   ```

   Only the keys that differ from the defaults (`m3refs.py`'s docstring and `tools/studio_export/README.md`):
   `fights` (handicap, rules, a fixed `left` / `right` seed, `must` coverage items), `scenarios`, `frames`
   (`left` 64, `right` 32, extra `times`), `roles` (the roles its part branches on, Table S3), `voice_notes`
   (`pitches`, `velocities`, `durations`). Then `python -m tools.studio_export fighter_<slug>`: about 7 minutes of
   fights and statistics (3 workers) and 4 to 5 minutes of picture, sound, voice and checks, each part in a fresh
   process. It writes `studio/reference/fighters/<slug>/`:

   | File | What it holds | Read by |
   | --- | --- | --- |
   | `fight.json` | the config, the two look fights, every scenario with its full rules, the pooled statistics, the coverage table | `coverage`, `replay`, `left_fight`, `prefix`, `stats` |
   | `left.bfrec` (or a pointer in fight.json) | the fighter left, winning, all three abilities fired | every always-run gate |
   | `look.json` | the left fight's frames (SHA-256 of the bytes, the hook calls, the draw log, the hull calls), the VS card at 16 times on either side, the title hooks, the sprites' digest, whether the kit look reads `self.cfg` | `look_frames`, `title`, `sprites`, `setup_as` |
   | `sound.json` | the left and right soundtracks with its real part and the kit's base `Sound` for the opponent: the 16-bit master's SHA-256, the buses, every stem's RMS per 100 ms, the first-time notes, the noise stream's end; each role in `roles`; the bed hooks its part calls | `sound`, `sound_stems`, `bed_calls` |
   | `voice.json` | 24 notes: length, peak, 10 ms block RMS, the noise state before and after | `voice` |
   | `checks.json` | every check of `kit/checks.py` on the left fight (both real looks and parts) | `checks` |
   | `exceptions.toml` | empty; only the controller's rulings ever go in it | `exceptions` |
   | `pins.json`, `manifest.json` | the cache's pins, the folder's manifest shard | `reference_folder` |

   The cache, `studio/reference/.cache/m3/fighters/<slug>/` (git-ignored, pinned): `right.bfrec`, the scenario
   recordings, `stats_rows.json`, `look_right.json`, `hull_left.json` / `hull_right.json` (every hull point),
   `sprites.npz` / `.json` (ruling M3-R18), the stem windows `stems_*.npz`, `voice_notes.npz`.
2. **The cache.** `python -m tools.studio_export cache fighters/<slug>`: regenerates every cache file and must match
   every pin and reproduce every committed file byte for byte (about 10 minutes on a busy PC). It never changes the
   tree.
3. **The budget.** `python -m tools.studio_export budget`: the folder at most 300 KB (FLUTE's and TROMBONE's are
   about 230 KB). `python -m tools.studio_export verify`: no problems.
4. **Open six frames** of the left fight with `fighter_stages` (step 8) or the kit and confirm they show the fighter.
5. **The test file** (above). Run it: every gate fails, each saying what is not ported yet.
6. **Port**, committing at each: the roster config and kinds (with `PORTED = true`; `cargo test -p bf-harness --test
   fighter_<slug> coverage left_fight`), the record keys and their `recording-format.md` section, the look, the
   voice, the part, the card. Bless the replay file once the fights agree: `BF_BLESS=1 cargo test -p bf-harness
   --test fighter_<slug> replay`.
7. **The gates:**

   ```text
   cargo test -p bf-harness --test fighter_<slug> -- --test-threads=3                     always-run gates
   cargo test -p bf-harness --test fighter_<slug> -- --test-threads=3 --include-ignored   + the cache's
   cargo test --release -p bf-harness --test fighter_<slug> -- --test-threads=1 stats     the statistics
   ```

   Milestone 1's gates stay unchanged: `cargo test -p bf-sim --test replay`, `cargo test --release -p bf-content
   --test golden_frames --test stems`.
8. **When a frame differs**: `python -m tools.studio_export fighter_stages <slug> [left | right] <frame>...` dumps
   every stage of the kit's renderer for those frames (drawn as the gate draws them) into
   `studio/reference/.cache/stages_fighter/<slug>/<fight>/`, and `cargo test -p bf-harness --test fighter_<slug>
   stages -- --ignored` names the first stage of each frame that is not the kit's, with the first pixel and the
   largest difference.
9. **When a number differs in the last bit** (Python's C runtime against `bf_core::dmath`): report it with the
   frame or sample, the stage, both numbers and the cause; never widen a bar, never add an exception. The controller
   rules; a ruling is recorded as an `exceptions.toml` entry naming it (`ruling = "M3-R<n>"`).
10. **If the spec changes before merge**, `python -m tools.studio_export remake fighters/<slug>` makes the folder
    whole again (committed files, shard and pins). On a merged folder it is the controller's decision (M3-R1).

## The gates of a fighter (`fighter_suite!`)

Times measured on FLUTE and TROMBONE on the reference PC, test profile, other lanes busy (see the Task 6 report).

| Test | Runs | What it proves |
| --- | --- | --- |
| `reference_folder` | always | every file the suites read is there and in its manifest shard; the left recording is the one fight.json describes |
| `exceptions` | always | `exceptions.toml` exists; every entry names a controller ruling, a known gate, what and why |
| `coverage` | always | every row of the coverage table occurs in the Rust run of its scenario (seed and full rules; no cache), at its index, with its fields to 1e-6, as the first of its item; every required item and every ability is covered |
| `replay` | always | `studio/replay/fighters/<slug>.txt`: both side orders, seeds 1, 2, 3, 500, 1 << 40 |
| `record_flag` | always | events, winner and K.O. equal with the recording on and off |
| `left_fight` | always | the fight file's rules are the recording's; the Rust fight agrees with the committed left fight for the 10 s floor |
| `prefix` | ignored (cache) | every fight of the folder (left, right, every scenario) agrees for `PREFIX_FLOOR_S` = 10 s of fighting or to its end (ruling M3-R5; a knife-edge clash is reported with its margin) |
| `stats` | release | 2,000 Rust fights, the kit's seeds, against the kit's pooled figures, milestone 1's bars |
| `stats_wide` | release | 20,000 Rust fights (an independent sample), the same bars |
| `look_frames` | always | the left fight drawn with the real look and `DefaultLook`: every frame's bytes, hook calls (both looks spied), draw log and hull calls (count, vertices, points within 1e-12 px) are the kit's; the build's hook calls too; a hull fallback STOPS the task (ruling M3-R4) |
| `look_frames_cached` | ignored (cache) | the same for the right fight (the fighter on side 1, its shards) |
| `hull_points` | ignored (cache) | every hull call's input points of both fights within 1e-12 px |
| `stages` | ignored (local dumps) | see step 8 |
| `scenarios_draw` | ignored (cache) | every cached scenario fight (each opponent the spec names: EUPHONIUM's copy sweep, ruling M3-R3) drawn with both real looks, every 30th frame and the K.O.: no panic, no hull fallback |
| `title` | always | the VS card alone on the fixed noise image with the fighter on either side (16 frames each), its close-up, its panel pattern on the fixed base, its glass, flare scale and shatter parts |
| `sprites` | always | every sprite of the kit's sprite cache, rebuilt from look.json's facts in the export's order, drawn and hashed: the digest is the kit's (on a mismatch the cache names the sprites) |
| `setup_as` | always | a look whose kit class reads `self.cfg` takes its roster numbers from `setup_as` (below) |
| `looks_take_numbers_from_setup_as` | always | a source scan of every `fighters/*/look.rs`: `cfg[` appears only as `self.setup_as(p, k, &p.cfg[k]);` |
| `voice` | always | 24 notes from their noise states: length exact, peak and every 10 ms block RMS within 1e-6 (-120 dBFS), the noise stream where the kit's stood after each |
| `voice_notes` | ignored (cache) | every sample of the 24 notes within 1e-6 |
| `sound` | always | the left soundtrack (real part, base `Sound` for the opponent) and each role: the 16-bit master's SHA-256, the buses in order, the stems, every 100 ms block within 0.01 dB above -90 dBFS, the first-time notes, the noise stream's end |
| `sound_stems` | ignored (cache) | the right soundtrack likewise, and every stem window of all of them within -120 dBFS |
| `bed_calls` | always | `Sound::bed_calls()` is every `bed.<hook>(` of the Python part not under `hasattr` |
| `checks` | ignored (slow: 1 to 2 minutes; reads committed files only, run by name in every review) | every check of the left fight with both real looks and parts: the soundtrack is the kit's `audio.wav` byte for byte; verdicts and numbers within `checks_parity.rs`'s bars |

Every fight any gate runs goes through `bf_sim::run_traced`: a socket stub reached (a kind whose `PORTED` is true
with an empty socket) fails with the stub's name. No gate passes by doing nothing: a missing folder or file names
the export to run, a missing or stale cache file names `cache <folder>`, a count below what the export wrote fails.

## Looks: the roster numbers come from `setup_as`

EUPHONIUM's copies draw another fighter's look for the euphonium's side with the copied fighter's numbers
(`euphonium.py` 192-201, `Look::setup_as`). So a look whose kit class reads `self.cfg`:

- reads every roster number from the `cfg` that `setup_as(p, k, cfg)` is given, never `p.cfg[self.k]` or
  `p.cfg[k]`; its `setup` is exactly `self.setup_as(p, k, &p.cfg[k]);` (the scan allows `cfg[` in that form only);
- implements `Look::cfg_summary`, returning the fields it took from `cfg` (FLUTE: `roll`, `rise`, `low`; TROMBONE:
  `low_radius`), so the `setup_as` gate can see that set up with the other fighter's numbers the look differs.

A look whose kit class reads no `self.cfg` keeps both defaults (`look.json`'s `reads_cfg` is false).

## The recipe of a theme (Template T)

`tools/studio_export/theme_<name>.py` with `SPEC = dict(name=..., voicings=(the Bed's voicing methods),
configs=(its reference kit configs, Table S4), options=(one variant per option value: dict(name, text, ff, why)))`
and `EXPORTS = {'theme_<name>': lambda: m3refs.theme_refs(SPEC)}` (`theme_big_top.py`). It writes `theme.json` (each
config's recording and its bed alone: the 16-bit master), `voicings.json` (12 roots x 13 qualities), `bed.json`
(the bed alone's buses, RMS and notes; a second tempo, a second key and each option, with their recordings cached;
the optional hooks the class defines, each called at three times with three gains, vectors cached) and the cache.
Then `cache themes/<name>`, `budget`, and `tests/theme_<name>.rs` (above). Gates: `reference_folder`, `exceptions`,
`bed_alone`, `hooks` (`Bed::hooks()` equals the class's methods), `card` (the card lists the configs as presets),
`voicings`; ignored: `bed_variants`, `bed_stems`, `hook_vectors`. The bed alone needs the config's two fighters'
voices and parts in the app (the music context reads them); a theme whose configs' fighters are not ported yet waits
for them.

## The recipe of a pair gate (Template P)

`studio/fights/<pair>.toml` (the kit config, field for field), `tools/studio_export/episode_<pair>.py` with
`SPEC = dict(config='ballfight_<pair>', links=(the pair's own events of Table S5), weapon_hit={fighter: how})` and
`EXPORTS = {'episode_<pair>': lambda: m3refs.episode_refs(SPEC)}` (`episode_flute_vs_trombone.py`). The kit's
recording of the config is pointed at when the theme folder (or milestone 1) holds it. It writes `episode.json`,
`frames.json` (the golden frames, both real looks, and 16 card frames), `sound.json` (the kit's `audio.wav` as a
16-bit SHA-256, buses, RMS, notes), `checks.json`, `stats.json` (4,000 seeds at the config's handicap; rows cached),
`pairs.json`, `exceptions.toml`. Gates: `fight_file_is_the_config`, `recording`, `golden_frames`, `title_card`,
`soundtrack`, `pair_links`; ignored: `checks` (slow), `stems`, `prefix`, `native_render` (a search and a whole render;
needs ffmpeg); release: `statistics`.
