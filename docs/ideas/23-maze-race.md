# 23. Maze Race

Board row: `docs/IDEAS.md` (#23). Series rules: none.

## 23. Maze Race — APPROVED
**Direction (user, Oct 23):**
- The video must say which team is which instrument, on screen (#21 didn't).
- Music doesn't have to be dark/moody, just not always super uplifting. This one: **mood B, heist chase** (chosen by
  the user).
- Lesson from #21 Falling Sand Wars (user): "too slow; it didn't feel like an actual competition, just sand falling
  out of a box with no story". So this race keeps the competition explicit and fast: something changes every 4–5 s
  (lead change, dead end, near-miss, a finisher), live leader callouts, and the legend shows the ranking.
- Earlier notes still apply: 61.0 s; music can open sparse rather than in full chaos; impact captions.
**Concept:** four explorers race through one maze from four corners to the centre. The maze has four-fold rotational
symmetry, so every corner faces exactly the same route; the only difference is how each explores. Fairness rule:
everyone uncovers the same number of new squares per beat (otherwise the explorer that floods in rings always wins).
- **Hook:** "WHO SOLVES IT" / "FIRST?" (13 + 6 chars, impact style, in the first 2 s). Post caption: "Which one would
  you bet on? 🎧"
- **The four explorers and what they play:**

| Explorer | How it explores | Plays |
|---|---|---|
| FLOOD (blue ink) | Spreads outward in all directions, ring by ring: thorough but spread thin | Strings: a staccato ostinato that surges on every new ring |
| DIVER (red thread) | Dives down one corridor and backtracks at dead ends | Bass: a picked spy-style line, climbing as it digs, falling as it backtracks |
| SEEKER (gold light) | Heads straight for the centre, guessing: fast, but dead ends near the goal trap it | Marimba lead |
| MARBLE (silver ball) | A real marble rolling as the board slowly tilts: the physics wildcard | Drums: every wall hit is a hit |

- **Who plays what, always on screen (user note):** first 4 s: a label pops up by each start corner with the impact
  style ("FLOOD · STRINGS", "DIVER · BASS", "SEEKER · MARIMBA", "MARBLE · DRUMS"), each instrument entering as its
  label lands. Then for the rest of the video: a compact race legend inside the safe zone under the maze: four slim
  lanes, each with the explorer's colour, name, instrument and a progress bar (how close to the centre), ordered or
  marked by live rank; a finisher's lane flashes its place (1ST, 2ND…). Build the lanes as a reusable `reelkit.ui`
  piece next to `share_bar` (a 4 × "NAME · INSTRUMENT" row is too wide for one line: 4 short lanes fit).
- **Music, mood B (heist chase):** a driving minor-key groove at ~122 BPM (`loop_bpm(122)` = 121.97: 31 bars in
  61 s): tense but fun, like a getaway; spy strings, picked bass, marimba, tight drums. Opens with the instruments
  entering one by one on their labels, then drives. Every new square an explorer uncovers plays the next note of its
  part, on the beat; dead ends get a "bonk" in that instrument; a finisher's chord lands on arrival. Original and
  synthesised. (Alternatives offered, not chosen: A curious puzzle; C lo-fi focus.)
- **Scene:** a carved wooden maze board on a scholar's desk at night, slightly tilted toward camera: desk lamp (key
  light), two candles (warm local lights); the explorers are glowing paints and threads on the board, the marble is
  real; long wall shadows across the board; small life: a moth circling the lamp, a ticking clock, drifting dust. The
  board tilts gently in a slow pattern (it steers only the marble).
- **Payoff / ending (loop at the file boundary):** after the finishers, the winning route lights up and plays back as
  a melody from corner to centre with all four instruments joining in; then the trails fade and the explorers slide
  back to their corners; frame 0 (61.0 s) = "GO" again.

**Beat sheet (61 s; events after 8 s are targets: the real run decides, so choose a maze seed + tilt pattern whose
race fits; something must change every 4–5 s):**
| Time | Visual | Caption (impact) | Sound (mood B) |
|---|---|---|---|
| 0–2 | GO: all four leave their corners; corner labels pop up | WHO SOLVES IT / FIRST? | Instruments enter one by one as their labels land |
| 2–8 | Labels slide down into the race legend (names, instruments, progress) | SAME MAZE / 4 STRATEGIES | Groove locks in |
| 8–14 | Flood rings spread, diver snakes deep, seeker darts ahead | (LEADER) LEADS | Four parts interlocking |
| 14–20 | Marble rattles; tilt sends it down a long corridor; lead changes | FAIR RACE: SAME / SQUARES PER BEAT | Drums fill; groove drives |
| 20–26 | Seeker trapped in a dead end near the centre | SO CLOSE… | Seeker's "bonk", marimba falls |
| 26–32 | Diver deep in the wrong branch; another lead change | (NEW LEADER) TAKES OVER | Bass backtracks downward, strings surge |
| 32–40 | Flood's front creeps in steadily; tilt frees the marble | SLOW AND STEADY…? | Build; snare drives |
| 40–46 | Final stretch: two contenders, close-up on the centre | (no caption) | Riser, tension |
| ~46 | First to the centre: lane flashes 1ST | (TEAM) WINS | Winner's chord |
| 46–52 | 2nd, 3rd and 4th finish (or the last one is still lost) | (no caption) | A finishing hit each |
| 52–58 | The winning route lights up and plays as a melody, corner to centre | THE WINNING / ROUTE | All four on the route melody, resolving chord |
| 58–61 | Trails fade, explorers slide back to their corners; 61.0 = frame 0 GO | (no caption) | Pickup into the opening |

**Flags:**
- Fairness is what viewers will check: same squares per beat for the three paint explorers; tune the marble's average
  pace to be comparable; the post caption says so in one line.
- Honest simulation: pick the maze seed and tilt pattern for a good race (lead changes, a near-miss, a close finish,
  varied winners across seeds), never hand-edit the race. Early-stopping search (`--stop 5`).
- Keep it a competition with a story (lesson from #21): if a seed's race has a long stretch with no change, reject it.
- Naming: generic explorer names (nothing from @algorithmbattles); the board must not look like the branded tilt-maze
  toy (ours is a carved wooden board on a desk).
- Instruments always on screen (corner labels, then the legend), per the user.
- Length 61.0 s (`DUR = 61.0`), tempo with whole bars (~122 BPM = 31 bars). Loop point = file boundary ("GO" at
  frame 0); `reelkit.loopcheck` must pass.

**Feasibility:** maze: a grid maze with four-fold rotational symmetry (generate one quarter, rotate, connect to the
centre); the three paint explorers are simple step rules on the grid with the same budget per beat; the marble is 2D
circle-vs-wall physics under a slowly changing tilt; run offline and cache. Render: maze walls = instanced low-poly
blocks on the board; trails = glowing paint on the floor cells; simple desk scene (lamp, candles, clock, moth). UI:
impact captions from `reelkit.ui` (added by #21) + the new race-legend lanes as a reusable piece. Audio: exploration
events per explorer step its part on the beat; dead ends and finishes trigger accents; the end route maps each cell to
a scale note. One event list drives picture, legend and mix.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01ABcE9a (2026-10-23) · Output: `maze_race.mp4`
- v1 delivered with caption + tags (early sheet sent first). Honest run (maze 12382, tilt 1094, 20x20 symmetric maze): MARBLE leads 8.5 s, FLOOD 11.5, DIVER 23.5, FLOOD (the tortoise) 32.0, SEEKER dead end next to the centre 34.7 (SO CLOSE...), SEEKER wins 44.4, FLOOD 2nd 46.5, DIVER 3rd 49.9, marble LOST at 56 %. Changes vs backlog: 1 cell per beat in the race (2 per beat from bar 20; 2/beat everywhere finished by ~10 s), the board rocks to a new seeded tilt on every beat (slow sweeps parked the marble), the race ends at bar 26 (51.2 s). Balance (30k mazes): SEEKER 74 %, DIVER 25 %, FLOOD 1 %; the marble never finished in ~10k tilts (wildcard). New reelkit: ui.race_lanes. Checks: safe zone OK; board CoC 0 on all frames; loop PASS on render + mp4 (seam 1.6x, mux.py: mbtree off + last-frame keyframe, audio a whole number of AAC frames, file 61.01 s); flash 0 general; opening -19.9 dBFS, build +3.2 dB. Waiting for feedback
