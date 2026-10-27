# #40 BALLFIGHT #14: Bari Sax vs Bass Clarinet (`projects/ballfight_barisax_vs_bassclarinet`)

The real one vs the lookalike: the bass clarinet keeps getting called "the black saxophone". A **crossover**: BARI SAX
(left, coral) exactly as built in #39 v2 and BASS CLARINET (right, slate blue) exactly as built in #38, straight from
`projects/ballfight/fighters.py` (weapons, numbers, abilities, looks, Pixel Pitch voices):

- **BARI SAX** WRECKING BALL: every other bar a crane cable drops from the spot's rig, hooks its harness and swings it
  across the ring as a pendulum (cable grows per landed smash). GROWL / SKANK / CABLE SNAP.
- **BASS CLARINET** ENDPIN ANCHOR: every bar it plants its floor peg and whirls round it like a hammer-thrower (orbit
  grows per landed smash). SLAP TONGUE / MULTIPHONICS / LIGHTS OUT.

## Sides

First named = left, the reverse of their own episodes. The engine seats fighters in the order they are passed
(`timeline.FIGHTERS = ('BARI SAX', 'BASS CLARINET')`); the roster's `corner` values are untouched. As for every
right-corner fighter, the engine mirrors spin by side, so the endpin whirl turns clockwise here.

## Engine fix (own commit 8776314, user's call)

The crane aimed the swing and CABLE SNAP by extrapolating the enemy along its velocity. Against a whirling (anchored)
bass clarinet that velocity is its spin, so on the old engine CABLE SNAP landed **3 %** of the time (96 % vs the bassoon
in #39), 77 % of craters hit an empty corner, and swings smashed 50 %. Flagged to the user (crossover rule); they chose
the fix: when the enemy is anchored the crane leads it with the engine's existing `predict()` (round its whirl, as the
euphonium's mortar does). It only acts when an anchorer is in the fight, so all 14 finished episodes replay identically
(`tools/ballfight_replay_hash.py`). With it: swings smash ~80 %, CABLE SNAP lands ~57 %.

## Crossover rule: the only lever is the damage handicap

`Rules(dmg_side)` (from #36), frozen in `timeline.DMG_SIDE`, shared scale 1.0: **BARI SAX x0.35, BASS CLARINET x0.68**
(`tune.py`, 250 seeds: 52 % / 48 % of 242 decided, median K.O. 49.7 s; both desperation moves fire in 88 %). For
reference the bari ran at 0.32 vs the bassoon (#39 v2), the bass clarinet at 0.68 vs the euphonium (#38).

## The run: honest seed 617

Search with the fight at 2 bars and the usual hard windows (`seed_search.py 1 5000 --stop 8`): 8 fits in 617 runs,
BARI SAX 48 % / BASS CLARINET 52 % of 602 decided. Seed 309 scored highest (8.37) but its CABLE SNAP misses and LIGHTS OUT
falls inside the 4 s caption gap; **617** (7.52): the wrecking ball smashes early, the bass clarinet takes the lead at
17.2 s and keeps it for 33 s (SLAP TONGUE, MULTIPHONICS' twin arm), CABLE SNAP hoists the bari over the bass clarinet's
spot at 38.6 s and **lands** at 40.0 s, LIGHTS OUT sinks the bass clarinet at 43.4 s (its eruption misses), and with the
bari on 1 HP its wrecking ball lands the K.O. at **50.55 s**: BARI SAX wins with **0.9 HP** (lead changes 17.2, 50.6 s).
Captions GROWL 12.9, MULTIPHONICS 22.4, SKANK 32.4, CABLE SNAP 38.6, LIGHTS OUT 43.4 (SLAP TONGUE 16.2 lights in the HUD
list only: 4 s rule). 14 swings, 11 smashes, 23 whirls.

## Picture

`render.py` = #39's renderer (ring, crane, harness, swing smear + trail, CABLE SNAP's target, HUD ability list, title)
with #38's bass clarinet merged in (endpin rod + peg, whirl trail, MULTIPHONICS' ghost twin, the slap paddle, LIGHTS
OUT's sink, pool, eruption and arena dim). `styles.py` imports both episodes' draw functions (no copies). LIGHTS OUT's dim
also keeps a band along the crane line, so the cable and both balls stay readable. Focus: sharp band 0.19 (was 0.17)
so the bari hoisted 3 up for CABLE SNAP stays at 0 px CoC.

VS title card: BARI SAX / VS / BASS / CLARINET. The shared card's two-line option only covered name A, so
`TITLE_LINES` may now also stack name B (own commit 23da138, opt-in; earlier layouts identical). Both panel patterns
(#38's grenadilla + pad cups, #39's key stack + pigtail) were drawn for the side each fought on, so `styles.mirrored`
reflects them across the divider onto the swapped panels (idempotent: `title.py` re-imports the renderer by name).
`TITLE_BALLS` moves the right ball clear of VS and the two lines.

## Music, mood A: industrial construction site

G minor at 125.90 BPM (32 bars; FIGHT! at 3.81 s), Gm Eb Cm D (every bari root, G2 / Eb2 / C2 / D2, inside its
range). The bed has no bass instrument: a tight distorted kick, an anvil backbeat, metal ticks, a struck-pipe ostinato on
the chord tones (G4-G5), a high distorted grinder chug, sheet-metal crashes, hydraulic hisses and air-wrench fills.
The BARI is the bass line: #39 v2's doppler honk on every swing (SKANK's up the octave) and, while it rolls between
swings, the root / fifth pumped on the beats (`timeline.BARI_PUMP`). The BASS CLARINET plays #38's growling counter-line
an octave up (`timeline.COUNTER_8THS`, G3-D4): the peg's root, 8ths through every whirl, slap pops on smashes,
MULTIPHONICS' twelfth, LIGHTS OUT's growl and the eruption's slap. The "loser thins" high-pass is capped at ~110 Hz
for the bari so the bass line keeps its fundamental. One master gain + compressor + soft limiter:
`reelkit.levels` PASS (opening +0.1 dB vs the ending, spread 3.3 dB, no dips); band check bari +5..+7 dB, bass clarinet
+4..+7 dB over the bed while they sound.

## Commands (from this folder)

```bash
python tune.py 250 hb=0.35 hc=0.68                          # handicap tuning (done once, frozen in timeline.DMG_SIDE)
python seed_search.py 1 5000 --stop 8                      # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 617                                          # the run's story
python render.py                                           # sprite cache (check/sprites_ballfight14.npz, ~45 s)
python -m reelkit.sheet render.py test/sheet.png 0.0 1.1 2.4 4.75 13.0 16.4 39.2 40.12 44.4 50.6 52.2 60.85
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                      # subject CoC must be 0 px on every frame
python audio.py && python band_check.py && python -m reelkit.levels audio.wav
python flash_test.py 38.4 51.6
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | Fighters + sides, title (two-line name B, moved ball), tempo, count-in, the frozen handicap, story targets, scripted tail, chords, the bari's pump, the counter-line, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine (with `dmg_side`); story analysis + score |
| `tune.py` | Handicap grid (`hb=` BARI SAX, `hc=` BASS CLARINET damage scale): win share, K.O. time, fit rate |
| `seed_search.py` | Early-stopping search (hard windows incl. no 4 s without a hit) |
| `styles.py` | No new weapon styles: imports #39's and #38's; `mirrored()` for the title panels |
| `render.py` | #39's ring / crane / HUD / title pipeline with #38's bass clarinet merged in |
| `audio.py` | The construction site + both fighters from the run |
| `band_check.py`, `focus_check.py`, `flash_test.py`, `mux.py` | Checks and the loop-clean encode |
