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
| 2 | The Lowest Note in the Universe | approved (v2) | session_01WcqqR7 (v1) | v1 feedback: looks cool, but near-silent for most of it (no beat until 29 s, no melody until 44 s), so it won't hold viewers. v2 rework approved: cold open on the climax, tape-stop, beat-synced spin-up, never quiet (see "v2 rework" in backlog). Next: re-cut in the original session | v1: `lowest_note.mp4` |
| 4 | The Staircase That Never Ends | idea | - | - | - |
| 6 | One String, Every Note | idea | - | - | - |
| 7 | Mozart's Dice Waltz | review | session_01UAUgj8 (2026-10-21) | Draft v1 sent. Column harmony verified from the table (all 16 columns share one harmony), so the climax stacks 10 waltzes (no echo fallback). Notes: 2-of-3 vote over three transcriptions. Next: user feedback | `dice_waltz.mp4` |
| 8 | Dinosaur Brass Band | review | session_019KQA5D (2026-10-21) | Draft v1 sent: real rigid-segment dinosaurs (fallback not needed), early sheet + video delivered with caption/tags. Awaiting feedback | `dinosaur_brass_band.mp4` |
| 9 | The 639-Year Song | idea | - | Copyright: don't use Cage's actual notes (still in copyright); use a generic held chord | - |
| 10 | One Melody, 100 Years of Speakers | idea | - | - | - |
| 11 | The Perfect Loop (15 pendulums) | review | session_017rbUTK (2026-10-21) | v1 delivered with caption + tags: sunset attic, 15 lantern-bobs, floor shadows redraw the wave, phase-ring HUD; seamless loop verified. Waiting for feedback | `perfect_loop.mp4` |
| 12 | Prime-Number Cicadas | review | session_01UkvSDH (2026-10-21) | v1 delivered with caption + tags (early sheet sent first). Exact grid kept (1 yr = 1/16 at 120 BPM, 2024 on the 51 s downbeat), so near-miss captions moved to 29.75 s / 44.5 s. Waiting for feedback | `prime_cicadas.mp4` |
| 13 | Count the Thunder | review | session_01QAsUpz (2026-10-21) | v1 delivered with caption + tags (early sheet sent first). Checks: focus CoC 0 on all frames; flash test 0 general flashes (max 4 % of frame per strike); thunder ≥8 dB over music in 40-400 Hz. Waiting for feedback | `count_the_thunder.mp4` |
| 14 | Chaos Makes a Perfect Curve (Galton board) | in production | session_015Sj8e1 (2026-10-21) | Started: building timeline, seeded 50/50 simulation and Victorian lecture-hall scene | - |

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

#### 2 — v2 rework (APPROVED, supersedes the v1 beat sheet above)
**User feedback on v1:** looks cool and it's a neat fact, but the video is near-silent for most of it, so it
won't capture attention. Engagement brief: constantly stimulating, especially at the start; open on chaos with
sound from frame 0; never go quiet; build to a satisfying lock-in; exaggerate where the phenomenon is calmer.
**What v1 did (from its timeline.py / audio.py):** until 29 s only faint space hiss, a few whooshes and one
pop (8–11 s "SPACE IS SILENT…" cut to near-silence on purpose); first real beat at 29 s; melody at 44 s; rings
creep slowly until ~16 s.

**v2 in one line:** open on the loudest moment, tape-stop to real speed, then speed the black hole back up in
big beat-synced jumps: audible rhythm from frame 0, explanation over the beats, not before them.
- **Cold open on chaos:** the climax itself (every octave roaring, rings blasting, camera shake), then a
  tape-stop dive to real speed.
- **Never quiet:** the tape-stop lands on a muffled groove, not silence: hot-gas crackle + a low-passed
  (underwater-style) bed whose filter opens step by step as the hole climbs toward hearing range. The crests
  are the kick throughout.
- **Spin-up in 8 s (was 24):** −57 → −9 in quick slams: YEAR → DAY → HOUR → MINUTE → SECOND, each a hit plus a
  visual (calendar pages flipping, clock hands spinning, galaxies whipping round in time-lapse).
- **Explain over the beat:** at 55 BPM each kick is visibly one bubble pop at the core = one ring (piston seen
  and heard at once).
- **Exaggeration:** each crest throws a screen-space ripple that bends the scene; dust, crystals and dishes jump
  on the kicks. A long log-frequency ruler along the bottom with a lit "YOUR HEARING" window (20 Hz–20 kHz);
  the B♭ marker races 57 octaves toward it.
- **New hard number:** NASA's 2022 sonification sped these waves up 144–288 quadrillion times; 2⁵⁷ = 144
  quadrillion (calc), exactly our climb, so the landing caption is sourced.
  Source: [NASA Science: data sonifications](https://science.nasa.gov/learners/highlights/data-sonifications).
- **Ending builds instead of fading,** so the last bar slams back into the cold open for the loop.

**v2 beat sheet (60 s):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–3 | Cold open: full climax, rings blasting, shake, ripples | THIS NOTE IS / 57 OCTAVES DOWN | Full B♭ stack + kicks + melody |
| 3–5 | Tape-stop: everything freezes, counter slams to −57 | REAL SPEED: / 1 WAVE / 10M YRS | Pitch-dive whoosh → sub thud; gas crackle + muffled bed |
| 5–7 | Spin-up; galaxies whip round | ≈1 WAVE A *YEAR* | Riser + hit; bed filter opens a notch |
| 7–9 | Clock hands spin | ≈1 A *DAY* · ≈1 AN *HOUR* | Two quick hits |
| 9–11 | Rings accelerate to a time-lapse blur | ≈1 A *MINUTE* | Hit; riser peaks |
| 11–13 | Counter lands on −9; ruler marker near the window | SPED UP / 281 TRILLION × | Big drop: first real kick |
| 13–16 | Each kick = a bubble pop at the core = one ring | JETS BLOW / BUBBLES | Kick 55 BPM + filtered bass |
| 16–19 | Ring rides out through the gas haze | EACH POP = / 1 WAVE | Kicks; hats enter |
| 19–25 | Doubles: rings twice as dense, shake on every hit | SPACE IS SILENT / EXCEPT IN HOT GAS | 109 BPM groove; filter opens further |
| 25–29 | Doubles again, camera pushes in | 219 BPM | 219 BPM, snare rolls |
| 29–33 | Rings blur to shimmer; marker touches the hearing window | BEATS → ? | Flutter 7 → 15 Hz, riser |
| 33–36 | Drop: first colour band; window lights | IT'S A NOTE: *B♭* | B♭0 bass drop (29 Hz) |
| 36–44 | Octave bands stack, one every 2 s; galaxies pulse | SAME BLACK HOLE / 4 OCTAVES UP | B♭1 → B♭4 over the full groove |
| 44–52 | Climax: the cold-open image re-forms | SPED UP 144 / QUADRILLION × | Original melody over the stack, everything at once |
| 52–60 | Hum prompt; counter flickers; last bar builds → loops to frame 0 | HUM A *B♭* / YOU'RE 57 OCTAVES UP | Hum over the groove, building; slams into the cold open |

**v2 flags:**
- Label the speed-ups ("SPED UP" / "REAL SPEED" whenever the rate changes). Still never say NASA "recorded" it.
- The crackle bed is texture, not data: don't caption it as "the sound of the cluster".
- Ripple and shake are stylisation: keep them tied to real crest times so the visuals still show the rhythm.
- 2⁴⁸ ≈ 281 trillion and 2⁵⁷ ≈ 144 quadrillion are calc; 144 quadrillion matches the low end of NASA's range.
- Spin-up captions last 1.5–2 s each: one short line, max ~18 chars.

**v2 feasibility:** a re-cut, not a rebuild. Keep v1's scene.py / render.py pipeline (island, observatory,
satellite, rings, colour bands, focus audit, phone-friendly kicks). Rewrite timeline.py and audio.py (new
section times, continuous filtered groove, tape-stop and spin-up effects). Add to render: camera shake, a
screen-space ripple per crest, calendar/clock milestone props, the hearing-range ruler. The last bar must match
the cold open's first frame (picture and audio). Keep a `*_backup.py` of v1 before changing anything.

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

### 11. The Perfect Loop (15 pendulums) — APPROVED
- **Hook:** "15 PENDULUMS" / "1 PERFECT LOOP" (2 lines, in the first 2 s).
- **Science:** 15 uncoupled pendulums (no motor, magnets or links). The longest swings 51 times in 60 s;
  each shorter one swings once more, up to 65. They drift into travelling waves, standing waves and apparent
  random motion, split into two opposite halves at 30 s, and at exactly 60 s every pendulum has done a whole
  number of swings, so they're back in line. T = 2π√(L/g), so lengths run ~34 cm → ~21 cm (calc).
  Group milestones (calc): adjacent pendulums slip one full swing apart every 60 s, so at t = 60/n s they form
  n groups: 6 @10 s, 5 @12, 4 @15, 3 @20, 2 @30, then 3 @40, 4 @45, 5 @48, 6 @50, one line @60.
  Source: [Harvard Natural Sciences Lecture Demonstrations](https://sciencedemonstrations.fas.harvard.edu/presentations/pendulum-waves).
- **Music:** each pendulum plucks its own note every time it reaches the far side (once per full swing).
  Longest = lowest, across 3 octaves of a pentatonic scale so "chaos" stays consonant. The groups ARE the
  rhythm: 2 groups = eighths, 3 = triplets, 4 = sixteenths, 5 = quintuplets, 6 = sextuplets. First half: the
  subdivisions slow into the halftime call-and-answer at 30 s. Back half: they speed back up 3 → 4 → 5 → 6
  into one unison chord at 60 s. Original bass + pad change chord with each pattern. All synthesised.
- **Scene:** low-poly clockmaker's attic at sunset: round window (low sun = key light), oil lamps (warm local
  lights), ticking wall clocks, a cat and dust motes (small life). A wooden frame holds 15 glowing lantern-bobs
  in rainbow colours, one per note, each flashing as it plays. Floor seen from above, so the bobs' shadows draw
  the wave a second time. Escalation is the physics: line → snake → groups → chaos → halves → groups → line.
- **Payoff / ending:** the back-half acceleration lands on the 60 s realignment: all 15 hit together, which is
  frame 0, so it's a true seamless loop.

**Beat sheet (60 s; physics clock in real time, t = 0 = release):**
| Time | Pendulums (calc) | Caption | Sound |
|---|---|---|---|
| 0–4 | All 15 released in a line, one flash | 15 PENDULUMS / 1 PERFECT LOOP | Unison chord, then rippling arpeggio |
| 4–8 | They peel into a travelling snake | 51 → 65 SWINGS / PER MINUTE | Arpeggio ripples |
| 8–12 | 6 groups (10 s), then 5 (12 s); length labels | LONGER = SLOWER | Sextuplets → quintuplets |
| 12–16 | 4 groups (15 s) | NO MOTOR / NO MAGNETS | Sixteenths |
| 16–22 | 3 groups (20 s) | 3 GROUPS / = TRIPLETS | Triplets |
| 22–30 | Apparent chaos | LOOKS LIKE / CHAOS… | Polyrhythmic shimmer |
| 30–35 | 2 opposite halves (30 s) | HALFTIME: / 2 TEAMS | Call and answer in eighths |
| 35–40 | Chaos again; group-counter HUD slides up | IT'S REWINDING | Bass + pad enter |
| 40–45 | 3 groups (40 s) | COUNT THE GROUPS | Triplets with the groove |
| 45–50 | 4 (45 s), 5 (48 s), 6 (50 s) | 4… 5… 6… | Subdivisions accelerate |
| 50–56 | Groups blur toward one line | TRY IT: 15 WASHERS / + STRING | Build |
| 56–60 | Climax: all 15 snap back in line at 60.0 s = frame 0 | (loop) | One unison chord |

**Flags:**
- Small swings only: period changes slightly with swing size (why real builds drift). Keep the amplitude
  modest on screen; the render can use exact analytic motion so the loop is perfect.
- The cm lengths assume ideal simple pendulums (calc). A real washer-on-string build needs tuning by ear, so
  keep "try it" captions general, not exact cm.
- Overlap with #1 Metronomes: differentiate with setting, colours and the opposite idea (metronomes sync;
  these drift apart and come back).
- Don't copy the famous Harvard video's look (white balls on black): ours is lanterns in a sunlit attic.
- Note density: 15 voices at 51–65 plucks/min ≈ 14 plucks/s. Keep plucks short; trigger once per full swing.

**Feasibility:** very easy: spheres, strings and a frame with θ = θ₀·cos(2π·n·t/60), n = 51…65; no simulation.
One list of trigger times drives both the flashes and the plucks (as on #2). Framing: a portrait frame only
shows ~0.43 × distance in width, so run the row toward the camera at an oblique angle (the iconic view, and
the snake reads best). Floor seen from above under the window light gives free shadow drama.

### 12. Prime-Number Cicadas — APPROVED
- **Hook:** "13 × 17 = 221 YRS" (17 chars, in the first 2 s). Alt: "LAST TIME: 1803".
- **Science:** periodical cicadas spend 13 or 17 years underground, then emerge together in their billions.
  In 2024 Brood XIX (13-year) and Brood XIII (17-year) emerged in the same year for the first time since
  1803 (next: 2245). They only overlap in a thin strip of central Illinois (around Springfield). 13 and 17
  are prime, so they line up only every 13 × 17 = 221 years; non-prime cycles meet far more often (12 & 18
  every 36 years, 12 & 16 every 48; calc). From 1803 to 2024, XIX emerges 17 times and XIII 13 times; the
  closest near misses are 1854/1855 and 1972/1973, one year apart (calc). Why prime? Leading ideas: avoiding
  hybrids with other cycles and, more loosely, predators. Unresolved.
  Sources: [Wikipedia: Brood XIX](https://en.wikipedia.org/wiki/Brood_XIX),
  [UW–La Crosse](https://www.uwlax.edu/currents/cicadas-2024/),
  [KING 5 fact-check](https://rssfeeds.king5.com/~/881604386/0/king5/home~No-this-isn%E2%80%99t-the-first-time-since-that-two-broods-of-cicadas-emerged-at-once),
  [Toivonen & Fromhage 2020](https://pmc.ncbi.nlm.nih.gov/articles/PMC7319174),
  [Wikipedia: Decim cicadas](https://en.wikipedia.org/wiki/Decim_periodical_cicadas),
  [CBS Chicago](https://cbsnews.com/amp/chicago/news/chicago-periodical-cicadas-sound-loud).
- **Music:** 1 year = 1 sixteenth at 120 BPM, so 1803 → 2024 = exactly 221 sixteenths = 27.6 s. XIX hits
  every 13 sixteenths, XIII every 17: a genuine 13-against-17 polyrhythm over an original 4/4 groove, landing
  together only on 2024 (the climax). Voices: synthesised "Pharaoh" calls (pure-tone buzz with a pitch drop).
  The 13-year M. tredecim sings slightly lower (~1.0–1.25 kHz), so XIX = lower voice, XIII = higher. Chorus
  sits above the music in its band (same carve as crickets / dice).
- **Scene:** low-poly Illinois woodland at golden hour as a cut-away diorama: a soil cross-section below the
  trees with nymphs, roots and tunnels. Left half = XIX (warm orange), right half = XIII (teal). Low sun = key
  light, farmhouse porch lamp = warm local light; birds snatching cicadas and shed shells on bark for small
  life. Each emergence makes its side's trees erupt with red-eyed cicadas.
- **Payoff / ending:** 2024: both halves erupt at once, the sky fills, both choruses + groove hit together.
  Then eggs drop from twigs, nymphs burrow back into the soil, and it loops to the opening.

**Beat sheet (60 s):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–4 | Soil cut-away, two nymphs side by side, counter | 13 × 17 = 221 YRS | Soil rumble, one buzz |
| 4–8 | Both burst out, trees erupt | 2024: THESE 2 / LAST MET 1803 | Double chorus swell |
| 8–12 | Split view: left and right nymphs underground | 13 YRS vs 17 YRS | Two buzz pitches (low vs high) |
| 12–16 | 13 and 17 glow as numbers | BOTH PRIME | Ping |
| 16–20 | Ghost 12- and 18-year broods meet over and over | 12 & 18: MEET / EVERY 36 YRS | Muted, frequent clashes |
| 20–24 | Ghosts fade; 13 & 17 return | 13 & 17: / EVERY 221 | — |
| 24–35 | Year race from 1803; each side bursts on its cycle | 1803 → … | Groove + 13:17 polyrhythm |
| 35–40 | 1854/1855 near miss | SO CLOSE… / 1 YEAR APART | Hits flam a sixteenth apart |
| 40–46 | Counter runs on; bursts interlock | XIX: 17 TIMES / XIII: 13 TIMES | Polyrhythm thickens |
| 46–50 | 1972/1973 near miss | 1 YEAR… AGAIN | Flam |
| 50–52 | Climax: 2024, both halves erupt | 2024 | Both choruses + groove together |
| 52–60 | Eggs drop, nymphs burrow, loop to opening | TRY IT: CLAP 13 / TAP 17 | Thins to one buzz, then soil |

**Flags:**
- Wording: "these two broods: first time since 1803", NEVER "first time two broods emerged together" (some
  pair co-emerges every ~5–6 years).
- Skip the hype ("a trillion cicadas", "cicada apocalypse"): the broods barely overlap.
- "Why prime" is debated: caption "WHY PRIME? STILL DEBATED" or say nothing. No periodic predator is known,
  so don't state the predator theory as fact.
- 221 applies to any 13/17 pair, not just XIII and XIX; "13 × 17 = 221" is safe as stated.
- Pitch claim is soft: M. tredecim is documented as slightly lower, but each brood has several species. Treat
  lower/higher as a voicing choice grounded in that difference.

**Feasibility:** easy: low-poly cicadas (body, wings, red eyes) instanced by the thousand, trees, soil
cut-away with tunnels, year counter HUD. Medium: mass emergence as particles; precompute burst times from the
year list so one list drives visuals and audio (as on #2 and #11). Audio: synthesise the Pharaoh call (buzz +
pitch drop) and cassini clicks; hit density at 120 BPM sixteenths is fine. Fallback: one tree with two
colour-coded broods and the soil cut-away below, if the split woodland is too busy.

### 13. Count the Thunder — APPROVED
- **Hook:** "SECONDS ÷ 5 = MI" / "SECONDS ÷ 3 = KM" (16 + 16 chars, in the first 2 s; both units for a global
  audience).
- **Science:** light arrives essentially instantly (~300,000 km/s); sound travels ~0.3 km/s. Count from flash
  to thunder: ÷ 5 for miles (NWS), ÷ 3 for km (Environment Canada). Thunder can be heard ~10 miles away (NWS)
  or 8–20 km depending on conditions (ECCC). Air absorbs high frequencies as sound travels, so distant thunder
  is low and rumbling; a strike to your side gives a sharp clap, while a channel running toward or away from
  you rumbles because its sound arrives over a stretch of time. Safety (NWS): if you can hear thunder you're
  within striking distance, there's no safe place outside, and you wait 30 min after the last thunder.
  Sources: [NWS: science of thunder](https://www.weather.gov/safety/lightning-science-thunder),
  [NWS: lightning safety](https://www.weather.gov/safety/lightning-safety-overview),
  [ECCC: how thunder sounds](https://www.canada.ca/en/environment-climate-change/services/lightning/science/how-thunder-sounds.html).
- **Music:** distance drives rhythm AND timbre. First half (real time): the count is the click track
  ("1… 2… 3…" at 60 BPM) and each thunder lands where the physics says. Back half: 120 BPM groove, one flash per
  bar on the downbeat (bright snare); thunder = the kick, delayed by the real flash-to-bang time (2 s = a bar
  late, then 1, ½, ¼, ⅛ s), walking back toward its flash and fusing on the downbeat at the overhead strike.
  The thunder's low-pass opens as it nears (rumble → clap). Rain = shaker. All synthesised and original.
- **Scene:** low-poly prairie cabin cut-away at night. Inside: warm lamp + fireplace (local lights), cat on the
  windowsill, rain streaking the glass. Moon = key light until the storm swallows it; then each flash throws
  window-frame shadows across the floor. Outside: fence posts with distance markers, a windmill spinning faster
  in the rising wind, a storm cell (anvil cloud + rain curtain) crossing from the horizon to the cabin.
- **Payoff / ending:** the overhead strike hits the barn's lightning rod: flash and clap on the same downbeat
  with the full groove. The storm passes, the moon returns, and it loops to the calm opening (one distant flash).

**Beat sheet (60 s; first-half delays in real time, distances from the on-screen rules, calc):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–4 | Calm cabin; distant flash at 0.5 s; counter starts | SECONDS ÷ 5 = MI / SECONDS ÷ 3 = KM | Count clicks (60 BPM) |
| 4–8 | Counter climbs | LIGHT: INSTANT / SOUND: 1 KM / 3 S | Clicks, light rain |
| 8–12 | Thunder arrives at ~10 s | 10 S = 2 MI | Low, muffled rumble |
| 12–20 | Flash at 12 s → thunder at 19.5 s | FAR = RUMBLE / AIR EATS HIGHS | Rumble, a bit brighter |
| 20–30 | Flashes 21 → 26 s and 27 → 30 s; storm crosses the fence posts | 5 S = 1 MI · 3 S = 1 KM | Rumbles turning into claps |
| 30–35 | Moon swallowed; cat moves off the sill | HEAR THUNDER? / GO INDOORS | Groove fades in under rain |
| 35–44 | One flash per bar; thunder 2 → 1 → ½ s late; distance HUD shrinks | 0.4 MI… 0.2 MI… | Kick walks back across the bar |
| 44–50 | Thunder ¼ → ⅛ s late; window shadows flash | 100 M… 40 M… | Flams tighten; filter opens |
| 50–52 | Climax: overhead strike on the lightning rod | 0 S | Flash + clap fuse with full groove |
| 52–60 | Storm passes, moon returns; loop to opening | TRY IT: COUNT / FROM INSIDE | Rain thins to one distant rumble |

**Flags:**
- Safety: the character is indoors throughout and the ending says "from inside". Never show or imply counting
  outdoors.
- Photosensitivity: at most 3 flashes per second (we're at ~0.5/s) and no full-frame white flashes. Flash the
  sky, window and floor shadows while the warm lamps keep the room lit (also matches the "too dark" lesson).
- The rules are approximations: at 343 m/s it's ~4.7 s/mile and ~2.9 s/km, and it varies with temperature.
  Fine for captions; don't claim precision.
- Hearing range varies (NWS ~10 miles, ECCC 8–20 km): no single "max distance" number on screen.
- No stock thunder recordings: synthesise everything.

**Feasibility:** easy: lightning (jagged polyline + glow), dithered low-poly cloud blobs, rain-streak
particles, fence posts, windmill. Reuse the #7 cut-away room lessons (narrow room ~3.4 units; moon pinned in
the window as a painted backdrop). Medium: flash-lit window shadows on the floor (short key-light spike through
the window mask; cheap with the existing shadow pipeline). Audio: thunder as filtered noise (sharp N-wave
transient for close claps, long low-passed tail for far rumbles); carve the music's low end under each hit and
add a click so it reads on phone speakers (#2 lesson). Font: add "÷" as a runtime glyph from the project (like
♭ and ≈ on #2).

### 14. Chaos Makes a Perfect Curve (Galton board) — APPROVED
**Engagement brief (user, Oct 2026):** constantly stimulating, especially at the start. Open on chaos with
sound from frame 0, never go quiet, and build to a satisfying lock-in (the 100 Metronomes arc). Exaggerate
where the phenomenon is calmer than that.
- **Hook:** "1 IN 4,096" (10 chars, on screen over a torrent of balls). Alt: "1,000 RANDOM BALLS" / "1 PERFECT CURVE".
- **Science:** balls fall through 12 rows of pegs; at each peg a ball bounces left or right with even odds.
  2¹² = 4,096 possible paths into 13 bins. Paths per bin follow Pascal's triangle: 1, 12, 66, 220, 495, 792,
  924, 792 … 1, so the middle bin can be reached 924 ways and each edge bin only 1 (1 in 4,096). With 1,000
  balls, expect ~226 in the middle and ~0.2 in each edge (calc). Random bounces reliably add up to a bell curve.
  Francis Galton described it in *Natural Inheritance* (1889).
  Sources: [Wikipedia: Galton board](https://en.wikipedia.org/wiki/Galton_board),
  [Linda Hall Library](https://www.lindahall.org/about/news/scientist-of-the-day/francis-galton/).
- **Music:** each peg hit is a ping, pitched by the peg's column on a pentatonic scale, so every ball plays a
  12-note random-walk melody, and most melodies end near the middle note (tonic). Each bin holds a chord note
  that gets louder as the bin fills, so the growing curve shapes the chord voicing (strong middle, quiet edges).
  Ball count escalates 1 → 10 → 100 → 1,000: pings go from melody to rain to hailstorm; an original groove enters
  at 100 balls. All synthesised.
- **Scene:** a giant brass-and-walnut Galton machine in a low-poly Victorian lecture hall. Tall windows with
  sunbeams (key light), gas lamps (warm local lights), dust in the beams; a pigeon on the gallery rail and a few
  pawn-figure spectators on tiered benches (small life). Ball shadows land on the wooden backplate (a big
  visible surface). Balls change hue at each peg (warmer = right, cooler = left), so the final colour matches
  the bin and the curve fills in as a rainbow gradient.
- **Payoff / ending:** a 1,000-ball flood completes the curve; an ideal bell-curve outline traces over the real
  bins as the chord blooms with the full groove. A trapdoor drains the bins and a bucket elevator carries the
  balls back up to the hopper, looping to the opening torrent.

**Beat sheet (60 s; never quiet, every beat adds balls or sound):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–4 | Cold open mid-pour: hundreds of balls hailing through the pegs | 1 IN 4,096 | Ping hailstorm + bass hit |
| 4–8 | Everything freezes mid-air and dims; one hero ball keeps falling | 1 BALL = 12 COIN FLIPS | Hero's 12-note melody, loud and clean |
| 8–12 | Arrows flash L/R at each peg | LEFT OR RIGHT? / 50 : 50 | Second ball's melody; light ticks |
| 12–16 | Pascal numbers light up on the pegs row by row | 12 ROWS = / 4,096 PATHS | Ascending pings, one per row |
| 16–20 | 10 balls stream; melodies overlap | MIDDLE: 924 WAYS / EDGE: 1 WAY | Busy overlapping arpeggio |
| 20–26 | Golden ball goes left at every peg; spotlight; edge bin lights up | 1 IN 4,096 | 12 falling notes, then a gong |
| 26–32 | 100 balls; bins act like an equaliser | RANDOM… | Groove enters; pings as rain |
| 32–40 | 300 balls; a hump forms | …BUT NOT MESSY | Bin chord grows; bass line |
| 40–46 | 1,000-ball flood fills the board | (expected-count numbers flicker) | Hailstorm + full groove |
| 46–52 | Climax: ideal curve outline traces over the bins | THE BELL CURVE | Curve chord blooms on the drop |
| 52–60 | Trapdoor drain, elevator up, loop to opening | TRY IT: FLIP 12 COINS / COUNT THE HEADS | Drain whoosh + elevator clanks into the opening hail |

**Flags:**
- The golden ball is staged: all-left happens 1 time in 4,096. Caption it as a 1-in-4,096 path ("WHAT IF…"),
  never imply it happened by chance in this run.
- Don't fake the curve: simulate real 50/50 bounces with a fixed seed so the bins are genuinely random. With
  1,000 balls it's close to ideal, and the outline overlay shows the gap honestly. Edge bins will likely stay
  empty (fits the hook).
- Real boards aren't perfectly 50/50 (momentum, double bounces). The render uses the idealised model; don't
  claim it's a real-board recording.
- History: *Natural Inheritance* (1889) is solid. Wikipedia dates a Royal Institution demo to 1874, but Linda
  Hall found no surviving board Galton built. Caption "1889" only.
- Design: don't copy the popular commercial desktop "Galton Board" toy's look; ours is a brass Victorian machine.
- Ping density: ~480 pings/s at the peak (calc). Play the first ~30 balls as discrete notes, then blend the rest
  into a density-controlled granular hiss. Keep the ping band above the music (same carve as #7) so the
  hailstorm stays exciting, not harsh.

**Feasibility:** very easy: no physics sim. Pre-roll each ball's 12 random bits, animate parabolic hops between
pegs, stack balls by bin count; 1,000 instanced spheres. One list of hop times drives both pings and hue shifts
(as on #2, #11, #12). The board is tall and narrow, so portrait suits it: push in on the hero ball, pull back
for the flood; the visible backplate makes ball shadows read. Pings are mid–high frequency, so they survive
phone speakers; keep the bass optional for impact.

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
