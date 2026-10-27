# #40 BALLFIGHT #14: Bari Sax vs Bass Clarinet (`projects/ballfight_barisax_vs_bassclarinet`)

The real one vs the lookalike: the bass clarinet keeps getting called "the black saxophone". A **crossover**: BARI SAX
(left, coral) exactly as built in #39 v2 and BASS CLARINET (right, slate blue) exactly as built in #38, straight from
`projects/ballfight/fighters.py` (weapons, numbers, abilities, looks, Pixel Pitch voices):

- **BARI SAX** WRECKING BALL: every other bar a crane cable drops from the spot's rig, hooks its harness and swings it
  across the ring as a pendulum (cable grows per landed smash). GROWL / SKANK / CABLE SNAP.
- **BASS CLARINET** ENDPIN ANCHOR: every bar it plants its floor peg and whirls round it like a hammer-thrower (orbit
  grows per landed smash). SLAP TONGUE / MULTIPHONICS / LIGHTS OUT.

## v2 (user feedback, Oct 27)

"The bari sax model needs more detail, as it is completely smooth right now ... While the Bari Sax is being swung, it
should be able to temporarily go outside/over the ropes so it can actually get a full swing without having to be in the
middle of the ring. The music in the intro is a little loud (only a problem in this one)."

- **Model** (`series/instrument-history/barisax_lowpoly.py`, own commit 567ced7): in the ball-sprite scene every tube is
  fattened x1.35, but the keys sat on the unscaled body, buried inside it; they were gold on gold; rods, guards and
  engraving were lines, which the sprites drop. Rebuilt from the Amati low-A product shot (Commons, studied in Chrome):
  deep cups on the real surface with dark pad rings, white pearl touches on posts, long rods on posts, guards with
  diamond feet (side keys, low keys, bell keys), bow bands, bow guard + felt, a wide bright rim, engraving, a gold
  ligature; all faces. Voice byte-identical. (Chrome asked for a site permission on a second Commons page mid-way; the
  one reference was enough, so that wasn't pushed.)
- **Over the ropes** (engine df67dcd, opt-in; roster 45c9190): the wrecking weapon's `overhang` lets the swing's ends
  reach 1.3 beyond the in-ring limit (the bottom stays in the ring) but never past a corner post (`corner_l1`,
  |x| + |z| <= 5.2; the left / right corners are at the frame edges); a far end outside drops onto the ropes and is
  slung back in (`return`, 3 16ths). Full-amplitude swings 35 % -> 76 %, cramped ones (< 25 deg) 41 % -> 6 %. It is the
  bari's standard now; #39's sim.py pins it off so #39 replays as delivered. Picture: the ropes are pressed down under
  the ball where it passes over them and spring back with a wobble (+ a low twang) as it lifts off; over a front rope
  it is drawn after the front ropes; the slung-back return uses the WRECK record's height.
- **Re-tune + new honest seed**: BARI SAX x0.30 / BASS CLARINET x0.66 (250 seeds 48 / 52 %, median K.O. 50.8 s);
  `seed_search.py 1 8000 --stop 6`: 6 fits in 189 runs (45 / 55 %). **Seed 128** (top score 10.05): the bari is
  hoisted out over the ropes on 11 of its 15 swings (12 of them full) and slung back in at 49.2 s; LIGHTS OUT erupts
  under it at 39.6 s, CABLE SNAP lands at 43.9 s, the lead swings back at 43.8 s and the wrecking ball K.O.s the bass
  clarinet at 50.80 s with the bari on 0.7 HP. Captions GROWL 12.9, SLAP TONGUE 18.1, SKANK 24.3, LIGHTS OUT 37.7,
  CABLE SNAP 42.4 (MULTIPHONICS 26.7 lights in the HUD list only: 4 s rule).
- **Intro**: RMS matched the fight, but K-weighted (`loudness.py`, BS.1770) the title card was +0.7 LU over it with
  momentary peaks 2-3 LU over: three full machine hits (sheet metal, pipes, grinder, anvil) on the card's slams and
  three more in the count-in. Thinned by arrangement, not level (one sheet-metal crash, lighter VS / name-B / count
  hits): intro -1.4 LU vs the fight, level with the ending; `reelkit.levels` PASS.
- **Focus**: this run's CABLE SNAP hoists the bari 3 up over the front corner, near the camera: sharp band 0.26.

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

## Crossover rule: the only lever is the damage handicap (v1 numbers; v2 above)

`Rules(dmg_side)` (from #36), frozen in `timeline.DMG_SIDE`, shared scale 1.0: **BARI SAX x0.35, BASS CLARINET x0.68**
(`tune.py`, 250 seeds: 52 % / 48 % of 242 decided, median K.O. 49.7 s; both desperation moves fire in 88 %). For
reference the bari ran at 0.32 vs the bassoon (#39 v2), the bass clarinet at 0.68 vs the euphonium (#38).

## v1's run: honest seed 617 (superseded by v2's seed 128; v1 is at commit 8115d67)

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
python tune.py 250 hb=0.30 hc=0.66                          # handicap tuning (v2, frozen in timeline.DMG_SIDE)
python seed_search.py 1 8000 --stop 6                      # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 128                                          # the run's story
python render.py                                           # sprite cache (check/sprites_ballfight14.npz, ~45 s)
python -m reelkit.sheet render.py test/sheet_v2.png 1.1 3.0 12.05 18.2 24.4 37.07 37.25 39.7 43.95 49.3 50.85 52.6
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                      # subject CoC must be 0 px on every frame
python audio.py && python band_check.py && python -m reelkit.levels audio.wav && python loudness.py
python flash_test.py 36.5 51.2
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
| `band_check.py`, `focus_check.py`, `flash_test.py`, `loudness.py`, `mux.py` | Checks (incl. K-weighted loudness per section) and the loop-clean encode |
