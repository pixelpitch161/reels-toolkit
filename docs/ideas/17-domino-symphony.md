# 17. Domino Symphony

Board row: `docs/IDEAS.md` (#17). Series rules: none.

## 17. Domino Symphony — APPROVED
**Direction (user, Oct 22):** satisfying-first: cool and satisfying to watch and listen to; science is optional
flavour. Engagement brief: chaos and sound from frame 0, never quiet, build to a satisfying lock-in, exaggerate.
**Formula:** chaos from the first second (one push explodes into 64 clattering lines) → the lines lock into one
rhythm (the #1 lock-in) → a holdout domino that almost doesn't fall → a rewind that loops.
- **Hook:** "10,000 DOMINOES" / "1 PUSH" (15 + 6 chars, in the first 2 s).
- **What you see:** a huge low-poly gym floor seen from above, covered in a winding domino layout: branching
  splits, a spiral, a hidden pixel-art field and a staircase of growing dominoes. A cat wanders the floor (threat
  + small life), and the cat's paw taps domino #1.
- **How it plays:** every domino click is a hat/tick. Timing is set by path length and spacing: lines with
  matching lengths click together, lines with different lengths clatter out of step. Every 4th domino in a
  "bell lane" tips a tuned chime bar (the melody). Each split doubles the voices; parallel lanes tuned to chord
  tones make the chords. Each step of the growing-domino staircase booms one bass note lower. Everything locks
  to 120 BPM sixteenths once the lines converge. All synthesised and original.
- **Scene:** late-afternoon sun through high gym windows (key light), warm work lamps (local lights), dust in
  the beams; long domino shadows raking across the floor (top-down, where shadows read best). Small life: the
  cat, a roaming robot vacuum, tape marks on the floor.
- **Payoff / ending:** the holdout tips and the staircase booms down to a giant final domino on the drop; the
  camera cranes up to show the whole layout drawing a giant pixel music note. Last 4 s: everything stands back
  up in a fast rewind and the cat's paw hovers over domino #1, looping to the opening tap.

**Beat sheet (60 s):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–1 | Cat's paw taps domino #1 | 10,000 DOMINOES / 1 PUSH | One click, then a bass hit |
| 1–4 | Line splits every 0.5 s: 1 → 2 → 4 … 64 lines fanning across the floor | (hook stays) | Clatter explodes into a chaotic wash |
| 4–8 | Branches funnel back together in pairs; clicks line up | 64 LINES… / 1 RHYTHM | Clatter tightens into steady 16ths |
| 8–14 | A bell lane peels off; every 4th domino rings a chime | EVERY 4TH ONE / RINGS | Melody starts over the click groove |
| 14–20 | Lane splits into 4 parallel tuned lanes | 4 LANES / = CHORDS | Chords join; kick enters |
| 20–26 | Field reveal: a wide domino field falls in one sweep, coloured faces revealing pixel art | (no caption) | Huge clatter swell + pad |
| 26–32 | Spiral: line winds inward, spacing tightening so clicks speed up; cat steps over a lane | (cat almost touches it…) | Clicks accelerate into a build |
| 32–38 | Holdout: wave reaches a lone tall domino on a ledge; it wobbles, settles, wobbles | WILL IT FALL? | Music drops to riser + snare roll |
| 38–40 | It tips; the growing-domino staircase booms, each step 1.5× bigger | EACH ONE / 1.5× BIGGER | Descending bass booms on the drop |
| 40–52 | Giant final domino slams; camera cranes up; all lanes in sync; layout shows a giant pixel music note | (no caption) | Full song: clicks + chimes + chords + bass |
| 52–56 | Last lanes finish on the final chord | 10,000 / IN 52 SECONDS | Final chord, last clicks |
| 56–60 | Fast rewind: the floor stands back up; cat's paw hovers over #1; loop | (no caption) | Rewind whoosh into the opening click |

**Flags:**
- Choreographed, not real: fall timing is set to the music grid (real chains vary). Never claim a real build or
  "no cuts"; the rewind makes that obvious anyway.
- The 1.5× caption is real: a domino can knock over one ~1.5× its size (Lorne Whitehead, 1983). From a standard
  ~5 cm domino, 8 steps reach ~1.3 m (calc). Optional flavour.
  Source: [Smithsonian](https://www.smithsonianmag.com/smart-news/just-twenty-nine-dominoes-could-knock-down-the-empire-state-building-2232941/).
- Don't copy famous domino builds or builders' signature tricks. Keep the pixel-art reveal original (a music
  note or the channel's own name; no characters or logos).
- The cat is a generic cat and a near-miss threat only; it never knocks the layout over by accident.
- Click density: ~500 clicks/s with 64 lines (calc). Short, varied wood clicks blended into a wash so the merged
  rhythm and the chimes punch through.

**Feasibility:** no physics sim: one schedule gives each domino a fall-start time from its path distance; each
domino rotates about its front edge until it rests on the next; the rewind is the same schedule played backward.
10,000 instanced boxes: use simplified models far from the camera (flat quads/pixels) and check render speed
early with a test sheet. Top-down gym floor = easy ground shadows; window light gives long raking shadows.
Audio: a bank of wood clicks with small random variation, struck-metal chimes, booms for the staircase, grouped
clicks for the dense sections. One schedule drives picture and sound.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01H2FWd5 (2026-10-22) · Output: `domino_symphony.mp4`
- v1 delivered with caption + tags (early sheet sent first). Exactly 10,000 dominoes on the 120 BPM grid (last pair lands on the final bar at 52.0 s), 0 standing overlaps; field cream until each domino tips so the note truly appears in the chevron sweep; red holdout on a plank ledge (wobble, settle, wobble, teeter); 1.5x staircase booms D..D into the drop; 3+3-lane frame plays the song on chime bars. Checks: subject CoC 0 on all 1800 frames; domino band 2-6 kHz +8..+43 dB over the music wherever clicks play; no 2 s window >12 dB under the peak; loop seam 1.9 vs 0.7-1.2 normal step. Waiting for feedback
