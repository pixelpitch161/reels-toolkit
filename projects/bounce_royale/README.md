# Bounce Royale (IDEAS #24)

Four balls fight over the wall of a holographic ring floating above a rain-soaked neon rooftop. Every bounce throws
a splash of paint on the wall in the ball's colour; a colour that loses all its wall pops. The ring shrinks from
16 s, so the battle speeds up by itself. Battle family (#20, #21), mood A: dark synthwave.

| ball | plays (every wall bounce) |
|---|---|
| RUBY (red) | drums: snare + clap high on the ring, pitched toms in the middle, low toms at the bottom |
| JADE (green) | bass synth: a driving octave bass; each bounce sets the note, 16th pulses until its next bounce |
| COBALT (blue) | arpeggio synth: each bounce fires a 4-note arpeggio up the chord, with a ping-pong echo |
| AMBER (gold) | lead synth: each bounce is a lead note, held until its next bounce |

Pitch comes from how high on the ring the bounce lands (chord tones of | Am | F | Dm | E |, low at the bottom),
pan from which side; volume follows each colour's share of the wall; repaints get an accent; a pop cuts the
instrument out. 118.03 BPM = `loop_bpm(118)`: 30 bars in 61 s.

## The engine (`royale.py`)

2D billiards in a circle; a run is decided by its seed (launch angles) and `PARAMS`. Rules, the same for every run:
- Same size, same speed, no gravity. Mirror bounces off the wall (the wall's own speed is included while it
  shrinks); real elastic collisions between balls (equal masses swap the normal component), after which every
  ball is set back to the shared speed (fairness rule).
- 48 tiles, 12 per colour (one quadrant each). The balls launch one per beat from the centre (RUBY, JADE, COBALT,
  AMBER), so the four instruments enter in turn.
- **Splash = share.** A bounce repaints every enemy tile whose centre its splash covers; the splash is as wide as
  the colour's share of the wall (1.8 tiles at 12 tiles). Changed from the backlog's one-tile repaint: bounces
  spread evenly round the ring, so with one tile per bounce every colour sits at 12 +- 3 tiles for ever (0 pops in
  24 test runs). With splash proportional to share every repaint is a fair coin for both colours (neutral drift),
  so colours really fall to 0, and lead swings and comebacks happen by themselves.
- A bevel turns grazing bounces at least 12 deg inward (without it a ball can glide along the wall, which in a
  simulation is thousands of "bounces" a second).
- 0 tiles = pop. From 16 s the ring shrinks linearly to 42 % at 50 s.

## The run (seed 25672)

`seed_search.py` ranks honest seeds (fit: pops near 27 / 38 / 49 s at least 6 s apart, a close duel, a colour down
to 1-2 tiles before the first pop; story: lead changes, nothing static for long, comebacks). 10 of ~12,700 seeds
fit; every colour wins some runs. Seed 25672 is played exactly as simulated: RUBY leads early (27 tiles at 8 s), JADE
takes over at 13.5 s; COBALT and AMBER both drop to 1 segment (20.9 / 22.7 s); COBALT pops at 24.85 while AMBER claws
back from 1 to 21; RUBY hangs on with 1-2 segments for 12 s and pops at 39.63; the JADE vs AMBER duel swings four
times (24-24 at 45 s) until AMBER pops at 47.58: JADE wins and floods the ring.

## Build

```bash
python seed_search.py 0 20000 --stop 5        # ranked list in check/
echo 25672 > seed.txt && python sim.py        # cache the run -> sim.npz
python timeline.py                            # event times + captions (all from the run)
python audio.py                               # -> audio.wav (+ section levels, bounces over the bed)
python -m reelkit.sheet render.py test/sheet.png 0.6 13.8 21.5 27.5 40 45 48.2
python -m reelkit.safezone render.py --step 0.25
python focus_check.py
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav bounce_royale.mp4
python -m reelkit.flashcheck bounce_royale.mp4
```

| file | what |
|---|---|
| `royale.py` | the billiards engine and rules; `Run(seed).run()` |
| `seed_search.py` | runs seeds in parallel, ranks their stories, stops early |
| `sim.py` | caches the chosen run (`sim.npz`) |
| `timeline.py` | one clock: ring/camera clocks, captions placed by the run's events, labels, bar |
| `scene.py` | neon rooftop: projector, stairwell hut + door lamp, water tank, mast, neon sign, AC units, vent; city towers, windows, banners, beacons, drones |
| `render.py` | sky + moon -> city -> wet roof per pixel (mirrored hologram, neon streaks) -> props -> hologram (disc, tiles, splashes, balls, pops) -> locked DOF -> rain -> UI |
| `audio.py` | the dark synthwave soundtrack driven by the bounces |
| `focus_check.py` | subject CoC = 0 on every frame |

Loop: the outro closes the winner's ring to a point that charges up lilac; frame 0 is the snap-open (the closing
point and the snap share the same light, so the roof lighting is continuous across the seam). The hook caption
lands at 59.5 s and holds through the seam.
