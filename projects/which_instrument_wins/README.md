# #26 Which Instrument Wins? (`projects/which_instrument_wins`)

The trending "Who Will Win? 16 entrants" format with 16 instruments. 16 glass balls, each holding a Pixel Pitch
instrument model, drop into a rotating ring with one gap over a pit in a sunset colosseum. Fall out through the gap
and you're out; the gap slowly widens. Honest 2D physics; seed 406 was picked for pacing and never edited.
Every wall bounce plays that ball's own Pixel Pitch voice (pitch = where it hits, loudness = impact speed, timing on
the 32nd grid of a dark D-minor tango at 118.03 BPM), so the soundtrack thins from 16 instruments to a duel to a solo.

## Commands (from this folder)

```bash
python seed_search.py 1 600 --stop 6        # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 406                           # one run's story (needs numba for speed; pure Python works, slowly)
python sprites.py 12                        # sprite test sheet of the 16 models (cached per size in sprites_r*.npz)
python -m reelkit.sheet render.py test/sheet.png 0.5 9 23.5 46 53 60.6
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                       # subject CoC must be 0 px on every frame
python audio.py                             # audio.wav + check/stem_*.wav
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav which_instrument_wins.mp4
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | Beat sheet, physics rules (frozen after `check/rulesearch.py`), story windows, chords, the winner's solo |
| `sim.py` | Ring physics (numba): rotating arc wall, gap end caps (they sweep balls: near-misses), exits, pit landings, events |
| `seed_search.py` | Early-stopping seed search with hard windows (first out, final 3, duel, win, longest lull) + soft story score |
| `pp_bridge.py` | Read-only bridge to `series/instrument-history`: the 16 modern models (display stands skipped at build time by runtime patches) and their synth voices |
| `sprites.py` | The models rendered with the series' own rasteriser and ink outline into per-size, per-yaw sprites |
| `scene.py` | Colosseum (stands collapsed on the sunset side), pit, braziers, torches, crowd, pennants, far ridges + aqueduct |
| `render.py` | Frame renderer (floor shadows per pixel, ring, glass balls, lens, post, names, roster, captions) |
| `audio.py` | Tango bed + bounce notes + rattles, clinks, pit landings, the win hit, the solo, the fly-back cascade; folds tails onto the start |

## Rules worth knowing

- The gap seals the moment one ball is left (so the winner can't fall during its bow); the ring turns exactly 20 times
  in 61 s, so frame 0 and the last frame line up.
- After the win the tail is scripted: the champion floats out to the spotlight and plays the solo; at 58.4 s the
  eliminated lift out of the pit (first out leaves last, each playing one note of a rising cascade) and everyone eases
  into the starting formation, arriving at rest on 61.0 = frame 0.
- `reelkit.ui.roster_grid` (new, reusable): the 4 x 4 scoreboard (survivors lit, the knocked out dimmed with their place).
