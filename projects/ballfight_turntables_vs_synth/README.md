# #31 BALLFIGHT #5: Turntables vs Synth (`projects/ballfight_turntables_vs_synth`)

DJ vs producer on #27's engine, ring, HUD and count-in (`projects/ballfight`: `fight.py`, `fighters.py`, `scene.py`,
imported, plus `reelkit.ui.fight_hud`). TURNTABLES (lime, left corner) throws vinyl frisbees that ricochet off the ropes
(volley with the new `bounce` option: up to 2 rope bounces, on the 16th grid via `on16`; +1 disc every second landed
volley, up to 8), and every throw is a scratch. SYNTH (cyan, right corner) is an arpeggiator: note gems orbit its ball
and hit what they touch (new weapon kind `orbit`: 2 -> 8 satellites, +1 every second landed hit; they also swat records
they touch), and every satellite passing the top of its orbit plays its note of the arpeggio. Abilities: SCRATCH (haste
x4 = discs on the 8ths for a bar) / REWIND (new ability kind `rewind`: undoes the last 2 s of damage taken and zips the
ball back along its own recorded path, intangible meanwhile; it waits until it would undo >= 5 HP) / BASS DROP (slam: a
giant record drops on the spot); LFO (haste x2 + an orbit-radius wobble) / SAWTOOTH (arc of 9 saw-tooth notes) / WHITE
NOISE (wave of static). Mood: dark breakbeat, F minor at 137.70 BPM (35 bars). Colours from the series table.

Honest run, seed 129 (5 fits in 666 runs; rules frozen after tune.py): the satellites edge the opening, the decks' records
take over at 21.2 s and build a 75-54 lead with two REWINDs (7.2 and 8.8 HP undone, 27.9 and 41.0 s), then 8 satellites
shred the decks from 51 to 4 HP in four seconds (lead change 47.0 s); WHITE NOISE (42.3) and BASS DROP (47.1) both fire
and TURNTABLES win at 49.72 s with 1.5 HP. Balance over 631 decided runs: TURNTABLES 47 %, SYNTH 53 %.

## Commands (from this folder)

```bash
python tune.py 60 S.base=0.9 S.grow=0.5 T.dmg=0.33 S.dmg=3.8,4.2 scale=0.9   # rule tuning (done once, frozen)
python seed_search.py 1 3000 --stop 5                       # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 129                                           # one run's story (cached in check/run_129.pkl)
python render.py                                            # sprite cache (check/sprites_ballfight5.npz, ~20 s)
python -m reelkit.sheet render.py test/sheet.png 1.0 13.7 18.5 28.1 42.5 47.75 49.8 55.0
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                       # subject CoC must be 0 px on every frame
python audio.py                                             # audio.wav (whole AAC frames) + check/stem_*.wav
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | The episode: fighters, tempo, count-in, frozen damage scale, story targets, scripted tail, chords, the arp table, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine; story analysis (counts REWIND heals) + score (#27's windows) |
| `tune.py` | Generic rule grid: `T.<weapon key>`, `S.<weapon key>`, `T.u2.<ability key>`, `scale`; balance, K.O., lead changes, rewinds |
| `seed_search.py` | #27's early-stopping search (copied; names from the timeline) |
| `styles.py` | Draw functions for the new looks: `vinyl` records, `arp` gems + the orbit line, `saw` notes, the BASS DROP record, the REWIND tape look + ◀◀, and sprites tipped toward the camera (`pitch`) |
| `render.py` | #28's renderer, copied, with this pair's weapons, REWIND ghosts, rope twangs on ricochets, shards (vinyl, label, tonearm / keys, knobs, gems) and the solo's groove rings |
| `audio.py` | Dark breakbeat bed; throws = scratches (the Pixel Pitch deck's own motion curves on a vocal-free stab), landed records = organ stabs; satellites crossing the top = the arpeggio |
| `mux.py` | Loop-clean encode via `projects/maze_race/mux.py` |

## Rules worth knowing

- `orbit` reuses the recorder: `TH` = the orbit angle, `NV` = satellites, `BL` = the live radius (it wobbles under LFO).
  Satellites sit on 8 fixed seats (0, 4, 2, 6, 1, 5, 3, 7) so a new one never moves the others.
- REWIND is a rule, not an edit: the ball keeps its own record (one entry per substep, 2 s deep); HP goes back to its
  value 2 s ago (a `heal` event, counted by `lead_changes`) while it zips back along that path in one beat.
- First tuning: the satellites barely reached the decks (12 hits vs 200 records) until the orbit moved out to 0.9 beyond
  the surface; +1 satellite per landed hit snowballed, so it grows every second hit. Lead changes are rare for this pair
  too (~0.6 per run); the fit rate (~0.75 %) is fine for an early-stopping search (~3 min).
- The deck model is black: its sprite is tipped 45 deg (round platter) and lifted (+30) so it reads through the glass.
