# #31 BALLFIGHT #5: Turntables vs Synth (`projects/ballfight_turntables_vs_synth`)

DJ vs producer on #27's engine, ring, HUD and count-in (`projects/ballfight`: `fight.py`, `fighters.py`, `scene.py`,
imported, plus `reelkit.ui.fight_hud` and the shared effects kit `reelkit.fx`). Colours from the series table: TURNTABLES
lime (left corner), SYNTH cyan (right). Mood: dark breakbeat, F minor at 137.70 BPM (35 bars).

## v2 (BALLFIGHT v2 redesign, user Oct 24): THE RING IS THE RECORD vs PATCH CABLES

- **TURNTABLES - THE RING IS THE RECORD** (new weapon kind `scratch_floor`): the ring floor is the DJ's record (grooves,
  strobe dots, a lime label; it turns once every 5 bars). Every bar the DJ scratches: the floor jerks forward, then back,
  and each jerk flings the synth around the ring centre into the ropes; the first rope it hits within 0.8 s is a slam
  (1.6 x impact speed / the shared speed, capped at 1.6x), and a landed slam grows the scratch strength (1.3 -> 2.2). The
  DJ's own ball is never moved by the floor: the tonearm's needle anchors it (the arm drops on FIGHT! and parks again for
  the loop). The scratch audio IS the floor motion: `render.scratch_q` is both the floor's extra angle and the record
  position the audio plays. SCRATCH = haste x2 (scratches twice as often), REWIND kept (new kind `rewind` from v1),
  BASS DROP = slam (a giant record drops on the predicted spot).
- **SYNTH - PATCH CABLES** (new weapon kind `tether`): every 2 beats a plug flies at the decks while fewer cables are in
  than its slots (1 -> 4, +1 every second landed plug); a landed plug jabs and plugs a cable in for 8 beats. Voltage pulses
  run down every cable on the 8ths (one arp note each, 0.17 damage on arrival). The leash keeps the decks within 2.4 of the
  synth (they bounce off the end) unless a scratch flings them harder than 0.15 x the cables plugged in: then they all
  snap. The waveform drawn on the cables (and the arp's tone) goes sine -> square -> saw with the synth's level. LFO =
  haste x2 (pulses at double rate, the waveform swells), SAWTOOTH = new ability kind `lash` (the cables go saw-wave and
  lash side to side, cutting), WHITE NOISE = wave.
- **Effects** (`reelkit.fx`): jerks light the whole record; rope slams = lime bursts + vinyl shards + cracks; plugs,
  pulses, snaps and leash bounces = cyan sparks; lash cuts = glitch blocks + claw marks; BASS DROP = shards, crack +
  scorch, heat shimmer; WHITE NOISE = static ring + glitch + shimmer; ability moments on the five captions; capped shake.
- **Honest run, seed 781** (6 fits in 781 runs; balance TURNTABLES 382 / SYNTH 389 of 771 decided): the decks lead early,
  the cables take over at 31.2 s, BASS DROP (39.2) and WHITE NOISE (40.1) land back to back, the decks retake the lead at
  40.4 s and, after three REWINDs (5.1 / 7.3 / 6.4 HP undone), win at 50.80 s with 1.4 HP.
- v1 (vinyl frisbees vs arp satellites, seed 129) is kept as `*_v1_backup.py` and `ballfight_turntables_vs_synth_v1.mp4`;
  its roster entries were replaced, so it no longer replays on the current roster.

## Commands (from this folder)

```bash
python tune.py 64 T.dmg=1.6 S.volt_dmg=0.17,0.185 S.snap=0.15 scale=0.6   # rule tuning (done once, frozen)
python seed_search.py 1 4000 --stop 6                       # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 781                                           # one run's story (cached in check/run_781.pkl)
python render.py                                            # sprite cache (check/sprites_ballfight5.npz, ~20 s)
python -m reelkit.sheet render.py test/sheet.png 1.95 11.0 15.4 19.8 25.2 34.17 40.0 50.85 55.0
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                       # subject CoC must be 0 px on every frame
python audio.py                                             # audio.wav (whole AAC frames) + check/stem_*.wav
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py ballfight_turntables_vs_synth_v2.mp4
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | The episode: fighters, tempo, count-in, frozen damage scale (0.6), story targets, scripted tail, chords, the arp table, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine; story analysis (counts REWIND heals) + score (#27's windows) |
| `tune.py` | Generic rule grid: `T.<weapon key>`, `S.<weapon key>`, `T.u2.<ability key>`, `scale`; balance, K.O., lead changes, rewinds |
| `seed_search.py` | #27's early-stopping search (copied; names from the timeline) |
| `styles.py` | v2: `record_albedo` (the record floor), `tonearm`, `cable_points` (waveform cables), `jack_quad`, `saw_arm_points`; kept: the BASS DROP record, the REWIND tape look + ◀◀, pitched sprites; v1 styles (vinyl discs, arp gems) for crossovers |
| `render.py` | #28's renderer, adapted: the record floor + its angle (`floor_angle`, `scratch_q`), the tonearm, cables / plugs / pulses / snaps / lashes from the run's events, REWIND ghosts, shards, and `reelkit.fx` wired as in projects/ballfight/README.md |
| `audio.py` | Dark breakbeat bed; the scratch audio from the floor curve, rope slams = organ stabs; one arp note per pulse, toned sine / square / saw by level; plug clicks, cable snaps, leash thrums |
| `mux.py` | Loop-clean encode via `projects/maze_race/mux.py` |

## Rules worth knowing

- `scratch_floor` detects rope slams from the engine's own 'rope' events after `move()` (no change to the shared motion
  code); the jerk is a knock, so it decays (KNOCK_T): the snap test happens when the flung ball reaches the leash, which
  is why `snap` is low (0.15 x cables).
- First tuning: slams at 4.0 ended fights by 15 s; at 1.6 with pulses at 0.2 the decks won 55 % but longer fights (lower
  damage scale) went to the synth every time, because a full leash never snapped and also kept the decks' rope slams from
  happening. Lowering `snap` and the pulse damage (0.17) balanced it at 50 % with ~1.1 lead changes per run (v1 ~0.6).
- `tether`, `scratch_floor` and `lash` are reconstructable from events (plug_fire / plug / unplug / volt / snap / leash /
  scratch / jerk / lash), so the recorder is unchanged; `Fight.lash_angle()` is shared by the engine and the renderer.
- #33's ability kind `whip` (straight flicks) is a different attack, so SAWTOOTH got its own kind (`lash`).
