# #34 BALLFIGHT #8: Trombone vs French Horn (`projects/ballfight_trombone_vs_horn`)

The brass section's pecking order on #27's engine, ring, HUD and count-in (`projects/ballfight`: `fight.py`,
`fighters.py`, `scene.py`, imported, plus `reelkit.ui.fight_hud`). TROMBONE (left corner, mint) aims its slide at the
enemy and shoots it out like a lance on every beat (new blade option `mode='thrust'`: reach 0.95 -> 2.5, +0.08 per
landed hit; the lance swats arrows it passes through), so its thrusts are the low-brass ostinato. HORN (right corner,
copper) sounds a hunting call every two beats: slow arrows that curve in on the enemy (#29's `curve` volley + the new
`curve_grow`: 52 -> 137 deg/s; +half an arrow per landed volley, max 6). Abilities: SAD TROMBONE (shockwave + the new
`slow`: the horn moves at 0.6x for 1 s, "wah wah wah waaah") / POWER SLIDE (haste: thrusts on the 8ths) / PEDAL TONE
(slam: a giant brass bell drops in and stomps); STOPPED HORN (shield: a copper bell rim) / HORN RIP (an arc of 7
curving arrows, the new arc option `curve=True`) / TALLY-HO (wave: a ring of chevrons). Mood: cinematic brass trailer,
D minor at 121.97 BPM (31 bars), a brass-less bed (low-string 16th ostinato, braams, taiko, high tremolo).

Honest run, seed 1261 (8 fits in 2,848; rules frozen after tune.py): the lance builds an early lead (90-79 at 14 s),
the horn's curving arrows drag it level by 32 s, the lead flips at 34.0 / 41.6 / 44.0 / 45.3 s, PEDAL TONE's bell
lands at 45.3 s, TALLY-HO answers at 45.8 s, and TROMBONE wins at 50.21 s with 4.8 HP. Balance over 2,776 decided runs:
TROMBONE 51 %, HORN 49 %.

## Commands (from this folder)

```bash
python tune.py 300 H.dmg=1.4,1.5,1.6 scale=0.47            # rule tuning (done once, frozen in fighters.py + timeline.py)
python seed_search.py 1 4000 --stop 8                      # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 1261                                         # one run's story (cached in check/run_1261.pkl)
python render.py                                           # sprite cache (check/sprites_ballfight8.npz, ~30 s)
python -m reelkit.sheet render.py test/sheet.png 0.0 1.0 9.0 13.05 17.85 31.6 45.3 50.25 54.6 59.6
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                      # subject CoC must be 0 px on every frame
python audio.py                                            # audio.wav (2860 AAC frames) + check/stem_*.wav
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
python ../../tools/ballfight_replay_hash.py                # engine changes: finished episodes must replay identically
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | The episode: fighters, title, tempo, count-in, frozen damage scale, story targets, scripted tail, chords, the trombone's ostinato and the horn's call, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine; story analysis + score (#27's windows) |
| `tune.py` | Generic rule grid: `T.<weapon key>`, `H.<weapon key>`, `T.u1.<ability key>`, `scale` (pass `scale=` or it runs at 1.0) |
| `seed_search.py` | #27's early-stopping search (copied; names from the timeline) |
| `styles.py` | Draw functions for the new styles: the `slide` lance (+ thrust streak), `arrow` (+ curving trail), the STOPPED HORN `bellrim`, PEDAL TONE's giant bell (flat-shaded surface of revolution) |
| `render.py` | #28's renderer, copied, with this pair's weapons, abilities, the slowed ball's wobble, the lance's lit line of attack, brass shards and the solo played on the slide (lower note = longer slide) |
| `audio.py` | Brass-less trailer bed; thrusts = the trombone ostinato (glissando into each stab), volleys = the horn's hunting call (horn fifths when it leads) |
| `mux.py` | Loop-clean encode via `projects/maze_race/mux.py` |

## Rules worth knowing

- First guesses were lopsided both ways: strong homing (1.2 rad/s, +0.2 per landed volley) won every run by ~20 s;
  slower arrows (3.0 vs the balls' 2.6) with weak homing and a parrying lance flipped it to the trombone winning all
  of them. The lance's parry is what balances curving arrows: the trombone swats ~100 arrows a fight.
- At the frozen 0.47 damage scale the desperation moves needed raw damage 24 / 20 to matter (they're scaled too).
- 80 seeds said 49 % trombone; 812 decided runs said 61 %, so HORN arrows went 1.4 -> 1.5 (300 seeds: 49 %) before the
  final search. Check balance on a few hundred seeds, not 80.
- Lead changes stay ~0.5 per run (as #27 / #28: both weapons grow on landed hits); fit rate ~0.3 %.
- The loop seam needed a longer pre-beat ramp on the beat pulses (120 ms instead of 70): this fight changes little per
  frame, so the downbeat on frame 0 stood out (2.6x -> 1.5x).
