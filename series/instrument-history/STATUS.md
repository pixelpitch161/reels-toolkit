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
| Trumpet | done | `trumpet_lowpoly.py` | modern trumpet corrected from reference photos (2026-10-24: Yamaha YTR-8335LA side + top views; bell pipe on top, leadpipe in front of the valves, main slide at the bell end, bell bow at the mouthpiece end, 1st / 3rd slides, hook, water key); the posted video is unchanged |
| Viola | done | — | script not in repo yet |
| Drum kit | done | `drums60_lowpoly.py` | |
| Double bass | done | — | script not in repo yet |
| Flute | done | `flute60_lowpoly.py` | |
| Trombone | done | `trombone60_lowpoly.py` | modern trombone corrected again from reference photos (2026-10-25: Yamaha YSL-891Z side view, measured; Jupiter student tenor 3/4 view): slide turned ~40 deg out of the bell plane about the shared lower line (was ~100 deg, edge-on), bell rim 31 cm ahead of the slide lock on a 78 cm slide (was 46 / 70), tuning bow 38 cm behind, Bessel bell fitted to the photo, brass outer slide with a nickel crook, stockings and bumper, two tuning-slide braces with the medallion, bell brace at the lock, a 4 cm mouthpiece, no F-attachment loop (a plain straight tenor); the modern-layout jazz / ska / salsa trombones share the proportions; the posted video is unchanged |
| French horn | done | `horn60_lowpoly.py` | model corrected from reference photos |
| Clarinet | done | `clarinet60_lowpoly.py` | |
| Oboe (model + voice only) | — | `oboe_lowpoly.py` | no video yet: the modern conservatoire oboe built for BALLFIGHT #11 (2026-10-26) from reference photos (Commons: Yamaha YOB-831 front product shot, measured at 590 mm; Lorée with reed; a modern reed against a ruler); scene `ball` = the same model with radii x1.6 for small sprites |
| Bass clarinet (model + voice only) | — | `bassclarinet_lowpoly.py` | no video yet: the modern low-C bass clarinet built for BALLFIGHT #12 (2026-10-26) from reference photos found in Chrome (Commons: Yamaha YCL-622II front product shot, scaled to a 40 mm body at 0.29 mm/px; Buffet Prestige 1193 3/4 view): silver neck loop + mouthpiece, three LH / three RH plates, pinky spatulas, two big open cups, thumb-rest brackets, the silver U-bow and tilted 104 mm bell, the floor peg; chalumeau-reed voice (odd harmonics, ~560 Hz formant, growl) + `slap()` slap-tongue pop; scene `ball` = radii x2.0 for small sprites |
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
| P1 | Synthesizer | review | [session](https://claude.ai/code/session_01Scjyafon3vLxQ2TTrnUXBk) · 2026-10-21 | Rendered 2026-10-21 (`synth60_lowpoly.py`, 62.6 s, lit). Stills contact sheet + video sent to the user for review. One change from the draft: Eurorack year shown as 1995 (Doepfer's A-100 launched in 1995; draft said 1996). Next: user review, then post. |
| P2 | Alto saxophone | review | [session](https://claude.ai/code/session_013UMaDd8usn4Db7tWhgAwUj) · 2026-10-21 | Rendered 2026-10-21 (`altosax60_lowpoly.py` → `altosax60_lowpoly.mp4`, 60.1 s, lit, −11.9 LUFS). Stills contact sheet + video sent to the user for review. Follows the draft; the 2020s YAS-62 row plays as the 9th scene, so the cut runs 60.1 s rather than the ≈53 s + chord estimate. Models from reference photos (Met 2005.82 Sax alto, NMM 13158 Conn, Yamaha, SMEM EWI1000). Note: NMM dates its Conn no. 16 to 1890–92; the 1888 date is Buescher's first prototype, as the draft cites. Next: user review, then post. |
| P3 | Accordion | review | [session](https://claude.ai/code/session_01RyAkaahcXy7gH8qpoCRfUF) · 2026-10-21 | Rendered 2026-10-21 (`accordion60_lowpoly.py` → `accordion60_lowpoly.mp4`, 62.6 s, lit, −11.7 dB mean). Stills contact sheet, video and captions sent to the user for review. Follows the draft; the 2020s Morino+ V row plays as the 10th scene, so the cut runs 62.6 s rather than the ≈55 s + chord estimate. Models from Chrome reference photos (Hohner Morino+ V, Zeng tomb sheng, Demian 1829, 142-tone bandoneons, Dallapè/Deiro-era piano accordion, Joe Falcon's accordion, 1930 French chromatic musette, 1950s pearloid, Corona III, Roland FR-7). One change from the model notes: the Cajun accordion has four stop knobs, as on Falcon's own instrument (draft said two). Musette reeds ±18 cents, phase-locked for an obvious ~7 Hz wobble. Next: user review, then post. |
| P4 | Ukulele | review | [session](https://claude.ai/code/session_01FmQrsdwGhdezdxp3sm8YPa) · 2026-10-21 | 2026-10-21: `ukulele60_lowpoly.py` rendered to `ukulele60_lowpoly.mp4` (55.7 s, lit, wave-barrel backdrop); stills sheet, video and captions sent. Models built from Chrome reference photos (Kamaka site, NMM Kumalae, Horniman machete, Nunes, Islander, Vega baritone). Modern fact shown in the showcase via the new optional `INTRO_STEPS`; outro returns to the soprano (`OUTRO_NAME`). Awaiting user review. |
| P5 | Bagpipes | review | [session](https://claude.ai/code/session_01Qm7rGbs5v6N1NmF33f4ZdM) · 2026-10-22 | Rendered 2026-10-22 (`bagpipes60_lowpoly.py` → `bagpipes60_lowpoly.mp4`, 62.6 s, lit, ≈ −11 dB per-second RMS throughout). Stills contact sheet, video and captions sent to the user for review. Follows the draft (10 rows, standing-stones storm, 100 BPM big beat); the closing modern row makes it 62.6 s rather than ≈ 60 s. Models from Chrome reference photos: Highland maker shots (Hardie/Wallace-type sets), a WWI piper set with a faded tartan cover, Campbeltown Pipe Band 1977 photos (red tartan, ribbons), the Alexandrian terracotta + Gloucester altar (Roman: bag in the crook of the arm, drone pipe + syrinx), Cantigas miniatures (Commons "Cantiga bagpipes"), Bruegel's Peasant Wedding, Met/MIM musettes (shuttle drone, two chalumeaux, bellows), union pipes + modern full sets (3 drones + 3 regulators in one stock), Northumbrian smallpipes (4 drones, 7-key closed chanter), Hevia product + live photos. One change from the model notes: on Hevia's instrument the long black pipe is a blowpipe (he plays through it in live photos), so its drones are synthesised, not a separate drone tube. A drone sounds under every scene (measured ~100% coverage); the Northumbrian chanter is silent 55% of its scene (staccato), all others continuous. Lightning (thunder + flash + bolt) on the first downbeat and on every pipe-band roll. Next: user review, then post. |
| P6 | Turntables | review | [session](https://claude.ai/code/session_01WFWhjRQP978z5jeTn6ZbEc) · 2026-10-22 | Rendered 2026-10-22 (`turntables60_lowpoly.py` → `turntables60_lowpoly.mp4`, 59.8 s incl. final chord at 130 BPM, lit). Stills contact sheet + video sent to the user for review. Follows the draft (all 13 scene rows, giant-record backdrop, era palettes, UK garage 2-step, scratch fill at every scene change). Models from Chrome reference photos (NMAH/Edison NHP tinfoil phonograph, Berliner hand-wind, victor-victrola.com VTLA, Philco M-15, 1950s Kingston sound systems, Technics SL-1200 / MK2 / MK7, Christie's Kool Herc lot + Shure Vocal Master, NMAAHC Grand Wizzard Theodore decks, CDJ-500, Final Scratch). Scratches are a sample pushed back and forth under the needle with fader cuts; the same motion drives the records and the gloved hand on screen. Record labels plain, no logos. Next: user review, then post. |
| P7 | Pipe organ | review | [session](https://claude.ai/code/session_01WEpQaZGXfpGprqXDV96RT4) · 2026-10-22 | Rendered 2026-10-22 (`organ60_lowpoly.py` → `organ60_lowpoly.mp4`, 60.6 s incl. final chord, lit, −11.9 LUFS). Stills contact sheet, video and captions sent to the user for review. Follows the draft at 120 BPM with all 12 scene rows; no BPM or chord nudge needed. Models from Chrome reference photos (Cage organ: Pipe Organ Map, 99% Invisible and Universes in Universe photos; Aquincum/Dion hydraulis; Wikipedia/Duke portatives; Praetorius, Syntagma Musicum plate XXV; Valère; Arnstadt; Saint-Sulpice; Riviera Wurlitzer; Boardwalk 7-manual console; Hammond Model A; Sydney Grand Organ; Elbphilharmonie). The four big organs read differently: Saint-Sulpice case façade with statues; Boardwalk console in its round booth; Sydney timber pipe-tower façade; Elbphilharmonie pipes rising between the seating terraces. The Wurlitzer riff has a car horn on beat 4 of each bar. Next: user review, then post. |
| P8 | Mellophone | review | [session](https://claude.ai/code/session_01R6gHV7JTVxgjzPoyg2tdmT) · 2026-10-23 | Rendered 2026-10-23 (`mellophone60_lowpoly.py` → `mellophone60_lowpoly.mp4`, 61.333 s incl. final chord, lit, ≈ −11 K-weighted). Stills contact sheet, video and captions sent to the user for review. Follows the draft: all 11 scene rows, brass-clockwork backdrop (gears tick one tooth per beat, background pistons pump on the notes, steam vents fire on the rimshots), era palettes, 111 BPM dark drumline. Runtime: 1,840 frames (61.333 s, inside the 60.5–61.5 s rule) instead of 61.000 s, because at 48 kHz that fills whole AAC frames, so the loop has no padding gap; `python -m reelkit.loopcheck` passes (seam 1.1×, audio step 1.5 dB, no click). The final chord runs up with a snare roll into the downbeat at 0 s and the picture dissolves into frame 0 (new optional engine hooks `FINAL_LEN`, `END_FADE`, `LOOP_END`, `AUDIO_RATE`; defaults leave other videos unchanged, see README). Models from Chrome reference photos: Yamaha YMP-204MS product page; Met 504629 alto saxhorn + Musée de la musique Sax alto; MIMEd 15365 Courtois Koenig horn (bell down, 3 Périnet valves); Met 503845 + 503839 over-the-shoulder horns (rotary valves, bell back over the shoulder); MIMEd 102850 Besson E♭ tenor horn; NMM 12013 Conn 4-E description + 1911 Conn concert mellophone (silver satin, gold bell interior, bell back-left); Conn 16E mellophonium; Olds Ultratone piston-rotor bugle; Reynolds M13 marching mellophone; King K-50 two-valve G bugle; King 1121 F mellophone. Bell-front horns told apart by valves + finish: 1960 gold coil + long horn bell, 1968 nickel-silver 1 piston + 1 rotor, 1969 lacquer with nickel valves, 1977 chrome 2 upright valves, 2000 silver 3 valves + lyre, modern gold 3 valves. No logos on the models. Next: user review, then post. |

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
> Before the full render, send me `--still` frames taken mid-scene for every scene (contact sheet is fine) so I can review them, but don't wait for my OK: carry straight on and render with pp_parallel.py, watch it through, send the video, update STATUS.md, commit, git pull --rebase and push.

### P2. Alto saxophone — REVIEW (approved 2026-10-21, rendered 2026-10-21)
- **Hook (showcase lines):** "ABOUT 600 PARTS," / "AND JUST ONE REED"   then   "BUT HOW DID WE" / "GET HERE?"
- **Headline:** "180 YEARS" (1846 → 2026). `HEAD_INTRO = "THE ALTO SAX"`, `START_YEAR = 1830`
- **Modern showcase:** Yamaha YAS-62 alto (gold lacquer), boom-bap jazz sax riff. Credit BOOM-BAP JAZZ.
- **Timeline:** 96 BPM, `SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1`, 8 history scenes (≈ 53 s + final chord).
- **Scenes (oldest → newest):**

| Year (ys) | Name | Credit | Fact | Model notes |
|---|---|---|---|---|
| 1846 | ADOLPHE SAX ALTO | FRENCH MILITARY BAND | BRASS POWER WITH / WOODWIND SPEED | Sax-made alto (1850s–60s museum examples); far fewer keys and simpler guards than a modern alto; check octave-key count and bell-key side from photos |
| 1888 | CONN'S FIRST SAX | GILDED AGE CONCERT BAND | MADE IN ELKHART, / COPIED FROM A SAX | Earliest Conn alto (NMM Conn "Wonder" no. 16, 1892): gold-plated, ornately engraved bell and bow, mother-of-pearl touchpieces, single octave key, low keys on the LEFT of the bell, 12.5 cm bell |
| 1920s (1922) | CONN NEW WONDER | 1920s DANCE BAND | OVER 1 MILLION / SOLD BY 1925 | Silver-plated body, gold-washed bell interior |
| 1940s (1940) | CONN 6M | SWING ERA BALLAD | JOHNNY HODGES, / ELLINGTON'S ALTO | "Naked lady" bell engraving; tuning mechanism on the neck |
| 1953 | GRAFTON ACRYLIC | BEBOP | PARKER PLAYS A / PLASTIC ALTO | Cream (not white, not clear) acrylic body with metal keys |
| 1959 | SELMER SUPER ACTION | COOL JAZZ: TAKE FIVE | 1ST JAZZ SINGLE TO / SELL A MILLION | 1950s Selmer Paris Super (Balanced) Action alto, lacquered brass |
| 1967 | SELMER VARITONE | 60s SOUL JAZZ | A MIC IN THE NECK, / PLUS SUB-OCTAVES | Selmer alto with the pickup built into the neck and the control box on the bottom key guard |
| 1987 | AKAI EWI1000 | 80s FUSION | KEYS SENSE TOUCH, / THEY NEVER MOVE | Straight tube, no bell, touch plates instead of keys, roller octave switches, cable to the separate EWV2000 module (can sit on the pedestal); check colour from photos |
| 2020s | YAMAHA YAS-62 | BOOM-BAP JAZZ | 25 TONE HOLES, / LEATHER PADS | Gold lacquer, hand-engraved two-piece bell, high F# and front F keys |

- **All altos:** bell points up and forward on the player's right; detachable curved neck; U-shaped bow turns the tubing upward. Check bell-key side, octave keys and palm keys per era/maker from photos.
- **Riff tones:** alto-sax voice throughout (darker, wide vibrato for Hodges; bright for bebop); Varitone riff doubled an octave down with echo; EWI riff as a synth lead.
- **Outro:** "180 YEARS FROM" / "ADOLPHE SAX", credit BOOM-BAP JAZZ
- **Look:** "A subway platform at night": tiled walls, arched tunnel mouths, train headlights rushing past on each bar's downbeat, invented platform posters (no real brands) that change per era, lights flicker on snare hits.
  Era palettes (tiles + accent): modern white tile + yellow safety line · 1846 cream + gas-lamp amber · 1888 Victorian maroon + brass · 1920s Art Nouveau iron green + amber · 1940s Art Deco cream + oxblood · 1953 mint + red · 1959 navy + sky blue · 1967 burnt orange + brown · 1987 grey + spray-paint graffiti colours.
  Groove: **boom bap**, 96 BPM: dusty kick, lazy swung snare on 2 and 4, vinyl crackle, upright-bass line, Rhodes chords. `LIGHTING = True`. No brand logos or wordmarks on models.
- **Reference photos to find:**
  - 1846 Sax alto: HMB Basel 1860 Sax alto; Edinburgh MIMEd (maker "Adolphe Sax", Paris); saxophone.org museum specimens
  - 1888 Conn: National Music Museum, Conn "Wonder" alto no. 16 (emuseum.nmmusd.org/objects/13158); other 1890s Conn altos
  - Conn New Wonder (1920s): saxophone.org Conn model pages
  - Conn 6M: "Conn 6M naked lady"
  - Grafton: "Grafton acrylic alto saxophone", museum photos
  - Selmer Super Action: Selmer's Paul Desmond artist page; 1950s Super (Balanced) Action altos
  - Varitone: tcgakki.com Selmer Varitone page; bassic-sax.info
  - Akai EWI1000 + EWV2000: SMEM museum page; Muzines WX7 vs EWI review
  - YAS-62: Yamaha product page
- **Sources:**
  - ~600 parts, 25 tone holes, leather pads: https://www.yamaha.com/en/musical_instrument_guide/saxophone/mechanism
  - Single reed; invented early 1840s; patent 28 June 1846; "projection of a brass instrument and the agility of a woodwind": https://en.wikipedia.org/wiki/Saxophone
  - Buescher built Conn's first sax prototype in 1888 after seeing an Adolphe Sax instrument; Conn in Elkhart: https://en.wikipedia.org/wiki/Buescher_Band_Instrument_Company · early Conn alto: https://emuseum.nmmusd.org/objects/13158/alto-saxophone-eflat-high-pitch
  - Over one million saxophones sold c. 1915–1925: https://www.metmuseum.org/perspectives/conn-o-sax-and-other-rare-saxophone-voices
  - Hodges, Ellington's alto, Conn 6M in the 1940s: https://en.wikipedia.org/wiki/Johnny_Hodges
  - Parker played a Grafton at Massey Hall, May 1953: https://en.wikipedia.org/wiki/Grafton_saxophone
  - Take Five (1959) first jazz single to pass a million; Desmond on alto: https://en.wikipedia.org/wiki/Take_Five · Desmond's Selmer Super Action: https://www.selmer.fr/en/artist/paul-desmond
  - Varitone (1967, neck pickup, sub-octaves): https://en.wikipedia.org/wiki/Varitone · https://en.wikipedia.org/wiki/Sonny_Stitt
  - EWI1000 (1987, touch-sensitive keys, roller octaves, EWV2000): https://www.smemmusic.ch/index.php/de/node/1833 · https://en.wikipedia.org/wiki/Electronic_wind_instrument
  - YAS-62 specs: https://ca.yamaha.com/en/products/musical_instruments/winds/saxophones/yas-62/specs.html
- **Production prompt:**

> Make Pixel Pitch P2: the alto saxophone. Use the pixel-pitch-history skill and the repo pixelpitch161/reels-toolkit (series/instrument-history/). The approved draft is in STATUS.md under Drafts → "P2. Alto saxophone": follow it exactly (hook, headline, all 8 scenes with their years, names, credits and facts, outro, subway-platform background, era palettes, 96 BPM boom-bap groove). First claim the P2 row on the ideas board (in production, this session's link, today's date), commit and push.
>
> Must-haves: new script `altosax60_lowpoly.py` (OUT "altosax60_lowpoly.mp4", TAG "as"); modern Yamaha YAS-62 first, then history oldest to newest, outro back to TODAY; `LIGHTING = True`; no brand logos on the models or posters; each scene's riff in its credited style.
>
> Reference photos: before modelling each instrument (the modern one and all 8 historical ones), use my open Chrome browser (Claude in Chrome) to find real reference photos yourself: image searches, museum collection pages, makers' sites. Use the "Reference photos to find" list and model notes in the draft. Check bell direction (up and forward, player's right), the curved neck and U-bow, which side the low bell keys are on, the octave keys, and how keywork differs between eras (the 1846 Sax alto has far fewer keys; the EWI has no bell and no moving keys). Don't ask me for images.
>
> Before the full render, send me `--still` frames taken mid-scene for every scene (contact sheet is fine) so I can review them, but don't wait for my OK: carry straight on and render with pp_parallel.py, watch it through, send the video, update STATUS.md, commit, git pull --rebase and push.

### P3. Accordion — REVIEW (approved 2026-10-21)
- **Hook (showcase lines):** "120 BASS BUTTONS," / "ALL FOR ONE HAND"   then   "BUT HOW DID WE" / "GET HERE?"
- **Headline:** "2,450 YEARS" (433 BC → 2026). `HEAD_INTRO = "THE ACCORDION"`, `START_YEAR = -500`
- **Modern showcase:** Hohner Morino+ V 120 (41 piano keys, 120 basses, black), digital cumbia riff. Credit DIGITAL CUMBIA.
- **Timeline:** 100 BPM, `SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1`, 9 history scenes (≈ 55 s + final chord).
- **Scenes (oldest → newest):**

| Year (ys) | Name | Credit | Fact | Model notes |
|---|---|---|---|---|
| -433 ("433 BC") | SHENG | ANCIENT CHINESE COURT | FREE REEDS SET IN / BAMBOO PIPES | Ancient form (Marquis Yi of Zeng tomb): round gourd wind chest, bamboo pipes standing in a ring, long curved mouthpiece. NOT the modern metal-keyed sheng |
| 1829 | DEMIAN'S ACCORDION | VIENNESE SALON | ONE BUTTON PLAYS / A WHOLE CHORD | Tiny box, buttons on one end only (a few chord keys), small bellows, no treble keyboard |
| 1870s (1870) | BANDONEON | ARGENTINE TANGO | MADE FOR HYMNS, / STOLEN BY TANGO | Buttons on both square ends (not a front keyboard), hand straps, played on the knee |
| 1910 | PIANO ACCORDION | VAUDEVILLE RAG | COINED THE NAME / PIANO-ACCORDION | Guido Deiro-era ornate 1910s piano accordion, fewer bass buttons than modern |
| 1928 | CAJUN ACCORDION | CAJUN TWO-STEP | 1ST CAJUN RECORD / STARS AN ACCORDION | Single-row diatonic (Acadian/Monarch type): ~10 buttons, two stop knobs, two bass levers |
| 1930s (1934) | MUSETTE ACCORDION | BAL-MUSETTE WALTZ | 3 REEDS DETUNED / FOR THAT WOBBLE | French chromatic button accordion: rows of round buttons on the treble side, no piano keys |
| 1955 | GOLDEN-AGE ACCORDION | POLKA | ON THE WELK SHOW / FOR 27 YEARS | 1950s US piano accordion: chrome grille, rhinestones, two-tone pearloid; check colours |
| 1968 | VALLENATO ACCORDION | VALLENATO | THREE ROWS, THREE / REEDS PER NOTE | Small three-row button accordion (Hohner Corona type), often brightly coloured |
| 2005 | ROLAND V-ACCORDION | EUROPOP | NO REEDS AT ALL: / IT'S ALL DIGITAL | Roland FR-5 / FR-7: looks acoustic, plus a display and a modern grille |
| 2020s | HOHNER MORINO+ V | DIGITAL CUMBIA | 13 REGISTERS IN / A 12.4 KG BOX | 41 piano keys, 120 basses, black |

- **All accordions:** treble side (keys or buttons) on the player's right, bass buttons on the left, bellows between. Show the treble side three-quarter, with the bellows folds visible.
- **Riff tones:** sheng breathy reed organ; bandoneon dark and staccato; musette three detuned voices (wobble); V-Accordion a slightly too-clean digital version.
- **Outro:** "2,450 YEARS FROM" / "THE SHENG", credit DIGITAL CUMBIA
- **Look:** "A river cruise at night": riding a paddle-steamer deck under string lights; the paddle wheel turns on the beat; the riverbank scrolling past changes per era (pagodas, Vienna, Buenos Aires docks, Paris quais, Louisiana bayou, 1950s boardwalk, Magdalena river).
  Era palettes (sky + string lights): modern deep teal + warm white · sheng ink black + vermilion · 1829 Biedermeier plum + candle gold · 1870s tango burgundy + brass · 1910 sepia + marquee bulbs · 1928 bayou olive + firefly yellow · 1930s Paris slate blue + accordion red · 1955 TV pink + chrome · 1968 tropical green + mango · 2005 silver + electric blue.
  Groove: **digital cumbia**, 100 BPM: guacharaca scrape on 8ths, caja + conga pattern, kick on 1 and 3, root–fifth bass, synth pad. `LIGHTING = True`. No brand logos or wordmarks on models.
- **Reference photos to find:**
  - Sheng: Marquis Yi of Zeng tomb sheng (Hubei Provincial Museum); Han dynasty sheng (Met, British Museum)
  - Demian's accordion: 1830s Viennese "Accordion" (Technisches Museum Wien, Deutsches Museum, Met; "Demian accordion 1829")
  - Bandoneon: 19th-century German bandoneons; tango photos ("bandoneon 142 tones rheinische")
  - 1910 piano accordion: Guido Deiro photos (henrydoktorski.com/GuidoDeiro); 1910s Italian piano accordions
  - Cajun: Joe Falcon photos; Acadian / Monarch single-row diatonic accordions
  - Musette: 1930s French chromatic button accordions ("accordéon chromatique 1930 musette")
  - 1955: 1950s US piano accordions; Myron Floren photos
  - Vallenato: Hohner Corona three-row; Festival de la Leyenda Vallenata photos
  - Roland V-Accordion: Roland FR-5 / FR-7 product pages
  - Morino+ V: Hohner product page
- **Sources:**
  - Sheng (Zeng tomb c. 433 BCE; bamboo pipes; free reeds): https://en.wikipedia.org/wiki/Sheng_(instrument)
  - Demian 1829 patent (one key = whole chord; name from "Akkord"); Stradella bass, 120-bass most common; reedless digital V-Accordions: https://en.wikipedia.org/wiki/Accordion
  - Bandoneon (religious and popular music; Argentina c. 1870; tango): https://en.wikipedia.org/wiki/Bandoneon
  - Guido Deiro coined "piano-accordion" (1910): https://henrydoktorski.com/free-reed/essays/whowasfirst.html
  - First Cajun record "Allons à Lafayette" (1928), Falcon on accordion: https://en.wikipedia.org/wiki/Joe_Falcon
  - Musette tuning (three detuned 8′ reeds): https://en.wikipedia.org/wiki/Accordion_reed_ranks_and_switches
  - Myron Floren on The Lawrence Welk Show 1955–1982: https://en.wikipedia.org/wiki/The_Lawrence_Welk_Show
  - Vallenato three-row accordion, three reeds per note: https://en.wikipedia.org/wiki/Vallenato · festival origin 1968: https://en.wikipedia.org/wiki/Festival_de_la_Leyenda_Vallenata · UNESCO (2015): https://ich.unesco.org/en/USL/traditional-vallenato-music-of-the-greater-magdalena-region-01095
  - Roland FR-5 / FR-7 reedless V-Accordions (2005): https://newatlas.com/the-piano-accordion-goes-digital/3666/
  - Morino+ V (41 keys, 120 basses, 13 treble registers, 12.4 kg): https://www.muziker.no/hohner-morino-v-120-black
- **Production prompt:**

> Make Pixel Pitch P3: the accordion. Use the pixel-pitch-history skill and the repo pixelpitch161/reels-toolkit (series/instrument-history/). The approved draft is in STATUS.md under Drafts → "P3. Accordion": follow it exactly (hook, headline, all 9 scenes with their years, names, credits and facts, outro, river-cruise background, era palettes, 100 BPM digital-cumbia groove). First claim the P3 row on the ideas board (in production, this session's link, today's date), commit and push.
>
> Must-haves: new script `accordion60_lowpoly.py` (OUT "accordion60_lowpoly.mp4", TAG "ad"); modern Hohner Morino+ V first, then history oldest to newest (the year counter starts in BC, like the flute video), outro back to TODAY; `LIGHTING = True`; no brand logos on the models; each scene's riff in its credited style, with the musette scene audibly detuned.
>
> Reference photos: before modelling each instrument (the modern one and all 9 historical ones), use my open Chrome browser (Claude in Chrome) to find real reference photos yourself: image searches, museum collection pages, makers' sites. Use the "Reference photos to find" list and model notes in the draft. Check the treble side on the player's right and bass on the left, the bellows, buttons vs piano keys per era, the bandoneon's buttons on both ends, and the ancient sheng's gourd chest and curved mouthpiece. Don't ask me for images.
>
> Before the full render, send me `--still` frames taken mid-scene for every scene (contact sheet is fine) so I can review them, but don't wait for my OK: carry straight on and render with pp_parallel.py, watch it through, send the video, update STATUS.md, commit, git pull --rebase and push.

### P4. Ukulele — REVIEW (approved 2026-10-21; rendered `ukulele60_lowpoly.py`)
- **Hook (showcase lines):** "ITS NAME MEANS" / "'JUMPING FLEA'"   then   "BUT HOW DID WE" / "GET HERE?"
- **Headline:** "147 YEARS" (1879 → 2026). `HEAD_INTRO = "THE UKULELE"`, `START_YEAR = 1870`
- **Modern showcase:** Kamaka standard soprano, koa (the original size), indie-folk stomp riff. Credit INDIE FOLK STOMP.
- **Timeline:** 104 BPM, `SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1`, 9 history scenes (≈ 53 s + final chord).
- **Scenes (oldest → newest):**

| Year (ys) | Name | Credit | Fact | Model notes |
|---|---|---|---|---|
| 1879 | MADEIRAN MACHETE | MADEIRAN FOLK DANCE | SAILED TO HAWAII / ON THE RAVENSCRAG | Small double-bulged (figure-8) body, four METAL strings, often wooden pegs |
| 1880s (1885) | HONOLULU UKULELE | HAWAIIAN COURT HULA | KING KALAKAUA / CHAMPIONED IT | Early plain koa soprano (Nunes / Dias / Santo) |
| 1915 | KUMALAE UKULELE | HAPA HAOLE SONG | A GOLD AWARD AT / THE 1915 SF EXPO | Koa soprano (NMM Kumalae example) |
| 1917 | BANJOLELE | VAUDEVILLE NOVELTY | A BANJO BODY FOR / VAUDEVILLE VOLUME | Small drum head with tension hooks round the rim, open back or resonator, ~16 frets |
| 1928 | KAMAKA PINEAPPLE | JAZZ AGE CHARLESTON | PINEAPPLE-SHAPED, / PATENTED IN 1928 | OVAL body, not figure-8 |
| 1949 | MACCAFERRI ISLANDER | 1950s TV SINGALONG | ABOUT 9 MILLION / PLASTIC UKES MADE | Moulded plastic; check colours and the moulded fingerboard |
| 1960s (1962) | BARITONE UKULELE | FOLK HOOTENANNY | ARTHUR GODFREY / PUSHED IT ON TV | Noticeably bigger than the others (Vega / Favilla baritones) |
| 1993 | TENOR UKULELE | ISLAND REGGAE | HAWAII'S FIRST / PLATINUM ALBUM | Mahogany-toned tenor so it reads differently from the koa Kamaka next |
| 2006 | KAMAKA TENOR | ROCK BALLAD | ONE OF YOUTUBE'S / FIRST VIRAL HITS | Koa Kamaka tenor, 4 strings (Jake Shimabukuro's) |
| 2020s | KAMAKA SOPRANO | INDIE FOLK STOMP | 21 INCHES LONG, / 13-INCH SCALE | Koa standard soprano |

- **All:** count the strings (4 everywhere; metal on the machete); figure-8 vs oval body; friction pegs vs geared tuners per era; round soundhole; how many frets clear the body.
- **Riff tones:** plucked/strummed nylon throughout; metal-string machete; banjo plunk for the banjolele; thin plastic tone for the Maccaferri; soft reggae skank for 1993; tremolo shred for 2006.
- **Outro:** "147 YEARS FROM" / "A MADEIRAN MACHETE", credit INDIE FOLK STOMP
- **Look:** "Inside a breaking wave": riding the barrel of a curling wave; foam bursts on each clap; sunlight ripples through the water wall; fish and turtle silhouettes drift past; water colour changes per era like vintage postcard tints.
  Era palettes (water + accent): modern turquoise + white foam · 1879 Atlantic navy + Madeira-wine red · 1880s royal red + gold · 1915 expo ivory + coral · 1917 vaudeville sepia + spotlight cream · 1928 pineapple yellow + jungle green · 1949 TV cyan + cream · 1960s hootenanny mustard + brown · 1993 pastel with a rainbow over the wave · 2006 night-surf purple + lime.
  Groove: **indie-folk stomp-clap**, 104 BPM: foot stomps on 1 and 3, hand claps on 2 and 4, tambourine 8ths, floor-tom pulse, whistled hook. `LIGHTING = True`. No brand logos or wordmarks on models.
- **Reference photos to find:**
  - Machete: Madeiran machete / braguinha ("machete de braga Madeira"; the Met; MIM Brussels)
  - Honolulu ukulele: early Nunes / Dias / Santo koa ukuleles (Bishop Museum; "Manuel Nunes ukulele")
  - Kumalae: National Music Museum soprano (emuseum.nmmusd.org/objects/18094)
  - Banjolele: 1920s banjo ukuleles; George Formby's instruments
  - Pineapple: Kamaka HP-1 page
  - Maccaferri Islander: NMM "Digging the Fifties" post; ukesterbrown.com Islanders page
  - Baritone: 1950s–60s Vega and Favilla baritones; Arthur Godfrey photos
  - 1993 tenor: 1990s tenor ukuleles (mahogany); Iz performance photos for era feel
  - Kamaka tenor: Jake Shimabukuro Kamaka tenor photos
  - Modern soprano: Kamaka soprano pages
- **Sources:**
  - "Jumping flea"; Ravenscrag late Aug 1879; first makers 1880s; Kalākaua; 1915 SF expo; ~9 million Maccaferri ukes; Godfrey and the baritone on TV; soprano 21 in / 13 in scale: https://en.wikipedia.org/wiki/Ukulele
  - Machete (Madeira, four metal strings, double-bulged body): https://en.wikipedia.org/wiki/Machete_(instrument)
  - Kumalae Gold Award, 1915 Panama-Pacific Exposition: https://en.wikipedia.org/wiki/Jonah_Kumalae
  - Banjolele (1917, vaudeville volume, Formby): https://en.wikipedia.org/wiki/Banjolele
  - Pineapple patented 1928 by Sam Kamaka: https://kamakahawaii.com/our-story/our-legacy/ · https://kamakahawaii.com/instruments/standard-HP1.html
  - Maccaferri plastic ukulele (1949, with Godfrey): https://en.wikipedia.org/wiki/Mario_Maccaferri
  - Facing Future (1993), Hawaiʻi's first certified platinum album (2005): https://en.wikipedia.org/wiki/Israel_Kamakawiwo%CA%BBole
  - Jake Shimabukuro video posted April 2006, one of YouTube's first viral videos; Kamaka tenor: https://en.wikipedia.org/wiki/Jake_Shimabukuro
- **Production prompt:**

> Make Pixel Pitch P4: the ukulele. Use the pixel-pitch-history skill and the repo pixelpitch161/reels-toolkit (series/instrument-history/). The approved draft is in STATUS.md under Drafts → "P4. Ukulele": follow it exactly (hook, headline, all 9 scenes with their years, names, credits and facts, outro, inside-a-wave background, era palettes, 104 BPM indie-folk stomp-clap groove). First claim the P4 row on the ideas board (in production, this session's link, today's date), commit and push.
>
> Must-haves: new script `ukulele60_lowpoly.py` (OUT "ukulele60_lowpoly.mp4", TAG "uk"); modern Kamaka soprano first, then history oldest to newest, outro back to TODAY; `LIGHTING = True`; no brand logos on the models; each scene's riff in its credited style. Copy from the closest plucked/guitar scripts (`harp60` / `acoustic` / `pp_guitar.py` helpers).
>
> Reference photos: before modelling each instrument (the modern one and all 9 historical ones), use my open Chrome browser (Claude in Chrome) to find real reference photos yourself: image searches, museum collection pages, makers' sites. Use the "Reference photos to find" list and model notes in the draft. Check string count (4; metal on the machete), figure-8 vs oval (pineapple) bodies, the banjolele's drum head and tension hooks, the baritone's larger size, and pegs vs tuners per era. Don't ask me for images.
>
> Before the full render, send me `--still` frames taken mid-scene for every scene (contact sheet is fine) so I can review them, but don't wait for my OK: carry straight on and render with pp_parallel.py, watch it through, send the video, update STATUS.md, commit, git pull --rebase and push.

### P5. Bagpipes — IN REVIEW (rendered 2026-10-22)
- **Hook (showcase lines):** "9 NOTES, 3 DRONES," / "AND ZERO RESTS"   then   "BUT HOW DID WE" / "GET HERE?"
- **Headline:** "1,960 YEARS" (60 AD → 2026). `HEAD_INTRO = "THE BAGPIPES"`, `START_YEAR = 0`
- **Modern showcase:** Great Highland bagpipe (blackwood, tartan bag cover), bagpipe big-beat riff. Credit BAGPIPE BIG BEAT.
- **Timeline:** 100 BPM, `SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1`, 10 scene rows (9 history + the closing modern row) = 25 bars ≈ 60 s + final chord.
- **Scenes (oldest → newest):**

| Year (ys) | Name | Credit | Fact | Model notes |
|---|---|---|---|---|
| 60 ("60 AD") | ROMAN BAGPIPE | ANCIENT ROMAN MODE | EMPEROR NERO / PLAYED ONE | No surviving instrument: copy a Roman / Greco-Roman depiction (figurine or relief); keep it simple, no Highland-style drones |
| 1255 ("1250s") | CANTIGAS BAGPIPE | MEDIEVAL CANTIGA | IN ALFONSO X'S / ROYAL SONGBOOK | Copy one Cantigas de Santa Maria miniature (drone count varies between them) |
| 1567 | FLEMISH PIJPZAK | FLEMISH PEASANT DANCE | TWO PIPERS PLAY / BRUEGEL'S WEDDING | Bruegel's Peasant Wedding: single long drone over the shoulder, blowpipe, chanter |
| 1672 | MUSETTE DE COUR | FRENCH BAROQUE GAVOTTE | BELLOWS-BLOWN FOR / FRENCH ARISTOCRATS | Bellows; two chanters (grand + petit chalumeau); short cylindrical "shuttle" drone with sliders (layettes); check bag cover and finish |
| 1775 ("1770s") | UILLEANN PIPES | IRISH JIG | 'UILLEANN' MEANS / 'OF THE ELBOW' | Bellows under the right arm, bag under the left; chanter on the knee; 3 drones + 3 keyed regulators in one stock across the thigh |
| 1805 ("1800s") | NORTHUMBRIAN PIPES | NORTHUMBRIAN RANT | A CLOSED CHANTER: / NOTES CAN STOP | Bellows; small keyed chanter (~7 keys); 4 drones in one common stock |
| 1914 | WARTIME HIGHLAND PIPES | PIPE BAND MARCH | PIPED TROOPS INTO / WWI BATTLES | Great Highland pipes, 2 tenor + 1 bass drone over the left shoulder; bag cover from period photos |
| 1977 | CAMPBELTOWN PIPE BAND | 70s FOLK-ROCK BALLAD | 1ST UK SINGLE TO / SELL 2 MILLION | Highland pipes with tartan cover, drone cords and ribbons |
| 1993 ("1990s") | ELECTRONIC BAGPIPE | CELTIC DANCE POP | HEVIA'S PATENTED / ELECTRONIC PIPES | Copy Hevia's actual electronic instrument from live photos (not a generic practice chanter) |
| 2020s | GREAT HIGHLAND BAGPIPE | BAGPIPE BIG BEAT | PLAYED ON THE ISS / IN NOVEMBER 2015 | Blackwood with mounts, tartan cover, 2 tenor + 1 bass drone, drone cords |

- **All:** count the drones and where they point; mouth-blown vs bellows (musette, uilleann, Northumbrian are bellows); one chanter or two. The three Highland scenes (1914, 1977, modern) must read differently (bag cover, finish, lighting).
- **Riff tones:** a drone sounds under every chanter riff; musette and uilleann sweeter and quieter; Northumbrian short and staccato (audibly different); electronic a clean synth-pipe tone.
- **Outro:** "1,960 YEARS FROM" / "NERO'S BAGPIPE", credit BAGPIPE BIG BEAT
- **Look:** "Standing stones in a storm": a ring of standing stones circles the pedestal on an open moor; sideways rain, racing clouds; lightning cracks on the first downbeat and on every snare roll; carvings on the stones glow with each chanter note. No lake, wave or big mountain range (keeps clear of the harp and ukulele).
  Era palettes (sky + glow): modern storm slate + lightning white · Roman terracotta + olive gold · Cantigas vellum + lapis and gold leaf · Bruegel harvest ochre + russet · musette Versailles pastel blue + rose gold · uilleann emerald + peat brown · Northumbrian heather purple + grey stone · 1914 khaki + poppy red · 1977 Kintyre sunset orange + sea blue · 1990s Asturian green + neon cyan.
  Groove: **big beat**, 100 BPM: chopped breakbeat, pipe-band snare rolls into each scene, distorted bass, a riser before the hook. `LIGHTING = True`. No brand logos or wordmarks on models.
- **Reference photos to find:**
  - Roman: "Roman bagpiper figurine", "tibia utricularis relief"; Met collection 503699
  - Cantigas: Cantigas de Santa Maria bagpipe miniatures (Wikimedia "Cantiga bagpipes")
  - Pijpzak: Bruegel, The Peasant Wedding (Kunsthistorisches Museum, khm.at)
  - Musette de cour: Met, MIM Brussels, Musée de la Musique Paris ("musette de cour")
  - Uilleann: "uilleann pipes full set"
  - Northumbrian: Northumbrian Pipers' Society; "Northumbrian smallpipes"
  - 1914: Imperial War Museum WWI piper photos
  - 1977: Campbeltown Pipe Band / Mull of Kintyre photos; 1970s pipe bands
  - Electronic: "Hevia gaita electrónica" live photos
  - Modern: makers' product pages ("Great Highland bagpipe blackwood")
- **Sources:**
  - Nine-note chanter; 2 tenor + 1 bass drone; continuous airflow, no rests: https://en.wikipedia.org/wiki/Great_Highland_bagpipe
  - Nero and the tibia utricularis (Suetonius, Dio Chrysostom); Cantigas bagpipes (mid-13th c.); WWI piping; Lindgren on the ISS: https://en.wikipedia.org/wiki/Bagpipes · https://www.wunc.org/2015-11-13/astronaut-kjell-lindgren-plays-bagpipes-in-space
  - Cantigas compiled under Alfonso X: https://en.wikipedia.org/wiki/Cantigas_de_Santa_Maria
  - The Peasant Wedding (1567), two pipers on the pijpzak: https://en.wikipedia.org/wiki/The_Peasant_Wedding
  - Musette de cour (bellows, court and aristocracy, Borjon 1672): https://en.wikipedia.org/wiki/Musette_de_cour
  - Uilleann = "of the elbow"; earliest surviving sets late 1700s: https://en.wikipedia.org/wiki/Uilleann_pipes
  - Northumbrian closed chanter, four drones, early-1800s collections: https://en.wikipedia.org/wiki/Northumbrian_smallpipes
  - Mull of Kintyre (1977, Campbeltown Pipe Band, first UK single to sell 2 million): https://en.wikipedia.org/wiki/Mull_of_Kintyre_(song)
  - Hevia team patented electronic bagpipes, early 1990s: https://en.wikipedia.org/wiki/Electronic_bagpipes · https://en.wikipedia.org/wiki/Hevia
- **Production prompt:**

> Make Pixel Pitch P5: the bagpipes. Use the pixel-pitch-history skill and the repo pixelpitch161/reels-toolkit (series/instrument-history/). The approved draft is in STATUS.md under Drafts → "P5. Bagpipes": follow it exactly (hook, headline, all 10 scene rows with their years, names, credits and facts, outro, standing-stones storm background, era palettes, 100 BPM big-beat groove). First claim the P5 row on the ideas board (in production, this session's link, today's date), commit and push.
>
> Must-haves: new script `bagpipes60_lowpoly.py` (OUT "bagpipes60_lowpoly.mp4", TAG "bp"); modern Great Highland bagpipe first, then history oldest to newest (year counter starts at 0 / 60 AD), outro back to TODAY; `LIGHTING = True`; no brand logos on the models; a drone sounds under every scene's riff, and the Northumbrian riff is audibly staccato; lightning on the first downbeat so the opening grabs attention; the three Highland scenes must look different from each other.
>
> Reference photos: before modelling each instrument (the modern one and all 9 historical ones), use my open Chrome browser (Claude in Chrome) to find real reference photos yourself: image searches, museum collection pages, makers' sites. Use the "Reference photos to find" list and model notes in the draft. Check drone count and direction, bellows vs blowpipe, one chanter or two, the uilleann regulators and stock, and the musette's shuttle drone. For the Roman scene, copy an actual ancient depiction. Don't ask me for images.
>
> Before the full render, send me `--still` frames taken mid-scene for every scene (contact sheet is fine) so I can review them, but don't wait for my OK: carry straight on and render with pp_parallel.py, watch it through, send the video, update STATUS.md, commit, git pull --rebase and push.

### P6. Turntables — REVIEW (approved 2026-10-22; rendered `turntables60_lowpoly.py`, 59.8 s)
- **Hook (showcase lines):** "OVER 3 MILLION" / "SOLD SINCE 1972"   then   "BUT HOW DID WE" / "GET HERE?"
- **Headline:** "149 YEARS" (1877 → 2026). `HEAD_INTRO = "THE TURNTABLE"`, `START_YEAR = 1870`
- **Modern showcase:** Technics SL-1200MK7 (matte black), scratch riff over the beat; the record's spin and scratches tie into the music. Credit UK GARAGE + SCRATCH.
- **Length: the finished video must run 60 s (±1 s), final chord included.** Timeline: 130 BPM, `SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1`, 13 scene rows (12 history + the closing modern row) = 31 bars ≈ 57.2 s + final chord ≈ 60 s. If the render lands outside 59–61 s, nudge the BPM (128–132) or the final-chord length; don't drop scenes.
- **Scenes (oldest → newest):**

| Year (ys) | Name | Credit | Fact | Model notes |
|---|---|---|---|---|
| 1877 | EDISON PHONOGRAPH | MARY HAD A LITTLE LAMB | RECORDS AND PLAYS / BACK ON TINFOIL | Hand crank turning a horizontal tinfoil cylinder on a threaded shaft; mouthpiece/diaphragm; no disc, no big horn |
| 1887 | BERLINER GRAMOPHONE | SOUSA-STYLE MARCH | PATENTED: FLAT / DISCS, NOT TUBES | Small hand-rotated turntable (no spring motor until 1896), small horn |
| 1906 | VICTROLA | RAGTIME | THE HORN HIDES / IN THE CABINET | 1906 "VTLA" tall upright cabinet with doors where the horn would be; NO external horn |
| 1948 | LP RECORD PLAYER | LATE-40s BIG BAND | THE LP ARRIVES, / SPINNING AT 33 1/3 | 1948 Columbia/Philco LP player; check shape from photos |
| 1950s (1955) | JAMAICAN SOUND SYSTEM | JAMAICAN R&B SHUFFLE | WARDROBE-SIZED / SPEAKER BOXES | Turntable on or beside giant homemade speaker boxes ("House of Joy") |
| 1972 | TECHNICS SL-1200 | PHILLY SOUL | MAGNET UNDER THE / PLATTER DRIVES IT | Silver, direct drive; original has a ROTARY pitch knob (no slider); check strobe dots and tonearm |
| 1973 | KOOL HERC'S TWIN DECKS | FUNK BREAKBEAT | TWO DECKS STRETCH / THE BREAKBEAT | Two turntables + Shure "Vocal Master" PA columns |
| 1975 | THE FIRST SCRATCH | BRONX B-BOY BREAK | A 12-YEAR-OLD / INVENTS SCRATCHING | Deck with slipmat; a stylised hand on the record is optional |
| 1976 | 12-INCH SINGLE | DISCO | WIDER GROOVES, / LOUDER DISCO | Disco-booth deck playing a 12-inch record, its big plain sleeve leaning beside it; must read differently from the SL-1200 scenes |
| 1979 | TECHNICS SL-1200MK2 | ELECTRO-FUNK | PITCH SLIDER: / THE DJ STANDARD | Silver; pitch slider on the right |
| 1994 | PIONEER CDJ-500 | 90s TRANCE | CDS GET A JOG / DIAL FOR CUEING | Top-loading CD player with a jog dial; clearly not a turntable |
| 2001 | FINAL SCRATCH | MINIMAL TECHNO | TIMECODE RECORDS / PLAY DIGITAL FILES | Two turntables with timecode records, round aluminium "ScratchAmp" box, a laptop |
| 2020s | TECHNICS SL-1200MK7 | UK GARAGE + SCRATCH | DISCONTINUED 2010, / BACK BY 2016 | Matte black |

- **Riff tones:** each riff sounds as if played through that machine: tinny and crackly phonograph; horn-filtered gramophone and Victrola; heavy bass through the 1950s speaker boxes; real scratch sounds layered from 1975 on.
- **Outro:** "149 YEARS FROM" / "EDISON'S TINFOIL", credit UK GARAGE + SCRATCH
- **Look:** "On a giant spinning record": the pedestal sits on the label of a huge spinning record; the grooves glint as rings around it; a giant tonearm sweeps overhead; a mirror ball throws light spots on each beat; the label art changes per era.
  Era palettes (record + accent): modern black vinyl + gold · 1877 tinfoil silver + brass · 1887 hard-rubber brown + cream · 1906 mahogany + gold leaf · 1948 red label + cream · 1950s Jamaican green and gold + sunset · 1972 orange + brown · 1973 Bronx brick + graffiti teal · 1975 purple + yellow · 1976 disco silver + hot orange · 1979 chrome + red · 1994 rave lime + UV purple · 2001 aluminium + blue LEDs. No pink/cyan neon (keeps clear of the electric guitar's synthwave).
  Groove: **UK garage 2-step**, 130 BPM: skippy kick, shuffled hats, a snare that lands late, sub bass, chopped stabs, a scratch fill at every scene change. `LIGHTING = True`. No brand logos or wordmarks on models.
- **Reference photos to find:**
  - Edison tinfoil phonograph: Smithsonian; Thomas Edison National Historical Park
  - Berliner gramophone: Smithsonian music.si.edu; Library of Congress Emile Berliner collection
  - Victrola: victor-victrola.com (1906 VTLA Pooley cabinet)
  - 1948 LP player: "1948 Columbia LP record player Philco"
  - Sound system: 1950s Kingston photos (Duke Reid, Coxsone Dodd, "House of Joy" speakers)
  - SL-1200 / MK2 / MK7: Technics product pages and archive photos
  - Herc's twin decks: 1970s Bronx party photos; Shure Vocal Master PA
  - First scratch: Grand Wizzard Theodore photos
  - 12-inch single: 1970s disco DJ booth photos; Salsoul 12-inch sleeves (plain, no logos)
  - CDJ-500: Pioneer archive / vintage listings
  - Final Scratch: ScratchAmp and timecode-vinyl photos
- **Sources:**
  - SL-1200 (Oct 1972, direct drive; MK2 1979 pitch slider, DJ standard; 3 million+ sold; discontinued 2010, revived 2016; MK7 2019): https://en.wikipedia.org/wiki/Technics_SL-1200
  - Edison 1877 tinfoil phonograph: https://en.wikipedia.org/wiki/Phonograph
  - Berliner patent Nov 1887; hand-rotated until 1896: https://en.wikipedia.org/wiki/Berliner_Gramophone
  - First Victrola, internal horn, Aug 1906 (VTLA): https://victor-victrola.com/timeline.htm
  - Columbia LP, 21 June 1948: https://en.wikipedia.org/wiki/Gramophone_record
  - Jamaican sound systems, "House of Joy" speakers: https://en.wikipedia.org/wiki/Sound_system_(Jamaican)
  - Kool Herc, 11 Aug 1973, two turntables, breakbeat DJing: https://en.wikipedia.org/wiki/DJ_Kool_Herc
  - Grand Wizzard Theodore, scratch 1975, aged about 12: https://en.wikipedia.org/wiki/Grand_Wizzard_Theodore
  - 12-inch single (wider grooves, louder cut; first commercial 1976): https://en.wikipedia.org/wiki/12-inch_single
  - CDJ-500 (1994, jog dial for cueing): https://en.wikipedia.org/wiki/CDJ
  - Final Scratch (2001, first DVS sold publicly; timecode records): https://en.wikipedia.org/wiki/Digital_vinyl_system · https://en.wikipedia.org/wiki/Final_Scratch
- **Production prompt:**

> Make Pixel Pitch P6: the turntables. Use the pixel-pitch-history skill and the repo pixelpitch161/reels-toolkit (series/instrument-history/). The approved draft is in STATUS.md under Drafts → "P6. Turntables": follow it exactly (hook, headline, all 13 scene rows with their years, names, credits and facts, outro, giant-spinning-record background, era palettes, 130 BPM UK garage 2-step groove). First claim the P6 row on the ideas board (in production, this session's link, today's date), commit and push.
>
> Must-haves: the finished video must run 60 s (±1 s), final chord included; if it doesn't, nudge the BPM (128–132) or the final-chord length rather than dropping scenes. New script `turntables60_lowpoly.py` (OUT "turntables60_lowpoly.mp4", TAG "tt"); modern Technics SL-1200MK7 first, then history oldest to newest, outro back to TODAY; `LIGHTING = True`; no brand logos on the models or record labels; each riff sounds as if played through that era's machine, with real scratch sounds from 1975 on and a scratch fill at every scene change.
>
> Reference photos: before modelling each machine (the modern one and all 12 historical ones), use my open Chrome browser (Claude in Chrome) to find real reference photos yourself: image searches, museum collection pages, makers' sites. Use the "Reference photos to find" list and model notes in the draft. Check: the Edison phonograph is a tinfoil cylinder (no disc); the Victrola hides its horn in the cabinet; the 1972 SL-1200 has a rotary pitch knob while the MK2 has the slider; the CDJ-500 is clearly a CD player; the SL-1200 scenes must each look distinct. Don't ask me for images.
>
> Before the full render, send me `--still` frames taken mid-scene for every scene (contact sheet is fine) so I can review them, but don't wait for my OK: carry straight on and render with pp_parallel.py, check the runtime is 60 s, watch it through, send the video, update STATUS.md, commit, git pull --rebase and push.

### P7. Pipe organ — REVIEW (approved 2026-10-22, rendered 2026-10-22)
- **Hook (showcase lines):** "ONE ORGAN PIECE" / "LASTS 639 YEARS"   then   "BUT HOW DID WE" / "GET HERE?"
- **Headline:** "2,275 YEARS" (250 BC → 2026). `HEAD_INTRO = "THE PIPE ORGAN"`, `START_YEAR = -300`
- **Modern showcase:** the Halberstadt Cage organ (St. Burchardi church, Halberstadt), which has been playing John Cage's *ORGAN²/ASLSP* since 2001 and is still playing today, so the hook is about the instrument on screen. Riff: organ techno. Credit ORGAN TECHNO.
- **Length: the finished video must run 60 s (±1 s), final chord included.** Timeline: 120 BPM, `SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1`, 12 scene rows (11 history + the closing modern row) = 29 bars = 58 s + final chord ≈ 60 s. If the render misses, nudge the BPM or the final-chord length; don't drop scenes.
- **Scenes (oldest → newest):**

| Year (ys) | Name | Credit | Fact | Model notes |
|---|---|---|---|---|
| -250 ("250 BC") | HYDRAULIS | ANCIENT GREEK MODE | WATER KEEPS ITS / AIR PRESSURE EVEN | Aquincum replica / Dion reconstruction: water-cistern base, a hand pump on each side, one row of bronze pipes |
| 1330 ("1300s") | PORTATIVE ORGAN | MEDIEVAL ESTAMPIE | ONE HAND PLAYS, / ONE HAND PUMPS | Small, carried on a strap; keys for one hand, bellows on the back for the other |
| 1361 | HALBERSTADT ORGAN | GOTHIC ORGANUM | FIRST PERMANENT / ORGAN ON RECORD | Copy Praetorius's woodcut of the Halberstadt keyboards (very broad, primitive keys) |
| 1435 | VALÈRE ORGAN, SION | RENAISSANCE MOTET | OLDEST ORGAN / STILL PLAYABLE | Swallow's-nest organ in the Valère Basilica; check painted wings and gallery from photos |
| 1703 | BACH'S ARNSTADT ORGAN | BAROQUE TOCCATA | BACH, AGED 18, / HIRED TO PLAY IT | Wender organ, Bachkirche Arnstadt: two manuals, 21 stops, high on the third gallery tier |
| 1862 | SAINT-SULPICE ORGAN | FRENCH ORGAN TOCCATA | WIDOR PLAYS IT / FOR 63 YEARS | Huge Chalgrin case with statues; five manuals |
| 1920s (1925) | MIGHTY WURLITZER | SILENT-FILM CHASE | CAR HORNS, DRUMS / FOR SILENT FILMS | Horseshoe console with rows of coloured stop tabs; pipes hidden in chambers, so show the console plus a glockenspiel or drum |
| 1932 | BOARDWALK HALL ORGAN | ART DECO SHOWTUNE | 33,112 PIPES: / LOUDEST EVER BUILT | The 7-manual console, 1,235 stop tabs |
| 1935 | HAMMOND ORGAN | GOSPEL | 50,000 CHURCHES / HAD ONE BY 1966 | Model A: two 61-key manuals, 25-note pedalboard, flat "waterfall" keys, drawbars; no pipes |
| 1979 | SYDNEY OPERA HOUSE | 70s PROG ROCK | LARGEST TRACKER / ORGAN EVER BUILT | Grand Organ façade in the Concert Hall |
| 2016 | ELBPHILHARMONIE ORGAN | CONTEMPORARY CLASSICAL | 4,765 PIPES, / MOSTLY HIDDEN | Built into the Grand Hall's seating rows; only part of it visible |
| 2020s | HALBERSTADT CAGE ORGAN | ORGAN TECHNO | SANDBAGS HOLD / THE NOTES DOWN | Small organ in an acrylic glass box (since 2011), sandbags on the pedals; bellows in the other transept |

- **Riff tones:** each scene in that organ's own voice: hydraulis breathy; portative a small flute; Bach full baroque plenum; Saint-Sulpice French reeds; Wurlitzer tremulant tibias plus a car horn; Hammond drawbars through a Leslie; the Cage organ a slowly shifting cluster chord while the beat keeps going.
- **Outro:** "2,275 YEARS FROM" / "A WATER ORGAN", credit ORGAN TECHNO
- **Look:** "A limestone cavern": stalactites hang like organ pipes and glow with each note; mist drifts; drips land on the hi-hats; shafts of light come down from above. No real organ pipes on any wall (keeps clear of the French horn's concert-hall organ).
  Era palettes (stone + glow): modern acrylic-clear white + pale green · hydraulis ochre + aqua · portative parchment + vermilion · 1361 Gothic slate + candle amber · Sion alpine grey + fresco red · Bach baroque white + gilt · Saint-Sulpice Paris blue-grey + gaslight · Wurlitzer cinema red velvet + marquee gold · 1932 Art Deco teal + chrome · Hammond church oak + stained-glass jewels · Sydney harbour blue + sail white · Elbphilharmonie white limestone + ice blue.
  Groove: **industrial techno**, 120 BPM: distorted rumble kick, metallic anvil hits, choir pad, organ stabs; no claps, open hats or piano (keeps clear of the tuba's brass house). `LIGHTING = True`. No brand logos or wordmarks on models.
- **Reference photos to find:**
  - Hydraulis: Aquincum Museum (Budapest) working replica; Dion hydraulis reconstruction
  - Portative: "portative organ medieval"; Memling's angel musicians
  - Halberstadt 1361: Praetorius, Syntagma Musicum woodcut (sourcelibrary.org)
  - Valère, Sion: Valère Basilica organ photos
  - Arnstadt: Bachkirche Arnstadt organ photos
  - Saint-Sulpice: organ façade photos
  - Mighty Wurlitzer: theatre-organ console photos
  - Boardwalk Hall: 7-manual console photos
  - Hammond Model A: vintage listings and archive photos
  - Sydney Opera House: Grand Organ photos
  - Elbphilharmonie: Klais / Elbphilharmonie organ photos
  - Halberstadt Cage organ: St. Burchardi "ASLSP organ" photos (acrylic box, sandbags)
- **Sources:**
  - Halberstadt ASLSP (began 5 Sept 2001, ends 2640, 639 years; sandbags; acrylic box since 2011; the 639 years count from the 1361 cathedral organ): https://en.wikipedia.org/wiki/As_Slow_as_Possible · https://www.classicfm.com/composers/cage/as-slow-a-possible-aslsp-germany-organ-chord-change/
  - Hydraulis (Ctesibius, 3rd c. BC; water steadies the air pressure; Aquincum replica): https://en.wikipedia.org/wiki/Hydraulis
  - Portative (one hand plays, the other pumps); Halberstadt 1361, first documented permanent organ: https://en.wikipedia.org/wiki/Pipe_organ · https://www.orgelbau-huefken.de/en/special/organ-town.html
  - Valère organ, 1435, oldest playable: https://swissinfo.ch/eng/culture/swiss-music-lovers-to-hear-oldest-playable-organ-in-the-world/48666122
  - Arnstadt organ (Wender, 1703), Bach hired aged 18: https://en.wikipedia.org/wiki/Bach_Church,_Arnstadt
  - Saint-Sulpice (Cavaillé-Coll, 1862, 102 stops, five manuals): https://en.wikipedia.org/wiki/Aristide_Cavaill%C3%A9-Coll · Widor 1870–1933, 63 years: https://www.aross.fr/en/the-great-organ/
  - Wurlitzer theatre organs (1911–1942; car horns, drums for silent films): https://en.wikipedia.org/wiki/Theatre_organ
  - Boardwalk Hall (33,112 pipes; Guinness largest and loudest; 7 manuals; 1929–32): https://en.wikipedia.org/wiki/Boardwalk_Hall_Auditorium_Organ
  - Hammond (1935; ~50,000 churches by 1966): https://en.wikipedia.org/wiki/Hammond_organ
  - Sydney Opera House Grand Organ (1979; largest tracker organ ever built): https://en.wikipedia.org/wiki/Sydney_Opera_House_Grand_Organ
  - Elbphilharmonie organ (4,765 pipes; 2016; Klais; mostly hidden; built into the seating): https://www.elbphilharmonie.de/en/mediatheque/elbphilharmonie-organ-the-pipe-cleaners/494
- **Production prompt:**

> Make Pixel Pitch P7: the pipe organ. Use the pixel-pitch-history skill and the repo pixelpitch161/reels-toolkit (series/instrument-history/). The approved draft is in STATUS.md under Drafts → "P7. Pipe organ": follow it exactly (hook, headline, all 12 scene rows with their years, names, credits and facts, outro, limestone-cavern background, era palettes, 120 BPM industrial-techno groove). First claim the P7 row on the ideas board (in production, this session's link, today's date), commit and push.
>
> Must-haves: the finished video must run 60 s (±1 s), final chord included; if it doesn't, nudge the BPM or the final-chord length rather than dropping scenes. New script `organ60_lowpoly.py` (OUT "organ60_lowpoly.mp4", TAG "po"); the Halberstadt Cage organ is the modern showcase (hook "ONE ORGAN PIECE / LASTS 639 YEARS") and the closing scene, with history oldest to newest in between; `LIGHTING = True`; no brand logos on the models; each scene's riff in that organ's own voice; no organ pipes in the background (it must not look like the French horn's concert hall).
>
> Reference photos: before modelling each instrument (the showcase and all 11 historical ones), use my open Chrome browser (Claude in Chrome) to find real reference photos yourself: image searches, museum collection pages, makers' sites. Use the "Reference photos to find" list and model notes in the draft. Check: the hydraulis's water cistern and pumps; the portative's bellows; copy Praetorius's woodcut for 1361; the Hammond has no pipes; the Cage organ sits in an acrylic box with sandbags on the pedals. The big organs (Saint-Sulpice, Boardwalk, Sydney, Elbphilharmonie) must each read differently, e.g. façade vs console. Don't ask me for images.
>
> Before the full render, send me `--still` frames taken mid-scene for every scene (contact sheet is fine) so I can review them, but don't wait for my OK: carry straight on and render with pp_parallel.py, check the runtime is 60 s, watch it through, send the video, update STATUS.md, commit, git pull --rebase and push.

### P8. Mellophone — REVIEW (approved 2026-10-23, rendered 2026-10-23)
- **Hook (showcase lines):** "HALF THE TUBING" / "OF A FRENCH HORN"   then   "BUT HOW DID WE" / "GET HERE?"
- **Headline:** "181 YEARS" (1845 → 2026). `HEAD_INTRO = "THE MELLOPHONE"`, `START_YEAR = 1840`
- **Modern showcase:** Yamaha YMP-204M marching mellophone in F, dark drumline riff. Credit DARK DRUMLINE.
- **Length: the finished video must run 61 s (60.5–61.5 s), final chord included, and loop cleanly.** Timeline: 111 BPM, `SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1`, 11 scene rows (10 history + the closing modern row) = 27 bars ≈ 58.4 s + final chord ≈ 61 s. If the render misses, nudge the BPM or the final-chord length; don't drop scenes.
- **Scenes (oldest → newest):**

| Year (ys) | Name | Credit | Fact | Model notes |
|---|---|---|---|---|
| 1845 | ALTO SAXHORN | PARISIAN POLKA | SAX PATENTS 7 / SAXHORN SIZES | Sax-made E♭ alto: bell UP, three valves |
| 1855 | KOENIG HORN | SECOND EMPIRE WALTZ | 1855 PARIS EXPO: / FIRST-CLASS MEDAL | Courtois (MIMEd 15365): circular, bell DOWN, three Périnet valves |
| 1860s (1862) | OVER-THE-SHOULDER HORN | CIVIL WAR QUICKSTEP | BELL POINTS BACK / FOR TROOPS BEHIND | Bell points backward over the left shoulder; check rotary vs piston valves from photos |
| 1880s (1885) | E-FLAT TENOR HORN | BRASS BAND HYMN | IN BRITAIN IT'S / THE 'TENOR HORN' | Upright bell, three pistons (Besson-style) |
| 1925 | CONCERT MELLOPHONE | 1920s DANCE BAND | 4 SWAP SLIDES: / F, E-FLAT, D OR C | Conn 4-E Wonder (NMM 12013): double-loop circular wrap, 12-inch bell pointing back-left, front-action pistons, swap slides |
| 1960 | CONN MELLOPHONIUM | KENTON BIG BAND | KENTON'S SECTION / WINS 2 GRAMMYS | Conn 16E: bell forward with a horn-like flare |
| 1968 | PISTON-ROTOR BUGLE | SCOUT CORPS FANFARE | KANSTUL'S G-F / PISTON-ROTOR BUGLE | G mellophone bugle with one piston + one rotor |
| 1969 | MARCHING MELLOPHONE | 60s MARCHING BAND | FIRST BELL-FRONT / MARCHING BRASS | Olds bell-front marching mellophone |
| 1977 | TWO-VALVE G BUGLE | 70s DRUM CORPS JAZZ-ROCK | DRUM CORPS SWITCH / TO TWO VALVES | G mellophone bugle with two upright valves |
| 2000 | F MELLOPHONE | DRUM CORPS OPENER | DCI DROPS ITS / G-ONLY RULE | Three-valve F marching mellophone (drum-corps era) |
| 2020s | YAMAHA YMP-204M | DARK DRUMLINE | SAME FINGERINGS / AS A TRUMPET | Bell-front, 10-inch bell, three pistons |

- **Bell direction tells the story:** up (saxhorn, tenor horn), down (Koenig), backward over the shoulder (Civil War), back-left (concert mellophone), forward (1960 onward). The bell-front horns (1960, 1968, 1969, 1977, 2000, modern) must be easy to tell apart: valve count (piston + rotor, two upright, three), finish, and era palette.
- **Outro:** "181 YEARS FROM" / "SAX'S ALTO HORN", credit DARK DRUMLINE
- **Look:** "Inside a giant brass clockwork": huge brass gears turn on the beat, pistons pump in time with the notes, steam vents hiss on the snare hits; low, coppery light.
  Era palettes (metal + glow): modern gunmetal + amber · 1845 brass + oxblood · 1855 Second Empire green + gold · 1860s Union blue + gunsmoke grey · 1880s colliery soot + band-uniform red · 1925 silver + champagne · 1960 midnight blue + spotlight white · 1968 scout khaki + orange · 1969 field green + white · 1977 burnt orange + chrome · 2000 silver + electric blue.
  Groove: **dark drumline**, 111 BPM, brooding and tense (not upbeat): snare-line cadence with rimshots, tenor-drum runs, split bass drums, halftime 808 sub, minor-key pad. `LIGHTING = True`. No brand logos or wordmarks on models.
- **Reference photos to find:**
  - Alto saxhorn: Sax E♭ altos (Met, MIM Brussels, MIMEd)
  - Koenig horn: MIMEd record 15365; robbstewart.com Courtois page
  - Over-the-shoulder horn: Civil War band saxhorns (Smithsonian, National Music Museum)
  - Tenor horn: Besson E♭ tenor horn
  - Concert mellophone: NMM Conn 4-E (emuseum.nmmusd.org/objects/12013), Conn 7-E (objects/9029)
  - Mellophonium: Conn 16E photos; Stan Kenton band photos
  - Bugles: "G mellophone bugle piston rotor", "two valve G mellophone bugle"
  - Marching mellophone: "Olds marching mellophone 1969"
  - YMP-204M: Yamaha product page
- **Sources:**
  - F mellophone ≈ half a French horn's tubing; trumpet fingerings; Koenig horn as earliest version; Conn mellophonium 1957; Kenton section 1960–63, 11 albums, 2 Grammy winners: https://en.wikipedia.org/wiki/Mellophone · https://en.wikipedia.org/wiki/Mellophonium
  - Saxhorn family patented 1845, seven sizes; over-the-shoulder bells for the troops behind: https://en.wikipedia.org/wiki/Saxhorn
  - Koenig horn (Courtois; 1855 Paris Expo first-class medal): https://collections.ed.ac.uk/mimed/record/15365 · https://www.robbstewart.com/courtois-melody-horn
  - E♭ "tenor horn" in British brass bands: https://en.wikipedia.org/wiki/Tenor_horn
  - Conn 4-E concert mellophone, 1925, F/E♭/D/C slides: https://emuseum.nmmusd.org/objects/12013/mellophone-f-eflat-d-c-low-pitch
  - Kanstul G-F piston-rotor bugle (1968); two-valve bugles (1976 vote, 1977 switch): https://www.dci.org/?p=37817 · https://legacy.kanstul.com/wp-content/uploads/2017/08/marching-brass-history-brochure.pdf
  - Olds first bell-front Marching Brass incl. mellophone (1969): https://legacy.kanstul.com/wp-content/uploads/2017/08/marching-brass-history-brochure.pdf
  - DCI G-only rule ended 1999, World Class adopted 2000: https://en.wikipedia.org/wiki/Drum_and_bugle_corps_(modern)
  - YMP-204M specs: https://usa.yamaha.com/products/musical_instruments/winds/marching_brass/ymp-204m/specs.html
- **Production prompt:**

> Make Pixel Pitch P8: the mellophone. Use the pixel-pitch-history skill and the repo pixelpitch161/reels-toolkit (series/instrument-history/). The approved draft is in STATUS.md under Drafts → "P8. Mellophone": follow it exactly (hook, headline, all 11 scene rows with their years, names, credits and facts, outro, brass-clockwork background, era palettes, 111 BPM dark-drumline groove). First claim the P8 row on the ideas board (in production, this session's link, today's date), commit and push.
>
> Must-haves: the finished video must run 61 s (60.5–61.5 s), final chord included, and loop cleanly (the loop restarts at the end of the file, no dead air); if the length misses, nudge the BPM or the final-chord length rather than dropping scenes. New script `mellophone60_lowpoly.py` (OUT "mellophone60_lowpoly.mp4", TAG "mp"); modern Yamaha YMP-204M first, then history oldest to newest, outro back to TODAY; `LIGHTING = True`; no brand logos on the models; the music mood is dark and tense, not upbeat; each scene's riff in its credited style.
>
> Reference photos: before modelling each instrument (the modern one and all 10 historical ones), use my open Chrome browser (Claude in Chrome) to find real reference photos yourself: image searches, museum collection pages, makers' sites. Use the "Reference photos to find" list and model notes in the draft. Check the bell direction in every era (up, down, backward over the shoulder, back-left, forward), and make the bell-front horns distinct by valve count (piston + rotor, two upright, three) and finish. Don't ask me for images.
>
> Before the full render, send me `--still` frames taken mid-scene for every scene (contact sheet is fine) so I can review them, but don't wait for my OK: carry straight on and render with pp_parallel.py, check the runtime is 61 s and the loop point is clean, watch it through, send the video, update STATUS.md, commit, git pull --rebase and push.
