# Porting an older fighter or theme: the reference kit module and Gate O

Milestone 4 ports the ten older fighters (ACCORDION, BAGPIPES, BARI SAX, DRUMS, HARP, SYNTH, TURNTABLES, UKULELE,
VIOLA, VIOLIN) and the twelve themes of the older episodes. Everything in `porting-a-fighter.md` applies unchanged:
the same harness (`fighter_suite!`, `theme_suite!`, `episode_suite!`), the same files a port owns, the same gates and
the same rules. This manual adds what is different: **Step 0**, before any Rust.

The kit (`projects/ballfight/kit/`) has no look, sound or mood for an older fighter or bed: they exist only inside the
episode folders, drawn by older copies of the renderer. So the port first writes the **reference kit module** the
kit's own README describes ("copy its draw functions from its latest episode's `styles.py` and its layers from that
episode's `render.py` into the Look hooks, its part of `audio.py` into Sound, the bed into a mood"), outside
`projects/`, and proves it against the episode with **Gate O** (the origin gate). Milestone 3's harness then proves the
Rust port against that module, bit for bit, as if the module had always been in the kit. Where the module and the plan
disagree, the module wins; where the module and the episode disagree on a gated layer or bus, the episode wins.

The Python side is `tools/studio_export/older/` (`__init__.py`: the tables and `install()`; `origin.py`: Gate O). Run
everything from the clone root as `porting-a-fighter.md` says (`PYTHONHASHSEED=0`, `PYTHONPATH` = the clone).

## Rules that never bend

- **Nothing under `projects/`, `reelkit/` or `series/` is edited or written, ever** (ruling M4-R1; not even the kit
  README). An episode's scripts run only from a scratch copy under `studio/reference/.cache/m4/origin/<episode>/`
  (`origin.scratch`): its delivered sources (the top-level `*.py` files but the `*_backup*.py` snapshots, and
  `seed.txt`), never `v1/`, `v2/`, ... or `check/`. Every runner refuses another working folder. The shared modules an
  episode imports (the engine, roster, ring and title card in `projects/ballfight/`, the Pixel Pitch bridge) are read
  where they are. Every Gate O run ends by checking `git status --short projects reelkit series` is empty, and stops
  if it is not.
- **A module is installed into the kit's names at run time only.** `older.install()` (called by `m3refs` and
  `m3parts` in every process before the kit is imported) makes every `older/fighters/<slug>.py` and
  `older/moods/<name>.py` whose source sets `PORTED = True` importable as `ballfight.kit.fighters.<slug>` /
  `ballfight.kit.moods.<name>`, so `kit.fighters.module(name)` and the music's `moods.<MOOD>` find it. Until then the
  kit draws the bare ball and plays nothing for that fighter, as before. An older module never shadows a kit module of
  the same name (install refuses).
- **The two retired fighters** (`bf_core::fightfile::RETIRED`) are never ported.
  Their episodes (#27, #29) are run only as data inside a Python Gate O of a theme (plan m4, Q2).
- **A gate is never loosened.** A layer or bus that differs is fixed in the module, never in a comparison. The sound
  bar is -120 dBFS (peak difference); an exception up to -80 dBFS exists only by a controller's ruling, written into
  the spec with its reason (and so into `origin.json`).

## Step 0 for a fighter: write the module

1. **Read the episode** named in plan m4 Table O1 (the fighter's latest episode folder): its `styles.py` (the draw
   functions), its `render.py` (the layers: everything the renderer does for this fighter, including in shared code:
   its hit kinds in the sparks, flashes, shakes, glows and fringes, its effects in `_fx_build`, its shards, its state in
   `fighter()`), its `audio.py` (its part).
2. **Write `tools/studio_export/older/fighters/<slug>.py`**, shaped like a kit fighter module (start from the closest
   kit module, as the kit's ports did; `kit/fighters/organ.py` is #35's ORGAN moved this way):

   ```python
   """BAGPIPES (tartan), as built in #30 v3 ..."""
   PORTED = True                        # only once Gate O passes; install() reads it without running the file
   from . import base                   # the kit's base Look / Sound (the module is loaded under the kit's name)

   class Look(base.Look):
       ...                              # the episode's layers, in the hooks of kit/fighters/base.py

   class Sound(base.Sound):
       ...                              # the episode's part of audio.py
   ```

   Draw functions are imported from the episode's `styles.py` through its scratch copy (the episodes' own rule:
   "styles live there so crossovers can import them"), never retyped; the `render.py` layer code and the `audio.py`
   part are moved into the hooks with their names. A layer no hook can express uses one of Task 4's picture opt-ins
   (`older/picture_patches.py`, and a `Look` method in Rust); a layer neither covers stops the port with NEEDS_CONTEXT
   naming the layer. The model and sprites come from the fighter's `pp_bridge` module as milestone 3's do.
3. **Give its export an `origin` key** (`tools/studio_export/fighter_<slug>.py`; `older.origin.origin_spec` checks it
   and fills the defaults; `m3refs.fighter_refs` refuses to run until its `origin.json` is Gate O's for that spec):

   ```python
   SPEC = dict(name='BAGPIPES', opponent='FLUTE', origin=dict(
       episode='ballfight_bagpipes_vs_accordion',
       config=dict(FX_SEED=30, MOOD='big_top'),     # what the episode's timeline.py does not say (see below)
       frames=24,                                   # at least 24 (or a list of frame numbers)
       other=[...],                                 # the other fighter drawn as the kit's bare ball
       layers=[dict(name='bag', patches=[...]), ...],   # this fighter's layers: all off = the kit's base Look
       harness=[...],                               # the kit matched to the episode's own series rules
       buses=['bagpipes'],                          # its bus(es): the kit's names = the episode's
   ))
   ```

   - **`config`**: Gate O runs a kit config of the episode made from its `timeline.py` (FIGHTERS, its seed, DMG_SCALE,
     DMG_SIDE, RULES, BPM, SERIES, KEY, SCALE, CHORDS, SOLO, FIGHT_BAR); `config` adds what only the renderer or the
     audio says (`SHARD_SEED`, `FX_SEED`: the seeds of its `default_rng(...)` shards and `fx.Particles(seed=...)`) and
     a stand-in `MOOD` while the episode's own bed is not a kit mood (the fighter's own buses do not depend on it). A
     string instead names a kit config to run as it is (`'examples/trombone_vs_horn'`).
   - **`other` and `layers`** are exact text patches of the scratch copy: `(file, old, new)`, each `old` matching
     once (else Gate O stops naming the patch). `other` is always applied: the other fighter drawn and lit as the kit's
     `base.Look` draws a bare ball (its layers gone; its hits' sparks, glows, shakes, flashes, block bursts, K.O. spray
     as `base.Look` gives them). `layers`, applied on top, turn this fighter's layers off so the episode draws it as
     `base.Look` would. Name each layer group (they are listed in `origin.json`). `origin_selfproof.py` holds #35's
     complete set for ORGAN and HARP: a worked example of every kind of patch.
   - **`harness`** matches the kit run to the episode's own series rules inside the window, never a fighter's layer:
     `dict(name=, camera_sy=<px>)` (#35's ring sat 8 px higher), `dict(name=, beat_pulse_pre=<s>)` (its beat ramp),
     or `dict(name=, call='<module>:<function>')` for anything else (`fn(stage, module)`, the stage `'timeline'`,
     `'picture'` or `'music'`). The card and the tail are the kit's series standard and are outside the window by
     construction.

## Step 0: run Gate O

```powershell
& $py -m tools.studio_export origin fighters/<slug>      # writes studio/reference/fighters/<slug>/origin.json
& $py -m tools.studio_export origin themes/<name>        # a theme (below)
```

(Template F' writes this step as `fighter_<slug> --origin`; an export takes no flags, so it is the `origin` command.)

What it does, every part in a fresh process (three at a time):

1. **The fight.** The replay guard's line of the episode's own `sim.py` for its seed (`tools/ballfight_replay_hash.py`'s
   child command, in the scratch copy) must be the kit's `story.replay_hash` line of the config. If it is not, the
   config is wrong (a value the timeline does not hold), or the episode predates today's roster: that is a finding to
   report, never to patch.
2. **The frames.** At least 24 inside the window, FIGHT! to the end of the shatter (the earlier of the two timelines'
   reassembly): each ability 0.2 s and 0.6 s after its first use, its first hit, its HP below 25, the K.O., its shards
   (when it loses) or its solo (when it wins), then an even spread (`origin.choose_frames`).
3. **Four pictures of each frame**: the episode with `other` (on) and with `other` + `layers` (off); the kit with the
   module and with `base.Look` (the other side `base.Look` in both). The two contributions, on minus off per pixel in
   `int16`, must be identical. The module's picture state (each key its `state()` sets or changes on its own ball,
   against `base.Look`'s) must equal the episode's `fighter()` value at that frame.
4. **The buses**: each of `buses` from the kit's score (the module's Sound, the other side `base.Sound`) against the
   episode's `audio.py`, both taken as placed, before the master (the master is the series' own in the kit), sample
   for sample in float64 over the window, against the -120 dBFS bar.

### Reading a failure

A failing frame stops the gate with one line, for example:

```
Gate O: ORGAN frame 144 (t = 4.800 s), layers thump, shake, ...: the contributions differ at pixel (x=180, y=301):
the episode [0, -24, -25], the kit [-40, -24, -25] (on: episode [255, 231, 230], kit [215, 231, 230]; off: episode
[255, 255, 255], kit [255, 255, 255]); 1 pixels differ; the two off frames differ in 0 pixels
```

- **The two off frames differ in N pixels** tells you where to look first. N > 0: the bare scenes do not match yet: a
  patch of `other` or `layers` is missing (the episode still draws something of a fighter that `base.Look` does not),
  or a `harness` entry is (a camera, a beat ramp). N = 0: the scenes match and the module itself draws differently.
- The four pictures of the run are in the cache: `studio/reference/.cache/m4/origin/out/<episode>/<slug>/{e_on,e_off,
  k_on,k_off}.npz` (members `f<frame>`, uint8 RGB). Look at the difference images first; most failures are a layer
  drawn in the wrong order, a missed patch site (search `render.py` for every place the fighter's hit kinds, ability
  names, `k in X_K` lists or colours appear), or an effect emitted in another order (`fx.Particles` draws its random
  numbers as effects are added, so the order of `PA.emit` calls is part of the picture).
- **Stage dumps**: for the kit side, `stages.tap(P)` / `stages.capture(P, f)` (`tools/studio_export/stages.py`) give the
  buffers after each stage of `kit.picture.render`; the episode's `render.py` is the same pipeline from the same
  `reelkit` calls, so the same taps can be pointed at the episode's render module inside a scratch-copy process
  (`origin._episode_env(folder)`, then `import render`) to find the first stage where the two contributions part.
- A bus failure names the bus, the first sample over the bar (with its time and channel) and the two values, and the
  peak difference over the window. The stems of the run are in the same cache folder (`e_bus.npz`, `k_bus.npz`).

### What goes into `origin.json`

`studio/reference/fighters/<slug>/origin.json` (committed; at most 20 KB, a `budget` row of its own): the episode and
the commit its folder is at, the seed, the kit config (its text and SHA-256), the fight line (equal on both sides), the
window, every frame compared (its time, the SHA-256 of the contribution, which is equal on both sides, and of the four
pictures, and whether the whole frames were equal too), every patch of `other` and `layers` and every harness entry
(the SHA-256 of each text), the state keys compared, each bus's peak difference ('bit-identical' or its dBFS, over the
window and over the whole buffer), every exception with its ruling and reason, and the SHA-256 of the origin spec
(`m3refs.fighter_refs` checks it is the spec's). The pictures and stems stay in the cache; only hashes are committed.

**Checkpoint:** the controller reviews the module and its `origin.json` before Step 1 (plan m4 section 7, "Gate O
checkpoint"). Then follow `porting-a-fighter.md` from step 1, against the module, with Template F''s additions: the
opponent FLUTE, the scenarios of Table O4's links that name the fighter, and the copy sweep (EUPHONIUM against it,
three seeds each side: fight parity and every copied kind drawn through `setup_as` without a panic).

## Step 0 for a theme: the reference mood and its Gate O

Write `tools/studio_export/older/moods/<name>.py`: a kit `Bed(M)` class with every method of the episode's bed in the
episode's order, every random draw in the episode's order, `hooks` exactly the optional methods it defines, and its
sections (card, count-in, fight bars, tape stop, K.O., stinger, victory, the build into frame 0) placed on the kit's
beat sheet; `PORTED = True` once its gate passes. Its export's SPEC (`theme_<name>.py`) gains:

```python
origin=dict(
    episode='ballfight_trombone_vs_horn',
    config='examples/trombone_vs_horn',        # or dict(...) over the episode's timeline, as for a fighter
    silence=[('audio.py', old, new), ...],     # the fighters' placements on the bed's buses, removed
    buses=['strings', 'taiko', 'braam', 'tremolo'],
)
```

Gate O (`mood_origin`) runs the kit's score with both fighters as `base.Sound` (silent parts) and the episode's
`audio.py` with `silence` applied, and compares the named buses over the whole buffer (the bed has no shatter window).
Where the episode places a fighter's sounds on a bed bus (the HORN's FINAL FLICK on the taiko and the braam in #34),
the `silence` patch removes exactly those placements; it is named in `origin.json`. Never compare a bus that mixes a
fighter with the bed without that patch.

## The self-proof

`python -m tools.studio_export origin_selfproof` runs Gate O on two modules the kit already holds, against the episodes
they were ported from, and on one made unfaithful at run time:

| Gate | Result (on the reference PC) |
| --- | --- |
| ORGAN (`kit/fighters/organ.py`) against #35 v4, seed 237, 24 frames | PASS: every contribution equal (and every whole frame); buses `organ` and `solo` bit-identical over the window |
| `brass_trailer` against #34 v5's audio (`examples/trombone_vs_horn`) | PASS: `strings`, `taiko`, `braam`, `tremolo` bit-identical over the whole buffer |
| ORGAN with one pixel of one frame changed after its `after_grade` | FAIL, naming the frame and the pixel |
| ORGAN with one sample of its bus changed after its `play()` | FAIL, naming the bus and the sample |

Its hashes and failure lines are in `studio/reference/older/selfproof/` (`organ.json`, `brass_trailer.json`,
`summary.json`). `python -m tools.studio_export.unit_checks.origin_checks` checks the scratch copy, the refusals, an
episode run leaving the kit folders clean, `install()`, and the two mutations (about 1.5 minutes).
