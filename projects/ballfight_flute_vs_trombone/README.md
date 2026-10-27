# #41 BALLFIGHT #15: Flute vs Trombone (`projects/ballfight_flute_vs_trombone`)

The front row vs the back row: a real band-room feud (the flutes sit up front, the trombones behind them poke them with
their slides); now the flute answers. A **crossover**: FLUTE (left, silver) exactly as built in #32 v2 and TROMBONE
(right, mint) exactly as built in #34, straight from `projects/ballfight/fighters.py` (weapons, numbers, abilities,
looks, Pixel Pitch voices):

- **FLUTE** SNAKE CHARMER: drops woven baskets where it stands (up to 2); silver cobras rise, sway and strike on the beat
  (strike reach + speed grow per landed bite). FLUTTER TONGUE / SNAKE DANCE / PICCOLO.
- **TROMBONE** SLIDE LANCE: aims and shoots the slide out on every beat (reach grows per landed hit), swats projectiles it
  passes through. SAD TROMBONE / POWER SLIDE / PEDAL TONE.

## Sides

First named = left, both swapped from their own episodes. The engine seats fighters in the order they are passed
(`timeline.FIGHTERS = ('FLUTE', 'TROMBONE')`); the roster's `corner` values are untouched. The title card's panel
patterns need no mirroring: the trombone's slide tubes (`title.py`) and the flute's key cups (#32's, registered here as
#32 registered them) both run parallel to the divider.

## Lance vs cobras: a new opt-in rule (engine commit b346575)

The engine didn't define it: the lance's `parry` only ever touched projectiles, and cobras are basket state the enemy
never checked. Added as an opt-in rule, `Rules(swat_cobras=True)` (on only here, `timeline.SWAT_COBRAS`): a live lance
(shooting out or held) that passes through a risen cobra - its body over the basket, or its head while a strike is in
the air - knocks it flat. A strike in the air misses, and the cobra can't strike for `swat_beats` (2) beats while it
curls back up. No damage, no growth for either side. Event `('swat', lance owner, cobra owner, basket id, x, y)`.
Off by default: all 15 finished episodes replay identically (`tools/ballfight_replay_hash.py`, 75 runs). Seed 392's
lance swats 23 cobras. Picture: the cobra is slapped flat in ~0.07 s (mint flash on the silver, mint burst + scales +
brass sparks), lies there a beat leaning away, then curls back up over the second. Sound: a leathery slap + a startled
hiss.

## Crossover rule: the only lever is the damage handicap

`Rules(dmg_side)` (from #36), frozen in `timeline.DMG_SIDE`, shared scale 1.0: **FLUTE x2.0, TROMBONE x0.40** (`tune.py`,
300 seeds: FLUTE 47 % / TROMBONE 53 % of 277 decided, median K.O. 49.6 s; both desperation moves fire in 69 %). The lance
lands ~62 hits a fight, the cobras ~36. For reference: the trombone ran x0.35 vs the tuba (#36), the shared scale 0.72
vs the horn (#34); the flute 1.06 vs the tuba (#32). The pair is a "whoever's ahead stays ahead" match-up (0.6 lead
changes per fight on average), so fits are rare (~1 %), but damage alone balances it: no fighter changed.

## The run: honest seed 392

`seed_search.py 1 6000 --stop 8` (fight at 2 bars, the usual hard windows): 8 fits in 968 runs, FLUTE 49 % / TROMBONE
51 % of 899 decided. **392** ranked first (score 7.78): dead even to 35 s (34-33 HP), lead changes at 22.5 and 42.9 s,
PICCOLO at 42.6 s (lands, 16 HP) answered 0.5 s later by PEDAL TONE (highest note vs lowest; the bell misses), and the
TROMBONE wins at **50.02 s** with **4.1 HP**. Captions FLUTTER TONGUE 11.2, SAD TROMBONE 16.2, POWER SLIDE 26.0, SNAKE
DANCE 30.1, PICCOLO 42.6 (PEDAL TONE 43.1 lights in the HUD list only: 4 s rule). Ranked list in `check/seeds_v1.txt`.

Tail: the flute was K.O.'d on the trombone's side of the ring, so the straight glides home crossed and the balls passed
through each other; each glide now bows sideways (`render.GLIDE_ARC`: sin(pi gl), zero at both ends, so the corners and
the loop seam are unchanged), 1.41 apart at the closest (2R = 1.2).

## Picture

`render.py` = #36's renderer (ring, #34's slide lance + PEDAL TONE's bell, effects, title card, HUD ability list) with
#32 v2's flute merged in (baskets, cobras, SNAKE DANCE's slither, PICCOLO's shivering ring + glass cracks, the pixel-art
flute sprite + cool glass, flute shards). `styles.py` imports both episodes' draw functions (no copies). Silver vs mint
checked on the early sheet: the flute's ball is cool steel glass, the trombone's green, plus their own HUD bars and
middle ropes: they read apart, so no extra edge glow. SNAKE DANCE stays lit in the HUD list while the cobras hunt.

## Music, mood A: sinister big top

E harmonic minor at 133.77 BPM (`loop_bpm(132)`, 34 bars; FIGHT! at 3.59 s), Em Am B7 Em | C Am B7 B7 from FIGHT! (the
last two bars fall into the title slam's Em). The bed has no brass and no flute: a calliope / band organ (breathy,
slightly out-of-tune steam pipes) chunking the chords on the off-beats, a concert bass drum on 1 + 3, a choked cymbal on
2 + 4, snare rolls into sections; from bar 18 the march snare's 8ths, tambourine, a held band-organ chord, glockenspiel.
The **TROMBONE is the bass and the smears**: each thrust is the circus bass (root on 1 + 3, fifth on 2 + 4) with a smear
up into it, POWER SLIDE walks 8ths, landed lances smear up the chord, SAD TROMBONE is the clown's wah-wah down to the
tonic, PEDAL TONE the pedal E1. The **FLUTE is the high melody** (`timeline.MEL_8THS`, E5-F#6, one note per 8th while
its cobras are up; FLUTTER TONGUE flutters it with 16th grace notes; SNAKE DANCE slithers a semitone down after each
note), hiss-clicks on strikes, accents on bites, PICCOLO's shriek on E7. Thinning when hurt: splitmix hash, <= 30 %,
never two dropped in a row (#37's fix). The winner's solo: 3 bars (Em C Am), then the champion goes back to the bass line
for the reassembly bar; the last bar climbs B7 into the slam. One master gain + compressor + soft limiter (no automation):
`reelkit.levels` PASS (opening +0.0 dB vs the ending, spread 3.6 dB, no dips); band check trombone +6.6..+7.8 dB
(70-700 Hz), flute +12..+16 dB (600-3200 Hz) over the bed; intro -1.5 LU vs the fight (`loudness.py`).

## Commands (from this folder)

```bash
python tune.py 300 hf=2.0 ht=0.40                          # handicap tuning (frozen in timeline.DMG_SIDE)
python seed_search.py 1 6000 --stop 8                      # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 392                                          # the run's story
python render.py                                           # sprite cache (check/sprites_ballfight15.npz, ~20 s)
python -m reelkit.sheet render.py test/sheet.png 0.0 1.2 2.4 7.65 11.3 16.3 30.4 35.05 42.75 43.7 50.1 54.5
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                      # subject CoC must be 0 px on every frame
python audio.py && python band_check.py && python -m reelkit.levels audio.wav && python loudness.py
python flash_test.py 40.5 51.5
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | Fighters + sides, title, tempo, count-in, the frozen handicap + the swat rule, story targets, scripted tail, chords, the bass / walk lines, the flute melody, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine (with `dmg_side`, `swat_cobras`); story analysis + score |
| `tune.py` | Handicap grid (`hf=` FLUTE, `ht=` TROMBONE damage scale): win share, K.O. time, fit rate |
| `seed_search.py` | Early-stopping search (hard windows incl. no 4 s without a hit) |
| `styles.py` | No new weapon styles: imports #34's and #32's |
| `render.py` | #36's ring / lance / HUD / title pipeline with #32's flute merged in, + the swat and the bowed glide home |
| `audio.py` | The big top + both fighters from the run |
| `band_check.py`, `focus_check.py`, `flash_test.py`, `loudness.py`, `mux.py` | Checks and the loop-clean encode |
