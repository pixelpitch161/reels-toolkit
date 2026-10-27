# 24. Bounce Royale

Board row: `docs/IDEAS.md` (#24). Series rules: none.

## 24. Bounce Royale — APPROVED
**Direction (user, Oct 22–23):** satisfying-first battle; names + instruments on screen all video; music not always
super uplifting (this one: **mood A, dark synthwave**); music can open sparse; impact captions; 61.0 s. Lesson from
#21 (user: "too slow… no story"): built for pace: something changes every 4–5 s, and the shrinking ring makes the
fight speed up by itself.
**Concept:** four balls fight over the wall of a ring that keeps shrinking. Every bounce repaints the wall segment it
hits in the ball's colour; a colour that loses all its wall pops; as the ring shrinks, bounces come faster, so the
battle accelerates all the way to the end.
- **Hook:** "LAST COLOUR" / "STANDING" (11 + 8 chars, impact style, in the first 2 s). Post caption: "Pick your colour
  before it starts 🎧"
- **The four balls and what they play (on screen all video):**

| Ball | Plays |
|---|---|
| RUBY (red) | Drums |
| JADE (green) | Bass synth |
| COBALT (blue) | Arpeggio synth |
| AMBER (gold) | Lead synth |

- **Rules:** the ring wall = 48 segments, 12 per colour at the start. The balls fly at the same speed with no gravity
  (billiards), bouncing off the wall and each other with real elastic collisions. Hitting an enemy segment repaints
  it; hitting your own does nothing. At 0 segments a ball pops (shatters; its instrument cuts out). From 16 s the ring
  shrinks steadily, packing the bounces closer and turning the battle frantic. Comebacks happen naturally (one lucky
  bounce streak can rebuild a colour from a single segment).
- **Instruments on screen (user rule):** first 2 s: a label pops up beside each ball with the impact style ("RUBY ·
  DRUMS", etc.), each instrument entering on that ball's first bounce. Then: `reelkit.ui.share_bar` under the ring
  (share of the wall) with each name AND its instrument in small text under the bar (a second small line), each
  joined to its colour by a leader line; a popped colour dims with "OUT".
- **Music, mood A (dark synthwave):** driving four-on-the-floor in a minor key at ~118 BPM (`loop_bpm(118)` = 118.03:
  30 bars in 61 s): neon, cool and tense, not happy. Every wall bounce plays its colour's instrument, pitch from where
  on the ring it hit (minor scale), so bouncing traces out melodies; quantised to the grid as the bounces speed up.
  Repaints get an accent; a pop = crash + cut-out; the ring's shrinking = a long riser. Volume follows each colour's
  share of the wall. Original and synthesised. (Alternatives offered, not chosen: B glitchy electro; C bittersweet
  chiptune.)
- **Scene:** a low-poly neon rooftop at night in a rain-soaked retro-futurist city; the ring is a glowing hologram
  floating above the roof; magenta moon (key light), warm neon signs (local lights); rain streaks, the ring's glow
  reflected in the wet roof; drones and blinking antenna lights in the skyline (small life).
- **Payoff / ending (loop at the file boundary):** the winner floods the whole ring with its colour and plays a solo
  motif while the city's neon pulses with it; in the last seconds the ring shrinks to a point; at 61.0 s = frame 0 it
  snaps back to full size in four colours as the four balls launch.

**Beat sheet (61 s; events after 8 s are targets: the real run decides, so choose a seed whose run fits, with
something changing every 4–5 s):**
| Time | Visual | Caption (impact) | Sound (mood A) |
|---|---|---|---|
| 0–2 | Ring snaps open, four balls launch; labels pop up beside each ball | LAST COLOUR / STANDING | Each instrument enters on its ball's first bounce |
| 2–8 | Labels slide into the share bar names (name + instrument) | HIT A WALL = / PAINT IT | Groove locks in; bounces play melodies |
| 8–16 | Repaint battle; lead changes; ball-on-ball collisions | LOSE YOUR WALL = / YOU POP | Driving beat, accents on repaints |
| 16–22 | The ring starts shrinking; bounces speed up | THE RING / SHRINKS | Long riser starts |
| 22–26 | A colour drops to 1 segment, then claws back (or doesn't) | 1 SEGMENT LEFT! | Its instrument stutters, then returns |
| ~27 | First pop | (BALL) POPS | Crash; its instrument cuts out |
| 27–38 | Three-way, faster; lead swings; second pop ~38 | 2 LEFT | Tighter subdivisions, rising tension |
| 38–48 | Duel in a small ring: rapid back-and-forth repaints | (no caption) | Two instruments trading fast phrases |
| ~49 | Final pop: the winner floods the ring | (BALL) WINS | Big hit |
| 49–57 | Winner's solo motif; the city's neon pulses with it | WHO DID YOU / PICK? | Winner's motif over the beat |
| 57–61 | Ring shrinks to a point; at 61.0 it snaps open = frame 0 | (no caption) | Riser into the opening |

**Flags:**
- Keep it a competition with a story (lesson from #21): something changes every 4–5 s; reject seeds with long
  stretches where nothing changes.
- Honest simulation: same speed for every ball, real collisions. Pick seeds for a good story (a near-death comeback,
  lead swings, a close duel, different winners across seeds), never hand-edit. Early-stopping search (`--stop 5`).
- Trend nod, not a copy: borrows the ring-escape trend's energy; no DVD logo, nothing from @algorithmbattles; our own
  neon rooftop style.
- Photosensitivity: pops and neon pulses stay moderate and small in area; run `reelkit.flashcheck`.
- Length 61.0 s (`DUR = 61.0`), tempo with whole bars (~118 BPM = 30 bars). Loop point = file boundary (the snap-open
  at frame 0); `reelkit.loopcheck` must pass.
- Safe zone: the ring and the share bar (names + instruments) inside the safe box; the city fills the rest.

**Feasibility:** sim: 2D billiards in a shrinking circle (exact reflections, elastic ball-ball collisions, segment
repainting, pops): cheap; run offline and cache with an early-stopping search. Render: a segmented neon ring (glowing
torus pieces), glowing balls with trails; the rooftop city = instanced low-poly blocks with neon signs and rain
particles; reflections = a simple mirrored glow on the wet roof. UI: impact captions + `share_bar` from `reelkit.ui`
(added by #21), with a second small line under each name for the instrument. Audio: bounce events play each colour's
instrument (pitch from hit angle, quantised); repaint accents; pop crashes; shrink riser; volume from wall share. One
event list drives picture, bar and mix.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_013tp84a (2026-10-23) · Output: `bounce_royale.mp4`
- v1 delivered with caption + tags (early sheet sent first). Honest billiards run (seed 25672 of ~12,700; every colour wins some runs): RUBY leads early, JADE takes over 13.5 s, COBALT + AMBER both at 1 segment, COBALT pops 24.85 while AMBER claws back 1 -> 21, RUBY hangs on 1-2 segments for 12 s (pops 39.63), JADE vs AMBER swings 4 times (24-24 at 45 s), JADE wins 47.58. Changes vs backlog: splash width = share of the wall (MORE WALL = BIGGER SPLASH; one tile per bounce never pops anyone: 0 of 24 test runs), speeds reset to the shared speed after each elastic collision, a 12 deg wall bevel on grazing bounces, balls launch one per beat so the instruments enter in turn; captions placed by the run's events. Checks: safe zone OK (0.25 s steps); ring + balls CoC 0 on all frames; loop PASS (seam 1.3x); 0 general flashes (max 5.8 %); opening -14.6 dBFS, build +3.1 dB, bounces 7-17 dB over the bed (0.3-4 kHz). Waiting for feedback
