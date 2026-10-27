# 20. Battle of the Bands

Board row: `docs/IDEAS.md` (#20). Series rules: none.

## 20. Battle of the Bands — APPROVED
**Direction (user, Oct 22–23):** satisfying-first; engagement brief (chaos and sound from frame 0, never quiet,
build to a payoff). Keep it 60 s (user, Oct 23), even though the reference account's reels run 23–34 s.
**Reference (studied Oct 23):** [@algorithmbattles](https://www.instagram.com/algorithmbattles/reels/): 17 posts,
1.7K followers, reels at 2.9M / 1.5M / 1.2M / 845K views. Four named strategies, each a colour, fill an arena,
then fight over territory until one wins. Static camera, no hook text, 23–34 s loops, muted autoplay with a song
behind. Four-team battles do far better than two-team (26–42K) or odd arenas (113K). The engine is comments:
post caption "Which strategy would you pick?" + one line per strategy; viewers turn colours into characters
("Red was FIGHTING for its right to live", 13.9K likes); confusion about the rules drives comments too (18.6K
likes). Sound plays no part in their battles: that's our gap.
**Core idea:** every team is a band member. Every move a team makes plays that member's next note; the more of
the stage a team owns, the louder its instrument (the mix is the scoreboard); eliminated teams drop out of the
song; the winner plays the last bars solo. The strategies are physical processes.
- **Hook:** "4 BANDS" / "1 STAGE" (7 + 7 chars, in the first 2 s). Post caption: "Which band would you pick? 🎧" +
  one line per team.
- **The four teams:**

| Team | Strategy (physics) | Plays |
|---|---|---|
| SAND (amber) | Grains pour from a hopper at the top, fall and pile under gravity, slide down slopes | Drums |
| BALL (purple) | A ball bounces around under gravity, painting every tile it crosses | Lead melody |
| RIPPLE (teal) | Rings spread out from its base, one ring per beat | Bass |
| VINE (green) | A tip grows, branches and backtracks through open space | Keys / arpeggio |

- **Rules (two acts):** FILL: teams claim only empty tiles. FIGHT: once the stage is full, each team attacks its
  borders using its own move (sand avalanches into neighbours, the ball ricochets through enemy land, ripples push
  the front line out, vines burrow in). A team with no tiles left is out.
- **Music:** an original 4-part arrangement at 120 BPM in one key, so the band always sounds in tune. Each team's
  claims step its part forward on a sixteenth-note grid (busy team = full part, stalled team = stutters). Volume
  follows territory share. When a team is out, its part drops out with a filter sweep and its speaker stack goes
  dark ("THE DRUMMER'S OUT!"); the others get louder to compensate, so the mix never thins into quiet.
- **Scene:** a floating festival stage in the vibrant void: the arena is a giant round LED wall at centre stage,
  four team-coloured speaker stacks around it, sweeping stage spotlights (key light), warm footlights (local light),
  the stage floor catching the shadows; a small speaker cone on each stack pumps with its instrument.
- **Payoff / ending:** the winner floods the last tiles on a big hit, then plays the full tune solo while the wall
  pulses in its colour. At 58 s the wall cracks back to empty with four new starting seeds, looping to the opening
  burst.

**Beat sheet (60 s; event times are a target: the real run decides who wins and when, so choose a seed whose run
matches this shape):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–2 | All four burst from their bases at once: sand pouring, ball bouncing, rings, vine | 4 BANDS / 1 STAGE | All four parts at once, chaotic but in key |
| 2–8 | Legend inside the safe zone: SAND · DRUMS, BALL · LEAD, RIPPLE · BASS, VINE · KEYS | (hook, then legend) | Parts lock onto the grid |
| 8–14 | FILL race; territory bar fills under the wall | EVERY MOVE / = A NOTE | Band thickening |
| 14–20 | Stage nearly full; a leader emerges | MORE STAGE / = LOUDER | Leader's instrument swells |
| 20–22 | Wall full: flash, rule card | FULL. / NOW THEY FIGHT | Drum fill + crash |
| 22–31 | Borders churn; a ricochet rips through enemy land; lead changes | WHO'S WINNING? / LISTEN 🎧 | Mix swinging between parts |
| ~31 | First elimination: its stack goes dark | (TEAM) IS OUT! | Its part sweeps out; others swell |
| 31–41 | Three-way fight; underdog comeback if the seed gives one | (no caption) | Tension builds |
| ~41 | Second elimination | 2 LEFT | Duel: two instruments trading phrases |
| 41–50 | Final duel; last tiles flipping | (no caption) | Riser + snare roll |
| ~50 | Winner takes the stage; wall floods one colour | (TEAM) WINS | Big hit |
| 50–58 | Winner's solo: full tune; wall pulses; its stack blazes | WHICH BAND / WOULD YOU PICK? | Winner's part, full arrangement |
| 58–60 | Wall cracks back to empty; four new seeds; loop | (no caption) | Riser into the opening burst |

**Flags:**
- Make it ours, not a copy: @algorithmbattles' team names (Ricochet, Explorer, Lightning, Dragon, Prism, Arcs)
  and look (flat black background, circle, 2×2 colour key) stay theirs. Ours: physical-process teams, music at the
  centre, a low-poly stage diorama, the legend inside the safe zone.
- Real simulation, honestly curated: run many seeds and pick one with a good story (early leader falls, underdog
  wins), but never hand-edit who wins.
- Keep the rules readable: one short rule card at 20 s keeps the confusion fun rather than frustrating.
- "Every move = a note" is gating: moves trigger the next step of each part, quantised to the beat (not one note
  per tile, which would be noise with thousands of tiles).
- Holding 60 s: three acts (fill, fight, solo), an elimination roughly every 10 s, never-quiet mix.
- Safe zone: the arena wall is the key action, so it fits inside the safe box with the legend just under it;
  speaker stacks and stage scenery fill the rest of the frame. (Expect the wall at ~230 internal px across,
  between the caption band and the legend.)
- Series potential: once the engine exists, each new episode is a new seed, arena shape or team line-up. Build the
  engine reusable (team rules, seed and arena shape as parameters).

**Feasibility:** sim: a ~100 × 100 tile grid run offline from a fixed seed, per-frame results cached. Sand =
simple falling-sand rules; ball = gravity + elastic bounces off the round wall; ripple = ring-by-ring spread; vine
= random growth with backtracking. Fight rules = border attacks using each team's move; tune the balance across
many seeds so any team can win. Render: the grid is a texture on the round LED wall inside a low-poly stage set
(stacks, lights, floor) in the void; the camera holds almost still so the battle stays readable. Audio: four
synthesised parts; cached claim events gate each part's next step; per-team volume from territory share,
smoothed; drop-outs with a filter sweep. One event list drives the wall, stacks, meters and mix. Checks: safe zone
(captions, legend, territory bar) and flash check (full-wall flash at 20 s, winner flood).

## Board history (moved here from the status board, Oct 27)

- Status: review (v2) · Session: session_01CSbeCD (2026-10-23) · Output: `battle_of_the_bands.mp4` (v2)
- v1 delivered with caption + tags (early sheet sent first). v1 feedback: start the video on the empty stage so the loop point is the video's start/end. v2 delivered: same honest run (seed 82), video time = run time: frame 0 = empty stage as the four seeds drop, bell 21.4 s, RIPPLE out 31.1, BALL 42.5, VINE 51.6, SAND wins and solos, wall cracks back to empty exactly at 60 s (bar fades in, cut by the crack; hopper slides home during the crack). Engine: reusable `battle.py` + `seed_search.py` (800 seeds, every team wins some). Changes vs backlog: knockout below 2.5 % of the stage, volume = power, round-based heat, capture, 4 s close-up before the bell; failed moves play ghost notes. Checks: safe zone OK (0.25 s steps); wall CoC 0 on all frames; 0 general flashes (max 14.5 %); -13.5 dBFS fill -> -10.9 solo; seam = the seed pop. v1 kept in check/. Waiting for feedback
