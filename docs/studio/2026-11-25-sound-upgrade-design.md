# Sound upgrade: design (2026-11-25)

Status: written for Aedan's approval. Nothing is built yet.

## Why

Aedan (2026-11-25): "The composition system is a great starting point, but a lot of the actual results sound
pretty messy/unpleasant to listen to. The higher register instruments are very loud and obnoxious, and the lower
register instruments sometimes can barely be heard."

Two rounds of review measured the causes and sketched 27 ideas with before and after clips:
https://claude.ai/artifact/MrSWyRSaecPY41gUhC7c2m (source, clips, measurements and the scratch tools are in the
project files under `ballfight-studio/sound-review/`; `tools/round2/README.md` explains the tools, and
`tools/round2/info.json` holds every number quoted here). The clips were made outside the app and tuned by
measurement only. They are the reference for what each item should do, not code to port.

## What Aedan picked

M1 M2 M3 M5 M6 F1 F2 F3 C2 C4 C6 S1, with two notes:

1. "The Bari sax kind of sounds like a trombone, and a commenter even pointed this out."
2. "The melodies of the instruments sound significantly more boring with the note fixes on." (About the
   "Mix + note fixes" clips, which had the hook tune C1 and the accents C3. Neither was picked.)
3. "I don't mean the melodies should always stay the same as today's, they should still be improved, but just
   not to be as boring as that initial example."

4. "For the bari sax, it sounds like a trombone because of the way it plays its notes. It plays a sliding note
   that sounds like a real-life trombone's 'Wah waahhh'." (Measured and confirmed, see "Bari sax: no slide".)
5. "The melodies are fine, could there maybe be different 'styles' or variations of how the melodies are
   composed within each theme?" (About the C10 clips. Read here as: fights on the same theme should not all
   write their melody the same way. Sketched as C11, melody styles.)
6. "The melody styles are all playing the same instrument melodies. I meant that the fighters would play new
   melodies in each style." (About the first C11 sketch, which kept each fighter's signature tune and only
   changed the bars between. Two causes, both fixed in this design: the styles were too alike, and the trumpet
   in that clip does not play the written melody at all. See "Every pitched fighter plays its written line" and
   C11.)

Because of notes 2 and 3 a new item, **C10 (melodies improved, but kept lively)**, was sketched after the picks
and is on the page with three-way clips (today, the boring first try, the new one). It includes C2. It is part
of this design if Aedan approves it; if he does not, C2 is built on today's lines as a harmony pass only.
(Aedan, later: "The melodies are fine.") **C11 (melody styles)** was sketched after note 5, reworked after note 6, and
builds on C10.

Not picked, so not built: M4, C1 (as sketched), C3, C5, C7, C8, C9, S2, all X ideas, T1.

## Goals

- Every fighter is clearly audible and none is piercing, on a phone speaker as well as on headphones.
- Every video has the same loudness.
- Melodies are less scattered and more tuneful, and stay as busy and varied as they are today. The two fighters
  never land on a harsh clash.
- Each theme's band plays its own style and is balanced on small speakers.
- Each instrument sounds like its real counterpart. First of all: the bari sax must not be mistaken for a trombone.

## Non-goals

- No slow hook (one note a beat, the same four bars repeated: Aedan found it boring), no accents, no rests, no
  change to how many notes a fighter plays or when (C3 and C5 were not picked).
- No recorded samples in the app. FluidR3 recordings are a measuring stick during development only and are never
  shipped or committed.
- No new sound effects.
- No change to the picture, the fight simulation, or fight files' format beyond what an item below needs.

## Anti-goals (a failure even if it "works")

- Melodies that sound slower or more repetitive than today.
- A render that stops because a sound check failed. Every check corrects the sound and the render goes on
  (Aedan's standing rule: checks that can't auto-fix are removed).
- Classic arrangement or any imported or older episode sounding different. Classic is pinned bit-exact by the
  kit-parity tests in `bf-harness/src/sound.rs`, and those tests must keep passing unchanged.

## Constraints

- Everything here applies when `ff.music.arrangement == themes::COMPOSED` and only then. Classic takes the old
  path through every stage (voices, levels, master, reverb, band tables). Where a shared function or table
  changes, Composed gets its own copy and Classic keeps the old one.
- Kit parity only pins Classic for the 14 kit themes and 20 kit fighters. The 36 Studio themes' Classic beds use
  the same `layers()` and shared `pattern()` as Composed, with no parity gate (`bf-harness/tests/studio_themes.rs`),
  and the Studio fighters' voices have none either. So before stage 1 changes anything, a golden test is added
  for Classic: a hash of the rendered soundtrack of one Classic fight per Studio theme and per Studio fighter,
  blessed on the PC from the build branch as it is today. Those hashes must not change in any stage. (My call:
  the page promised Classic stays as it is. Aedan can relax this.)
- Re-rendering an existing Composed fight will sound different (better). That is intended.
- Sound stays locked to the action: a note still starts on the sixteenth its hit lands on.
- Project rules in `docs/studio/HANDOFF.md` hold: Windows only, tests run on Aedan's PC through `PC-Tests:` lines
  in the last commit (never in the cloud), only the tests for what changed, feature branches merge into the build
  branch `claude/project-thread-h102zt`, no install unless Aedan asks.
- Rendering sound with the release binary in the cloud is fine and is how listening pages are made.

## Approaches considered

1. **Three stages, each its own pull request into the build branch (chosen).** Stage 1 is the mix, the final
   polish, the melody items and the bari sax slide. Stage 2 is the bands. Stage 3 is the instruments. Stage 1 lands first
   because the level of every instrument is measured there and stages 2 and 3 are tuned on top of it. Stages 2 and
   3 touch different files (themes, fighter voices) and can run one after the other or side by side. Each stage
   ends with a listening page of real renders, so Aedan hears progress early and can redirect.
2. One big pull request with all twelve. Fewer merges, but nothing can be heard or installed until the largest
   jobs (62 themes, 42 instruments) are done, and a wrong turn is found late.
3. Twelve small pull requests. Most flexible, but M1, M2, M3, F1 and F2 all meet in the same few functions of the
   master and would conflict or be tuned against a moving target.

## How "loud to the ear" is measured

One measure is used throughout, the same one the review used (`tools/remix.py`, `heard` and `over_band`):

- **Heard loudness**: the signal through a 2nd-order 250 Hz high-pass and the BS.1770 K-weighting, in 400 ms
  blocks every 100 ms.
- **Over the band**: the mean heard loudness of a fighter's stem, less that of the summed band stems, over the
  blocks where the fighter sounds (within 20 dB of its loudest block), from FIGHT! to the K.O.
- **On a phone**: the signal through a 3rd-order 350 Hz high-pass first.
- **Piercing share / rumble share**: the share of a stem's energy in 2 to 6 kHz / below 200 Hz.

These belong in one small module the master and the dev tools share. The existing K-weighting in
`bf-export/src/checks/loudness.rs` is the same filter.

The ten review fights are the fight files in the project files under
`ballfight-studio/sound-review/tools/round2/f/` (thirteen files; the ten are the ones named in `info.json`).

## Order of the last stages, and the existing fixes (Composed)

Today the export runs `fix_sound` after the master (`bf-export/src/job.rs`, `bf-export/src/fix.rs`): it can lift a
fighter, even out 2 s windows and re-limit. For Composed the order becomes: parts placed, M2 and M3, M1 trim,
F3 room, F2 polish, then the existing checks and their fixes, then **F1 last**, so the loudness target holds
whatever a fix did. The limiter of F2 is the one used wherever a fix re-limits.

The `band` check measures each fighter in its own band (the tuba's is 40 to 260 Hz), which M2 thins. For Composed
the check uses the heard measure above in place of the fighter's band, with the same pass rule (over the band
while it sounds). With M1 in place it should never fire; if it does, its fix runs and F1 follows.

No check stops a Composed render. Where `HANDOFF.md` says a check that still fails after its fix stops the
render, that line is corrected for Composed: the render finishes and the result is noted in the render's report.

## Stage 1: mix, polish, harmony

### M1. Every instrument equally loud to the ear

- Today each fighter's `Sound::bus_gain()` is a hand-set number. Against the same band the 42 fighters spread
  over 20 dB (piccolo +9.8, octobass -10.6).
- New, two layers:
  1. A **saved level per fighter**: a dev command renders each fighter against a fixed reference band and writes
     the gain that puts it 5 dB over the band into a table in `bf-content`. The table is committed. It is
     re-run when a voice changes (stage 3 re-runs it for each reworked instrument).
  2. A **trim per render**: after the two parts are placed and before the master, measure each fighter's stem
     over this fight's band and move it to 5 dB over, by at most 6 dB either way. The winner's solo bus takes the
     winner's trim.
- Order matters: M2 and M3 shape the tone first, then M1 measures.
- Done when: in the ten review fights every fighter sits 4 to 6 dB over the band by the heard measure, and the
  `band` check (as changed above) passes without its fix.

### M2. Low instruments you can hear on a phone

- For fighters whose rumble share is over half (tuba, octobass, timpani, contrabass, sousaphone, bass drum;
  check trombone, drums, contrabass clarinet): add overtones one and two octaves above the note, stronger the
  lower the note, and remove what lies under 45 Hz.
- This is the quick, general version, applied as a shared treatment on the fighter's bus. Stage 3 rebuilds the
  voices properly; when a voice has been rebuilt and meets the target by itself, the treatment is switched off for
  it.
- Done when: each of those fighters loses at most 9 dB on a phone (the tuba loses 17 today) and its rumble
  share is at most 70%.

### M3. Take the sting out of high instruments

- For fighters whose piercing share is over a quarter (piccolo 77%, bagpipes 46%, snare 34%, violin 27%): a
  gentle cut of the 2 to 6 kHz range that acts only while the fighter is loud there, and notes that play softer
  the higher they are above C6.
- For those same fighters only, ordinary notes stay out of the top octave of the instrument's register: the
  composed line, and the part's own ordinary sounds too (the piccolo's bounce ticks climb to the top of its
  register in `fighters/piccolo/sound.rs`). Special moves may still use the top. Rhythm and the shape of the
  melody do not change; only the very highest notes fold down an octave.
- Done when: in each review fight with one of those fighters, its heard level in 2 to 6 kHz alone is no more
  than 3 dB above the other fighter's overall heard level. (The piercing share is reported but is not the
  target: a piccolo is mostly high sound by nature, and the sketches left it near 59%.)

### M5. One note at a time for blown and bowed instruments

- Today notes of one fighter overlap: 2.7 older violin notes are still sounding at each new one.
- New: for wind, brass and bowed-string fighters, when the fighter's part places a new pitched note, its
  previous note fades out over 30 ms starting at that instant. Struck, plucked and keyed instruments (piano,
  marimba, harp, guitars, ukulele, drums, timpani, organ, accordion, synth, turntables) are not touched. Notes
  that a special move places together as a chord are one event and do not cut each other.
- Each fighter declares which kind it is (a flag on its `Sound` or personality).
- Done when: for those fighters the count of older notes still sounding at a new note is zero outside chords.

### F1. One loudness for every video

- The master brings every Composed soundtrack to -14 LUFS integrated (BS.1770, gated), within half a dB, with
  the true peak at or under -1 dB. It measures, sets the gain, and measures again.
- The existing `levels` and `evenout` checks stay and run as they do now.

### F2. A cleaner final polish

- Replaces the single compressor and the tanh clip for Composed: the mix is split into three ranges (around
  200 Hz and 3 kHz), each gently compressed by itself, summed, then a limiter that looks 5 ms ahead holds the
  peaks.
- Done when: in Piccolo vs Tuba a tuba note no longer pulls the piccolo down by more than 1 dB (today's
  compressor gain is shared by everything), and no sample is clipped.

### F3. A better room

- Today's `synth::reverb` is decaying noise with no high-pass: 82% of the room's energy is under 200 Hz.
- New for Composed: 20 ms before the room starts, about seven early reflections, a tail of about 1.25 s whose
  highs fade faster, nothing under 220 Hz and nothing above 7 kHz in the room. Sends by kind of instrument: low
  fighters 0.04, piercing fighters 0.2, the others 0.3. Band layers keep their relative sends. The room's
  overall loudness against the dry mix stays what it is today (about 18 dB under).
- Done when: the room's rumble share is under 10%.

### Every pitched fighter plays its written line (needed for C2, C10 and C11)

- Found while checking Aedan's note 6. In Composed the composer writes a whole-song line for each fighter
  (`ctx.composed`), but a fighter's part only sounds it if the part asks for it with `ctx.line_at`. Measured
  (`tools/round2/b10.py`, `line_survey.json`): every fighter rendered twice, once as today and once with a
  wholly different line; a stem that comes out bit-identical does not follow the line.
- **Do not follow the line (stem identical):** ACCORDION, ACOUSTIC, BARI SAX, BASS GUITAR, CONTRABASSOON,
  CORNET, HARP, MELLOPHONE, ORGAN, PIANO, SOPRANO SAX, TIMPANI, TUBA, UKULELE, and SYNTH (by its code; its test
  fight failed to render with an unrelated error, "sosfilt: x is empty", in ALTO SAX vs SYNTH, which the builder
  should look at). They play their own fixed notes from the chords. The unpitched BASS DRUM, DRUMS, SNARE DRUM
  and TURNTABLES do not either and need not.
- **TRUMPET** follows the line only when the fight file itself has a line for it
  (`ctx.ff.music.lines.contains_key(name)` in `trumpet/sound.rs`), which some themes supply (hot jazz) and
  others do not (mambo). So in Trumpet vs Cello it plays combo notes, and in Trumpet vs Clarinet the line.
- **Follow the line:** the other 22 (ALTO SAX by its code).
- So today the harmony pass, the livelier melodies and the styles would not reach 16 of the 38 pitched
  fighters. Earlier review clips with the trumpet on mambo or the tuba changed only the other fighter.
- Fix, Composed only: each of the 15 pitched fighters above takes the pitch of its ordinary notes from
  `ctx.line_at(name, t)` at the note's time, as the 22 do, and the trumpet uses the line whenever
  `ctx.composed` has one for it. Timing, tone and effects of each part stay as they are. Notes that belong to
  a special move or a losing sound may keep their own pitch. A bass-role fighter gets its bass line this way
  (the bari sax's swing honk plays the line's note at the swing instead of the chord's root).
- Done when, in a test per pitched fighter: two renders with different lines give different stems, and every
  ordinary note's pitch is the line's note at its time.
- This changes how Composed fights with these fighters sound on a new render, like the rest of stage 1.
  Classic is untouched.

### C10 with C2. Melodies improved but kept lively, and in harmony

- Today (`composer.rs`, `loop_line`): each four-bar phrase opens with the fighter's signature motif and echoes
  it in bar three, but the bars in between are a walk by the fighter's favourite leaps, picked at random. That
  walk is the scattered part (in Piccolo vs Tuba 48% of the piccolo's moves are bigger than a fourth).
- The first fix (C1) used one note a beat and the same four bars for the whole fight. Aedan found it boring.
  Measured, it has 2.7 note changes a bar against 7.0 today and repeats a bar exactly 81% of the time.
- New, for melody and answer roles (a bass line is not changed). The motif, its echo, the rhythm cells, the
  number of notes and the story shapes (`shape_upper`) stay. Only the in-between bars are written differently:
  1. **The answer** (bar two of a phrase): the motif's own shape, in scale steps, turned upside down, starting
     on the chord tone nearest the last note, on the bar's rhythm cell.
  2. **The cadence** (last bar): a run by step into the chord tone nearest the motif's first note.
  3. **Any other bar**: a walk by step with at most one of the fighter's favourite leaps, turned back after it.
  4. **Variation**: on every other pass of the chord loop the answer bar is mirrored, so two passes in a row are
     never the same. The builder may add more variations of this kind (the echo moved, the cadence from the
     other side) as long as the measures below hold.
- Then the harmony pass (C2) runs over the two finished lines:
  - Which fighter gives way: the melody when the other fighter has the bass role, otherwise the instrument with
    the lower register.
  - Per eighth-note slot where both lines have a note: if the two notes are a semitone, a tritone or a major
    seventh apart (in any octave), or a whole tone apart on a strong eighth (the first and fifth of the bar),
    the giving fighter's note moves to the nearest note of the fight's scale, within four semitones and inside
    its register, that makes a unison, third, fifth, sixth or octave with the other. Ties go to the note closer
    to the line's previous note. If no such note exists the note stays.
- Sketch: spikes `BF_SPIKE_LIVELY` and `BF_SPIKE_HARMONY` in `spikes2.patch`; clips `c10`, `c10x`, `c10y`
  (three-way) and `c2h`, `c2hx` (harmony pass alone on today's lines); numbers in `lively_stats.json`.
  Trumpet vs Cello: note changes a bar 7.0 today, 6.5 new; exact bar repeats 46% today, 27% new; trumpet moves
  bigger than a fourth 16% today, 13% new; piccolo 48% to 25%; violin 45% to 19%; harsh clashes to zero in all
  three. The harmony pass alone changes between 1 note in 10 and 1 in 70.
- Done when, over generated pairings in a unit test: no semitone, tritone or major-seventh pair; note changes
  per bar are at least 85% of today's for the same seed; exact bar repeats are no more frequent than today's;
  and moves bigger than a fourth are at most 30% of a melody's moves.
- If Aedan does not approve C10: skip items 1 to 4 and run only the harmony pass on today's lines.

### C11. Melody styles: a new tune in each

- Why: Aedan asked for different styles of composing within each theme, and then made clear that each style
  should give the fighters **new melodies**, not the same tune with different filling.
- A **style** is a way of writing the whole four-bar phrase, for both fighters, the bass role included. Tones
  come from the fight's scale and chords inside the fighter's register; seeded choices (which figure, how high)
  make the two fighters and two fights differ. The story shapes (`shape_upper`, `shape_bass`), the variation on
  every other pass and the harmony pass run on top, as for C10. Six styles:
  1. **Signature**: the C10 melody (the fighter's own motif, its answer, the echo, a run into the cadence).
  2. **Climbing**: a short rising figure that starts two scale steps higher each bar; the last bar falls back
     to a chord tone. Bass: a walk up by step from the root.
  3. **Riff**: three or four close notes, the same bar three times on the changing chords, then a changed
     ending. Bass: root, octave and fifth figures.
  4. **Runs**: up the scale and back on every eighth, the last bar running into its chord tone. Bass: the
     scale up one bar, down the next.
  5. **Broken chords**: up and down the notes of the bar's chord. Bass: root, third, fifth.
  6. **Anchor note**: one note keeps returning on the off eighths while a second voice moves by step against
     it. Bass: the root with its octave at the end of the bar.
- The signature motif is used in the Signature style and in the victory solo, in every style, so the fighter's
  own tune is still heard at the win.
- Each theme's Composed arrangement lists the styles that suit it (three to six, by genre family, the same
  grouping as C4's patterns; the builder assigns them). A fight picks one from its theme's list with its music
  seed, the same for both fighters, so a fight still sounds like one piece. The fight file stores the choice.
  The Music panel gets a **Melody style** menu: Auto (default) and the theme's styles by name.
- An existing Composed fight file without the field renders with Signature. Classic has no styles.
- Sketch: `BF_SPIKE_STYLE=climb|riff|runs|arp|pedal` (with `BF_SPIKE_LIVELY=1` and `BF_SPIKE_HARMONY=1`) in
  `spikes2.patch`, script `b9.py`, clips `c11` (Trumpet vs Clarinet, hot jazz), `c11x` (Violin vs Contrabass),
  `c11y` (Flute vs Octobass) with one button per style, numbers in `style_stats.json` and `info.json`.
  Measured on the notes actually played in the clips: between any two styles at least 70% of the trumpet's and
  75% of the clarinet's notes differ, 41% of the violin's, 52% of the flute's, 25% of the contrabass's and 19%
  of the octobass's. Written lines over the whole fight: 58% to 99% for the melody fighters. Harsh clashes
  are zero in all six. The first sketch (`BF_SPIKE_LIVELY=climb|...`, superseded) kept the motif bars and
  differed in only 26% to 57% of the written notes; Aedan heard it as the same melodies.
- The figures in the sketch are a start. The builder may write more figures per style and tune them by ear on
  the stage's listening page, as long as the measures hold.
- Done when, in a unit test over generated pairings: every style has no semitone, tritone or major-seventh
  pair and at most 30% of a melody's moves bigger than a fourth (Broken chords and Anchor note are exempt from
  the last, since leaps are their nature); and for the same fight any two styles differ in at least half of
  the melody fighter's written notes and a fifth of the bass fighter's.

### Bari sax: no slide (part of S1, brought forward to stage 1)

- Aedan's ear: the bari sax "plays a sliding note that sounds like a real-life trombone's 'Wah waahhh'".
- Measured (`tools/round2/b7.py`, pitch tracked through each note of BARI SAX vs TROMBONE and SOPRANO SAX vs
  BARI SAX): nearly everything the bari sax plays in a fight is its swing honk, a note about a second long and
  the loudest thing on its bus. Every one starts 2 to 2.6 semitones sharp and slides down to 1.6 flat, **4.2
  semitones in all**. A real bari sax note moves 0.14 to 0.35 semitones; a real trombone note 0.5; the app's
  own trombone 0.6 on ordinary notes. So the bari sax slides seven times further than the trombone does.
- Where it comes from (`fighters/bari_sax/sound.rs`, `voice.rs`):
  - `doppler(sig, .., up, down, 0.09)` on every swing honk (1.6 up, 2.6 down; 1.0 and 1.6 for SKANK swings) and
    on the octave on top. It was meant as the wrecking ball passing by.
  - `bari_tone`'s scoop, `0.035 + 0.04 * honk`: a honk at 0.85 starts 7% flat, over a semitone.
  - `bend(sig, -1.5, ..)` and `bend(sig, -1.0, ..)` on the crash honks.
- Fix, in Composed only: the swing honk keeps `doppler`'s swell (the loudness rising to the pass) but its pitch
  stays put; the scoop is at most 1.2% (about a fifth of a semitone, like the real instrument); the crash honks
  are not bent and get their weight from growl and level. The falling honk of `fall_apart` (the instrument
  breaking when it loses) stays.
- Sketch: `BF_SPIKE_BARI` in `spikes2.patch`; clips `s1b` (the fight) and `s1bs` (the bari sax alone), Today
  against No slide. Slide per note: median 4.2 semitones today, 0.12 with the fix.
- Done when: no bari sax note outside `fall_apart` moves more than half a semitone from start to end.

### C6. A bass fighter replaces the band's bass

- Trigger: a fighter has the bass role in this fight **and** is a true low instrument, meaning its saved profile
  from M1's calibration run shows at least half of its energy under 200 Hz before M2 (tuba, sousaphone,
  contrabass, octobass and the like). A fighter that may take the bass role but has little bottom (bari sax,
  bass guitar, organ, contrabassoon hold 1 to 21% under 200 Hz) does not trigger it, and neither does a bass
  instrument playing the melody role. Without the trigger the band's bass plays as today.
- With the trigger, the band's own bass layer or layers are silent from FIGHT! to the K.O. and play as usual in
  the intro and after the K.O. Each theme's Composed arrangement names which of its layers are bass. With two
  such fighters, the lower instrument counts.
- Done when: in Violin vs Contrabass the contrabass holds at least 70% of the energy under 200 Hz during the
  fight (30% today).

## Stage 2: the bands

### C4. Every theme gets its own band patterns

- Today all 62 themes share the six `pattern()`s in `bf-sound/src/music/layers.rs`. New: each theme's Composed
  arrangement carries its own drum, bass and chord rhythms in its style (the mambo sketch on the page: bell
  pattern, tumbao bass, piano montuno).
- A little swing where the style swings, and small seeded differences in timing (a few ms) and strength, so the
  band does not sound stamped. Seeded from the fight's music seed so a render is repeatable.
- The story-following behaviour of the layered bed (fills at lead changes, the thin-out for desperation moves,
  everything in for the final exchange, band hits under abilities) stays.
- Work by family, one batch at a time: latin, jazz and swing, rock and pop, orchestral and film, electronic and
  hip-hop, folk and world, marching and the rest. The builder groups the 62 themes.

### M6. A fuller, lighter band

- Done together with C4, theme by theme. The target that counts is the phone one: the band loses at most 5 dB
  on a phone (the mambo loses 8.3 today, 4.0 in the sketch) with its total loudness unchanged.
- The octave balance from 63 Hz to 8 kHz of about 0, -1, -3.5, -5.5, -7.5, -9.5, -11.5, -14 dB (each octave
  against the strongest) is a guide to aim at, not a gate: the sketch reached the phone target while still
  5 dB under that curve at 2 to 4 kHz.
- No layer moves by more than 12 dB against the others. (The sketch moved drums and sax 11.5 dB up against the
  bass once loudness was put back.) Since C4 rewrites the patterns anyway, a theme whose band cannot reach the
  target by levels alone may get a layer moved up an octave or a mid-range layer added.
- A dev command proposes the levels; the builder checks them by measurement.

## Stage 3: the instruments

### S1. Polish every instrument, keep the style

- Each of the 42 voices is compared with a recording of the real instrument (FluidR3, development only) on four
  things: how long a note takes to speak, how strong each overtone is against the strongest, how the note
  ends, and how far its pitch moves during the note. The voice is then reworked toward that fingerprint, with small seeded differences between repeated
  notes. It stays a built voice with the series' character.
- **The fingerprint distance** used below and in tests: for a built note and a reference note at the same
  pitch, the levels of the first eight overtones over the steady part of the note (0.15 to 0.45 s), each in dB
  under the strongest and floored at -50, compared as a root-mean-square difference. Script and numbers:
  `tools/round2/b6.py`, `sf.py`.
- **References.** FluidR3 is a General MIDI set. Where it has the instrument, that is the reference. Where it
  has not, the nearest relative is used and the difference is the builder's judgement: octobass from the
  contrabass, bass and contrabass clarinet from the clarinet's low register, contrabassoon from the bassoon,
  euphonium and sousaphone from the tuba, cornet and flugelhorn from the trumpet (rounder), mellophone from the
  French horn, ukulele from the nylon guitar. Turntables and synth have no real counterpart and only get the
  small differences between repeated notes.
- **Pitch movement.** Sixteen fighters' parts use `gliss`, `bend` or a doppler slide somewhere, and several
  voices scoop into the note. For each, the builder measures the pitch travel of its ordinary notes as in
  `b7.py`. An ordinary note may travel at most half a semitone more than the real instrument's does. Slides
  stay where they are the instrument's trademark (the trombone) and on special moves and losing sounds.
- **Saxes first, bari sax first of all.** The slide is fixed in stage 1 (above). Its tone is next. Measured: the built bari sax has all of its first eight overtones
  within 2 dB of each other, a smooth even spectrum like brass. A real bari sax is uneven: a weak fundamental on
  low notes and strong and weak overtones side by side (for example -20, -2, -2, -1, -14, 0, -11, -21 dB on its
  low E-flat), plus reed buzz and breath. By this measure the built bari sax is as close to a real trombone as
  to a real bari sax, and on one note twice as close (distances 11.1 and 11.1, 12.8 and 10.8, 11.5 and 5.0).
  The rework must make its distance to the real bari sax smaller than to the real trombone on every note of its
  register, and at most 6. The same test is run for each sax against the brass.
- Then, in this order: low brass and the other low instruments (tuba, sousaphone, euphonium, trombone, octobass,
  contrabass, timpani, bass drum), high winds (piccolo, flute, oboe, clarinets, bagpipes), brass (trumpet,
  cornet, flugelhorn, horn, mellophone), bowed strings, then everything else.
- The new voice is used for Composed only. Classic keeps calling the ported voice.
- After each family: re-run the M1 level table for those fighters, switch off M2 or M3 for a voice that now
  meets their targets without them, and publish a listening page (today, new, and the real recording for
  reference).

## Listening pages

Each stage, and each batch in stages 2 and 3, ends with a page in the style of the review page: real fights
rendered by the app from the build branch before the change and from the feature branch after, level-matched,
with a Today and a New button that keep their place. Stage 1's page uses the ten review fights. The build does not
wait for an answer after a page unless Aedan objects to what he hears.

## Tests

Only for what changes, run on the PC through `PC-Tests:` lines: the loudness target, the melody and harmony
measures (for every style), that every pitched fighter follows its line, the bari sax pitch travel, the one-note-at-a-time fade, the bass hand-over, the room's rumble share, the existing kit-parity
tests and the new Classic golden hashes (together they prove Classic is untouched). No full suite.

## Left to the builder

- Module and function names, where the shared measure lives, the exact compressor and limiter settings in F2,
  the reflection times in F3, how the overtones in M2 are made, and how each theme's patterns are stored.
- The grouping of the 62 themes into batches and the order inside a family.
- Whether stages 2 and 3 run one after the other or side by side.
