# 26. Which Instrument Wins?

Board row: `docs/IDEAS.md` (#26). Series rules: none.

## 26. Which Instrument Wins? — APPROVED
**Direction (user, Oct 23):** new idea from the trend research above; standing notes as in #25. Mood A (dark tango)
by default (user approved without picking).
**Concept:** the trending "Who Will Win? 16 entrants" format with 16 instruments. Each ball plays its own instrument
whenever it bounces: the arena starts as a wall of 16 instruments and clears down to a duel, then a solo (chaos thinning
to clarity).
- **Hook:** "16 INSTRUMENTS" / "1 SURVIVOR" (14 + 10 chars, impact style, in the first 2 s). Post caption: "Pick yours
  before it starts 👇🎧"
- **The 16 entrants** (all already modelled and voiced in Pixel Pitch, `series/instrument-history/`): piano, violin,
  cello, harp, flute, clarinet, sax, trumpet, trombone, tuba, French horn, electric guitar, drums, accordion, bagpipes,
  ukulele. Ukulele and bagpipes are natural underdogs for comments. Each ball = a glass sphere with the instrument's
  low-poly model inside; its name rides along under it in small text.
- **Rules:** the balls drop into a big rotating ring with one gap (gravity, real bouncing and collisions). Fall out
  through the gap = out (drop into the pit below with one last dull note). The gap slowly widens, so eliminations
  speed up: 15 eliminations in ~45 s (one every ~3 s). The last two duel until one falls. Equal size and weight for
  everyone (fairness).
- **On-screen roster:** a 4×4 grid of the 16 names inside the safe zone: survivors lit, the eliminated dimmed with
  their place ("16TH"…). Build it as a reusable `reelkit.ui` piece.
- **Music, mood A (dark tango):** habanera bass + staccato strings in a minor key at ~118 BPM (`loop_bpm(118)`: 30
  bars): dramatic, duel-like, and it gives the bounces a rhythm to fall into. Every bounce plays that ball's
  instrument (pitch quantised to the key, timing to the grid). The winner's solo is an original motif in its own
  idiom. (Alternatives offered, not chosen: B game-show suspense; C epic royale.)
- **Scene:** a low-poly colosseum at sunset, the ring floating over the arena floor; low sun (key light), torches on
  the walls (local lights); pennants flutter, pigeons circle (small life). Time-saving fallback: the shared
  `reelkit.backdrops.VibrantVoid`.

**Beat sheet (61 s; events after 2 s are targets: the real run decides, so choose a seed whose run fits):**
| Time | Visual | Caption (impact) | Sound |
|---|---|---|---|
| 0–2 | 16 balls drop into the ring; names under each | 16 INSTRUMENTS / 1 SURVIVOR | Every instrument at once over the tango pulse |
| 2–8 | Ring turns, the gap sweeps past; first falls | FALL OUT = / YOU'RE OUT | First drop-outs; the bed tightens |
| 8–20 | Eliminations every ~3 s; the roster dims | (INSTRUMENT) IS OUT (for upsets) | The arrangement thins; individual instruments emerge |
| 20–32 | Gap widens; an underdog survives a near-miss | STILL ALIVE?! | Faster pulse |
| 32–44 | Down to 4, then 3 | FINAL 3 | Three instruments trading phrases |
| 44–50 | Final duel: two balls, close calls at the gap | (no caption) | Call and response, tension rising |
| ~50 | Last fall | (INSTRUMENT) WINS | Big hit |
| 50–58 | Winner plays a solo motif alone; the colosseum lights up | WHO DID YOU / PICK? | Winner's solo |
| 58–61 | The ring closes; at 61.0, 16 balls drop in again = frame 0 | (no caption) | Pickup into the opening |

- **Cross-promo:** every entrant has (or will have) a Pixel Pitch history video; an end caption like "ITS STORY: / ON
  OUR PAGE" can send the winner's fans to the matching episode.
- **Series:** "WHICH INSTRUMENT WINS? #1" with an all-time win tally across episodes; later themes (brass only,
  strings only, "weird instruments").
- **Flags:** honest simulation: pick seeds for pacing (no long gaps without an elimination, an upset, a close final,
  varied winners across seeds), never hand-edit; early-stopping search. Generic instruments only (no brand models or
  logos). 16 voices at the start: keep each bounce note short so it reads as an exciting mess, not mush. Loop point =
  file boundary (balls drop in at frame 0); safe-zone, flash and loop checks must pass.
- **Effort:** low–moderate: cheap 2D physics; instrument models + synth voices reused from Pixel Pitch (import or copy
  into the project; don't edit the Pixel Pitch series files); roster grid = small new UI piece; the colosseum is the
  main new build (or use the void).

## Board history (moved here from the status board, Oct 27)

- Status: review (v2) · Session: session_01Ug3Awo (2026-10-23) · Output: `which_instrument_wins.mp4` (v2)
- v1 feedback: drop "ITS STORY: / ON OUR PAGE" at the end; move the roster to the loosened safe-zone defaults. v2 delivered: "WHO DID YOU / PICK?" holds to 58.25 s, roster at the bottom of the new zone (x 24-292, y 452-497, left of the button column), ring recentred under the captions (x 180); full re-render (the roster is on screen throughout), safe zone OK, loopcheck + flashcheck PASS on the mp4. v1 notes: delivered with caption + tags (early sheet sent first). Honest ring run, seed 406 (2 fits in 600): DRUMS out first at 3.6 s after rattling in the gap, BAGPIPES stuck in the gap 6.5-8.9 s, PIANO + TRUMPET out together at 15.4/15.7, UKULELE 9th after three near-misses, VIOLIN survives five edge hits, final 3 at 40.3, VIOLIN vs CELLO from 43.1 (7 close calls), CELLO wins 49.41. Ring seals on the win; the cello floats out and plays an original solo in its Pixel Pitch voice while the stands light up; at 58.4 s the 15 fly back from the pit (one note each) into formation, at rest on 61.0 = frame 0. Every wall bounce = that instrument's Pixel Pitch voice (pitch by hit position in the chord, 32nd grid) over a D-minor tango (118.03 BPM). Changes vs backlog: rules tuned once on median pacing (2.06 rad/s = 20 turns in 61 s, gap 17.4 -> 27.5 deg), the ring seals on the win (so the winner can't fall during its bow), places shown as numbers in the roster chip slot; series files untouched (models/voices imported, stands skipped by runtime patches). New reelkit.ui.roster_grid. Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames; flash 0 general flashes; bounces 7-15 dB over the bed (400-4000 Hz); opening -18.7 dBFS, build +4.1 dB; loopcheck PASS on render and mp4 (audio = 2860 AAC frames, muxed with #23's mux, file 61.01 s). Waiting for feedback
