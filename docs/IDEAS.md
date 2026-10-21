# Ideas & status board

This is the single place that tracks every reel. The ideas chat adds and refines ideas here.
Each production session updates its own row on the status board: when it starts, when it sends
a draft, when it gets feedback, and when it delivers.

**Statuses:** `idea` → `approved` → `in production` → `review` → `done` (or `parked`)

## Status board

| # | Video | Status | Session | Last feedback / next step | Output |
|---|---|---|---|---|---|
| 1 | 100 Metronomes Find the Beat | done | original chat | Delivered with caption + tags | `100_metronomes.mp4` |
| 3 | Crickets Are Thermometers | done | original chat | Final pick: brighter diorama version (v4); the moody version read as too dark; focus locked on the cricket + thermometer | `cricket_diorama_v4.mp4` (also `cricket_thermometer.mp4`, open-environment cut) |
| 5 | Sand That Draws Sound | done | original chat | v2: new lighting/shadows, richer courtyard, sand rattle cut about 24 dB. Next: confirm the sand sound level is right, or remove it completely | `sand_draws_sound_v2.mp4` |
| 2 | The Lowest Note in the Universe | approved | ideas chat | Fully developed (beat sheet + sources in backlog). Next: claim it in a production session | - |
| 4 | The Staircase That Never Ends | idea | - | - | - |
| 6 | One String, Every Note | idea | - | - | - |
| 7 | Mozart's Dice Waltz | idea | - | - | - |
| 8 | Dinosaur Brass Band | idea | - | top-3 pick from the original research | - |
| 9 | The 639-Year Song | idea | - | Copyright: don't use Cage's actual notes (still in copyright); use a generic held chord | - |
| 10 | One Melody, 100 Years of Speakers | idea | - | - | - |

## Backlog (idea details)

Research basis: a study of @bit_wonder (Oct 2026). Their top reels are all science with a hard number in
the hook, for example nuclear fission (6.8M views) and hummingbird vs whale heartbeat (5.1M). Their music-only
reels do far less. **Lead with a surprising real fact; make the music the payoff.**

### 2. The Lowest Note in the Universe — APPROVED
- **Hook:** "57 OCTAVES DOWN" (in the first 2 s). Alt: "SPACE HAS A SOUND".
- **Science:** jets from the supermassive black hole in the Perseus cluster (~250M light-years away) blow
  bubbles in the cluster's hot gas. Each bubble pushes out like a piston and launches a pressure wave; bubbles
  form at regular intervals, so the waves make a steady tone: a B♭ ~57 octaves below the B♭ above middle C.
  Crests arrive just under 10 million years apart (calc: ≈9.8M yr). Found in Chandra X-ray data (observed 2002,
  announced 2003, Andrew Fabian's team). Space is mostly vacuum, but the cluster's hot gas carries sound.
  Optional: the note has held roughly steady for ~2.5 billion years.
  Sources: [NASA Imagine (2003)](https://imagine.gsfc.nasa.gov/news/12sep03.html),
  [Chandra Chronicle](https://chandra.harvard.edu/chronicle/0303/perseus/),
  [NASA sonifications (2022)](https://science.nasa.gov/learners/highlights/data-sonifications).
- **Music:** an "OCTAVE −57" counter climbs; each step doubles the frequency (B♭4 = 466.16 Hz ÷ 2^k).
  **Rhythm turns into pitch:** at −9 a crest comes once a second → each ring is an audible kick (~55 BPM),
  doubling to 109 then 219 BPM, blurring into a flutter, and at −4 (29 Hz) it is heard as a pitch (B♭0).
  Each further octave adds a layer (B♭0 → B♭4) under an original B♭-major melody. All synthesised/original.
- **Scene:** low-poly galaxy cluster diorama: bright core with jets, bubbles inflating, concentric pressure
  rings in dithered hot-gas haze, small drifting galaxies. Key light = core/jets; small Chandra-style satellite
  in the foreground for scale, a lit surface and warm local glow. Escalation: 1 ring per 10M years → per year
  → per day → per second → buzz.
- **Payoff / ending:** every octave at once, every ring and galaxy pulsing on the beat as the melody lands on
  B♭4 (not middle C). Then "HUM A B♭", music decays to one hum, counter rewinds, rings slow, loop to opening.

**Milestones (calc):** −57 ≈ 1 beat / 9.8M yr · −34 ≈ 1 / 1.2 yr · −25 ≈ 1 / 20 h · −21 ≈ 1 / 75 min ·
−15 ≈ 1 / 70 s · −9 = 0.91 Hz (55 BPM) · −8 109 BPM · −7 219 BPM · −6 7.3 Hz · −5 14.6 Hz · −4 29.1 Hz (B♭0).

**Beat sheet (60 s):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–4 | Glowing cluster, one ring crawling out | 57 OCTAVES DOWN | Near-silence, faint air |
| 4–8 | Push in to core; jets fire, bubble inflates | A BLACK HOLE / IS HUMMING | Soft swell |
| 8–11 | Hot-gas haze lights up | SPACE IS SILENT… / EXCEPT HERE | — |
| 11–14 | Bubble pops, ring launches (piston) | EACH BUBBLE = 1 WAVE | Thump |
| 14–17 | Counter panel slides up from bottom | OCTAVE −57 / 1 BEAT / 10M YRS | — |
| 17–29 | Rings tighten, caption every 3 s | −34 ≈1/YEAR · −25 ≈1/DAY · −21 ≈1/HOUR · −15 ≈1/MINUTE | Rising riser |
| 29–32 | Each ring flashes | −9: 1 / SECOND | Kick 55 BPM |
| 32–35 | Flashes double, double again | −8, −7 | 109 → 219 BPM |
| 35–38 | Rings blur to shimmer | BEATS → ? | Flutter 7 → 15 Hz |
| 38–44 | Rings become colour bands per octave | IT'S A NOTE NOW | B♭0 drone, octaves stack |
| 44–52 | Climax: every ring and galaxy pulses | B♭ | Full stack + original melody |
| 52–60 | Pull back, counter rewinds, loop to opening | HUM A B♭ / YOU'RE 57 OCTAVES UP | Decays to one hum |

**Flags:** never say NASA "recorded" it (it's inferred from X-ray ripples): use "humming"/"makes a note".
Chandra says 57 octaves; NASA 2022 says 57–58: use 57. Rhythm → pitch at ~20 Hz is standard psychoacoustics.

**Feasibility:** rings (flat tori), bubbles, jets (cones), counter HUD and banded haze are all easy; no fluids
or characters. Hard part is the playbook's shadow/"small life" rules in empty space. Fallback: sit the cluster
on a dark diorama disc/plinth so shadows and contact AO have somewhere to land. Lock focus on core + inner rings
with the counter panel inside the sharp band.

### 4. The Staircase That Never Ends
- **Hook:** "ALWAYS GOING UP?"
- **Science:** the Shepard tone, an illusion of a pitch that rises forever.
- **Music:** coloured bars for each tone layer fade in at the bottom and out at the top. It loops seamlessly.
- **Scene:** a character climbing an endless (Penrose-style) staircase.

### 6. One String, Every Note
- **Hook:** "1 STRING, 16 NOTES"
- **Science:** a string vibrates in 1, 2, 3, 4… segments, and each overtone climbs a glowing ladder. Stacked overtones form a major chord.
- **Music:** payoff: a bugle can only play that ladder, which is why Taps (1862, public domain) uses four notes.

### 7. Mozart's Dice Waltz
- **Hook:** "46 QUADRILLION WALTZES"
- **Science:** the musical dice game attributed to Mozart: 16 rolls of two dice, each picking one bar from a table, gives 11¹⁶ ≈ 46 quadrillion waltzes.
- **Music:** dice tumble, each roll drops a bar onto a staff, a counter runs, and the waltz builds.

### 8. Dinosaur Brass Band
- **Hook:** "A DINOSAUR TROMBONE"
- **Science:** Parasaurolophus's hollow crest worked like a brass instrument; its low honk has been simulated from CT scans.
- **Music:** built like the heartbeat-band reel: each dinosaur adds a part, and the crest-honkers carry the bass line.

### 9. The 639-Year Song
- **Hook:** "ENDS IN 2640"
- **Science:** an organ in Halberstadt has played one extremely slow piece since 2001, and it ends in 2640.
- **Scene:** a fixed camera on a church while centuries rush past outside the window, with a year counter racing forward.
- **Copyright:** the piece itself (Cage) is still in copyright, so use a generic sustained chord.

### 10. One Melody, 100 Years of Speakers
- **Hook:** "1925 → 2025"
- **Science:** one static room and one public-domain melody, played in turn by a gramophone, radio, cassette player, Game Boy and earbuds. The sound quality changes to match each era.
- **Style:** a story-driven piece.

## Adding a new idea

Copy this block into the backlog and add an `idea` row to the board:

```
### N. Title
- **Hook:** "BIG NUMBER OR QUESTION" (in the first 2 s)
- **Science:** the real, checkable fact (with a source)
- **Music:** how the science drives tempo, pitch or rhythm, and which music (original or public domain)
- **Scene:** the environment, the subject, and what escalates
- **Payoff / ending:** the "all at once" climax, then a loop back to the opening
```
