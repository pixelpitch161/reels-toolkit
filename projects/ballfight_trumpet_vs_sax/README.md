# #29 BALLFIGHT #3: Trumpet vs Sax (`projects/ballfight_trumpet_vs_sax`)

Jazz's oldest argument, who's the lead horn, on #27's engine, ring, HUD and count-in (`projects/ballfight`: `fight.py`,
`fighters.py`, `scene.py`, imported, plus `reelkit.ui.fight_hud`), with effects from `reelkit.fx`.

**v3 (BALLFIGHT v2 redesign, user Oct 24): the instrument's body is the weapon.** TRUMPET (left, yellow): **VALVE
COMBO** (new weapon kind `combo`): every 2 beats three brass pistons punch out of the ball 1-2-3 on the 8ths, each aimed
at the sax, steam bursting from the valves; the trumpet inside the ball turns its valves toward the punches; a landed
punch lengthens the reach. SAX (right, teal): **STRAP FLAIL** (new kind `flail`): its bell swings round on the neck
strap, spinning up as momentum builds (damage grows with it), smoke streaming off the bell; on contact the strap wraps
the trumpet, reels it in for two beats and yanks it; a landed hit lengthens the strap. Abilities: FANFARE (shockwave: a
ring of golden banners) / DOUBLE TONGUE (haste: combos twice as fast) / HIGH C (new ability kind `beam`: v2's bell
blast as the finisher, a giant golden beam with heat shimmer); SMOKE SCREEN (shield: the flail swings on out of the
cloud) / BEBOP RUN (haste: flat out for 4 s) / HONK (slam: the bell slams down, smoke-ring crater).
Noir jazz: C minor, i - bVI7 - iv - V7, 133.77 BPM (34 bars), walking bass, brushes then sticks, muted piano comping.

Honest run, seed 191 (6 fits in 604; rules frozen after tune.py): the sax leads from the opening (up 11-13 for most of
the fight), the trumpet's growing reach keeps it close, HIGH C lands at 43.1 s and puts the trumpet ahead, the sax
retakes it at 45.4 s (HONK 43.5) and wins at 49.16 s with 8.6 HP. Balance (tune, 100 seeds): TRUMPET 47 %, SAX 53 %.
v2 (beam vs smoke rings, seed 337) is kept as `*_v2_backup.py`; its roster entries were replaced, so it no longer replays.

## Commands (from this folder)

```bash
python tune.py 100 S.dmg=7.4,7.6 scale=0.6               # rule tuning (done once, frozen in fighters.py + timeline.py)
python seed_search.py 1 4000 --stop 6                      # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 191                                          # one run's story (cached in check/run_191.pkl)
python render.py                                           # sprite cache (check/sprites_ballfight3.npz, ~15 s)
python -m reelkit.sheet render.py test/sheet.png 1.2 2.95 3.25 6.1 10.9 17.6 24.1 29.4 42.8 43.15 44.45 49.2 51.0 60.7
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                      # subject CoC must be 0 px on every frame
python audio.py                                            # audio.wav (2860 AAC frames) + check/stem_*.wav
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | The episode: fighters, tempo, count-in, frozen damage scale, story targets, scripted tail, chords, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine; story analysis + score (same windows as #27) |
| `tune.py` | Rule grid: `T.<weapon key>`, `S.<weapon key>`, `T.u1.<ability key>`, `scale`; balance, K.O., lead changes, punch accuracy, wraps, HIGH C hits |
| `seed_search.py` | #27's early-stopping search (copied; names from the timeline) |
| `styles.py` | Draw functions: pistons (pearl buttons, collars, motion streak), the strap flail (curved strap, brass bell head, swing trail), wrap coils, FANFARE banners, the HIGH C sight + blast (light out of the bell), the smoke screen, the honk bell, solo notes |
| `render.py` | #28's renderer, copied, with this pair's weapons and abilities and `reelkit.fx` (attack lights, bursts, steam / smoke / brass particles, fading marks, heat shimmer, fringe, ability moments, shake); the trumpet turns its valves to the punches, its bell down the HIGH C line |
| `audio.py` | Noir-jazz band; each combo = three notes of one valve fingering (the trumpet's harmonic series), the flail's half turns play an original blues line (faster as it spins up), growl + strap crack per hit |
| `mux.py` | Loop-clean encode via `projects/maze_race/mux.py` |

## Rules worth knowing

- v3 balance: the wrap reels the trumpet in to 2.3 (centre distance), just outside the pistons' starting reach (1.9), so
  the trumpet's growing reach is its way back in; reeling to 1.6 let the trumpet win every run by 27 s. Balance flips
  fast on the flail's damage (7 -> 8 = trumpet 83 % -> 22 %).
- Unlocks pair up in close fights (both fighters level together), so ~1 % of runs fit the hard windows; the
  early-stopping search still takes about a minute.
- HIGH C's visible blast ends at the ropes (`clip_len`): its 7-unit length would leave the lens's sharp band.
- The honk bell stays low (mouth <= 2.9, 1.0 tall): any higher and its throat leaves the sharp band near the front corner.
- History: v1 beam vs smoke rings (seed 337); v2 trumpet model rebuilt from reference photos + the blast out of the
  bell; v3 the redesign above.
