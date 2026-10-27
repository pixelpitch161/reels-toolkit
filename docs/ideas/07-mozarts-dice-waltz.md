# 7. Mozart's Dice Waltz

Board row: `docs/IDEAS.md` (#7). Series rules: none.

## 7. Mozart's Dice Waltz — APPROVED
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

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01UAUgj8 (2026-10-21) · Output: `dice_waltz.mp4`
- Draft v1 sent. Column harmony verified from the table (all 16 columns share one harmony), so the climax stacks 10 waltzes (no echo fallback). Notes: 2-of-3 vote over three transcriptions. Next: user feedback
