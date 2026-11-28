# Voice Engine 2: the instrument sheets

One sheet per instrument, written before its family is built (design, "Understanding each instrument"). Each sheet
says how the real instrument works, how its tone changes with strength and pitch, how a note starts, lives and stops,
its noises, how it is played, and how far the recordings of it agree. Every claim names its source: a recording set
(see `voice-engine-2-sources.md`) or a published text (listed at the end of each sheet as [1], [2] ...). Where the
literature and the measurements disagree, the measurements win and the sheet says so.

**How the tables were measured.** `cargo run --release -p bf-harness --example recipes -- study <INST>` measures every
recording of every studied set the same way the yardstick does (`bf-harness/src/e2measure.rs`), and prints the medians
by register (low: under C3 = MIDI 48; middle: C3 to B4; high: C5 and up), set and strength:

- *loudness*: the note's level over its first 0.3 s against the same key's loudest recording in that set, dB;
- *brightness*: the tone's centre of gravity (spectral centroid) as a multiple of the note's pitch, early (0.05 to
  0.5 s) and late (0.8 to 1.2 s); 1.0 is a pure tone, higher is brighter;
- *fall*: how fast the fundamental dies while held, from 0.3 to 2.5 s, dB per second;
- *start*: how long the note takes to reach 90% of its loudest (about 1 dB under it), ms;
- *noise*: the part of the sound that lies between the overtones (the hammer's knock, the wind's breath, a
  recording's room noise), early (0 to 0.1 s) and in the held tone (0.3 to 1 s), %.

The PC-only sets were measured on the owner's PC: the Iowa piano recordings keep the room before each note, so each
was first cut 10 ms before its note begins (the tool's `lead_in`); Lars Palo's 4' and 2' ranks are filed by key and
sound one and two octaves over it, so their pitches were moved up (the tool now does both).

---

## Piano (fighter PIANO)

**Plays (since stage 1b):** the struck-string engine, Aedan's pick P16, built from the working parts below and checked
against the four real pianos (`bf-sound/src/voice/engine2/strings.rs` and `board.rs`; "The built engine" below).
**Studied:** VCSL Steinway B, Iowa Steinway B (PC), Salamander Yamaha C5, Headroom Yamaha C3. **Kept, played by no
fighter:** stage 1's recipe voice from the VCSL Steinway B (CC0), strengths vl2, vl3, vl4 (`engine2/piano.rs`, recipe
`bf-content/data/voices/PIANO.ve2`, `RECIPE` in the fighter's voice).

### Working parts

- **Key, hammer and felt.** The key throws a felt-covered hammer at the strings; it flies free for the last few
  millimetres and bounces off [3][4]. The felt is a *hardening spring*: the harder it is squeezed, the stiffer it gets
  (force rising roughly as the compression to a power between 2 and 4) [3]. So a harder blow is not only louder: the
  hammer stays on the string for less time and excites more high overtones, and the tone gets brighter [3][4]. Contact
  lasts a few milliseconds in the bass and under a millisecond at the top [4].
- **Strings.** One string per key in the low bass, two in the upper bass, three from the tenor up [1][2]. The unison
  strings are tuned a hair apart and pass energy to each other through the bridge, so a note first dies quickly (the
  *prompt sound*, the strings moving together) and then slowly (the *aftersound*, the strings out of step) - the
  two-stage decay [1][2]. The same coupling makes single overtones beat, swelling and fading by tens of dB while the
  sum of the note dies smoothly (measured: see "Where the literature and the measurements disagree").
- **Stiffness.** The steel's stiffness makes every overtone a little sharp of a true harmonic, more so the higher the
  overtone: overtone *n* sits at *n* × pitch × sqrt(1 + *B* n²) [1][9]. Measured from the VCSL Steinway B (the recipe
  stores *B* per note): about 2.4e-4 at C1, lowest at C3 (1.2e-4), 3.3e-4 at C4, 1.3e-3 at C5 and 2-3e-3 at C6,
  which is the shape the literature gives (smallest in the tenor, rising steeply in the treble) [9].
- **Soundboard and case.** The bridge passes the strings' motion to the soundboard, which does most of the radiating
  and lends the low notes their body [9].
- **Dampers.** Felt pads that rest on the strings and lift while the key is down. When the key comes up the damper
  lands and the note dies in a fraction of a second. On a grand the top 18 or so keys (from about G6) have no dampers:
  their strings are short and die by themselves [7][8].
- **Pedals.** The *sustain* pedal lifts every damper: the note rings on past the key, and the other strings, all
  free, ring along softly with the note's overtones (sympathetic resonance) [8][9]. The *soft* pedal (una corda)
  shifts the action so the hammers strike one string fewer, with a less worn part of the felt: softer and darker [9].

### How the tone changes with strength and pitch

Measured:

| register | set | strength | loudness dB | bright early | bright late | fall dB/s | start ms | noise early % | noise held % |
|---|---|---|---|---|---|---|---|---|---|
| low | headroom | 0.20 | -17.4 | 3.61 | 3.29 | -3.2 | 30 | 3.2 | 0.1 |
| low | headroom | 0.40 | -9.9 | 4.04 | 3.69 | -3.1 | 40 | 5.0 | 0.2 |
| low | headroom | 0.60 | -7.1 | 4.59 | 3.99 | -3.0 | 30 | 6.7 | 0.5 |
| low | headroom | 0.80 | -3.0 | 5.13 | 4.33 | -2.8 | 30 | 7.9 | 1.0 |
| low | headroom | 1.00 | 0.0 | 5.54 | 4.74 | -2.7 | 30 | 10.5 | 1.7 |
| low | salamander | 0.25 | -9.7 | 2.81 | 2.67 | -1.7 | 28 | 3.7 | 0.1 |
| low | salamander | 0.50 | -6.1 | 3.03 | 2.77 | -2.6 | 38 | 3.9 | 0.1 |
| low | salamander | 0.75 | -3.9 | 3.21 | 2.98 | -2.5 | 30 | 4.8 | 0.1 |
| low | salamander | 1.00 | 0.0 | 3.77 | 3.21 | -2.3 | 38 | 4.8 | 0.3 |
| low | iowa-piano | 0.30 pp | -26.3 | 4.74 | 5.01 | -3.1 | 220 | 47.4 | 0.1 |
| low | iowa-piano | 0.60 mf | -9.7 | 6.89 | 7.45 | -3.0 | 25 | 7.3 | 0.1 |
| low | iowa-piano | 0.90 ff | 0.0 | 9.50 | 9.86 | -3.2 | 20 | 8.6 | 0.9 |
| low | vcsl | 0.45 (vl2) | -0.6 | 3.29 | 3.50 | -2.8 | 32 | 2.9 | 0.1 |
| low | vcsl | 0.70 (vl3) | -0.4 | 5.64 | 5.50 | -3.7 | 20 | 3.1 | 0.2 |
| low | vcsl | 0.95 (vl4) | 0.0 | 6.48 | 5.79 | -3.5 | 18 | 4.5 | 0.8 |
| middle | headroom | 0.20 | -17.5 | 1.63 | 1.42 | -12.6 | 18 | 3.0 | 0.1 |
| middle | headroom | 0.40 | -11.2 | 1.93 | 1.82 | -13.0 | 15 | 3.7 | 0.0 |
| middle | headroom | 0.60 | -7.5 | 2.04 | 2.01 | -13.2 | 15 | 4.2 | 0.1 |
| middle | headroom | 0.80 | -3.7 | 2.09 | 2.12 | -13.2 | 10 | 3.9 | 0.1 |
| middle | headroom | 1.00 | 0.0 | 2.11 | 2.13 | -13.3 | 28 | 4.9 | 0.1 |
| middle | salamander | 0.25 | -12.5 | 1.52 | 1.60 | -10.4 | 15 | 4.6 | 0.1 |
| middle | salamander | 0.50 | -7.7 | 1.78 | 1.65 | -10.8 | 15 | 4.5 | 0.1 |
| middle | salamander | 0.75 | -4.8 | 1.87 | 2.22 | -11.1 | 15 | 4.4 | 0.1 |
| middle | salamander | 1.00 | 0.0 | 1.94 | 2.26 | -11.3 | 20 | 5.5 | 0.0 |
| middle | iowa-piano | 0.30 pp | -25.0 | 1.43 | 1.70 | -9.4 | 70 | 52.3 | 0.2 |
| middle | iowa-piano | 0.60 mf | -10.7 | 2.30 | 2.29 | -9.4 | 20 | 7.0 | 0.1 |
| middle | iowa-piano | 0.90 ff | 0.0 | 2.48 | 2.87 | -11.9 | 20 | 6.3 | 0.1 |
| middle | vcsl | 0.45 (vl2) | -1.0 | 1.25 | 1.21 | -9.5 | 15 | 5.1 | 0.3 |
| middle | vcsl | 0.70 (vl3) | -0.4 | 1.76 | 1.63 | -11.1 | 15 | 4.7 | 0.1 |
| middle | vcsl | 0.95 (vl4) | 0.0 | 1.84 | 1.85 | -10.6 | 15 | 4.6 | 0.1 |
| high | headroom | 0.20 | -18.3 | 0.95 | 0.31 | -12.7 | 5 | 21.8 | 5.1 |
| high | headroom | 0.40 | -12.4 | 1.01 | 0.77 | -15.1 | 5 | 23.3 | 3.4 |
| high | headroom | 0.60 | -8.1 | 1.01 | 0.90 | -15.8 | 5 | 22.0 | 3.5 |
| high | headroom | 0.80 | -5.0 | 1.02 | 0.96 | -16.8 | 5 | 21.4 | 3.3 |
| high | headroom | 1.00 | 0.0 | 1.03 | 0.98 | -15.9 | 5 | 19.0 | 3.2 |
| high | salamander | 0.25 | -15.8 | 0.97 | 0.81 | -15.2 | 10 | 17.2 | 6.7 |
| high | salamander | 0.50 | -11.1 | 0.99 | 0.84 | -15.0 | 10 | 15.0 | 2.8 |
| high | salamander | 0.75 | -6.9 | 1.01 | 1.00 | -15.8 | 10 | 18.1 | 3.5 |
| high | salamander | 1.00 | 0.0 | 1.03 | 1.00 | -14.7 | 10 | 12.8 | 2.8 |
| high | iowa-piano | 0.30 pp | -29.2 | 0.90 | 0.90 | -9.4 | 65 | 100.0 | 11.2 |
| high | iowa-piano | 0.60 mf | -13.3 | 1.00 | 1.00 | -12.3 | 15 | 15.5 | 0.9 |
| high | iowa-piano | 0.90 ff | 0.0 | 1.00 | 1.00 | -14.4 | 10 | 13.6 | 0.8 |
| high | vcsl | 0.45 (vl2) | -0.3 | 0.97 | 0.25 | -18.1 | 10 | 22.8 | 2.8 |
| high | vcsl | 0.70 (vl3) | -0.7 | 1.00 | 0.95 | -17.8 | 5 | 18.3 | 2.2 |
| high | vcsl | 0.95 (vl4) | -0.2 | 1.00 | 0.93 | -19.1 | 5 | 22.5 | 3.0 |

(Notes per cell: VCSL 14/12/16 by register, Salamander 10/7/13, Headroom 9/8/13, Iowa 44, 42 and 44 recordings in
all for pp, mf and ff. A late brightness under 1.0 in the high register means the note has died into the recording's
noise floor by 1 s. Iowa's pp start and noise are the room: its soft notes, the high ones most, are only 15 to 45 dB
over the room's noise.)

What every piano agrees on:

- **Harder is brighter and louder, in every register** (every set, every row) [3][4]. From the softest to the hardest
  recording a piano gets 10 to 29 dB louder (Iowa's pianissimo 25-29 dB, Headroom 17-18 dB, Salamander 10-16 dB:
  how soft each player's softest was) and its early tone's centre rises by 20 to 100% in the low and middle
  registers. At the top the tone is nearly a pure fundamental at any
  strength (bright early about 1.0).
- **Low notes are darker in proportion but far richer**: a bass note's tone centre sits 3 to 6 overtones up, a middle
  note's under 2.
- **Higher dies faster** (every set): about 2-4 dB/s held in the bass, 9-13 dB/s in the middle, 9-19 dB/s at the
  top (Iowa's Steinway the slowest at the top) [2][5].
- **The attack is quick and quicker at the top**: at 90% of its loudest in 20-40 ms in the bass, 10-25 ms in the
  middle and 5-15 ms at the top (Iowa's pp aside, which starts in its room's noise).
- **The hammer's knock is a bigger share of the top notes**: the noise is 3-9% of an early bass or middle note and
  13-23% of a top note [4] (Iowa's softer notes again aside).

### Start, life and stop

- Start: the hammer's knock and the strings' first, brightest moment; the high overtones die first (the late tone is
  darker than the early one in the low register of every set).
- Life: the two-stage decay with beating overtones [1][2]; the recipe carries each overtone's own level over time,
  so the beating of the real strings is kept as measured.
- Stop: with the damper down, the note dies with a time constant measured from VCSL's `Rel` recordings (below);
  undamped top keys and pedalled notes ring on.

### Noises

The hammer's knock against the string and the key hitting the keybed (touch noise) [4]; at the top this knock is a
large part of the sound (table). The damper's soft thud at key-up (in the `Rel` recordings; not modelled). VCSL's
soft layers carry a raised steady hiss from being evened out in level: the recipe drops the steady part (Task 2's
`dehiss`), which none of the other pianos have (noise held 0.0-0.3% in the middle register for Headroom and
Salamander).

### How it is played

Struck notes of any length; chords; the sustain pedal for legato and washes; accents strike harder. No vibrato, no
slides (a piano cannot bend). A repeated key re-strikes a string that is still ringing.

Every technique, as the built engine plays it (asked for as in `voice-engine-2-playing.md`): strength and accent;
staccato to legato by the key's length; the sustain pedal, a half pedal and the pedal coming up at a set moment; the
soft pedal (one string fewer, softer felt); the middle pedal (this note held, the others quiet); a hand damping the
strings; a choked stop; harmonics (the string touched at 1/2 to 1/6 as it is struck); a string plucked inside the
piano. Trills, runs, repeated notes, glissandos and clusters are made of single notes by the composition system.

### The built engine (stage 1b, P16)

Each key's strings are modelled from their parts and run one sample at a time, at any sample rate:

- **Strings.** Each string is a set of stiff-string modes (overtone *n* at *n* × pitch × sqrt(1 + *B* n²)), one to
  three strings a key as on a grand, tuned a hair apart (0.6 cents, wider at the top) so they beat and pass energy
  through the bridge (the two-stage decay comes out of that coupling, not out of a table). The tension glides a hair
  flat as a loud note settles.
- **Hammer.** A felt hammer as a hardening spring (force rising as the squeeze to a power), with its stiffness set
  per key so its contact time matches the register's; a faster hammer squeezes harder, leaves sooner and is brighter.
  Struck at about an eighth of the string.
- **Board.** The strings' push on the bridge goes out straight and through 1400 small resonances of the board (the
  middle of the sketch's two microphones), then the board's average response (a little less low end and upper
  middle, a little more top).
- **Dampers** land over 20 ms when the key comes up; the top keys from F6 have none. Under the sustain pedal the
  strings within two octaves ring along softly (about 26 dB under), driven by the bridge.
- **Room.** The fighter's own small room (2.4 s, fading 60 dB in 1.3 s) and width are added at the bus.

Checked by `bf-harness/tests/engine2_built.rs`: the listening riff's octave bands lie inside the four real pianos'
(within 1.5 dB), the high notes keep their energy over 4 kHz as low as P16's (the "very synthesized" P12 had 17 dB
more there), and repeats differ. Each technique has its own check in `strings.rs`.

### The consensus carried into the recipe

- **The strength steps.** VCSL evened its three strengths out to one loudness (its loudness column reads 0 to -1 dB),
  so the shipped piano's loudness steps are taken from what the other pianos agree on, matched by brightness: each
  VCSL strength's early brightness against its hardest is matched to the strength of each other piano that is as
  bright, and that piano's loudness there is read off (`recipes measure`, `piano_gains`). vl2 is darker than
  Headroom's and Salamander's softest (0.68 of vl4's brightness in the middle register against their 0.77 and 0.78),
  so it takes their softest loudness there; Iowa's pianissimo is darker still (0.58), so vl2 falls between Iowa's pp
  and mf. The median over the four sets and the low and middle registers:

  | VCSL strength | played at | brightness against vl4 (low / middle) | loudness under vl4 |
  |---|---|---|---|
  | vl2 | 0.45 | 0.51 / 0.68 | -17.4 dB |
  | vl3 | 0.70 | 0.87 / 0.96 | -5.3 dB |
  | vl4 | 0.95 | 1 / 1 | 0 dB |

- **The brightness step** is VCSL's own (its recordings were evened in level, not in tone).
- **The damper's time constant by register**, from VCSL's `Rel` recordings (the broadband fall from 3 to 20 dB under
  the peak; the recipe stores it per note, the same for the three strengths), in seconds (the note falls 8.7 dB in
  each):

  | register | keys | time constant, median (range) |
  |---|---|---|
  | low | A#0 to A#2 | 0.097 s (0.059-0.133) |
  | middle | C3 to A#4 | 0.080 s (0.056-0.146) |
  | high, damped | C5 to F#6 | 0.13 s (0.064-0.275) |
  | top, undamped | from G6 | no damper (the recipe's 0.162 s is unused) |

  None of the other studied sets has key-up recordings that the study reads (Salamander's `rel` files were not
  studied), so the damper cannot be checked against them; it is checked against its own recording in
  the yardstick test (the engine's stop time against VCSL's).

### Where the literature and the measurements disagree

- **Single overtones.** The literature describes a smooth two-stage decay [1][2]; measured, a single overtone of a
  real note beats by up to 33 dB within 2.5 s while the whole note dies smoothly. The yardstick therefore compares
  single overtones only in a note's first 0.3 s and the overtones' median after that (the recipe voice's yardstick,
retired with it in stage 1b; the built engine's beating comes from its unison strings).
- **The soft layer's darkness.** VCSL's softest strength is darker than any other piano's softest; it is kept as
  recorded (treated as pianissimo), since the other pianos' softest recordings are not the softest a piano can play.

### Sources

1. G. Weinreich, "Coupled piano strings" (J. Acoust. Soc. Am. 62, 1977), and his lecture
   https://www.speech.kth.se/music/5_lectures/weinreic/
2. J. O. Smith, "Coupled strings", *Physical Audio Signal Processing*, https://ccrma.stanford.edu/~jos/CoupledStrings/
3. D. Russell, "The piano hammer as a nonlinear spring" and "Piano dynamics",
   https://www.acs.psu.edu/drussell/piano/nonlinearhammer.html and https://www.acs.psu.edu/drussell/piano/Dynamics.html
4. A. Askenfelt (ed.), *Five lectures on the acoustics of the piano* (KTH), https://www.speech.kth.se/music/5_lectures/askenflt/
5. K. Wogram, "Decay of the piano tone", in [4]: https://www.speech.kth.se/music/5_lectures/wogram/decay.html
6. (reserved)
7. Mark Goodwin Pianos, "Why the top of a piano has no dampers", https://www.markgoodwinpianos.co.uk/yamaha/top-register-no-dampers
8. Roland support, piano modelling notes on damper and string resonance, https://support.roland.com
9. N. H. Fletcher and T. D. Rossing, *The Physics of Musical Instruments* (2nd ed., Springer 1998), chapter 12.

---

## Organ (fighter ORGAN)

**Plays (since stage 1b):** the pipe engine, Aedan's pick O8: a grand chorus built pipe by pipe from the working parts
below (`bf-sound/src/voice/engine2/pipes.rs`; "The built engine" below). **Studied:** VSCO 2 CE, VCSL Renaissance
organ (Full), Lars Palo's Bureå and Piteå organs' principals, one rank at a time (PC). **Kept, played by no fighter:**
stage 1's recipe voice from the VSCO 2 CE pipe organ (CC0), main chorus, quiet and loud (`engine2/organ.rs`, recipe
`bf-content/data/voices/ORGAN.ve2`, `RECIPE` in the fighter's voice).

### Working parts

- **Wind.** A blower fills a reservoir that holds the wind at a steady low pressure; the wind runs through ducts to
  the windchests under the pipes [10][13]. When many pipes speak at once the pressure dips a little, so a big chord
  sags slightly in pitch and level, and the wind's small unsteadiness gives every note a faint, slow waver [13].
- **Pallet and pipe.** Pressing a key opens a valve (pallet) that lets wind into the pipes of the drawn stops [10].
  A *flue* pipe (the principals of the main chorus) speaks like a whistle: a sheet of air from the pipe's mouth
  strikes the upper lip and is driven back and forth by the air column, which sets the pitch [10][11].
- **Stops and the chorus.** Each stop is a rank of pipes, one per key, at its own octave: 16' an octave under the
  written note, 8' at it, 4' an octave over, 2 2/3', 2' and mixtures higher still [10]. The main chorus is the
  principals at several octaves sounding together, and the ranks are never exactly in tune with each other, which
  gives the chorus its shimmer [10].
- **How loud.** A pipe sounds the same however hard the key is pressed: the organist makes it louder by drawing more
  stops (and with the swell box, not modelled) [10]. So the organ's "strength" is the setting: quiet (fewer stops) to
  loud (the full chorus with the 16'), and a crescendo is stops coming in.

### How the tone changes with setting and pitch

Measured:

| register | set | setting | loudness dB | bright early | bright late | fall dB/s | start ms | noise early % | noise held % |
|---|---|---|---|---|---|---|---|---|---|
| low | vcsl (Renaissance, Full) | 1.00 | 0.0 | 4.54 | 4.51 | -0.3 | 130 | 9.5 | 0.1 |
| low | vsco | 0.50 quiet | -14.8 | 3.52 | 2.99 | 0.5 | 540 | 22.6 | 0.4 |
| low | vsco | 1.00 loud | 0.0 | 7.26 | 6.73 | 1.2 | 490 | 20.1 | 3.0 |
| middle | vcsl (Renaissance, Full) | 1.00 | 0.0 | 5.16 | 5.21 | -0.0 | 45 | 2.8 | 0.4 |
| middle | vsco | 0.50 quiet | -5.1 | 1.19 | 1.18 | 1.0 | 380 | 5.9 | 0.3 |
| middle | vsco | 1.00 loud | 0.0 | 3.95 | 3.61 | 0.8 | 895 | 5.4 | 0.8 |
| high | vcsl (Renaissance, Full) | 1.00 | 0.0 | 7.03 | 6.75 | 3.0 | 75 | 9.9 | 4.4 |
| high | vsco | 0.50 quiet | 0.0 | 1.00 | 1.01 | 0.3 | 295 | 9.1 | 0.3 |
| high | vsco | 1.00 loud | -3.3 | 1.70 | 1.71 | -0.1 | 605 | 12.3 | 1.6 |
| low | palo-burea | principal | 0.0 | 2.02 | 1.50 | -0.9 | 445 | 19.0 | 0.1 |
| low | palo-pitea | principal | 0.0 | 1.71 | 1.51 | -3.1 | 320 | 5.1 | 0.0 |
| middle | palo-burea | principal | -4.7 | 1.72 | 1.66 | -0.0 | 170 | 3.3 | 0.0 |
| middle | palo-pitea | principal | 0.0 | 1.08 | 1.09 | 0.6 | 150 | 2.5 | 0.0 |
| high | palo-burea | principal | -3.8 | 1.23 | 1.21 | 0.3 | 155 | 2.3 | 0.1 |
| high | palo-pitea | principal | 0.0 | 1.05 | 1.03 | -0.0 | 125 | 3.4 | 0.1 |

(Notes per cell: VCSL 12/12/3, VSCO 13/11/8; Bureå 239 and Piteå 146 recordings in all, its 8', 4' and 2'
principals and Bureå's pedal 8' one rank at a time, so their loudness compares ranks, not settings. Brightness is
against the written note's pitch; VSCO's loud chorus has a 16' an octave under it.)

What the organs agree on:

- **A held note does not die** (fall within ±1 dB/s in the middle and high registers of every set; Piteå's low
  principal's -3 dB/s and the VCSL top's 3 dB/s rise are a few notes only) [10].
- **The tone does not change while held** (early and late brightness within 10% in every row) [10].
- **The loud setting is much brighter than the quiet one** (VSCO 1.7-3.3 times), and the full chorus of the
  Renaissance organ is bright at every pitch: more stops, more high ranks [10]. One principal rank on its own is
  mellow (Palo's 1.05-2.0 times the pitch), close to VSCO's quiet setting.

### Start, life and stop

- Start: a flue pipe takes some tens of milliseconds to speak, longer for big low pipes; its overtones come in at
  different times, and many pipes make a short breathy *chiff* as they start [11][12]. How quickly the wind reaches
  the pipe also shapes the start [11].
- Life: steady, with the wind's waver and the chorus's shimmer.
- Stop: when the pallet closes the pipes stop within a few hundredths of a second; what is heard after is the
  church's echo [10]. The engine stops a note with a time constant of 0.03 s (top) to 0.12 s (bottom)
  (`OrganParts.release`).

### Noises

The wind's breath in the held tone (VSCO 0.3-3%, Renaissance 0.1-4.4%), the chiff at the start [12], and the action's
clatter (not modelled). The recipe keeps the organ's wind noise (no `dehiss`): it is part of its sound.

### How it is played

Held notes and chords, legato from key to key, no accents by touch (an accent is a wider chord or a new stop). A
crescendo brings stops in: the fighter's CATHEDRAL, PULL ALL STOPS and title blast swell from part way through the
stops to the full set (Aedan's "main chorus, building up").

Every technique, as the built engine plays it (asked for as in `voice-engine-2-playing.md`): any registration of its
fourteen manual and four pedal stops (nine ready sets, from a soft stopped flute to the full organ with its reeds);
sub and octave couplers; a crescendo bringing the stops in; the swell box (a volume pedal, darker shut); the
tremulant; an accent that speaks quicker; the shortest stop. The wind sags a hair under a big chord.

### The built engine (stage 1b, O8)

Each note sounds every pipe its stops draw, added up from their overtones, in short blocks at any sample rate:

- **Pipes.** Each kind has its own overtones: principals, stopped flutes (odd overtones), chimney flutes, strings
  (bright and thin), and the reeds (trumpet, trombone, oboe). The mixture and cymbal break back an octave above
  about 2.7 kHz, as real mixtures do. Every pipe is a hair out of tune with the next (about 1.3 cents, reeds 1.6), the
  celeste 6 cents sharp so it beats; a unison's pipes start near each other's phase, so they add.
- **Speech.** A flue pipe speaks in tens of milliseconds, flicking up through its octave first, with a puff of chiff;
  a reed starts a hair sharp. The wind's breath is band-passed per pipe. A flue pipe stops in 12 ms plus four of its
  own periods, a reed in 12 ms.
- **Wind.** It wavers slowly (0.35 cents, 0.2 dB, 1.5 a second) and sags under chords of three or more.
- **Polish.** A gentle tone lift per note (a little warmth at 110 and 250 Hz, presence at 2.8 kHz, air at 9 kHz).
- **Room.** The fighter's own room (3.3 s, fading 60 dB in 1.8 s, kept light as Aedan asked) and width are added at
  the bus.

Checked by `bf-harness/tests/engine2_built.rs`: the listening riff keeps O8's octave balance (within 2 dB a band),
and repeats differ. Each technique has its own check in `pipes.rs`.

### Where the literature and the measurements disagree

- **The start.** The literature gives a pipe's speech as tens of milliseconds [11][12]. The recordings take longer
  to reach 90% of their full level, and the longer the bigger the room and the farther the microphones: VCSL's small
  Renaissance organ 45-130 ms, Palo's two church organs 125-445 ms, VSCO's 0.3-0.9 s. What builds up after the pipe
  has spoken is the room. The engine takes the quick end of the spread: it lifts the VSCO recipe to full level within
  0.06 s (0.12 s two octaves under C4, `OrganParts.speak`) and adds a light room of its own, so the fights' short
  stabs are not late and weak; this also follows Aedan's "the organ probably shouldn't have so much of its own
  echo".
- **The yardstick** compares the organ's held tone (power over 0.3, 1 and 2.5 s) rather than single moments: the
  chorus's ranks beat against each other by 20 dB and more at any moment, as the shimmer the literature describes
  [10].

### Sources

10. N. H. Fletcher and T. D. Rossing, *The Physics of Musical Instruments* (2nd ed., Springer 1998), chapters 16 (flue
    organ pipes) and 17 (the pipe organ).
11. M. Castellengo, "Acoustical analysis of initial transients in flute-like instruments" (Acta Acustica 85, 1999),
    and her 2004 ISMA paper on the pressure rise time in organ pipes,
    http://www.lam.jussieu.fr/Membres/Castellengo/publications/
12. Organ pipe chiff analysis, University of Bremen (2007), https://www.ant.uni-bremen.de
13. US patent 5,508,472, wind supply effects (pitch and level of organ tones with the wind), https://image-ppubs.uspto.gov

---

## The plucked strings (fighters ACOUSTIC, ELECTRIC, BASS GUITAR, UKULELE, HARP)

One engine for the family (`bf-sound/src/voice/engine2/plucked.rs`), built from how a plucked string works and tuned
by ear in two listening rounds (sketch `pl.py`, `/mnt/project-files/ballfight-studio/voice-engine-2/pluck1/` and
`pluck2/`). Aedan's picks (2026-11-27): A6, E2, B3, U6, H3. Round 1 found A1 and U1 "too dull"; round 2 brightened
them (newer strings, the body heard a little closer, a light polish).

### Working parts

- **The string.** A stiff string's overtones (overtone *n* at *n* × pitch × sqrt(1 + *B* n²)), each ringing two ways:
  across the top, where the bridge moves most readily, so the body drains it sooner; and along it, a hair sharp
  (0.2 to 0.6 per thousand), a quieter share (a quarter to a half of 0.35), ringing longer. The two ways together give
  a plucked note's quick first fall and slower ring.
- **The pluck.** The string pulled aside at a point and let go: the point sets which overtones ring (one at the middle
  leaves out every second), the width of the finger, nail or pick sets how far up they reach, and a harder pluck
  leaves the finger sooner. A pick's quick release also lifts the top and clicks. A fretted note is stopped shorter,
  so the pluck point and the finger are a larger part of what sounds (higher up the neck: rounder); a harp's strings
  shorten with pitch.
- **Losses.** The air and the string's own damping (the high overtones far sooner: steel new and bright, nylon and
  gut softer), and the body's drain where the body is most open. When a note is let go, the fretting finger lifts
  over 15 to 50 ms and damps the string (a harp's barely: harpists let notes ring).
- **The body** (`body.rs`) for the acoustic, the ukulele and the harp: a few strong low resonances (the air in the
  box, the top's first ways of bending), many small ones above, and the force going straight out; then the average
  response fitted to a target over thirds of an octave, keeping each body's own peaks and dips.
- **The pickups and the amp** (`amp.rs`) for the electric and the bass: the pickups hear the string where they sit
  (by the bridge: thin and bright), their coil adds a soft resonant top, and the amp's gain stages round off the peaks
  (each a valve's slightly lopsided clip), then a tone stack, the power stage and the speaker cabinet. The amp plays
  on the fighter's whole part, so a chord's notes beat into each other and distort as one.

### The picks

| Code | Fighter | The string | Played | Through |
|---|---|---|---|---|
| A6 | ACOUSTIC | new steel strings (rings 8 s at the low E) | fingers at 0.16 of the string | a dreadnought's box, heard close; a polish: a little less boom and box, more presence and air |
| E2 | ELECTRIC | steel, long sustain (12 s) | a pick at 0.12, bridge pickup | a high-gain amp: tight lows, a scooped middle, a 4x12 cabinet |
| B3 | BASS GUITAR | roundwound bass strings (16 s) | a pick at 0.14, one pickup at 0.21 | a bass amp pushed for a little grit |
| U6 | UKULELE | new fluorocarbon (4 s at middle C) | fingers at 0.22, re-entrant G C E A | a soprano's small box, heard close; a light polish |
| H3 | HARP | gut (9 s at E3, shorter up high) | fingers at a third of each string | a concert pedal harp's long soundboard and deep box, a soft top |

### How it is played

Every technique, as the engine plays it (asked for as in `voice-engine-2-playing.md`): strength and accent; let ring
or staccato; fingers, pick, nails or thumb; the pluck point (by the bridge to over the neck); which string; slap and
pop (the bass); tapping; hammer-ons and pull-offs; slides; bends, scoops and falls; vibrato; harmonics; pinch
harmonics (the electric); palm mute (a harp's etouffe); dead notes; the shortest note. Strums, rolled chords, tremolo
picking, trills, glissandos and bisbigliando are the composition system's, made of single notes.

### The built engine (stage 2)

- Notes run in blocks of 32 samples at any sample rate: each overtone a turning phasor, its pitch and loss set once a
  block (a bend, a slide, a vibrato, the release land smoothly), dropped once it is 140 dB down.
- The body is the middle of the sketch's two microphones' bodies, fitted (the fighter's width spreads it again); its
  level is the same at any sample rate.
- The amp's drive is set from the part's own loudness against the level the listening riffs drove it at, so it
  distorts the same however loud the part is mixed, and comes out as loud as it went in.
- Each fighter's room is the sketch's: the guitars', the bass's and the ukulele's small (2.4 s), the harp's light
  (3.3 s); the electric and the bass sit in the middle (their amps are one speaker).

Checked against the sketch: single notes within 0.6 dB in every octave (correlation 0.99 or more), the amp exactly,
each fighter's riff of the listening rounds within 2 dB in every octave (`bf-harness/tests/engine2_built.rs`). Each
technique has its own check in `plucked.rs`, the bodies in `body.rs`, the amps in `amp.rs`.

### Licences

Nothing recorded ships. The harp and bass recordings looked at while sketching (VSCO 2 CE harp, Karoryfer
growlybass, both CC0) were yardsticks only.

---

## The bowed strings (fighters VIOLIN, VIOLA, CELLO, CONTRABASS, OCTOBASS)

One engine for the family (`bf-sound/src/voice/engine2/bowed.rs`), built from how a bowed string works and tuned by
ear in one listening round and five rounds of fixes (sketch `bw.rs` with `bowed.py`,
`/mnt/project-files/ballfight-studio/voice-engine-2/bow1/`). Aedan's picks (2026-11-27): Vn3, Va3, Vc1, Cb1, Oc2;
the sketch approved at 06:49Z after fix 5. The fixes, all carried into the engine:

1. The free ring: a note rings on after the bow leaves, until the finger lifts (a stopped string) or it dies away (an
   open one).
2. (A wobbling bowing point sounded "broken": undone.)
3. The "synth layer following the notes": the twist's own resonances sat on whole overtones of every note, so the
   same overtones dropped out of every note. Each instrument's twist speed now puts them between the overtones (the
   pattern under 2 dB, as a real violin's 1.2 dB).
4. The squeaks in short and accented notes, three causes measured from the sound: a martelé's weight left on the
   string at rest pinned it like a fret (a jump of 200 cents), now let off with the speed; twist speeds that bowed
   the upper notes 40 cents flat for the ear to scoop up, now within 5 cents; and accents that took 80 ms to settle,
   now speed and weight together, a strong accent quicker (25 to 35 ms).
5. The squeak left in the violin's spiccato run and the viola's first accents: an open string is bowed 15% heavier
   (no finger damps it, so it slipped twice a period), the octobass excepted (it creaks); the viola's accents lighter
   and shorter.

### Working parts

- **The string.** Two delay lines meeting at the bow, one to the bridge and one to the stopping finger or the nut;
  each end a reflection that loses a little (a gain and a gentle low-pass, set so the string rings its time at the
  fundamental and at 4 kHz). A finger stops a string with more loss than the nut does.
- **The bow.** It holds the string by friction: while the string moves with the bow it sticks; once the force that
  would take passes what the rosin can hold, it slips, the friction falling as it slips faster (hyperbolic friction),
  until it can stick again. That stick and slip, once a period, is the Helmholtz motion: a corner running round the
  string, a sawtooth push on the bridge. The bow's speed, its weight and where it sits (0.07 to 0.12 of the string
  from the bridge) set the tone: nearer the bridge and heavier is brighter and grittier, further out sweeter.
- **The twist.** The bow turns the string's surface, so it twists; those waves run 6 to 9 times quicker and die fast,
  and they soak up the ripples that would make the bow slip twice a period.
- **The player.** The ear hears the time between slips and moves the finger a hair to bring the note to pitch; the
  rosin makes the force jitter; the vibrato rocks the finger, coming in after the start; the bow's speed and weight
  wander a little.
- **The body** (`body.rs`): each instrument's air and wood resonances and its bridge hill, the strong ones measured
  from free recordings, fitted over thirds of an octave; the plucks lose energy to it where it moves most readily.

### The picks

| Code | Fighter | The string | Bowed | Through |
|---|---|---|---|---|
| Vn3 | VIOLIN | rings 3 s (open), 0.5 s free after the bow | further from the bridge (0.12), a lighter bow, a wide slow vibrato (24 cents, 5 Hz) | a warm violin body: sweeter and darker |
| Va3 | VIOLA | rings 3 s, 0.6 s free | as the violin, a wide vibrato (22 cents) | a large viola, its body resonating lower: darker and fuller, the low C filled in |
| Vc1 | CELLO | rings 4 s, 0.8 s free | the usual spot (0.09), vibrato 16 cents at 5.2 Hz | a concert cello |
| Cb1 | CONTRABASS | steel, rings 6 s, 1.2 s free | the usual spot (0.10) | an orchestral bass: a full, even bowed tone and a ringing pluck |
| Oc2 | OCTOBASS | rings 8 s, 1.5 s free | nearer the bridge (0.07), a firmer bow | the octobass's body with more low-middle: more grind, which carries on small speakers |

### How it is played

Every technique, as the engine plays it (asked for as in `voice-engine-2-playing.md`): strength (bow speed and
weight together); accents; détaché (a plain bowed note); legato slurs and ties (one bow, no new start); portamento;
martelé; spiccato; tremolo; col legno battuto and tratto; sul ponticello to sul tasto (the bowing point); natural
harmonics; con sordino; pizzicato, the snap (Bartók) pizzicato and the left-hand pizzicato; vibrato, its depth, rate
and wait, or none; swells (crescendo, diminuendo) and the bow's expression over a note; choked and clipped notes; which string; the basses'
slap. A plain contrabass or octobass note under half a second is plucked, longer ones bowed, as the walking line was
written. Double stops are two notes at once (two `Play`s). Trills, runs, glissandos made of notes, sautillé and ricochet
runs are the composition system's, made of single notes.

### The built engine (stage 3)

- The string runs at a rate of its own (192 kHz for the violin and the viola, 96 kHz for the cello, the heard rate for
  the basses; as near as a whole multiple of the sample rate goes) and is brought down by a windowed-sinc decimator;
  at 44.1 kHz and 48 kHz alike every note is in tune within 5 cents.
- Plucked and struck notes (pizzicato, snap, left-hand, slap, col legno battuto) are the same string as its own modes,
  pulled aside or struck and let go.
- Each repeat of a note is bowed a hair differently (three takes in turn), as a player never bows a note twice alike.
- Once the bow has left the string for good, the bow and the twist are no longer worked out (they no longer touch the
  sound), which keeps a fight's string parts within the speed budget; the same note comes out to the last bit.
- Each fighter's room is the sketch's (3.3 s, 14 LU under for the violin, viola and cello; 2.4 s, 15 LU under for the
  contrabass; 3.3 s for the octobass); the violin, viola and cello are spread a little (0.25), the basses sit in the
  middle.

Checked against the sketch: each fighter's riff of the listening round within 2 dB in every octave
(`bf-harness/tests/engine2_built.rs`, `tests/reference/bowed_riffs.txt`). Each technique has its own check in
`bowed.rs`: in tune at two sample rates, one slip a period, the ear off still near the note, accents settling on
pitch, a martelé never above its note, no overtone pattern following the notes, harder louder and no darker, spiccato,
tremolo at 14 strokes a second without a thump, harmonics, plucks, the snap and the wood, the mute, every bowing
point heard, the free ring, joins without a click, repeats, odd asks finite, and every note ending quietly.

### Licences

Nothing recorded ships. The VSCO 2 CE string recordings (CC0) the bodies were fitted to while sketching were
yardsticks only; the bodies are their own resonances, not the recordings.

## The brass and flutes (fighters TRUMPET, CORNET, FLUGELHORN, HORN, MELLOPHONE, TROMBONE, EUPHONIUM, TUBA, SOUSAPHONE, FLUTE, PICCOLO)

Two engines for the family, built from how the instruments work and tuned by ear in one listening round with five
fixes (sketch `br.rs` with `winds.py`, `/mnt/project-files/ballfight-studio/voice-engine-2/brass1/`): the brass
engine (`bf-sound/src/voice/engine2/brass.rs`, the players' ears made offline in `brass_tune.rs`) and the flute engine
(`flute.rs`). Aedan's picks (2026-11-27 18:44Z): Tp1, Co1, Fh1, Hn1, Me1, Tb1, Eu1, Tu1, Su1, Fl1, Pc1, version 1 of
each. The round's fixes, all carried into the engines: the standard fingerings (fewest valves, no crowded high natural
notes); slurred notes tuned as they are played, from the note they slur from, the lips set afresh where they land
between natural notes; the flute's ear reading pitch from the spectrum; the stop easing the horn's ringing onto the
note; the euphonium leaning on a quick settle. Tu1 and Su1 were heard before the round's last change (short low notes
tuned over the part heard): they play as heard.

### Working parts

- **The lips** (brass). A mass on a spring, striking inward: the breath pushes them shut, the flow through the gap
  between them pulls them apart, and they buzz at the horn's resonance they lock onto. Their tension picks the natural
  note; tongued starts kick them open, a back-tongued ("k") start is softer, a soft ("d") start slower, a breath start
  has no tongue.
- **The horn** (brass). Its input as a set of resonances, one per natural note, strongest round the mouthpiece's
  ring and gone above the bell's cutoff, with the tube's own resistance over them; the valves or the slide scale every
  resonance down together. The flow is worked out against them every sample, at 4 times the sample rate (twice for
  the tuba and the sousaphone), brought down with a windowed-sinc decimator.
- **The blare** (brass). The wave going out steepens over the bore's cylindrical length, more the louder it is: the
  brassy edge of a loud note. Then the wall's loss, the bell's two high-passes and its soft top.
- **The player** (brass). Each note's lip tension and the horn's trim come from trial runs of the note (the ear): the
  tension that sounds the right natural note cleanly with the least lipping and a quick start, then the horn trimmed
  until it is in tune; another fingering where the usual gives no clean note. The trial runs for every plain note of
  every pick over its fighter's register and an octave round it, at every strength in the round's steps of 0.05 (0
  to 1.2), are made once offline into `brass_tune.rs`, so a fight never runs them; a note the table lacks (off its
  semitones, at another sample rate, forced onto a natural note or fingering) is tried at run time and kept. A
  slurred note is tried as played, from its lead-in. A note over the player's reliable top, or one no lip tension
  sounds cleanly, is taken down the octave.
- **After the horn** (brass). A pulse limiter (each pulse held to 2.2 times the typical wave peak around it), the
  mutes and the horn player's hand as filters with the round's levels against the open horn, a soft limiter.
- **The flute.** An air jet crossing the embouchure hole: delayed by its travel, split by the edge through a soft
  curve, driving the tube's resonances; breath noise in the loop; a slow embouchure control holding the jet steady.
  Its ear is three trial passes a note, made offline into `brass_tune.rs` too at every strength step (a strength
  between two steps takes the trim between theirs, within a quarter of a cent of the ear's own); a swell, a harmonic,
  a whistle tone or another rate is tried as asked and kept. The same passes give the embouchure each note settles
  on, and the player sets it before the note: the tongue lets the air go at once (the jet up to speed in 12 ms) and
  the embouchure waits for the note to sound (80 ms at most) before it pushes, so a note from rest starts on its
  breath and the tone grows out of it in 20 to 75 ms over the fighters' registers (the sketch's control, starting
  relaxed, wound the jet's gain up while the tube was still quiet: every start overdriven, a bright clean blip).
- **Speed.** The breath's random draws are made ahead on a second thread in the generator's own order (the tongue's
  puffs after the last tongued start not worked at all), the resonances are set again only when the tuning moves, the
  bell's filters run in one pass and the decimator and the pulse limiter's windows work four outputs at once: the
  same samples, about twice as fast.

### The picks

| Code | Fighter | The instrument | Heard as |
|---|---|---|---|
| Tp1 | TRUMPET | orchestra C trumpet | clear and ringing, little vibrato |
| Co1 | CORNET | brass band cornet | mellow and sweet, a gentle vibrato |
| Fh1 | FLUGELHORN | flugelhorn | soft, dark and velvety |
| Hn1 | HORN | double horn, the hand in the bell | warm and noble |
| Me1 | MELLOPHONE | marching mellophone | bright and loud, like a big trumpet |
| Tb1 | TROMBONE | orchestra tenor trombone with the F valve | full, then a brassy roar |
| Eu1 | EUPHONIUM | euphonium | warm, rich and singing |
| Tu1 | TUBA | orchestra tuba in C | deep, solid and round |
| Su1 | SOUSAPHONE | brass sousaphone | big, punchy and buzzy |
| Fl1 | FLUTE | silver flute | clear and sweet, a little breath |
| Pc1 | PICCOLO | silver piccolo | bright and sparkling |

### How it is played

Every technique, as the engines play it (asked for as in `voice-engine-2-playing.md`): strength (the breath);
accents and sforzandos; tongued, back-tongued (double and triple tonguing are the notes in a row), soft-tongued and
breath starts; slurs and lip slurs, smears and the trombone's glide; scoops, falls, doits and bends; crescendos,
diminuendos and a breath swell over the note; vibrato, its depth, rate and wait, or none; the tongue stop and the
shortest note; on the brass the lip shake, flutter tongue, growl, half valve, a natural note asked for, the fingering
(valves or slide, the double horn's other side), the straight, cup, harmon (stem in and out), plunger (its wah over
the note), practice and stopping mutes, the hand-stopped horn; on the flutes the breathy start, flutter tongue,
harmonics, whistle tone, jet whistle, key clicks, singing while playing and air alone. Trills, runs, rips up the
natural notes and repeated tonguing are the composition system's, made of single notes.

### The built engines (stage 5)

- One note at a time, at any sample rate: a note is the sketch's line of that one note; a slurred note plays the note
  it slurs from for 0.2 s as its lead-in, and a back-tongued note its tongued note, then comes in over 12 ms as the
  note before goes out.
- The kits' own notes go through the engines under Composed: the euphonium's, the sousaphone's and the piccolo's own
  notes, the tuba's long notes (held by the engine, no sustain under them) and its pedal under the oom, the flute's
  shriek and its flutter tonguing, the horn's STOPPED HORN, the cornet's plunger wahs.
- Each fighter's room is the sketch's light room (2.6 s, 14 LU under), the dry in the middle.

Checked against the sketch: the engines play the round's riffs as one line to the sketch's numbers (correlation
1.000000, every pick); note by note, as a fight plays them, every pick's riff within 4 cents and 1 dB of the round's
on nearly every note (`bf-harness/tests/engine2_built.rs`, `tests/reference/wind_riffs.txt`, each riff's octave
bands within 2 dB). Each engine has its own checks in `brass.rs` and `flute.rs`: in tune across the range and at
44.1 kHz, notes the table lacks tried and in tune, louder brighter, every technique finite and changing the note,
mutes quieter and shaped, the hand darker, joins without a click, repeats, untried notes and the top of every range.

### Licences

Nothing recorded ships. Literature numbers (input impedances, lip and jet models) set the parts; no recording was
fitted.
