# #38 BALLFIGHT #12: Bass Clarinet vs Euphonium (`projects/ballfight_bassclarinet_vs_euphonium`)

The two instruments nobody can name: "a black saxophone" vs "a small tuba". Two new fighters, each with a style no one
else has:

- **BASS CLARINET** (left, slate blue): the only anchor-and-pivot fighter. **ENDPIN ANCHOR**: every bar it drives its
  floor peg into the canvas and whirls round it like a hammer-thrower (1.5 turns in 2 beats); the arm from the peg
  sweeps the whole circle and smashes what it crosses; then it pulls the peg and is flung off on the tangent. The orbit
  grows per landed smash.
- **EUPHONIUM** (right, amber): the only artillery. **BELL MORTAR**: every 2 beats the upright bell lobs brass shells
  that land 2 beats later on the enemy's predicted spot (round its whirl if it is anchored), shadow circles growing
  where they will hit; splash damage; half a shell more per landed salvo.

All are game rules in the shared engine (`projects/ballfight/fight.py`), added in their own small commits (all 11
finished episodes replay identically, `tools/ballfight_replay_hash.py`):

| Kind | What it does |
|---|---|
| weapon `anchor` | the peg goes in `orbit` ahead toward the enemy (clipped so the circle stays inside the ropes), the ball whirls `turns` turns in `whirl_beats`, a swept-arm hit test (one rest per arm), a tangent release with a `release` boost; anchored balls skip `move()`; recorded `ANC` |
| weapon `mortar` | salvos of int(n) shells on the grid (haste divides it), shell 0 on the predicted spot, the rest on a ring of `spread`; `splash` damage on landing; `predict()` follows an anchored enemy round its whirl; recorded `SHELLS` |
| ability `barrage` | NOT A TUBA: `n` shells over `beats` beats on the 8ths, `aimed` at the enemy, the rest on a jittered grid over the ring |
| ability `twin` | MULTIPHONICS: a ghost twin for `dur_beats` - a second whirl arm opposite the peg, or a doubled salvo |
| ability `copy` | IDENTITY CRISIS: the enemy's real last-used ability fired back (same kind + numbers, `trigger(..., ab=, quiet=True)`); none yet: one round of its signature |
| ability `lights_out` | LIGHTS OUT: sunk into shadow for `dur_beats` (intangible, no clash, no attacks, sliding toward the enemy), then an eruption within `radius` |

SLAP TONGUE reuses #37's `snip` kind (a quick wind-up, a heavy knockback smack in front), drawn as a reed paddle.

Abilities: BASS CLARINET LV2 SLAP TONGUE / LV3 MULTIPHONICS / LOW LIGHTS OUT; EUPHONIUM LV2 MARCHING ORDERS (haste: a
salvo every beat) / LV3 IDENTITY CRISIS / LOW NOT A TUBA.

## The two new instruments (Pixel Pitch folder)

`series/instrument-history/bassclarinet_lowpoly.py` and `euphonium_lowpoly.py` (one commit each), bridge entries
`BASS CLARINET` / `EUPHONIUM` in `projects/which_instrument_wins/pp_bridge.py`:
- **Bass clarinet** from reference photos found in Chrome (Commons): the Yamaha YCL-622II front product shot, scaled to
  a 40 mm body (0.29 mm/px), and a Buffet Prestige 1193 at 3/4. Silver neck loop + mouthpiece, keywork, the silver
  U-bow and tilted 104 mm bell, the floor peg. Voice: a chalumeau reed (odd harmonics, ~560 Hz formant, growl) and
  `slap()`, the slap-tongue pop. Scene `ball` = radii x2 for the sprites.
- **Euphonium** from the Yamaha YEP-321 front shot (Commons), scaled with Yamaha's published 280 mm bell / 664 mm
  height (0.385 mm/px), and a B&S 3171A side view. Bell stack, bottom bow, top arch, three top pistons + the diagonal
  4th valve, slides, leadpipe + mouthpiece. Voice: warm conical bore (strong fundamental, soft attack, singing vibrato).
  Scene `ball` = tubing x1.35.

## Tuning and the run

Tuned once (`tune.py`), frozen in `fighters.py` + `timeline.DMG_SCALE = 0.68`: 150 seeds 52 / 48 %, median K.O.
50.6 s. Honest search with the fight at 2 bars and the usual hard windows (`seed_search.py 1 5000 --stop 8`): 8 fits in
2,171 runs, BASS CLARINET 54 % / EUPHONIUM 46 % over 2,092 decided. Seed **1269** (ranked 2nd, 5.18; seed 23 ranked
first but its 52.90 s K.O. left ~0.6 s for COMMENT THE NEXT MATCH-UP, #36's reason): the euphonium's shells chip away
early, IDENTITY CRISIS copies SLAP TONGUE (32.9 s) and later MULTIPHONICS (42.4 s), NOT A TUBA (43.85 s) puts the
euphonium ahead at 45.3 s, LIGHTS OUT erupts under it at 47.2 s and swings the lead back at 47.8 s, BASS CLARINET wins
at 51.66 s with 7.9 HP. Captions SLAP TONGUE 16.2, MARCHING ORDERS 21.4, MULTIPHONICS 27.2, IDENTITY CRISIS 32.9,
NOT A TUBA 43.8 (LIGHTS OUT lights in the HUD list only: 4 s rule).

## Music, mood A: dark drumline

C minor at 125.90 BPM (32 bars; FIGHT! at 3.81 s): snare line, quads (tuned high), five pitched bass drums, cymbal line,
a high marimba + timpani pit; no low brass or woodwinds in the bed. The BASS CLARINET is the bottom (the root as the
peg goes in, an 8th rumble through every whirl, a held growl as it rolls on, slap pops on smashes, the twelfth with
MULTIPHONICS), the EUPHONIUM the tenor melody (a note per salvo, a chord tone a sixth under on a landed salvo, a falling
run for NOT A TUBA). Builds with density, never level: one master gain + compressor + soft limiter. `reelkit.levels`
PASS (opening +0.1 dB vs the ending, spread 3.1 dB, no dips); band check bass clarinet +6.5..+9.3 dB, euphonium
+7.2..+11.7 dB over the bed.

## Commands (from this folder)

```bash
python tune.py 150 ad=4.5 md=2.6 sc=0.68                   # (done once, frozen)
python seed_search.py 1 5000 --stop 8                      # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 1269                                         # the run's story
python render.py                                           # sprite cache (check/sprites_ballfight12.npz, ~1.5 min)
python -m reelkit.sheet render.py test/sheet.png 1.05 2.6 13.55 16.3 27.6 33.3 44.6 46.2 47.25 51.8 55.0 60.85
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                      # subject CoC must be 0 px on every frame
python audio.py && python band_check.py && python -m reelkit.levels audio.wav
python flash_test.py 43.5 52.6
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | Fighters, title (two-line BASS / CLARINET, moved left ball), tempo, count-in, frozen damage scale, story targets, scripted tail, chords, the rumble + melody, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine; story analysis + score |
| `tune.py` | The tuning grid (both fighters' own numbers + the shared scale) |
| `seed_search.py` | Early-stopping search (hard windows incl. no 4 s without a hit) |
| `styles.py` | New styles: the endpin + peg, the whirl's swept light, the twin ghost, the slap paddle, LIGHTS OUT's pool + eruption, the shells' arcs, shadows and splashes |
| `render.py` | #37's ring / HUD / title pipeline with the two new fighters (IDENTITY CRISIS shows an amber bass clarinet in the euphonium's ball) |
| `audio.py` | The drumline + both fighters from the run |
| `band_check.py`, `focus_check.py`, `flash_test.py`, `mux.py` | Checks and the loop-clean encode |
