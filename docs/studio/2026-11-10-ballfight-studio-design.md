# BALLFIGHT Studio: design (Nov 10, 2026)

Approved by the user section by section in the planning thread on Nov 10. This replaces the technical shape in
`docs/studio/PLAN.md` (Python backend + browser page). PLAN.md's goal still stands; `docs/studio/HANDOFF.md` is still
the reading list for the existing Python kit, which this design treats as the reference to port from.

Interface mock-ups (five screens): https://claude.ai/artifact/DWR46mnrEU6mTxyt6iXBrW

## 1. Goal and what success looks like

A Windows desktop app, written in Rust, that lets the user pick two instruments, find a good honest fight, and render
the finished 61 s BALLFIGHT video, with no Claude tokens spent on fights between existing fighters. Claude sessions
are only needed for new fighters, new themes and new features.

Success for version one:

- The user makes a complete video in the app (match-up, balance, seed, music, render) and it passes every series
  check with no manual step outside the app.
- A video made in the Studio looks the same as the series today: every ported fighter's frames match the Python kit's
  frames for the same recorded fight.
- A fight file reopened in the same app version rebuilds the identical frames and soundtrack on any PC.
- All 30 fighters and all 26 themes are available. GUITAR and SAX never appear.

## 2. Decisions made with the user

| Topic | Decision |
|---|---|
| How much is rebuilt | Full native Rust engine: fight, picture and sound. The Python kit stays in the repo as the reference. |
| Look | Same as today: 360x640 internal, 3x hard-pixel upscale to 1080x1920, same lighting, dither and effects. |
| Picture approach | Faithful port drawn on the CPU across all cores. wgpu runs the window, interface and live display. |
| Seeds | The Studio has its own seed numbers. Old Python seeds are not promised to give the same fight. |
| Seed picking | Ranked shortlist plus filters, and a box to watch any seed number. |
| New fighters and themes | Claude builds them in a session; they arrive in an app update. No in-app workshop, no script mods. |
| Rendering | A render queue. The user picks every seed. Software x264 only. |
| Music | "Reacts and varies": layered themes that follow the fight, fighters that compose fresh lines per take. |
| Scope now | Core loop, music controls, the ten older fighters and their themes, cover image = the VS title card. |
| Scope later | Boss fight (band director), tournament brackets, 3 to 4 fighter brawls, arenas. |
| Not planned | Match-up table, hands-off batch, viewer request tracker. |
| Code location | `studio/` inside `pixelpitch161/reels-toolkit`. The user's clone lives at `F:\!Ai Stuff\InstrumentBattleMachine`. |
| Interface look | The user's Ball Studio design system, plus three fight-specific touches (section 11). |

The user's PC: Radeon RX 9070 XT (16 GB), Ryzen 7 9800X3D (8 cores), 64 GB RAM, Windows 11, 670 GB free, Rust
installed.

## 3. Series rules the app must enforce

From `docs/ballfight/RULES.md` and `docs/PLAYBOOK.md`. None of these is negotiable.

- **Honest fights.** The app runs the real simulation. The user picks a seed and never edits a fight. Balance comes
  only from the per-fighter damage handicap; reach, timing and abilities never change per fight.
- **Determinism.** Same fight file and same app version give the same frames and soundtrack, bit for bit, on any PC.
- **Video.** 61.0 s, 1080x1920, 30 fps, h264 + aac. The loop point is the file boundary. The soundtrack is 2,860 whole
  AAC frames (61.013 s) so it wraps without a click.
- **Safe zone.** Captions, HUD and key action inside internal x 20 to 340, y 96 to 500, minus x >= 300 below y 290.
- **Loudness.** Consistent from frame 0: opening within 1.5 dB of the ending, every 2 s window within 3 dB of it (one
  deliberate dip allowed), spread <= 5 dB. Energy comes from the arrangement, never from level.
- **No full-screen flashes**, no episode number on screen, one unique colour per instrument (the table in RULES.md).
- **Retired fighters.** GUITAR and SAX are never offered and are not ported.
- **Beat sheet.** Bar 0 VS title card, bar 1 count-in, FIGHT! on bar 2, K.O. at 49 to 53 s, winner's solo, reassembly
  into frame 0.

## 4. How the app works

Five steps, shown left to right, any of which can be revisited. A sixth view, the Library, lists past fights.

1. **Match-up.** Two fighters from the roster, who stands left and right, a theme, and a tempo. Tempo snaps with the
   `loop_bpm` rule so a whole number of bars fits in 61 s. Pair rules that the series already uses for specific
   pairs (for example the lance swatting cobras, the tuba swallowing rollers) are switched on automatically from a
   table in the content and shown read-only.
2. **Balance.** Auto-balance sets the two damage handicaps so each side wins 40 to 60 percent and the typical K.O.
   lands near 50 s. The screen shows the win split, K.O. timing and how many fights fit the beat sheet. The two
   handicap sliders are the only controls.
3. **Seeds.** A background search fills a ranked shortlist. Filters narrow it. Clicking a fight plays it in the real
   picture with sound, with scrubbing.
4. **Music.** Theme, key, chord loop, who plays bass, and a music seed with a re-roll button.
5. **Render.** Caption text per platform, output folder, add to queue. The queue renders back to back.

**What feeds the fight.** Fighters, sides, tempo, handicap and pair rules change the fight, so changing any of them
invalidates the shortlist and restarts the search. Theme, key, chord loop, roles and music seed do not, with one
exception: a fighter whose notes are its weapon (PIANO's roll) makes key, chord loop and music seed fight inputs too.
Each fighter declares which music settings feed its fight, and the app derives the search key from that.

**The fight file.** A small TOML file that is the whole recipe: format version, app version, fighters as an ordered
list, arena name (`ring` in v1), handicap, pair rules, tempo, seed, theme, key, scale, chord loop, roles, music seed,
optional hand-written lines that override the composer, and the caption texts. Fighters are a list and the arena is
named so later features need no format change.

**Library.** Past exports found under the output folder, shown as a grid of title-card covers with the match-up and
date. Reopen loads the fight file into the five steps.

**Settings.** Output folder (default `Videos\BALLFIGHT\`), number of worker threads (default cores minus one).

## 5. Architecture

One Cargo workspace in `studio/`. Each crate has one job and a narrow interface, so fighters, themes and features can
be added without touching the others.

| Crate | Job | Depends on |
|---|---|---|
| `bf-core` | Shared types (fight file, recording, events), the bar and beat grid, deterministic maths, the numpy-compatible random generator | nothing |
| `bf-sim` | The fight engine: rules, balls, one module per weapon kind and ability kind, the roster numbers | core |
| `bf-story` | Story analysis of a recording, the beat-sheet fit and score, seed search, balance, shortlist filters | core, sim |
| `bf-picture` | The renderer: raster, camera, lighting, shadows, lens, post, effects, pixel font, HUD, captions, ring, title card, the `Look` trait | core |
| `bf-sound` | Synthesis and mixing: oscillators, filters, envelopes, reverb, buses, master, the `Voice`, `Sound` and `Theme` traits, the arranger and composer | core |
| `bf-content` | Every fighter and every theme, one folder each, plus the registry and the pair-rule table | sim, picture, sound |
| `bf-export` | The checks, the ffmpeg encode, the render job, output folders | story, picture, sound, content |
| `bf-cli` | `ballfight` command line: every step without a window. Used by tests, CI and Claude sessions | all |
| `bf-app` | The desktop app (eframe + egui on wgpu) | all |

**Data flow.** Fight file -> `bf-sim` produces a **recording** (per-frame state arrays plus the event list) ->
`bf-story` reads the recording for scoring and for the music's story signal -> `bf-picture` draws frame N from the
recording alone, and `bf-sound` builds the soundtrack from the recording and the music settings -> `bf-export` checks
and encodes.

**The recording is the seam.** It is a documented, versioned container that both Rust and Python can write and Rust
can read. A recording exported from the Python kit can be drawn and scored by the Studio, which is how the picture
and sound are tested without needing the two fight engines to agree, and how delivered episodes can be opened.

**The interface never blocks.** Search, balance, soundtrack builds and renders run on worker threads and report
progress through channels. The preview keeps one core free.

## 6. Fight engine

**Port.** `projects/ballfight/fight.py` (6,242 lines) and `fighters.py` are translated rule for rule: same ring, same
constants, 120 steps a second, same order of operations. The single Python class is split into one module per weapon
kind and ability kind, but the order in which kinds are stepped must stay the Python order. Roster numbers are
exported from `fighters.py` by a script, not retyped.

**Determinism rules for `bf-sim` and `bf-story`.**

- `f64` throughout, operations in the same order as Python, no fused multiply-add, no fast-math flags.
- Every transcendental function (sin, cos, atan2, exp, pow, hypot, log) goes through `bf-core`'s maths module, which
  is pure Rust. The platform's maths library is never called; a lint forbids the std methods in these crates.
- Randomness is a port of numpy's `default_rng`: `SeedSequence` seeding, PCG64, and the exact `random`, `uniform`,
  `normal` and `permutation` algorithms, verified against test vectors exported from numpy. This includes the
  list-seeded generators the engine creates mid-fight.
- No iteration over hash maps anywhere a result depends on order.

**Why not exact replay of Python seeds.** Fights are chaotic and the Python results depend on the maths library of
the machine that ran them, so bit-equality with "Python" is not well defined. The Studio's own results are fully
defined by the rules above.

**How the port is proven.**

1. *Prefix agreement.* For each reference fight, Rust and Python run the same seed. Positions, HP and events must
   agree to 1e-6 for an opening window. The window length is measured in milestone 1 (rounding noise is expected to
   stay invisible for 15 s or more); a logic error shows up as an early break at a specific event.
2. *Kind coverage.* Scenario fights with lowered level and low-HP thresholds make every weapon kind and ability kind
   fire inside the agreement window. Every kind must be covered by at least one scenario.
3. *Statistics.* Over at least 2,000 seeds per reference pairing, win share (within 3 points), median K.O. time
   (within 0.7 s), mean lead changes and beat-sheet fit rate must match Python's.
4. *Self-replay.* A committed set of Studio recordings is replayed on every change and must hash identically, on
   Linux and Windows. This replaces `tools/ballfight_replay_hash.py` for Studio fights.

**Two fighters inside, a list outside.** The v1 engine stays two-fighter internally, because it must match Python.
Everything around it (fight file, recording, story, picture, sound, interface) treats fighters as a list.

**Search.** Seeds are tried upward from 1 on all workers. A fight "fits" when it passes the series' hard windows
(K.O. 49 to 53 s, at least 2 lead changes, winner at 30 HP or less, at least 5 ability captions 4 s apart, no long
gap between hits) and is ranked by the soft story score, both ported from `kit/story.py`. The search runs until the
user stops it or 200 fits are found. Results are cached per search key.

**Balance.** Uses a seed range the search never touches. Auto-balance alternates two one-dimensional searches: the
ratio between the two handicaps for a 50 percent win share, then their common scale for a median K.O. near 50.5 s,
with 400 fights per evaluation and a final 3,000-fight confirmation.

**Filters.** Winner; K.O. time range; minimum lead changes; close finish (winner at 12 HP or less); comeback (the
winner trailed by 20 HP or more after 15 s, or dropped below 25 HP first); a chosen ability lands. A switch shows
fights outside the beat sheet, marked as such.

## 7. Picture

**Port.** `reelkit/` (raster, camera, lighting, shadows, lens, post, fx, font, ui, config), `projects/ballfight/`
`scene.py`, `title.py`, `kit/picture.py`, and each fighter's `Look`. The pipeline order is kept exactly: floor albedo,
per-pixel light with shadow masks, objects in painter's order, lens, glows, bloom, grade, vignette and scrim, HUD and
captions. Each fighter adds layers through the same hooks, in the same order, as `kit/fighters/base.py`.

**A frame is a pure function** of the recording, the fight file and the frame number. Effects are built once from the
events in time order, so frames can be drawn in any order and in parallel.

**Known traps the port must reproduce exactly.**

- Pillow's polygon fill rules. The Pillow version used to make the reference frames is pinned and recorded.
- Mixed `uint8`, `float32` and `float64` arithmetic and numpy's rounding and clipping at each cast.
- The caption puff uses Python's string hash under `PYTHONHASHSEED=0`; Rust reproduces that hash.
- The scipy image and spatial functions the renderer calls.

**Models and sprites.** The low-poly instrument models are exported from `series/instrument-history/` as mesh data
(faces, normals, colours). The sprite renderer is ported so future fighters need no Python; its output is tested
against sprites exported from Python.

**Proving the look.** For every fighter, at least ten frames from imported Python recordings (each ability, the K.O.
shatter, the VS card, the HUD states) are drawn in Rust and compared with Python's frames. The bar is identical RGB.
A frame that cannot be made identical goes in an exceptions file with the reason and must differ by at most 2 levels
on at most 0.5 percent of pixels; each exception is reviewed by eye.

**Live preview.** The same renderer draws ahead of the playhead on worker threads into a small buffer. wgpu shows
each frame as a texture with nearest filtering, at a whole-number scale when the window allows. Audio plays through
the system output and is the clock. Scrubbing draws the requested frame on demand. A Safe zone toggle tints the area
outside the safe box.

**Speed.** Targets, to be measured in milestone 1 and reported before more is built on them: live playback at 30 fps
with headroom, and a full 1,830-frame render in under 60 s on the user's PC.

## 8. Sound and music

**Sound port.** `reelkit/audio.py`, `kit/music.py`, the Pixel Pitch voices reached through `pp_bridge`, each
fighter's `Sound`, each theme's instruments and patterns, and the master chain. 48 kHz, `f64` processing. Filter
design and filtering reproduce scipy's; seeded noise uses the numpy generator port. Everything stays synthesised.

**Proving the sound.** Each voice, each fighter's part and each theme part is compared with Python's stem for an
imported recording. Target: sample differences below -120 dB full scale. Documented exceptions up to -80 dB (for
example convolution reverb). Each ported theme keeps a "classic" arrangement that reproduces the Python bed, used
for these tests and for imported episodes.

**The music system (new design, not a port).**

- *Story signal.* From the recording, on the bar and beat grid: who leads, the HP gap, each lead change, each
  captioned ability, each desperation move, the final exchange, the K.O.
- *Themes are layered.* A theme provides a home key and scale, one or more chord loops, the tempo range it suits, a
  set of layers (each a pattern generator over the ported instruments), and an arrangement table that maps the story
  signal to which layers play: a fill at each lead change, a breakdown at a desperation move, every layer in for the
  final exchange, a band hit under each captioned ability, the tape stop at the K.O., the victory section, the build
  into frame 0.
- *Fighters have a musical personality.* Register, rhythm vocabulary, favourite intervals, ornaments, and two to four
  signature motifs written in scale degrees, taken from the lines and solos of the fighter's existing episodes. A
  deterministic composer seeded by the music seed writes the fighter's line over the chord loop and its victory solo:
  chord tones on strong beats, motifs at phrase starts, inside the register, shaped by its role (bass, melody, or
  answering line).
- *Roles follow the lead.* The leader's part fills out and the trailing part thins and roughens as HP drops, as the
  series already does.
- *Unchanged series rules.* Every hit plays the attacker's instrument in key on the nearest sixteenth.
- *Loudness is checked on the Music step.* Each take is run through the levels check as soon as it is built, so a
  failing take is seen there, not in the queue.
- *Overrides.* A fight file may pin hand-written lines and solos for a fighter, which the composer then uses as is.

**Controls.** Theme, key, chord loop (when the theme has several), who plays bass, music seed, re-roll. Tempo is
shown but changed on Match-up.

## 9. Render queue and export

**A job** runs: build soundtrack -> pre-render checks (safe zone, focus, levels, each fighter's band above the bed,
perceived loudness, flashes on the busiest stretch, loop point) -> draw all frames -> encode -> verify the mp4
(size, length, streams, loop and flashes again) -> move the folder into place. Checks and thresholds are ported from
`reelkit` and `kit/checks.py` and tested against the Python checks on the same inputs.

**Queue behaviour.** One job at a time. Reorder, pause, cancel, remove. A failed check stops that job with the check
named and the measured numbers shown, and the queue continues. The queue survives an app restart.

**Encode.** Frames are piped to a bundled ffmpeg with the kit's loop-clean settings: libx264, preset slow, CRF 18,
yuv420p, high profile, tune animation, `mbtree=0`, keyframes forced on frame 0 and the last frame, AAC 256k at
48 kHz with the soundtrack untrimmed. No hardware encoder. ffmpeg is fetched by a setup script at a pinned version
with a checksum and is not committed to the repo. Raw frames are not kept.

**Output.** `<output folder>\<a>_vs_<b>\` holding `<a>_vs_<b>.mp4`, `cover.png` (the VS title card at 1080x1920, on
the beat-4 hold when both names and VS are on screen), `caption.txt` (caption and tags for Instagram, TikTok and
Shorts) and the fight file. If the folder exists, the new one is named `<a>_vs_<b>_seed<N>`. Everything is written
to a temporary folder and moved when complete.

**Captions.** Drafted from each fighter's emoji and tags in its content folder plus the series line "Comment the
next match-up", then editable before queueing.

## 10. Fighters and themes

**One folder per fighter** in `bf-content`: its roster numbers and colour (exported data), its `Look`, its `Sound`
and voice, its musical personality, its model, its emoji and tags, its VS-card pattern, which music settings feed
its fight, and its reference frames and stems. **One folder per theme**: its instruments and patterns, layers,
arrangement table, classic arrangement, key, scale, chord loops and tempo range. A registry lists them; the app reads
the roster and theme list from the registry.

**Version one content.**

- The 20 kit fighters: ACOUSTIC, ALTO SAX, BASS CLARINET, BASS DRUM, BASSOON, CELLO, CLARINET, CONTRABASS, ELECTRIC,
  EUPHONIUM, FLUTE, HORN, OBOE, ORGAN, PIANO, SOUSAPHONE, TENOR SAX, TROMBONE, TRUMPET, TUBA.
- The ten older fighters, ported from their latest episode folders: ACCORDION, BAGPIPES, BARI SAX, DRUMS, HARP,
  SYNTH, TURNTABLES, UKULELE, VIOLA, VIOLIN.
- The 14 kit themes and the 12 older episodes' themes (drift phonk, Balkan fire, noir jazz, sea shanty, dark
  breakbeat, villain march, spaghetti western, gothic horror, brass house, ska-punk, industrial, orchestral drill).

**Adding a fighter after launch.** A Claude session adds any new weapon or ability kind as a new module in `bf-sim`
without changing existing ones, creates the fighter's folder, and runs: the self-replay test (no existing fight
changed), balance against several opponents, a preview clip, and the safe-zone and flash checks. Once the port is
complete, Rust is the source of truth and the Python kit is a frozen archive. `HANDOFF.md` and the kit README are
updated to say so.

## 11. Interface

**System.** The user's Ball Studio design system (https://claude.ai/artifact/SPfnA1YGCJr8SHFstvjxji): neutral greys
(`ground` #1e1e1e, `panel` #282828, `stage` #2b2b2b, `raised` #333333, `line` #444444, `muted` #a0a0a0, `text`
#e2e2e2), IBM Plex Sans and Mono built into the exe, 26 px controls, 2 px corners, panels that touch with 1 px
dividers, and an inverted fill as the only "on" and primary signal. Its egui notes apply as written.

**Colour belongs to fighters.** Fighter colours take the role of Ball Studio's track colours. A colour always means
that fighter and never a state, so errors are stated in words. Large fills use the fighter colour mixed 22 percent
over `panel` with a 1 px edge in the full colour.

**Three fight touches.**

1. The two fighter names in the bout bar are drawn with the series' own 5x7 pixel font at whole-number scales.
2. The preview shows the VS title card on Match-up and Balance, since it is also the cover image.
3. The **lead ribbon**: time runs left to right, the left fighter's colour rises above a centre line while it leads,
   the right fighter's drops below, and a white bar marks the K.O. Shortlist rows are small ribbons. The same ribbon
   is the scrub bar under the preview, with a lane above and below for each fighter's first use of each ability.

**Layout (1440x900 reference, minimum window 1280x800).** Toolbar 40 px with the five numbered steps, Library, Queue
and Update. Bout bar 48 px with the two names and the fight's settings. Body: the step's workspace on the left, the
9:16 preview on a 410 px `stage` column on the right. Story timeline 112 px with transport, timecode, Loop and Safe
zone. Status bar 24 px.

**Screens.** As drawn in the mock-ups: Match-up (corner cards, roster grouped by section, theme and tempo), Balance
(win split with the 40 to 60 marks, handicap sliders, K.O. timing), Seeds (filters, shortlist, selection), Music
(controls, personalities, arrangement lanes, what the music listens for), Render (caption, output, queue, the job's
check list). Library is a grid of covers.

**Wording.** Sentence case, plain verbs, buttons that say what happens ("Add to queue", "Use seed 20418"). Empty and
failed states say what to do next.

**Keys.** Space plays and pauses, left and right step a frame, 1 to 5 switch steps.

## 12. Reference tools (Python side)

`tools/studio_export/`, with pinned Python dependencies, run on Linux. They only read the kit; nothing in
`projects/` changes and the existing replay guard stays green.

- Roster numbers to JSON. Model meshes and sprites. numpy random test vectors.
- Recordings of reference fights, plus scenario fights for kind coverage and bulk statistics per pairing.
- Reference frames per fighter and the VS card. Stems per voice, fighter part and theme part.
- A manifest recording the commit, Python and library versions for every reference file.

Reference files are committed under `studio/reference/` while the set stays under about 150 MB; if it grows past
that, it moves to Git LFS.

## 13. Testing and CI

- Unit tests per crate, written before the code they test.
- Parity suites: maths and random vectors, prefix agreement, kind coverage, statistics, golden frames, stems, checks.
- Self-replay hashes for Studio fights and renders, on Linux and Windows.
- Interface snapshot tests of each screen against the mock-ups, drawn with a software graphics adapter.
- GitHub Actions: Linux runs everything. Windows builds the app and CLI, runs the tests, and renders a 3 s clip end
  to end.

## 14. Build order

Each milestone ends with something that runs and is checked. Each gets its own implementation plan written when it
starts; the first plan covers milestones 0 and 1.

0. **Foundations.** Workspace, core types, deterministic maths, numpy random port, recording container, the export
   tools, and the reference set for Flute vs Trombone.
1. **One match-up end to end from the command line.** FLUTE and TROMBONE in the fight engine, story, search and
   balance, picture, classic sound, checks, encode. The imported episode recording matches Python's frames and
   stems; a native Studio fight renders to a passing mp4. Measure and report fights per second, frame time, render
   time, soundtrack build time and the prefix-agreement window.
2. **The app.** Five steps, preview, shortlist, queue, Library, settings, on the one match-up.
3. **The kit roster.** The other 18 kit fighters and 13 kit themes. Fighters are independent of each other, so this
   is parallel work, each with its own tests.
4. **The older ten** fighters and 12 themes, from their episode folders.
5. **The music system.** Story signal, layered arrangements for all 26 themes, personalities for all 30 fighters, the
   composer, the Music screen.
6. **Packaging.** ffmpeg setup, the Update button (`git pull` then a release build), Windows CI, a short user guide,
   and updated handoff docs.

## 15. Room for later features

- **Brawls and the boss fight** need the fight engine generalised beyond two fighters and new content; formats,
  story, picture, sound and interface already take a list of fighters.
- **Arenas** plug in as a named scene in `bf-picture` plus rule constants in `bf-sim`; the fight file already names
  the arena.
- **Tournament brackets** are a layer above the queue: a bracket is a set of fight files with results feeding the
  next round.

## 16. Risks

- **Size of the port.** About 31,000 lines of Python, and the ten older fighters live in per-episode copies of the
  scripts, which is slower to port than kit fighters. Milestone order puts a full working app before the long tail.
- **Exact frame match.** Pillow's fill rules, numpy casts and the Python string hash are the likely sources of
  mismatches. Milestone 1 proves the approach on two fighters before the rest are ported.
- **Unmeasured speed.** All speed figures here are estimates until milestone 1 measures them.
- **Generated music quality** is a matter of taste. Milestone 5 delivers listening samples for the user to judge
  before the composer is called done.
- **No graphics card in the build environment.** The picture is CPU-drawn and fully testable there; the window is
  tested with a software adapter and on Windows CI, and finally on the user's PC.
- **Windows CI minutes** on a private repo are limited; the Windows job stays small.
