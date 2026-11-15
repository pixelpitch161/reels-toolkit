# BALLFIGHT Studio, milestone 5: the music system (lean plan)

**Goal.** Spec §8 "The music system" and §14 item 5: a story signal, layered arrangements for all 26 themes, a
musical personality for all 30 fighters, a deterministic composer for each fighter's line and victory solo, and a
Music screen that picks between them. The owner asked for it in full (Nov 15) and for quick work with few checks
(Nov 14): **the result is judged by ear**, so each task has unit tests for the rules and, where it makes sound, one
rendered listening sample. No pixel, bit-exact or full-roster gates.

**Scope rules.** Work in `studio/` only. Nothing under `projects/`, `reelkit/` or `series/` is edited. The classic
beds and every existing test stay exactly as they are.

## The design decision: two arrangements, Composed is the default for new fights

- **Classic** stays what it is today: each theme's ported bed, the fighters' kit lines and solos. It is kit-exact
  for kit configs, it is what imported episodes always use, and the milestone 1 to 4 tests keep pinning it.
- **Composed** is new: the theme's layered arrangement driven by the story signal, and each fighter's line and
  victory solo written by the composer from its personality and the music seed.
- The fight file already has `music.arrangement` (only `classic` is accepted today). It gains `composed`. Imported
  episodes and starters built from a kit config stay `classic`.
- **Recommendation: new pairings start on Composed**, with Classic one click away on the Music screen. The flip of
  the default is the last line of Task 9, made only after the owner has listened to the samples. Until then new
  fights stay on Classic.

**Where the composer plugs in.** The fighters already play whatever `music.lines` and `music.solos` the fight file
holds (`MusicCtx::line_at`, `MusicCtx::solo` in `bf-sound/src/music/timeline.rs`), and new fights leave them empty.
So the composer's output is simply those two fields, filled at build time in `bf-content/src/soundtrack.rs`; the 30
fighter `Sound`s need no change. The layered arrangement is a second `Bed` returned by `Theme::bed("composed", ..)`
(`bf-content/src/themes/mod.rs`), which hands the sections it does not change (title, count-in, K.O. tape stop,
stinger, hooks such as `boom` and `hit`) straight to the theme's classic bed.

## Tasks

| # | Task | Size | Wave |
| --- | --- | --- | --- |
| 1 | Story signal | S | 1 |
| 2 | Personality type and the 30 personalities | M | 1 |
| 3 | Layer engine | M | 1 |
| 4 | Arrangement tables, themes A to M (13) | M | 2 |
| 5 | Arrangement tables, themes N to V (13) | M | 2 |
| 6 | The composer | M | 2 |
| 7 | Composed mode wired into the build | M | 3 |
| 8 | The Music screen | M | 3 |
| 9 | Listening samples for the owner | S | 4 |

Waves run up to 3 workers at once. Wave 3's two tasks can run side by side: Task 8 only needs the new
`arrangement` value, and it can use a stand-in until Task 7 lands.

### Task 1: Story signal

- **Files:** `studio/crates/bf-sound/src/music/signal.rs` (new), `music/mod.rs` (one `pub mod` line).
- **Produces:** `StorySignal::from(ctx)`: per bar of the song, who leads, the HP gap, and the moments on the
  sixteenth grid: each lead change, each captioned ability, each desperation move, the start of the final exchange
  (the last 2 bars before the K.O.), the K.O., the victory bars and the bars that build back into frame 0. Read from
  the run's events and `hp_at`, which `MusicCtx` already has.
- **Check:** unit tests on the FLUTE vs TROMBONE episode recording (in `bf-testdata`): the lead changes and K.O.
  bar match the fight, every moment lands on a sixteenth, and the result is the same on a second run.

### Task 2: Personality type and the 30 personalities

- **Files:** `studio/crates/bf-content/src/personality.rs` (new: the type and a lookup by fighter name), one
  `personality.rs` per folder in `bf-content/src/fighters/`.
- **Produces:** for each of the 30 fighters: register (lowest and highest MIDI note), rhythm vocabulary (a few
  one-bar rhythm cells in sixteenths), favourite intervals, ornaments (grace note, slide, trill, stab, or none), the
  roles it can take (bass, melody, answer) and **2 to 4 signature motifs in scale degrees**, each taken by hand from
  the lines and solos of that fighter's existing episodes (the kit configs and old episode folders are read, never
  edited). Each motif names the episode it came from in a comment.
- **Check:** one test over all 30: 2 to 4 motifs each, every motif fits in the register from the fighter's home
  key, every fighter has at least one role. No listening for this task alone.
- Can be split by a worker into three batches of ten if wave 1 has room.

### Task 3: Layer engine

- **Files:** `studio/crates/bf-sound/src/music/layers.rs` (new).
- **Produces:** the shared pieces every theme's arrangement is made of, so a theme is a data table, not new code:
  `Layer` (a name, a pattern generator over the theme's own instruments: drum groove, bass, chord comp, pad, arp,
  percussion, each with a few density levels) and `ArrangementTable` (for each part of the song and each story
  moment, which layers play and at what density): a fill at each lead change, a breakdown at a desperation move,
  every layer in for the final exchange, a band hit under each captioned ability, the tape stop at the K.O., the
  victory section, the build into frame 0. A `LayeredBed` that implements `Bed` from a table plus the classic bed it
  hands the other sections to.
- **Check:** unit tests with a toy two-layer table: the right layers sound in the right bars, a lead change adds a
  fill in that bar, the same seed gives the same samples.

### Task 4 and Task 5: Arrangement tables for the 26 themes

- **Files:** `studio/crates/bf-content/src/themes/<theme>/layers.rs` (new, one per theme) and one line per theme's
  `mod.rs` so `Theme::bed("composed", ..)` returns its `LayeredBed`. Task 4: balkan_fire to mambo (13 themes).
  Task 5: noir_jazz to villain_march (13 themes).
- **Produces:** per theme: home key and scale, one or more chord loops (the classic loop first, plus one or two
  more where the theme suits it), its tempo range, its layers built from the instruments its classic port already
  has, and its arrangement table.
- **Check:** one smoke test per theme: a composed bed builds and plays a whole song for one pairing the theme
  offers without error, and the master passes the levels check. One 20 s listening clip for the first theme of each
  task, played to the lead before moving on, so a wrong feel is caught early.

### Task 6: The composer

- **Files:** `studio/crates/bf-content/src/composer.rs` (new).
- **Produces:** `compose(personality, role, chord loop, key, scale, signal, music seed) -> (line, victory solo)`,
  in the fight file's existing `lines` and `solos` shapes. Rules from §8: chord tones on strong beats, a signature
  motif at each phrase start, every note inside the register, shaped by role (bass: roots and fifths on the beat;
  melody: motifs and the rhythm cells; answering line: replies in the gaps of the melody). **Roles follow the lead:**
  bar by bar the leader's line uses the denser rhythm cells and the trailing line thins and roughens as its HP
  drops, as the series already does. Seeded only by the music seed and the fighter's side, so a re-roll changes the
  music and nothing else.
- **Check:** unit tests: same inputs give the same notes; a different seed gives different notes; strong-beat notes
  are chord tones; every note is in register; a motif opens each phrase; the trailing side plays fewer notes in its
  last bars than the leader.

### Task 7: Composed mode wired into the build

- **Files:** `studio/crates/bf-core/src/fightfile/mod.rs` (`ARRANGEMENTS` gains `composed`; a `pinned` list of
  fighters whose lines and solos are kept as written), `bf-content/src/soundtrack.rs`, `bf-content/src/themes/mod.rs`
  (`Theme::bed` accepts `composed`), `bf-content/src/starter.rs` (new pairings get `classic` until Task 9's verdict),
  `studio/crates/bf-cli/src/cmd/` (`--arrangement` and `--music-seed` on the sound and render commands).
- **Produces:** when the arrangement is `composed`: the story signal is built, each fighter not pinned gets its
  composed line and solo, and the theme's layered bed plays. **Overrides:** a pinned fighter's hand-written line and
  solo are used as is. Hits stay the series rule: the attacker's instrument, in key, on the nearest sixteenth (the
  existing `q16` path, unchanged).
- **Check:** one test that a classic fight file builds the same soundtrack hash as before (Classic untouched); one
  that a composed build of FLUTE vs TROMBONE passes the levels check and puts every hit on a sixteenth in key; one
  that a pinned fighter keeps its lines. One full 61 s render from the command line, listened to once.

### Task 8: The Music screen

- **Files:** `studio/crates/bf-app/src/screens/music.rs`, the matching view model in `bf-app/src/model/` and its UI
  tests.
- **Produces:** an "Arrangement" switch (Classic / Composed); the music seed and "Re-roll music" become usable on
  Composed (today they are drawn disabled with `TAKES_LINE`, which is removed); a chord-loop picker when the theme has
  several; "Who plays bass" as today; a "Keep this fighter's line" toggle per fighter (the pin). **Levels on the
  Music step:** each take is run through the levels check as soon as it is built, and the screen shows "Levels:
  pass" or the reason it fails, so a failing take is seen there and not in the queue. Tempo stays shown, changed on
  Match-up.
- **Check:** UI tests that the switch sets the arrangement, re-roll changes only the music seed, and a failing
  take shows its reason. Re-bless only the Music screen snapshots.

### Task 9: Listening samples for the owner

- **Files:** none in the code. Output in `%USERPROFILE%\Videos\BALLFIGHT\Music samples\`.
- **Produces:** six short clips (about 25 s each, from the last lead change through the victory solo, as mp4 with
  picture so the music is heard against the fight), made by the command line from three pairings, each as Classic
  and as Composed:
  1. FLUTE vs TROMBONE on big top (a kit pair on a kit theme).
  2. CELLO vs TRUMPET on dark tango (a kit crossover).
  3. VIOLIN vs BAGPIPES on Balkan fire (two older fighters on an older theme).
  Each Composed clip uses a passing seed from the shortlist. A one-page note beside them says which is which.
- **Check:** every clip passes the levels check. Then the owner listens. **If the owner likes Composed**, one small
  follow-up commit makes it the default for new pairings in `starter.rs` and updates `docs/studio/HANDOFF.md`;
  otherwise their notes become the next round of tuning in Tasks 4 to 6.

## Owner question

Should Composed become the default for new pairings once you have heard the samples? Recommended default: yes,
Classic stays one click away and is always used for imported episodes.
