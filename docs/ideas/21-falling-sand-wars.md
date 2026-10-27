# 21. Falling Sand Wars

Board row: `docs/IDEAS.md` (#21). Series rules: none.

## 21. Falling Sand Wars — APPROVED
**Direction (user, Oct 23):**
- The music doesn't have to open in complete chaos (as #20 did): open sparse but tense, never silent, and build.
- Explore different moods: not every video should have upbeat / happy-go-lucky music. This one is **mood A:
  dark and hypnotic** (chosen by the user).
- Share bar: keep each team's name displayed: small text below the bar with a thin line connecting it to its
  colour on the bar.
- Text should appear slightly more interestingly, with a very slight "impact".
**Concept:** a battle family member (see #20), but different: four kinds of sand with different physics are poured
into a glass box with one hole. The box turns, piles avalanche, sand drains out; the last colour still in the box
wins.
- **Hook:** "4 SANDS" / "1 HOLE" (7 + 6 chars, in the first 2 s). Post caption: "Which sand would you pick? 🎧" +
  one line per sand.
- **The four sands (each one's physics is its strategy):**

| Sand | Behaviour | Plays |
|---|---|---|
| IRON (deep red) | Heavy: sinks through the others to the bottom | Low piano strikes / sub bass |
| PUMICE (pale cyan) | Light: gets pushed up and rides on top | High glassy bells |
| CLAY (amber) | Sticky: builds steep piles, clings to walls | Muted frame drum |
| GLASS (violet) | Slippery: flows almost like water | Plucked dulcimer |

- **Rules:** POUR (0–20 s): four hoppers on the box's corners fill it about halfway; iron sinks, pumice rises, so
  the layers sort themselves while you watch. DRAIN (20 s on): the hoppers shut and a plug pops out of one wall;
  the box turns in 45° steps on the beat; after each turn the sand avalanches, and whatever reaches the hole pours
  out into the night. A team with no grains left is out. Survival depends on where each sand settles (bottom-heavy
  iron drains first when the hole is low, sticky clay hangs on to the walls, slippery glass escapes fast).
- **Scene:** low-poly desert ruin at night: the glass box hangs on chains under a full moon (key light) between two
  braziers (warm local lights); wind-blown dust drifts through (small life). Drained sand falls onto the dune below
  and builds four coloured "graveyard" piles, so the losers' sand stays visible.
- **Ending (loop at the file boundary):** the winner's sand sits alone in the box on a stark final chord. In the
  last 4 s the box rolls fully over and its sand pours out, so the box is empty exactly at 61.0 s. Frame 0 = the
  empty box as the four streams start again.

**Music, mood A (dark and hypnotic):** slow minimal techno at ~94.4 BPM (`loop_bpm(96)`: 24 bars in 61 s, so the
groove is in phase at the loop point) in D Phrygian (dark, uneasy flavour). Opens
sparse but tense: a low drone + heartbeat kick from frame 0, first sand ticks; builds from there. Sand landing =
metallic ticks stepping each team's part (grouped per beat, as #20); turns = deep impact hits; avalanches = low tom
rolls; each draining stream = a held tone in its team's pitch, scaled by flow. Volume follows how much sand a team
still has; an eliminated team's instrument drops out. Ends on a stark, unresolved minor chord (an hourglass running
out, not a victory lap). Original and synthesised. (Alternatives offered, not chosen: B melancholy piano/cello
lament; C ominous cinematic taiko + low brass. Keep varying moods across videos.)

**Share bar with names (user note):** a thin horizontal bar inside the safe zone just under the box, four coloured
segments sized by each team's share of the sand. Names in small text under the bar in fixed slots (IRON, PUMICE,
CLAY, GLASS), each with its share ("IRON 31%"); a thin leader line runs from each name up to the centre of its
colour segment, so the lines slide as segments shrink and grow but the names never collide. A team that's out: its
segment closes, its line disappears, its name dims with a small "OUT". Build it as a reusable `reelkit.ui` piece
(Battle of the Bands episodes can use the same bar). Check the small text on a phone-size preview; if 1× text is
too small, enlarge it, staying inside the safe zone.

**Captions with a slight impact (user note):** each caption line drops a few px and lands with a tiny scale punch
(~110 % → 100 % over ~0.1 s); on landing a 1–2 px shake of the text only (never the box, so the battle stays
readable), a small puff of pixel sand under the letters, and a soft thud in the mix. Weighty but subtle. Build it as
a reusable caption style in `reelkit.ui` (e.g. `draw_captions(..., style='impact')`) so later videos can use it.

**Beat sheet (61 s, user Oct 23: TikTok monetisation needs >= 1 min; events after 20 s are targets: the real run
decides, so choose seed + balance whose run fits):**
| Time | Visual | Caption (with impact) | Sound (mood A) |
|---|---|---|---|
| 0–3 | Empty box turning slowly under the moon; four thin streams start | 4 SANDS / 1 HOLE | Low drone, heartbeat kick, first ticks |
| 3–6 | Streams thicken; iron sinking already | IRON SINKS / PUMICE FLOATS | Ticks gather into a pattern |
| 6–9 | Clay piles steeply, glass runs flat | CLAY STICKS / GLASS FLOWS | Bass motif enters |
| 9–20 | Box fills, layers sort; share bar and names shuffle | LAST SAND / IN THE BOX WINS | Ostinato builds, tension rising |
| 20–22 | Hoppers shut; the plug pops out of the wall | THE HOLE OPENS | Deep impact hit |
| 22–34 | Turns on the downbeat; avalanches; streams pour out onto the dune; lead changes | (no caption) | Turn hits, tom rolls, draining tones |
| ~33 | First team out (two can fall close together for a change of pace) | (TEAM) IS OUT | Its instrument cuts out; drone deepens |
| 34–44 | Turns every bar; the graveyard piles grow | 2 LEFT | Faster turns, rising pulse |
| 44–52 | Duel: the loser's last grains trickle toward the hole | (no caption) | Only two instruments + the heartbeat |
| ~52 | Last grain of the loser drops; the box stops | (TEAM) WINS | Stark final chord |
| 52–57 | Winner's sand alone in the box, moonlight; its motif plays | WHICH SAND / WOULD YOU PICK? | Winner's motif, unresolved |
| 57–61 | Box rolls over, the winner pours out; empty at 61.0; loop to the new pour | (no caption) | Long stream tone into the opening drone |

**Flags:**
- The physics is stylised (falling-sand rules with per-material weight, stickiness and flow). Captions only describe
  behaviour (sinks, floats, sticks, flows), no scientific claims.
- Honest simulation: pick seeds and balance settings, never hand-edit the outcome; tune the four sands so each wins
  some runs. Use an early-stopping search (`--stop 5`, see PLAYBOOK "Working efficiently").
- Turns land on the beat: a stylisation (45° steps, each animated over ~0.4 s).
- Different from #20 (materials instead of processes, gravity and drainage instead of border fights, dark mood, new
  setting) but can share the battle look (share bar, eliminations) as a family.
- Length 61.0 s (`DUR = 61.0` in timeline.py). Loop point = file boundary: the box is empty at 61.0 = frame 0, no
  reset earlier; `reelkit.loopcheck` must pass.

**Feasibility:** sim: a falling-sand grid of ~160 × 160 cells inside the box with rules for heavy, light, sticky and
slippery grains; gravity in 8 directions (45° steps); hole cells remove grains; run offline and cache; reuse the
search pattern from `projects/battle_of_the_bands/seed_search.py`. Render: the grain grid as a texture on the front
of the rotating low-poly glass box; ruin, dunes, chains, moon and braziers are simple pieces; graveyard piles grow
from the drained counts. UI: share bar with names + impact caption style as reusable `reelkit.ui` pieces (small
separate commits). Audio: sand-landing events grouped per beat step each team's part; draining streams = held tones
scaled by flow; turn hits from the turn list; tom fills from big avalanche frames. One event list drives picture and
sound.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01BxTdPv (2026-10-23) · Output: `falling_sand_wars.mp4`
- v1 delivered with caption + tags (early sheet sent first). Honest falling-sand run (seed 176): one braided inlet so the pour sorts itself (iron core, clay steep, glass wide, pumice outside); plug pops at bar 8 (20.33 s) in a seeded bottom-corner notch; 45 deg turns on downbeats; IRON out 32.7, PUMICE 44.7, GLASS 51.1, CLAY wins with 388 grains; closing roll pours the winner out, empty box upright on 61.0 = frame 0. Changes vs backlog: knockout at 12 % of the start (last grains go dark) because draining cuts across layers and the last teams would empty together; one strata dome instead of four piles (the stream lands in one place); 33-44 s is a slow stretch (remnants the hole can't reach). Balance: clay/pumice win most runs, glass a few, iron almost never. New reelkit: ui.share_bar, captions style="impact", audio.thud. Checks: safe zone OK; box CoC 0 on all frames; loop PASS (seam 2.2x); 0 general flashes; opening -16.4 dBFS, build +3.2 dB. Waiting for feedback
