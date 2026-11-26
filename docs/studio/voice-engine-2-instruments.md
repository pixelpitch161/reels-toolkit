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

**Ships from:** VCSL Steinway B (CC0), strengths vl2, vl3, vl4. **Studied:** VCSL Steinway B, Iowa Steinway B (PC),
Salamander Yamaha C5, Headroom Yamaha C3. **Code:** `bf-sound/src/voice/engine2/piano.rs`, recipe
`bf-content/data/voices/PIANO.ve2`.

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
  single overtones only in a note's first 0.3 s and the overtones' median after that (`engine2_voices.rs`).
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

**Ships from:** VSCO 2 CE pipe organ (CC0), main chorus, quiet and loud. **Studied:** VSCO 2 CE, VCSL Renaissance organ
(Full), Lars Palo's Bureå and Piteå organs' principals, one rank at a time (PC). **Code:** `bf-sound/src/voice/engine2/organ.rs`, recipe
`bf-content/data/voices/ORGAN.ve2`.

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

Held notes and chords, legato from key to key, no accents by touch (an accent is a wider chord or a new stop), no
vibrato (the tremulant is not used). A crescendo brings stops in: the fighter's CATHEDRAL, PULL ALL STOPS and title
blast swell from the quiet setting to the loud one (Aedan's "main chorus, building up").

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
