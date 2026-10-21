# Pixel Pitch status board

**Statuses:** `idea` → `approved` → `in production` → `review` → `done` (or `parked`).
Each session updates its own row.

| Instrument | Status | Script in repo | Notes |
|---|---|---|---|
| Piano | done | `piano_lowpoly.py` (original, short) | 60 s version not in repo yet |
| Violin | done | `violin_lowpoly.py` | |
| Cello (35 s) | done | `cello_lowpoly.py` | |
| Cello (60 s) | done | `cello60_lowpoly.py` | |
| Acoustic guitar | done | `acoustic_lowpoly.py` | |
| Electric guitar (short) | done | `electric_lowpoly.py` | |
| Electric guitar (60 s) | done | `electric60_lowpoly.py` | |
| Trumpet | done | `trumpet_lowpoly.py` | |
| Viola | done | — | script not in repo yet |
| Drum kit | done | `drums60_lowpoly.py` | |
| Double bass | done | — | script not in repo yet |
| Flute | done | `flute60_lowpoly.py` | |
| Trombone | done | `trombone60_lowpoly.py` | model corrected from reference photos |
| French horn | done | `horn60_lowpoly.py` | model corrected from reference photos |
| Clarinet | done | `clarinet60_lowpoly.py` | |
| Trubergle | done | — | script not in repo yet |
| Tuba | done | `tuba60_lowpoly.py` | posted unlit; the lit test render (`LIGHTING = True`) was approved as the new standard |
| Harp | done | `harp60_lowpoly.py` | |
| Frequency explainers | done | — | scripts not in repo yet |

All the videos above are already posted. Don't re-render them.

## Next instruments (ideas board)

The Pixel Pitch ideas chat owns this section. It adds instruments as `idea`, refines each one into a full draft
below, and marks it `approved` when the user signs off. A production session then claims the row
(`in production`, session link, date) and keeps it updated until `done`.

| # | Instrument | Status | Session | Last feedback / next step |
|---|---|---|---|---|
| P1 | Synthesizer | approved | — | Approved 2026-10-21. Ready for a production session (prompt under Drafts → P1). |

## Drafts

<!-- one block per instrument, using this template:
### P<n>. <Instrument> — <STATUS>
- **Hook (showcase lines):** "<LINE 1>," / "<LINE 2>"   then   "BUT HOW DID WE" / "GET HERE?"
- **Headline:** "<N> YEARS" (oldest scene year to today)
- **Modern showcase:** <exact modern model to show + riff style>
- **Scenes (oldest → newest):** year | name (≤ 22 chars) | credit = music style | fact line 1 / line 2 (≤ 18 chars each) | model notes
- **Outro:** "<LINE 1>" / "<LINE 2>", credit
- **Look:** background concept + era palettes (must differ from every posted video), BPM, groove/kit
- **Reference photos to find:** list per scene
- **Sources:** links for every date and fact
- **Production prompt:** the exact message to paste into a new production session
-->

### P1. Synthesizer — APPROVED (2026-10-21)
- **Hook (showcase lines):** "THE FIRST SYNTH," / "WEIGHED 200 TONS"   then   "BUT HOW DID WE" / "GET HERE?"
  (Telharmonium Mark II weighed "almost 200 tons"; rounding approved.)
- **Headline:** "120 YEARS" (1906 → 2026). `HEAD_INTRO = "THE SYNTHESIZER"`, `START_YEAR = 1890`
- **Modern showcase:** Korg minilogue xd (37 keys, 4 voices), modern electro arpeggio over the beat. Credit MODERN ELECTRO.
- **Timeline:** 116 BPM, `SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1`, 11 history scenes (≈ 56 s + final chord).
- **Scenes (oldest → newest):**

| Year (ys) | Name | Credit | Fact | Model notes |
|---|---|---|---|---|
| 1906 | TELHARMONIUM | EDWARDIAN PARLOUR WALTZ | MUSIC SENT DOWN / PHONE LINES | Telharmonic Hall console: organ-like, several manuals + banks of stops; optional alternator behind it (the 200 tons was the basement machinery) |
| 1920 | THEREMIN | SALON ROMANCE | PLAYED WITHOUT / EVER TOUCHING IT | RCA AR-1264 style cabinet on legs; upright pitch rod on the RIGHT, horizontal volume loop on the LEFT; no keys |
| 1928 | ONDES MARTENOT | FRENCH IMPRESSIONISM | A RING ON A WIRE / SLIDES THE PITCH | Keyboard with the ring wire running along its front; expression drawer on the left; palme (lyre with strings) and/or métallique (gong) speaker |
| 1939 | HAMMOND NOVACHORD | 1930s HOLLYWOOD SCORE | FIRST POLYPHONIC / SYNTH FOR SALE | Wooden furniture cabinet (≈ two spinet pianos), 72 keys, knob panel above the keys, no drawbars |
| 1957 | RCA MARK II | 1950s AVANT-GARDE | 1,700 VACUUM TUBES / READ PUNCHED PAPER | Wall of racks ≈ 7 ft × 20 ft + paper-tape punch desk; size ≈ 0.6 |
| 1964 | MOOG MODULAR | SWITCHED-ON BAROQUE | ITS 1968 BACH LP / WENT PLATINUM | Tiered walnut cabinets, black module panels, drooping patch-cord loops, separate keyboard below |
| 1970 | MINIMOOG MODEL D | 70s PROG ROCK | FIRST SYNTH SOLD / IN MUSIC STORES | Wooden case, 44 keys, front panel hinged up and tilted back behind the keys, pitch + mod wheels LEFT of the keys |
| 1978 | PROPHET-5 | NEW WAVE | SAVES ITS SOUNDS / AT A BUTTON PRESS | Wooden case (koa early / walnut later), 61 keys, flat black panel of knobs and buttons |
| 1981 | ROLAND TB-303 | CHICAGO ACID HOUSE | FLOPPED AS A BASS, / BIRTHED ACID HOUSE | Small silver box, pattern buttons laid out like a piano octave, knob row on top; scale up |
| 1983 | YAMAHA DX7 | 80s POWER BALLAD | ITS E.PIANO WAS ON / 40% OF 1986 NO.1S | 61 keys, LCD, rows of colour-coded membrane buttons; check body + button colours from photos |
| 1996 | EURORACK MODULAR | BERLIN TECHNO | 15,000+ MODULES / FROM 600+ MAKERS | Doepfer A-100 style case, rows of 3U modules, patch cables, LEDs |
| 2020s | KORG MINILOGUE XD | MODERN ELECTRO | 200 TONS THEN, / 2.8 KG TODAY | 37 slim keys, aluminium panel, real-wood back panel, OLED scope, joystick on the left |

- **Riff tones:** each era's riff in that instrument's own sound (theremin sine, Moog square/saw, 303 squelchy resonant filter, DX7 FM bell / E. piano).
- **Outro:** "120 YEARS FROM" / "THE TELHARMONIUM", credit MODERN ELECTRO
- **Look:** "Inside a circuit board": a giant upright circuit board, chips/capacitors/resistors standing like low-poly buildings; every note sends light pulses along the copper traces; an oscilloscope screen high in the frame draws the current riff's waveform (one line, not bars, so it differs from the cello's equalizer). No perspective grid, sunset sun or pink/cyan neon (keeps clear of the electric guitar's synthwave).
  Era palettes (board + glow): modern deep green + cyan · Telharmonium oxblood + copper · theremin black bakelite + amber · ondes violet + silver · Novachord walnut + gold · RCA steel grey + phosphor green · Moog walnut + orange · Minimoog chocolate + mustard · Prophet navy + red · TB-303 silver + acid yellow · DX7 maroon + teal · Eurorack matte black + red LEDs.
  Groove: **808 electro**, 116 BPM: syncopated 808 kick, clap on 2 and 4, 16th hats, cowbell, tom fills, synth bass (broken beat; not four-on-the-floor). `LIGHTING = True`. No brand logos or wordmarks on models.
- **Reference photos to find:**
  - Telharmonium: 1906 Telharmonic Hall console photos + alternator room ("Telharmonium console 1906"; Cabinet magazine "Thaddeus Cahill's Music Plant")
  - Theremin: RCA AR-1264 (The Henry Ford collection; Science Museum Group)
  - Ondes Martenot: museum/performer photos ("ondes Martenot diffuseur palme", "ondes Martenot ring")
  - Novachord: National Music Centre (nmc.emuseum.com/objects/289); EMEAPP "The Lost Novachord"
  - RCA Mark II: Columbia Computer Music Center photos; ETHW; Perfect Circuit history article
  - Moog modular: Moog 55 / IIIc (Moog Music archive, museum photos)
  - Minimoog Model D, Prophet-5, TB-303, DX7: maker archives, museum and vintage-synth photos (front, side and three-quarter views)
  - Eurorack: Doepfer A-100 case photos (doepfer.de)
  - minilogue xd: Korg product page
- **Sources:**
  - Telharmonium (almost 200 tons, phone-line distribution, Twain attended): https://en.wikipedia.org/wiki/Telharmonium · "first synthesizer": https://www.openculture.com/2016/01/meet-the-telharmonium-the-first-synthesizer.html
  - Theremin (1920, no contact): https://en.wikipedia.org/wiki/Theremin
  - Ondes Martenot (1928, ring on a wire): https://en.wikipedia.org/wiki/Ondes_Martenot · https://www.classical-music.com/features/instruments/ondes-martenot
  - Novachord (1939, first commercial polyphonic synth): https://en.wikipedia.org/wiki/Novachord
  - RCA Mark II (1957; ~1,700 tubes; punched paper tape): https://en.wikipedia.org/wiki/RCA_Mark_II_Sound_Synthesizer · https://www.perfectcircuit.com/signal/rca-mkii-synthesizer-history · https://ethw.org/RCA_Mark_I_and_Mark_II_Synthesizers
  - Moog modular (1964 AES; Switched-On Bach first platinum classical album): https://en.wikipedia.org/wiki/Moog_synthesizer
  - Minimoog (1970; first synth sold in retail stores; 44 keys): https://en.wikipedia.org/wiki/Minimoog
  - Prophet-5 (1978; programmable patch memory): https://en.wikipedia.org/wiki/Sequential_Circuits_Prophet-5
  - TB-303 (1981; commercial flop; Phuture "Acid Tracks" 1987): https://en.wikipedia.org/wiki/Roland_TB-303
  - DX7 (1983; E. Piano 1 in 40% of 1986 Billboard Hot 100 number ones): https://en.wikipedia.org/wiki/Yamaha_DX7
  - Eurorack (Doepfer 1995/96; 15,000+ modules from 600+ makers as of 2022): https://en.wikipedia.org/wiki/Eurorack
  - minilogue xd (2.8 kg, 37 keys, wood back panel): https://www.musik-produktiv.com/products/korg-minilogue-xd/
- **Production prompt:**

> Make Pixel Pitch P1: the synthesizer. Use the pixel-pitch-history skill and the repo pixelpitch161/reels-toolkit (series/instrument-history/). The approved draft is in STATUS.md under Drafts → "P1. Synthesizer": follow it exactly (hook, headline, all 11 scenes with their years, names, credits and facts, outro, circuit-board background, era palettes, 116 BPM 808-electro groove). First claim the P1 row on the ideas board (in production, this session's link, today's date), commit and push.
>
> Must-haves: new script `synth60_lowpoly.py` (OUT "synth60_lowpoly.mp4", TAG "sx"); modern Korg minilogue xd first, then history oldest to newest, outro back to TODAY; `LIGHTING = True`; each scene's riff uses that instrument's own sound; no brand logos on the models; the background must not look like the electric guitar's synthwave grid or the cello's equalizer.
>
> Reference photos: before modelling each instrument (the modern one and all 11 historical ones), use my open Chrome browser (Claude in Chrome) to find real reference photos yourself: image searches, museum collection pages, makers' sites. Use the "Reference photos to find" list and model notes in the draft, and check the gotchas (theremin loop on the left, Minimoog panel tilted behind the keys with wheels on the left, Ondes wire in front of the keys, RCA Mark II as a wall of racks). Don't ask me for images.
>
> Before the full render, send me `--still` frames taken mid-scene for every scene (contact sheet is fine) and wait for my OK. Then render with pp_parallel.py, watch it through, send the video, update STATUS.md, commit, git pull --rebase and push.
