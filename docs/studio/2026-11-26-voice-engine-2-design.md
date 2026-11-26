# Voice Engine 2: design (2026-11-26)

Status: written for Aedan's approval. Nothing is built yet.

## Why

Aedan (sound build thread, 2026-11-26 07:34Z), after the sound upgrade's stage 3: "Good improvement. Was anything
limiting you from making the instruments sound better? A few of them still don't really sound like the instrument
(piano sounds weird and tacky, organ sounds like saw waves). [...] Maybe a new quality DSP for the software and an
internalized advanced synthesizer capable of making better and more real/pleasant instrument voices?"

The diagnosis (project memory `voice-engine-limits`): every fighter's voice is a note-for-note port of the Pixel
Pitch series voice. Stage 3's `real::Rework` re-levels the overtones of each finished note, which is an
average-tone EQ. It cannot change how a note starts, moves or ends, and most of what makes an instrument sound real
is there. The piano is at most 14 sines with one shimmer copy shared by every key, a 2 ms fade-in, no hammer, no
soundboard and no damper. The organ is three reed ranks detuned by plus or minus 2 cents at a dead-steady pitch, which
is the recipe for a synth "supersaw". The room is a 1.25 s tail. Every note is a separate one-shot.

## What Aedan decided

1. **Scope: the fighters first, then the band** (07:46Z): "Fighters and then the band." The band means each theme's
   own instruments (its bass, drums, chords, pads and choir, made today by per-theme synth recipes in
   `bf-content/src/themes/*/`).
2. **The composition system is not part of this** (07:46Z): "I will do a separate design for a better composition
   system taking advantage of the new capabilities of the instruments." So this work gives every new voice a way of
   being *played* (slurs, accents, swells, vibrato, pedal; see "Playing") that a later composer can call. It does not
   change what notes the parts write.
3. **The way to build it: both together** (08:12Z, on the sound examples page
   https://claude.ai/artifact/25tLBi68p2mSArbyfunXnm). Each note's tone comes from a **recipe measured off a real
   recording**, and the **working parts of the instrument** are modelled for what a recording cannot do. Aedan on the
   rebuilt clips: "The rebuilt ones are extremely impressive."
4. **Understand each instrument from many sources** (08:35Z), after asking whether better recordings exist: "I'd like
   you to make sure you're cross referencing and getting as good of an understanding as to how each instrument works
   as possible." So no voice is learned from one recording alone (see "Understanding each instrument").
5. **The piano is built from the Steinway, with no credit** (08:41Z, after hearing the Yamaha and the Steinway side by
   side on the examples page). The piano fighter's recipes come from VCSL's Steinway B (CC0). What that set lacks is
   learned from the other pianos (see "Working parts").
6. **The organ plays the main chorus, building up** (08:41Z). The organ fighter leaves today's trumpet-like reed stops
   for the church organ's main chorus of pipes, the sound of the real, built and rebuilt clips. Its ordinary notes
   play the chorus. Its big moves (CATHEDRAL's held full-organ chord, PULL ALL STOPS' swell through the stops, and the
   title blast) build from the quiet recorded setting up to the full loud chorus, so they still grow without the buzz
   coming back. No reeds are built in this pass.
7. **Approved** (08:56Z): "The organ probably shouldn't have so much of it's own echo. Other than that, approve". So
   the organ's own echo is light (see "Rooms").
8. **Aedan reviews each family before the next** (08:57Z): "For this one, I do want you to wait for my review after
   each family of instruments is built." So each stage stops at its listening page until Aedan has reviewed it (see
   "Stages").

The sketches behind that page are in the project files under `ballfight-studio/voice-engine-2/approaches/`
(`sketches/*.py`: `recipe.py` measures and rebuilds, `pianoA.py`/`organA.py` are the physical sketches, `real.py`
stitches the recordings, `render_all.py` makes the clips). They are the reference for what the engine should do, not
code to port.

## Goals

- Every fighter with a real counterpart sounds like that instrument, note by note: the way each note starts, how it
  changes while it sounds, and how it stops. Piano and organ first.
- Repeated notes never sound stamped: each strike differs a little, as on a real instrument.
- Every new voice can be played, not only triggered: accents, slurs, swells, vibrato and, for the piano, the pedal,
  ready for Aedan's composition system.
- After the fighters, the band's real instruments (bass, piano, organ, strings, brass, guitar, drums) use the same
  voices.
- Each stage ends with a listening page with spectrograms, and the last stage is a spectrogram review of every new
  voice (Aedan's standing request).

## Non-goals

- No new notes, melodies, rhythms or arrangement. What a part plays, and when, stays as it is (the composition system
  is Aedan's separate design).
- No change to Classic, to the picture, to the fight simulation, or to fight files.
- No recorded audio in the app or the repository. Recordings are fetched into a git-ignored cache to be measured, and
  only the measured recipes are committed and shipped. Playing recorded samples stays the fallback, used only if a
  family cannot reach the goal any other way, and only after asking Aedan.
- No change to the electronic fighters' character: SYNTH and TURNTABLES keep their own voices.

## Anti-goals (a failure even if it "works")

- A voice that matches the recording on one note but sounds stamped or lifeless in a fight: the same note, identical,
  three times in a row.
- Any fighter quieter, muddier on a phone, or more piercing than after the sound upgrade (the stage 1 and 2 rules
  hold: M1 levels, M2 lows on a phone, M3 highs).
- A render that stops because a sound check failed (Aedan's rule: every check fixes, and the render goes on).
- Classic sounding different by even one sample.
- A soundtrack build that gets noticeably slower (see "Speed").

## Constraints

- **Composed only.** Everything here applies when `ff.music.arrangement == themes::COMPOSED`. Classic keeps the old
  path through every stage and is pinned bit-exact by `bf-harness/tests/classic_golden.rs` and the kit-parity tests.
  Those must pass unchanged.
- **Deterministic and the same on every PC.** Engine crates use `bf_core::dmath` for maths (the `no_std_maths` test
  in bf-core), and every random choice comes from a seeded `bf_core::rng::Generator`. Two renders of a fight give the
  same samples.
- **Notes stay what the parts expect.** A part asks `MusicCtx::note` for a mono note of exactly `cap` seconds, mixes
  chords by adding notes of equal length, and runs its own HP colour on it. The new voices keep that contract (see
  "How the engine plugs in").
- **Project rules** (`docs/studio/HANDOFF.md`): Windows only. Tests run only on Aedan's PC, through `PC-Tests:` lines
  in the last commit's message (`tools/pc-runner/README.md`), and only the tests for what changed. Feature branches
  come off the build branch `claude/project-thread-h102zt` and merge into it. Nothing is installed unless Aedan asks.
  Rendering sound with the release binary in the cloud is fine and is how listening pages are made.
- **Starting point.** Work starts from the build branch once the sound upgrade's PR #28 has merged, and builds on its
  `bf_sound::voice::real`, `bf-harness/examples/references.rs` and `bf-harness/tests/real_voices.rs`.
- **Recordings: free and openly licensed only**, listed with their licence in `docs/studio/voice-engine-2-sources.md`
  (written in stage 1). Anything that ships is measured from a CC0 set or one that is "free for any use" (VSCO 2 CE,
  VCSL, Karoryfer, Univ. of Iowa MIS), so no video or caption needs a credit. A CC BY set (Salamander, MTG saxes) is
  used only if no CC0 set reaches the goal for that instrument, and then only after asking Aedan, because its credit
  would have to travel with the sound (see "Credits"). The cloud can clone these from github.com
  (`git clone --filter=blob:none --no-checkout` then `git sparse-checkout set --no-cone <paths>`; never `git ls-tree -l`
  on such a clone, which downloads every file). Sites that are blocked in the cloud (Univ. of Iowa MIS, Karoryfer's
  shop) are fetched on the PC.

## Approaches considered

1. **Built from how it works** (physical models only). Every note made fresh, so playing comes naturally, small, and
   each fighter keeps a sound of its own. But it is the hardest way to sound truly real and the slowest to tune, and
   Claude cannot listen. On the page the built piano and organ were close but plainer than the recordings.
2. **Rebuilt from real recordings** (recipes only). The closest to real straight away, with one engine for every
   family. But it only does what was recorded: slides, swells, the pedal and notes longer than the recording need extra
   work, and a few fighters (the octobass, the accordion) have no good free recording.
3. **Both together (chosen by Aedan).** Recipes give each note its true tone. Modelled working parts add what a
   recording cannot do: the piano's dampers, pedal and sympathetic strings, the organ's wind and church, and the bow,
   breath and lips behind slides, swells and vibrato. Fighters with no recording get a physical model, or start from
   their nearest relative's recipe. On one note it sounds like the rebuilt clips; in a phrase it plays like the built
   ones.

Samples were the fourth way and stay the fallback: the most real per note, but big, identical on every repeat, and
unable to slide or swell.

## Understanding each instrument (cross-referenced)

Before a family is built, each of its instruments is studied from every source that can be used, so the voice
follows how the instrument works and not the quirks of one recording (one player, one microphone, one room, one tired
string):

- **Every usable recording of it is measured**, not only the one the recipe ships from. For the piano that is the
  VCSL Steinway B, the Iowa Steinway B (PC), Salamander's Yamaha C5 and the Headroom Yamaha C3; for the organ, VSCO's
  and VCSL's chorus and Lars Palo's Bureå and Piteå organs (whose reed stops are the only free recorded reeds). The
  same measurements as the yardstick (below) are taken from each, so they line up.
- **What they agree on is how the instrument works; what they don't is the spread.** A recipe's source is checked
  against the others first, and anything only it does (a dull key, a room's ring, a loud microphone hiss) is evened
  out or measured from a different note. The built working parts are tuned to what all sources share.
- **Published acoustics** (for example Fletcher and Rossing, *The Physics of Musical Instruments*, and the research
  each family has, such as the two-stage decay of coupled piano strings, or how a reed's tone brightens as it is
  blown harder) is read for each family and checked against the measurements. Where the two disagree, the
  measurements win and the note says so.
- **Each instrument gets a short sheet** in `docs/studio/voice-engine-2-instruments.md`: its working parts, how its
  tone changes with strength and pitch, how a note starts, lives and stops, its noises, how players play it (vibrato
  speed and depth, slurs, accents), and the spread across the sources, with each source named. The sheets feed the
  working parts and the playing table, and later Aedan's composition system.

**Which recordings may be used for what.** Studying and comparing (measuring general facts about an instrument, which
belong to nobody) may use any set whose licence allows any use or adaptation: CC0, "free for any use", CC BY and
CC BY-SA. What ships (the recipes) still comes only from CC0 and "free for any use" sets (see "Constraints"). Sets
that are non-commercial or "all rights reserved" (the Virtual-Organs.nl organs, the Maestro Concert Grand, paid
libraries) are not used at all. Blending several sets never changes this: a blend carries every source's licence.

## The engine

### Recipes (measured once, committed)

A recipe is what the sketch's `recipe.measure` makes, compacted. For each recorded note (a pitch and a strength):

- **Every overtone over time.** For each overtone k (up to 16 kHz): where it sits (the piano's overtones are stretched,
  `f_k = k f0 sqrt(1 + B k^2)` with `B` measured per note), and its complex level over time, which carries its
  loudness, its beating and its small pitch wobble. Measured by tuning in to each overtone (multiply by `e^(-i 2 pi f_k
  t)` and smooth with a Blackman window of 6 / f0 seconds), per channel.
- **The noise over time.** What is left once every overtone is taken out (hammer knock, pipe air, breath, bow hiss,
  key and valve clicks), as levels in 24 bands from 60 Hz to 16 kHz. The recording's own steady hiss and hum are
  removed (the sketch faded the noise out after 0.25 s for the piano; the engine needs a proper noise-floor estimate
  per band from the recording's silent tail so a breathy instrument keeps its breath).
- **The stopping.** Where the recording has a key-release (Salamander's `rel*` files, the organ's release), how the
  tones and the noise die.

The sketch kept 400 frames a second of every overtone, about 9 MB a note. The committed format is compact (log-spaced
time points: fine early, coarse later; levels in dB as 16-bit numbers), versioned, and documented next to its reader.
Budget: at most 1.5 MB per instrument and 40 MB for all of them, in `bf-content/data/voices/` (the fighters' models
already live in `bf-content/data`).

The tool that makes them is a `bf-harness` example, beside `references`:

```text
cargo run --release -p bf-harness --example recipes -- fetch <instrument>      # into reference/.cache/recordings
cargo run --release -p bf-harness --example recipes -- measure <instrument>    # writes bf-content/data/voices/...
cargo run --release -p bf-harness --example recipes -- check <instrument>      # rebuilt vs the recording, over time
```

### Playing a recipe

A voice turns a recipe into a note:

- **Pitch and strength between recordings.** Recordings come every few semitones and at a few strengths. A note
  between them takes each overtone's level from the two nearest recorded pitches and the two nearest strengths,
  interpolated in dB by overtone number, at its own frequencies. (The sketch only moved the nearest note by up to a
  semitone. Interpolating keeps each instrument's body resonances in place.)
- **Length.** A note shorter than the recording stops the instrument's own way (damper, pipe release, end of breath).
  A note longer than the recording carries on with the measured late decay (piano) or the measured steady state with
  its natural slow wobble continued (organ, winds, bowed). It never loops audibly.
- **Repeats differ.** The cache's three variants (`real::TURNS`) differ like real strikes do: a little in strength and
  brightness, in the overtones' starting phases, in the noise, and in the hair of tuning.
- **Oscillators** are clean (no aliasing: no overtone is made above 0.45 of the sample rate) and fast (see "Speed").

### Working parts (modelled)

These are the "built" half, added where the recording falls short, each per family:

- **Piano:** measured from VCSL's Steinway B (CC0), Aedan's choice. That set was evened out to one loudness across its
  three strengths, so its soft notes carry raised microphone hiss and no natural loudness steps: the recipes drop the
  steady hiss, put back the loudness of each strength, and take how the tone brightens from soft to loud from what the
  other pianos (Iowa Steinway B, Salamander, Headroom) agree on. The working parts are the damper (each overtone damped
  at the key's release at the rate measured from the release recordings, the top 18 keys undamped as on a real
  grand), the sustain pedal (no damping, plus the other strings ringing along softly), and the hammer knock's
  strength following the strike. A note's `dur` is the moment the key comes up, as stage 3 already treats it.
- **Organ:** the main chorus (Aedan's choice), measured from VSCO 2 CE's quiet and loud settings (CC0), with VCSL's
  organ and Lars Palo's two organs studied alongside. Growth between the two settings is a crossfade of their recipes
  as the stops come in, in the timing the moves already use. The working parts are the wind (a
  slow shared waver, a slight sag on big chords), the speech of each pipe (low pipes speak slower), and the church (see
  "Rooms").
- **Bowed, reeds, brass, flutes** (later stages): bow, breath and lip noise that follow the playing, slurs that glide
  between notes without a new attack, swells, and vibrato that grows on a long note.
- **Plucked and struck** (later stage): string damping and buzz, palm mutes, the electric guitar's amplifier.

### Rooms and stereo

- **Rooms.** The Composed room (`polish/room.rs`, F3 of the sound upgrade) stays as the shared room of the mix. A
  voice may also have a **home room** of its own, sent from its bus. The organ's is a light one (Aedan: it "probably
  shouldn't have so much of it's own echo"): a short church tail of about 1.5 to 2 s, nothing under 200 Hz, mixed well
  under the dry sound, much less than the examples page used. The echo is just heard after a big chord and never
  blurs a run. It is checked on a phone. Rooms for whole themes (a club, a hall, a stadium)
  belong to the band stage, because the room is shared by everything in the theme.
- **Stereo.** Notes stay mono, as the parts expect. Width comes from a stereo stage on the bus of each fighter that has
  moved to Voice Engine 2 (only those, so a stage changes only its own fighters), under Composed: a decorrelated side signal added to the panned bus, so that the mono sum is still the dry sound (most
  phones play mono). The piano and the organ are wide, the winds narrow. The piano also spreads its bass a little
  left and its treble a little right, as a player hears it.

### How the engine plugs in

- A new voice implements the existing `Voice` trait (so Classic, the registry, the levels tool and every test keep
  working), and a new `Voice::engine2()` says it has one. `MusicCtx::note_in` then calls the new playing call
  instead of `note_real` under Composed, and passes it the cap so the voice can end its note naturally inside it.
  Under Classic nothing changes.
- When a fighter moves to Voice Engine 2, its stage 3 rework tables (`FILL`, `CORRECTION`, `tail`) and their
  `real_voices.rs` checks go, and the new checks (see "Tests") take their place. The ported voice stays for Classic.
- The level table (`bf-content/src/levels_table.rs`, `ballfight levels --write`) is rewritten for every changed
  fighter, so M1's "5 dB over the band" still holds.
- A theme that plays a fighter's voice changes with it. The mambo and noir jazz bands play the PIANO voice through
  `ctx.note` (`themes/mambo/mod.rs`, `themes/noir_jazz/mod.rs`), so their band pianos move to Voice Engine 2 in stage 1,
  as they did with stage 3's rework. That is wanted (it is what stage 7 would do), and stage 1's tests include
  `bands.rs` and those themes' Composed suites.

## Playing (for the composition system)

Every new voice takes a played note, not only a pitch, a length and a strength:

| Field | What it does | Voices |
| --- | --- | --- |
| accent | a harder, brighter start | all |
| slur from (a pitch) | starts from the previous note's pitch and sound, with no new attack | bowed, reeds, brass, flutes, organ (no gap) |
| slide / bend / fall / scoop | the pitch moves during the note | bowed, reeds, brass, trombone most |
| swell | the strength changes during the note (louder, softer, or both) | bowed, reeds, brass, flutes; organ via its swell box |
| vibrato | depth, speed and how long it waits before it grows | bowed, reeds, brass, flutes |
| pedal | the piano's sustain pedal | piano |
| mute / stop | a muted, choked or clipped note | plucked, brass mutes, drums |

A voice says which fields it takes and ignores the rest. A phrase call plays a run of played notes as one connected
line (slurs need the next note), so a composer that knows its whole line can ask for it. The types are defined in
stage 1 and documented in `docs/studio/voice-engine-2-playing.md` for Aedan's composition design. Today's parts use
them only where it is automatic and safe: strength becomes accent, a long held note on a bowed or blown voice gets
growing vibrato, and a one-voice note that starts the moment the previous one ends is a slur. Today's parts place
notes in any order (a part may place its bounces before its swings), so a slur needs the fighter's notes in time
order. The suggested way is a first pass that records each one-voice part's notes, then the real pass that plays
them knowing their neighbours. That is the builder's to settle in stage 3, the first stage that needs slurs.

## The yardstick (tests against real recordings, over time)

Stage 3 of the sound upgrade compared the average tone of a whole note. The new yardstick compares over time, against
the recording the recipe was measured from (and, for a fighter built from a relative, against that relative), and
against the spread of every other recording of the instrument (see "Understanding each instrument"): a voice must sit
inside what the real instruments do, not only near one of them.

- **Overtones over time:** each of the first 16 overtones' level at 30 ms, 100 ms, 300 ms, 1 s and 2.5 s, against the
  recording's, at every recorded pitch and strength.
- **Start and stop:** the time to reach full level and the time to die by 40 dB after release.
- **Noise:** the noise's share of the note in its first 100 ms and in its steady part.
- **Not a rival:** each note stays nearer its own instrument than the rivals it could be taken for (the
  `real_voices.rs` rule, kept).
- **Repeats:** the three variants of a note differ by more than a set amount and less than another.

The recordings' measurements over time are committed as small reference files (as `tests/reference/fluidr3.json`
is), so the tests run on the PC without the recordings. Thresholds are set in stage 1 from what the rebuilt clips
reached (about 2 to 3 dB per overtone on the C4 the sketch compared) and kept for every family.

## Work that has to happen on the PC

The PC runner only runs tests. Everything else that needs the PC (fetching recordings from sites the cloud cannot
reach, measuring them, timing a build on the PC) goes through a Remote Control session in the project's folder
(`F:\!Ai Stuff\InstrumentBattleMachine`), on the stage's own branch, sharing the main build folder, and pushes only
recipes and reference measurements, never recordings. It does not need Aedan's time beyond approving the session.

## Speed

Notes are cached per key and repeats take turns among three variants, so a fight needs only a few hundred distinct
notes. Even so, a note of 60 overtones per channel at 48 kHz is real work. The engine drops overtones that are 70 dB
under the strongest, stops an overtone once it has died, and uses recursive oscillators. The budget: a Composed
soundtrack build may take at most 25% longer than today's for the same fight, measured in stage 1 on the review
fights with `ballfight sound`, before and after, on the same machine (the cloud is fine for the ratio; today a
Composed build of a review fight takes 25 to 40 s there, the fight's simulation included).

## Credits

Because only CC0 and "free for any use" sets ship, no credit is required. The recordings are still thanked, as a
courtesy: a "Sound" paragraph in the program folder's `ABOUT.txt` and in the guide names every set and its author (no
new screen). `docs/studio/voice-engine-2-sources.md` lists every set, author, licence and what was measured from it.
If a CC BY set is ever needed, its credit has to go with the sound (the caption of every video that uses it), which
is why that needs Aedan first.

## Stages

Each stage is its own pull request into the build branch. Unlike Aedan's standing rule for feature work, the stages
do **not** run on by themselves: each stage stops at its listening page, which is posted in the thread, and the next
family starts only after Aedan has reviewed it (his ask, 08:57Z). His notes are fixed in the same stage first. Each stage ends with a listening page (the fighter's riff today and new,
the real recording where there is one, a stretch of a real fight rendered with `ballfight sound` cut the stage 1 way,
`-ss 10 -t 22`, -16 LUFS, with phone versions, and spectrograms with a playhead), and merges once its PC tests pass and Aedan has reviewed it.
Each stage's plan is written when it starts, and its first step is the instrument sheets for that family.

| Stage | What | Recordings (see the sources note) |
| --- | --- | --- |
| 1. Pilot | The engine, recipes, the yardstick, the playing types, rooms and stereo, credits; **piano and organ** (and the mambo and noir jazz band pianos, which share the PIANO voice) | VCSL Steinway B (CC0); VSCO 2 CE organ, quiet and loud (CC0). Studied also: Iowa Steinway B (PC), Salamander, Headroom; VCSL organ, Lars Palo's Bureå and Piteå organs |
| 2. Plucked and struck | acoustic, electric (with its amplifier), bass guitar, ukulele, harp, marimba, timpani | Karoryfer basses (CC0), VSCO and VCSL harp, marimba, timpani; guitars and ukulele mostly modelled |
| 3. Bowed | violin, viola, cello, contrabass, octobass; slurs, vibrato and bow noise | VSCO solo violin, viola and cello sections, solo contrabass; Iowa solo viola and cello (PC); octobass from the contrabass |
| 4. Reeds | the four saxes, the three clarinets, oboe, bassoon, contrabassoon, bagpipes, accordion | Weresax alto (CC0), VCSL tenor sax and saxello (soprano), Bear Sax bari (CC0, PC), VSCO winds, Iowa bass clarinet (PC), Squidpipes (CC0); accordion from VCSL's harmonicas and a reed model |
| 5. Brass and flutes | trumpet, cornet, flugelhorn, horn, mellophone, trombone, euphonium, tuba, sousaphone, flute, piccolo | VSCO brass and flutes; cornet, flugelhorn, mellophone, euphonium and sousaphone from their nearest relatives |
| 6. Drums | drums, snare drum, bass drum | VCSL and VSCO percussion (CC0) |
| 7. The band | each theme's real instruments (bass, piano, organ, strings, brass, guitar, drums) use these voices; rooms per theme | as above |
| 8. Review | a spectrogram review of every new voice for harshness, fullness and bad spots, and the fixes it finds | |

Synth-genre themes (synthwave, chiptune, dubstep and the like) keep their synths in stage 7.

## Tests

All through `PC-Tests:` lines, only what changed:

- `classic_golden` and the kit-parity tests, unchanged, in every stage.
- The yardstick tests for each converted fighter (a new `bf-harness` test, `engine2_voices.rs`, in place of that
  fighter's `real_voices.rs` checks).
- The engine's own unit tests (recipe reading, interpolation, determinism: the same note twice is identical, repeats
  differ, no overtone above 0.45 of the sample rate, notes exactly `cap` long).
- The fighter's `fighter_*` and `episode_*` suites that cover it, and `composed_build`.
- The band stage: `bands.rs` (phone loss) and the composed theme suites.

## Calls made while writing this (Claude's, for Aedan to check)

- What ships is measured only from CC0 or "free for any use" sets, so no video needs a credit; the sets are thanked
  as a courtesy in `ABOUT.txt` and the guide.
- The mambo and noir jazz band pianos change with the piano fighter in stage 1, because they share its voice.
- Notes stay mono; stereo width is added only on the buses of converted fighters.
- Work that needs the PC (fetching the Iowa and Lars Palo recordings, measuring them) goes through a Remote Control
  session on the stage's branch and pushes only recipes and measurements.
- Speed: a Composed soundtrack build may take at most 25% longer, measured before and after on the same machine.
- The organ's home room is a light church tail of about 1.5 to 2 s, after Aedan's note that it shouldn't have so much
  echo.
- Eight stages in the order above, each merged after its PC tests and Aedan's review of its listening page.

## Left to the builder

- The compact recipe format, the time points kept, and how levels are packed.
- How many recorded pitches and strengths each instrument keeps, within the size budget.
- The oscillator and noise-shaping method, within the speed budget.
- The exact rooms, the stereo widths and the home-room levels, tuned on the listening pages and checked on a phone.
- How slurs find a note's neighbours in today's parts (stage 3).
- The thresholds of the yardstick, set in stage 1.

## Changed in stage 1

Decided differently while building the pilot (each is a ruling in the stage's ledger; Aedan hears the result on the
listening page):

- **The organ speaks quickly.** The VSCO organ recordings take 0.3 to 0.9 s to reach full level, because the
  church's echo builds up around a distant microphone; a pipe speaks in tens of milliseconds (the literature, and the
  closer VCSL Renaissance organ's 45 to 130 ms). The engine lifts the recipe to full level within 0.06 s (0.12 s two
  octaves under middle C), so the fights' short organ stabs are not late and weak. It follows Aedan's "less of its
  own echo" too.
- **The yardstick compares tone moment by moment against that moment's strongest overtone**, single overtones only in
  a piano note's first 0.3 s, and the overtones' median after that; the organ by its held tone. Real single overtones
  beat in and out by up to 33 dB (unison strings, the chorus's ranks), so "each overtone within a few dB at each
  time" cannot hold even for the recordings themselves. The organ's start is tested in its own unit test, not
  against the recordings.
- **Rivals against the recordings.** "Nearer its own instrument than the rivals" takes the real recording as its own
  instrument and FluidR3 only for the rivals: FluidR3's own piano and church organ were further from the real ones
  than FluidR3's harp or accordion were.
- **The piano's strength steps** come from the other pianos, matched by brightness (VCSL evened its strengths out to
  one loudness): soft -17.4 dB, medium -5.3 dB, loud 0 (see the PIANO sheet; the Iowa Steinway, measured on the
  PC, moved the soft step from -15 dB).
