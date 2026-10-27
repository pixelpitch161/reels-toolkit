# 2. The Lowest Note in the Universe

Board row: `docs/IDEAS.md` (#2). Series rules: none.

## 2. The Lowest Note in the Universe — APPROVED
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

## Board history (moved here from the status board, Oct 27)

- Status: review (v2) · Session: session_01WcqqR7 (v1); session_01UoKXf4 (v2, 2026-10-21) · Output: `lowest_note_v2.mp4` (v1: `lowest_note.mp4`)
- v1 feedback: looks cool, but near-silent for most of it, so it won't hold viewers. v2 re-cut delivered with caption + tags (early sheet sent first): cold open on the climax, tape-stop to real speed, spin-up slams 5-10 s, drop at 11.7 s, all on one 109.3 BPM grid. Checks: every 2 s window -13 to -16 dBFS; crests 7-21 dB over the music; subject CoC 0 on all frames; loop seam = a normal downbeat. v1 kept as `*_backup.py`. Waiting for feedback
