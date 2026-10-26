# #37 BALLFIGHT #11: Clarinet vs Oboe (`projects/ballfight_clarinet_vs_oboe`)

The reed war, single reed vs double reed. Two new fighters, each with a style no one else has:

- **CLARINET** (left, lavender): the only teleporter. **TWELFTH JUMP**: the register key makes the clarinet jump a
  twelfth, so on every beat the ball vanishes and reappears past the enemy; the zip line between its two spots cuts.
- **OBOE** (right, olive): the only debuffer. **TUNING A**: every bar a narrow tuning beam; a hit knocks the enemy
  **OUT OF TUNE** for 2 bars: its hits land weaker, it wobbles (a doubled, warped, sour outline) and every one of the
  oboe's nasal 8th notes stings it. Its notes play flat (up to a quarter-tone) with a beating second reed.

Both are game rules in the shared engine (`projects/ballfight/fight.py`), added in their own small commits (all 10
finished episodes replay identically, `tools/ballfight_replay_hash.py`):

| Kind | What it does |
|---|---|
| weapon `jump` | target locks `lock16` 16ths before the beat; the ball lands `land` past it (seeded +-`jitter` deg), at most `reach` away (+`grow` per landed cut); the zip line cuts the enemy if it passes within R + `cut` |
| ability `rhapsody` | RHAPSODY: 5 zig-zag jumps through the enemy on the 8ths, each a cut |
| ability `stun` | SQUEAK: a shrill burst ring; the fighter it hits holds still and starts no attack for 0.5 s (every weapon dispatch checks it) |
| weapon `tune` + status `detune` | the A beam (lock 3 16ths, fired 3 16ths after the bar line); a landed beam = OUT OF TUNE: hits x (1 - 0.6 x depth), stings `sting` x depth on the 8ths; depth 0.4 +0.1 per landed beam |
| ability `endless` | ENDLESS NOTE: the beam held for a bar, turning to the enemy at 1.4 rad/s, biting on the 8ths |
| wave option `detune` | ILL WIND: a wave that puts the enemy fully out of tune |
| ability `snip` | DOUBLE REED: the reed's blades scissor out at a close enemy and snap shut |

Abilities: CLARINET LV2 SQUEAK / LV3 CLARION (haste: jumps on the 8ths) / LOW RHAPSODY; OBOE LV2 DOUBLE REED / LV3
ENDLESS NOTE / LOW ILL WIND.

## The oboe (new in the Pixel Pitch folder)

`series/instrument-history/oboe_lowpoly.py` (own commits: model, then voice), bridge entry `OBOE` in
`projects/which_instrument_wins/pp_bridge.py`:
- **Model** from reference photos (Wikimedia Commons, via the built-in browser: Chrome wasn't reachable): the Yamaha
  YOB-831 front product shot (measured at 590 mm body length: crown bulb 25 mm, upper joint 16.8 -> 21 mm, tenon ring
  at 39.5 %, lower joint to 26 mm, bell ring at 79 %, the bulb-rimmed bell 49 mm), the Lorée with reed (green thread),
  a modern reed against a ruler (67 mm). Scene `ball` = radii x1.6 about the bore so it reads in the small ball sprites.
- **Voice**: additive double reed, every harmonic (conical bore), formants ~1.2 kHz and ~3 kHz, a weak fundamental in
  the low register, a tongued attack with reed buzz, vibrato blooming after it.

## Tuning and the run

Tuned once (`tune.py`), frozen in `fighters.py` + `timeline.DMG_SCALE = 0.55`: 200 seeds CLARINET 47 % / OBOE 53 %,
median K.O. 50.8 s; beams land ~64 %, cuts ~60 %. Honest search with the fight at 2 bars and the usual hard windows
(`seed_search.py 1 3000 --stop 8`): 8 fits in 654 runs, balance 51 / 49; seed **654** ranked first (7.87): even to
20 s, the oboe's detune grinds out the lead from 22 s (65-52 at 28 s), RHAPSODY at 42.8 s and ILL WIND at 43.6 s trade,
18-18 at 46 s, and the clarinet retakes the lead 0.2 s before its K.O. at 51.90 s with 6.5 HP. Captions SQUEAK 12.3,
DOUBLE REED 17.0, ENDLESS NOTE 23.7, CLARION 28.4, RHAPSODY 42.8 (ILL WIND lights in the HUD list only: 4 s rule).

## Music, mood A: Baroque storm

D minor at 141.64 BPM (36 bars; FIGHT! at 3.39 s). Harpsichord continuo, racing violin 16ths, driving cello / bass 8ths,
timpani, thunder; no woodwinds in the bed. The CLARINET is the lower line (a chalumeau note per jump, the twelfth above
on every landed cut), the OBOE the higher one (its A with every beam, running 8ths = the stings). The storm builds with
parts (violin II, violas, arpeggios, timpani on 3), never with level: one master gain + compressor + soft limiter.
`reelkit.levels` PASS (opening +0.3 dB vs the ending, spread 4.3 dB, no dips); band check clarinet +7.3..+9.2 dB,
oboe +9.5..+12.1 dB over the bed.

## Commands (from this folder)

```bash
python tune.py 200 oo=3 ol=3 oe=0 ox=0.12 cc=0 cl=2 cj=40 cd=3.6 os=0.75 sc=0.55 ow=0.6   # (done once, frozen)
python seed_search.py 1 3000 --stop 8                      # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 654                                          # the run's story
python render.py                                           # sprite cache (check/sprites_ballfight11.npz, ~1.5 min)
python -m reelkit.sheet render.py test/sheet.png 1.2 3.5 12.4 17.1 17.4 24.4 29.0 43.0 43.9 51.95 55.2 60.85
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                      # subject CoC must be 0 px on every frame
python audio.py && python band_check.py && python -m reelkit.levels audio.wav
python flash_test.py 42.0 52.6
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | Fighters, title, tempo, count-in, frozen damage scale, story targets, scripted tail, chords, both lines, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine; story analysis + score |
| `tune.py` | The tuning grid (both fighters' own numbers + the shared scale) |
| `seed_search.py` | Early-stopping search (hard windows incl. no 4 s without a hit) |
| `styles.py` | New styles: zip line + vanish / appear bursts, the reed + its scissoring blades, the A beam, OUT OF TUNE wobble, SQUEAK ring, stun stars |
| `render.py` | #36's ring / HUD / title pipeline with the two new fighters |
| `audio.py` | The Baroque storm + both fighters from the run |
| `band_check.py`, `focus_check.py`, `flash_test.py`, `mux.py` | Checks and the loop-clean encode |
