# Battle of the Bands (IDEAS #20)

Four physical-process teams fight over a round LED wall on a floating festival stage. Every team is a
band member: its moves play its notes, its share of the stage is its volume (the mix is the scoreboard),
a knocked-out team drops out of the song, and the winner plays the last bars solo.

| team | process (physics) | plays | stack |
|---|---|---|---|
| SAND (amber) | grains pour from a swinging hopper, fall, slide off slopes, pile up; in the fight they pour through their own sand and smash into enemy tiles below, and the pile avalanches down/sideways (never up) | drums | hung, top left |
| BALL (purple) | a ball bounces under gravity, elastic everywhere; paints what it crosses, then (fight) flips every enemy tile it hits and bounces back, smashing small craters, tunnelling when fast | lead | hung, top right |
| RIPPLE (teal) | one ring per beat from its base through its own land; where a ring reaches the edge it claims / pushes the front line out | bass | ground, left |
| VINE (green) | tips grow, branch and backtrack through open space; in the fight they burrow into enemy land | keys | ground, right |

## The engine (reusable for the series)

`battle.py` knows nothing about the video. An episode is a config: arena shape (`Arena(n, 'circle' | 'square' |
'hexagon' | 'diamond' | 'ring' | callable)`), the team line-up with their balance parameters, the seed and the
bell time (`episode.py`). `Battle.run()` returns the recording: owner grid, vine links and last-claim times at
every frame, every claim event, knockouts, winner, and each team's per-frame process state for drawing.

Rules (same for every episode):
- **FILL:** teams claim only empty tiles. In the last 4 s before the bell the stage closes up (empty tiles next
  to a team fill in, faster and faster); at the bell any last gaps go to a neighbour.
- **FIGHT:** each team attacks its borders with its own move. Volume = power: a team's attack odds and the speed
  of its process scale with its share of the stage (what the viewer hears), and the fight heats up in rounds
  (the ramp restarts after each knockout). A tile surrounded on 7 of 8 sides by one rival falls to it. A team
  under 2.5 % of the stage is knocked out: its last tiles go dark and its neighbours absorb them. The last team
  standing floods the stage.

New episode: copy `episode.py`, change the arena / line-up / parameters, run `seed_search.py`, set `SEED`.

## The seed (honest curation)

`seed_search.py 0 800` ranked 800 runs of episode 1; 18 fit the beat-sheet shape, and every team wins some runs
(SAND 155, BALL 47, RIPPLE 245, VINE 81, undecided 272). Seed 82 has the best story and is played exactly as
simulated: VINE leads at the bell (37 %), BALL takes the lead (25.1 s), VINE retakes it (25.9 s), RIPPLE is out
(29.8 s), SAND overtakes (38.1 s), BALL is out (41.1 s), VINE is out (50.2 s): SAND, tied last at the bell (17 %),
wins. Top of the ranking:

```
seed   82 score  19.6  bell SA 17 BA 29 RI 17 VI 37  outs RI@29.8 BA@41.1 VI@50.2  win SA  lead changes 3 (VI>BA>VI>SA)  duel 21%  linger 2.7s
seed  737 score  17.0  bell SA 17 BA 28 RI 21 VI 33  outs RI@33.7 BA@41.6 VI@50.3  win SA  lead changes 1 (VI>SA)  duel 15%  linger 3.0s
seed  410 score  16.3  bell SA 17 BA 25 RI 25 VI 32  outs SA@31.6 BA@41.8 VI@52.0  win RI  lead changes 3 (VI>RI>VI>RI)  duel 14%  linger 2.8s
seed   45 score  15.8  bell SA 19 BA 40 RI 18 VI 23  outs VI@33.2 RI@40.5 BA@48.4  win SA  lead changes 1 (BA>SA)  duel 21%  linger 3.1s
seed  623 score  12.6  bell SA 23 BA 20 RI 22 VI 35  outs BA@30.2 RI@37.4 VI@47.0  win SA  lead changes 1 (VI>SA)  duel 15%  linger 2.6s
seed  140 score  12.1  bell SA 15 BA 24 RI 25 VI 36  outs SA@30.2 BA@42.8 VI@51.6  win RI  lead changes 1 (VI>RI)  duel 14%  linger 3.9s
18/800 fit the shape; wins SA 155 BA 47 RI 245 VI 81; undecided 272
```

## Build

```bash
python seed_search.py 0 800          # rank honest seeds by story (shape 31/41/50 s, leader falls, underdog...)
python sim.py                        # episode.SEED -> sim.npz + sim_state.pkl
python audio.py                      # -> audio.wav (+ levels, band vs crowd/fx, quietest window)
python focus_check.py                # LED wall + props CoC on all 1800 frames (expect 0)
python -m reelkit.sheet render.py test/sheet.png 1 5 10 17 20.3 26 31 38 45 51 55 58.3
python -m reelkit.safezone render.py
python -m reelkit.video render.py audio.wav battle_of_the_bands.mp4
python flash_check.py battle_of_the_bands.mp4
```

| file | what |
|---|---|
| `battle.py` | the engine: arena shapes, the four team processes, the fill/fight rules, recording |
| `episode.py` | episode 1 config: line-up, balance, rules, chosen seed |
| `seed_search.py` | runs seeds in parallel and ranks their stories (nothing is ever hand-edited) |
| `sim.py` | runs the chosen seed and caches it |
| `timeline.py` | video time <-> sim time (frame 0 is 1.4 s into the run; the next run starts at 58.6 s), knockouts, captions |
| `music.py` | the original arrangement (A minor, 120 BPM) + move gating -> one note list for sound and picture |
| `scene.py` | the stage: slab, LED wall + bezel + truss, four speaker stacks, team props on the rim, crowd, searchlights |
| `render.py` | void -> searchlights -> per-pixel boards (moon shadows, wall glow, pools, glossy wall reflection) -> rig, stacks, props, the per-pixel LED wall, crowd -> locked DOF -> glows, fireworks -> grade -> bar, legend, captions |
| `audio.py` | one voice per band member, every note from `music.notes()`, share-following volume, power-downs, bell, crowd, win hit, crack, riser; periodic buses + circular reverb |
| `focus_check.py`, `flash_check.py` | checks without / after rendering |
