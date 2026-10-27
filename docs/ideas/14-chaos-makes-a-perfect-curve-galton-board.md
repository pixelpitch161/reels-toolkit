# 14. Chaos Makes a Perfect Curve (Galton board)

Board row: `docs/IDEAS.md` (#14). Series rules: none.

## 14. Chaos Makes a Perfect Curve (Galton board) — APPROVED
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

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_015Sj8e1 (2026-10-21) · Output: `galton_board.mp4`
- v1 delivered with caption + tags (early sheet sent first). Honest seeded run (1,000 balls, seed 1889) + staged golden "what if" (never counted). Checks: focus CoC 0 on all frames; science band 9-25 dB over the music; quietest 0.5 s -22.7 dBFS; seamless loop. Waiting for feedback
