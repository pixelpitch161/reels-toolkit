# Voice Engine 2: the recording sets

Every set of recordings Voice Engine 2 has measured, who made it, its licence, what was measured from it, and whether
anything made from it ships in the program. The rule (design, "Constraints"): **what ships comes only from CC0 or
"free for any use" sets.** Studying and comparing (measuring general facts about how an instrument behaves, which
belong to nobody) may also use CC BY and CC BY-SA sets. Non-commercial and "all rights reserved" sets are not used at
all, and blending sets never changes a licence.

The recordings themselves are never committed: `recipes fetch <set>` puts them in the git-ignored
`studio/reference/.cache/recordings/` (cloud sets from GitHub; the PC-only sets by hand, as `recipes sets` says).

## Stage 1: piano and organ

| Set | What it is | Made by | Licence | Measured from it | Ships |
|---|---|---|---|---|---|
| `vcsl` | Versilian Community Sample Library: Steinway B grand (no pedal, plus key-up `Rel` recordings), whole-tone grid A#0 to G#7, three strengths (vl2, vl3, vl4), 24-bit 44.1 kHz. Also the Renaissance organ (Full). | Versilian Studios (Sam Gossner and contributors), https://github.com/sgossner/VCSL | CC0 | **PIANO recipe**: every note's overtones, tuning, stretch and noise over 6 s (126 recordings); the damper's time constant per note from the `Rel` recordings. Renaissance organ: studied for ORGAN (start, brightness, wind noise), not shipped. | Yes (PIANO) |
| `vsco` | VSCO 2 Community Edition: pipe organ, main chorus with the 16', quiet and loud settings, every third key of the manual plus pedal notes. | Simon Dalzell of Ivy Audio, for Versilian Studios, https://github.com/sgossner/VSCO-2-CE | CC0 | **ORGAN recipe**: overtones, chorus tuning and wind noise over 4 s, quiet and loud (64 recordings studied; for the recipe a pedal note is left out where the manual has the same pitch). | Yes (ORGAN) |
| `salamander` | Salamander Grand Piano V3: Yamaha C5, minor-third grid, 16 strengths (v4, v8, v12 and v16 studied). | Alexander Holm, https://github.com/sfzinstruments/SalamanderGrandPiano | CC BY 3.0 | Study only (120 recordings): how much louder and brighter a piano gets from soft to hard, its fall and noise by register. These general numbers set how far apart the shipped piano's three strengths are in loudness (see the PIANO sheet) and widen the yardstick's spread. No sound from it is used. | No |
| `headroom` | Headroom Piano: Yamaha C3, five strengths, close microphones. | Bengt Nilsson, https://github.com/sfzinstruments/BengtNilsson.HeadroomPiano | CC BY 4.0 | Study only (150 recordings), as Salamander. | No |
| `iowa-piano` | University of Iowa Musical Instrument Samples: Steinway B, pp, mf and ff, every key, 16-bit 44.1 kHz. | University of Iowa Electronic Music Studios (Lawrence Fritts), https://theremin.music.uiowa.edu/MISpiano.html | Free for any use, without restrictions | Study only (fetched and measured on the PC: the site does not answer the cloud). A second Steinway B: checks that what VCSL's piano does is the instrument and not that one recording. | No (study only in stage 1) |
| `palo-burea` | Bureå Church organ, GrandOrgue sample set: principals studied. | Lars Palo, https://sourceforge.net/p/ourorgan/samplesets/ | CC BY-SA 2.5 | Study only, on the PC: another church organ's speech, brightness and wind. Its reed stops (trumpet, posaune, oboe, krumhorn) are the only free recorded organ reeds; they are a yardstick for a later stage, never a source. | No |
| `palo-pitea` | Piteå MHS (school of music) organ, GrandOrgue sample set: principals studied. | Lars Palo, https://sourceforge.net/p/ourorgan/samplesets/ | CC BY-SA 2.5 | Study only, on the PC, as Bureå. | No |
| FluidR3 GM | General MIDI SoundFont, played through fluidsynth on the PC (`bf-harness/tests/reference/fluidr3.json`, from the sound upgrade's stage 3). | Frank Wen | MIT | Only the **rivals** in the `engine2_voices` test: the engine's piano must be nearer the real Steinway than FluidR3's harp, marimba and nylon guitar; its organ nearer the real organ than FluidR3's accordion, drawbar organ and trumpet. | No |

The PC part (Iowa, Bureå, Piteå) is measured by `recipes study PIANO` and `recipes study ORGAN` in a Remote Control
session; its tables go into the instrument sheets. If a PC set moves the piano's agreed strength steps by more than
0.5 dB, the PIANO recipe is measured again.

## Looked at and not used

- **Virtual-Organs.nl** organ sets: CC BY-NC-SA (non-commercial). Not used.
- **Maestro Concert Grand**: all rights reserved. Not used.
- **Paid libraries**: their licences forbid making another instrument from them. Not used.
- **VSCO 2 CE upright piano**: CC0, but an upright with a different tone from the grand the fighter is; not studied in
  stage 1.

## Thanks

No credit is required for anything that ships (CC0). The guide's "About the sounds" thanks VCSL (Versilian Studios)
and VSCO 2 CE (Versilian Studios and Simon Dalzell of Ivy Audio) by name as a courtesy.
