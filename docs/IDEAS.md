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
| 2 | The Lowest Note in the Universe | in production | session_01WcqqR7 (2026-10-21) | Claimed; building in `projects/lowest_note/` | - |
| 4 | The Staircase That Never Ends | idea | - | - | - |
| 6 | One String, Every Note | idea | - | - | - |
| 7 | Mozart's Dice Waltz | in production | session_01UAUgj8 (2026-10-21) | Claimed; building in `projects/dice_waltz/`. Verifying column harmony from the table first | - |
| 8 | Dinosaur Brass Band | approved | ideas chat | Fully developed (beat sheet + sources in backlog). Mechanism: crest size sets pitch. Hardest build (dinosaurs): see fallback | - |
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

### 7. Mozart's Dice Waltz — APPROVED
- **Hook:** "759 TRILLION" / "WALTZES" (2 lines, in the first 2 s). Alt: "2 DICE, 1 WALTZ".
  (Not "46 quadrillion": 11¹⁶ counts roll sequences, but bar 8 has 1 real option and bar 16 has 2.)
- **Science (probability):** a 1792 Berlin game (Simrock) gives a table of 176 one-bar fragments. Roll two
  dice for each of 16 bars; the sum (2–12) picks one of 11 options. Distinct waltzes: 2 × 11¹⁴ =
  759,499,667,166,482. Two dice are unfair pickers: a 7 (6/36) is 6× more likely than a 2 or 12 (1/36).
  Hearing every waltz at ~30 s each would take ~720 million years (calc).
  Sources: [Wikipedia](https://en.wikipedia.org/wiki/Musikalisches_W%C3%BCrfelspiel),
  [Science News](https://sciencenews.org/article/mozarts-melody-machine-0),
  [Donald Sauter](https://www.oocities.org/donaldsauter/mozart-dice-game.htm),
  [CCARH kern data](https://www.ccarh.org/courses/253/lab/kerndice).
- **Music:** the dice literally compose the track: each roll's sum selects the next bar, which drops onto the
  staff and plays on a synth fortepiano at waltz tempo. Music = the 1792 table (public domain). Back half:
  more cups join, their waltzes layered on different timbres (fortepiano, music box, pizzicato, flute).
- **Scene:** low-poly 18th-century Viennese salon diorama: moonlit window (key light), candelabras (warm
  local lights), self-playing fortepiano, dice tray, giant parchment staff above it. Small life: curtains,
  candle flicker, maybe a cat. Escalation: 1 cup → 2 → 10 → dice rain; a sum histogram on the wall grows
  into the 2–12 triangle.
- **Payoff / ending:** all cups roll at once, the histogram snaps to a perfect triangle and the waltzes play
  together. "ROLL 2 DICE": dice settle into one cup, loop to the opening clack.

**Beat sheet (60 s):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–4 | Dice clack in tray, bar 1 drops onto staff | 759 TRILLION / WALTZES | Clack + 1 fortepiano bar |
| 4–8 | Parchment table of 176 bars unfurls | FROM 2 DICE / (1792) | Bar 2 |
| 8–12 | Roll → 7 → row 7 lights → bar flies to staff | DICE SUM = BAR | Waltz builds bar by bar |
| 12–16 | Camera drifts across the table grid | 11 SUMS × 16 BARS | " |
| 16–20 | Sum histogram appears on the wall | 7 BEATS 2 / 6 TO 1 | " |
| 20–24 | Bars 8 and 16 glow (1–2 real choices) | 46 QUADRILLION ROLLS… | Half cadence at bar 8 |
| 24–28 | Number snaps down | …759 TRILLION / WALTZES | Waltz #1 finishes |
| 28–35 | Calendar flips, moon arcs across window | ALL OF THEM: / ~720M YEARS | Short reprise |
| 35–40 | Second cup rolls, two staves fill | 2 AT ONCE | Two waltzes layered |
| 40–46 | 10 cups, histogram climbs | 10 DICE CUPS | Layers stack on new timbres |
| 46–52 | Climax: dice rain, histogram locks to triangle | 7 ALWAYS WINS | Full stack |
| 52–60 | Dice settle into one cup, loop to opening | ROLL 2 DICE / WRITE BAR 1 | Thins to one fortepiano bar |

**Flags:**
- Attribution: the 1792 cover credited Mozart, but it has never been authenticated (most scholars reject it).
  Mozart did write a 1787 manuscript (K. 516f) of 176 one-bar fragments, with no evidence of dice.
  Caption "1792" or "sold as Mozart's", never "Mozart wrote".
- Climax: stacking waltzes only works if all 11 options per column share a harmony. Often claimed, NOT
  verified: check the actual table first. Fallback: one lead waltz on top, other cups as quiet staggered echoes.
- Use the 1792 *waltz* table, not the related contredanse version some catalogues list.
- Note data: the music is public domain, but digital transcriptions may not be (CCARH files state no
  license). Safest: transcribe from a scan of the 1792 print or re-enter the notes from a reference.

**Feasibility:** easy: dice, table grid, bars flying to the staff, histogram, candles, moon key light (salon
floor seen from above gives good shadows). Medium: dice tumble, so fake it with pre-baked tumble/bounce
keyframes (no rigid-body sim); dice rain = instanced dice, no collisions. Avoid a Mozart character: use a
self-playing fortepiano. Audio: cap audible layers at ~6–8, the rest a soft shimmer.

### 8. Dinosaur Brass Band — APPROVED
- **Hook:** "A 75M-YEAR-OLD" / "HORN" (2 lines, in the first 2 s). Alt question: "WHAT DID A DINO" / "SOUND LIKE?"
- **Science:** Parasaurolophus's crest is a hollow bony tube full of air passages connected to its airway.
  A nearly complete ~75-million-year-old skull was found near Farmington, New Mexico, in 1995. Sandia
  National Labs and the New Mexico Museum of Natural History CT-scanned it (~350 slices at 3 mm) and modelled
  air through the passages: a deep, rumbling, pitch-shifting call (published 1997). Earlier, Weishampel (1981)
  showed the crest could work as a resonator and suggested juveniles (smaller crests) made higher,
  shorter-range sounds while adults made low, far-carrying honks.
  Sources: [Sandia (1997)](https://newsreleases.sandia.gov/scientists-use-digital-paleontology-to-produce-voice-of-parasaurolophus-dinosaur/),
  [Scientific American](https://www.scientificamerican.com/article/dino-roar),
  [Smithsonian](https://www.smithsonianmag.com/science-nature/how-parasaurolophus-set-the-mood-94657740/),
  [Carleton Hooper Museum](https://hoopermuseum.earthsci.carleton.ca/hadrosaur/dyk3.html),
  [Kowatek blog](https://blog.kowatek.com/2024/11/21/were-closer-to-re-creating-the-sounds-of-parasaurolophus/).
- **Music:** longer air column = lower note (same physics as brass or a blown bottle). Each dinosaur's crest
  size sets its pitch: juveniles high, adults bass. A herd of 5 sizes = 5 notes, stacking into an original
  brass chorale with a bass around G1. All synthesised. (Our own mechanism, NOT the Bit Wonder heartbeat-band
  structure.)
- **Scene:** Late-Cretaceous floodplain at sunset: low-poly ferns, conifers, river. Low sun = key light,
  distant volcano glow = warm local light; dragonflies and swaying ferns for small life. Escalation: 1 adult →
  a juvenile answers → herd of 5 sizes → whole herd honks the chord.
- **Payoff / ending:** the herd raises crests together into a full chorale as the sun flares.
  "BLOW ACROSS A BOTTLE", then loop to the lone adult raising its head.

**Beat sheet (60 s):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–4 | One adult lifts its head and honks | A 75M-YEAR-OLD / HORN | Deep honk |
| 4–8 | Push in; crest turns X-ray, hollow passages | IT'S HOLLOW | Low resonance |
| 8–12 | CT slices stack up and build the skull | 350 CT SLICES (counter) | Scanner ticks |
| 12–16 | Glowing air flows through the passages | AIR IN → NOTE OUT | Rumble with vibrato |
| 16–20 | Tube-length bar grows next to a bottle icon | LONGER TUBE / = LOWER NOTE | Two test tones |
| 20–24 | Juvenile with a short crest answers | BABIES: HIGHER | High honk |
| 24–29 | Adult and juvenile call and answer | (interval forms) | Call / answer |
| 29–35 | Herd arrives, 5 crest sizes lined up | 5 SIZES = 5 NOTES | Notes stack into a chord |
| 35–40 | Adults set the rhythm | — | Bass line (≈G1) |
| 40–46 | Mid-sized animals join | — | Harmony layers |
| 46–52 | Climax: all crests up, sun flares | THE FIRST BRASS BAND? | Full chorale |
| 52–60 | Bottle icon, pull back to the lone adult | BLOW ACROSS A BOTTLE / SAME PHYSICS | Fades to opening honk |

**Flags:**
- It's a simulation: beak, nostrils and soft tissue weren't fossilised, and vocal cords are unknown (Sandia
  modelled both). Caption "could sound like", never "this is its voice".
- Avoid "trombone" on screen: CT showed more tubes than the old trombone-loop model (Smithsonian compares
  the inside to a crumhorn). "Horn" / "brass" is safe.
- No Hz on screen: summaries of Weishampel conflict (48 Hz fundamental vs a 55–720 Hz range). The G1 bass is
  a music choice, not a claim.
- Juveniles-higher is a reasoned suggestion, not a measurement: keep the wording soft.
- Don't echo Jurassic Park's theme or its dinosaur sound design; don't mirror Bit Wonder's heartbeat-band reel.

**Feasibility:** dinosaurs are the hardest characters on the board. Plan: rigid low-poly segments (body,
neck, head) with only neck/head pivots (raise, honk, lower); no walk cycles. The herd "arrives" by sliding in
behind ferns, and the sizes are scaled copies. Easy: CT slices (flat slabs), X-ray crest (transparent shell +
glowing tube spline), bottle icon, sun, ferns. Fallback: a museum hall at night with warm lamps, 5 fossil
skulls on plinths, glowing air flowing through each crest as it sounds (same science and music, no animated
animals).

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
