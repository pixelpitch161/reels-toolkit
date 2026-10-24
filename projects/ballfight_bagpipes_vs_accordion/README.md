# #30 BALLFIGHT #4: Bagpipes vs Accordion (`projects/ballfight_bagpipes_vs_accordion`)

The "most hated instrument" final on #27's engine, ring, HUD and count-in (`projects/ballfight`: `fight.py`,
`fighters.py`, `scene.py`, imported, plus `reelkit.ui.fight_hud` and the shared effects kit `reelkit.fx`).

**v2 (BALLFIGHT v2 redesign, user Oct 24): the bag and the bellows are the weapons.**
- BAGPIPES (left, tartan green): **BAG INFLATE** (weapon kind `inflate`). Every bar the bag swells over 2 beats to
  ~1.6x size and mass (per-ball radius / mass / speed in `move()`; heavy, slow, body slams), then lets go and rockets
  around like a released balloon for up to 1.75 beats at 2.6x speed, zig-zagging (seeded turns every 0.10-0.17 s,
  alternating sides, a little homing) as it deflates. Landed hits grow the inflation (1.4 -> 1.65x) and the flight.
  WAR CRY (`shockwave`: a tartan ring), HIGHLAND FLING (`haste` x2: two flights per bar), BAG BURST (`wave`: the bag
  pops in a tartan-shred explosion).
- ACCORDION (right, magenta): **BELLOWS BITE** (weapon kind `bite`). Every 2 beats the end boards fly apart with the
  pleats fanned between them like jaws, aimed at the enemy's spot an 8th ahead, and snap shut 2 sixteenths later: a hit
  if the enemy is within R + reach and 38 deg of the aim. Landed bites grow the reach (0.9 -> 2.0). POLKA (`haste` x2:
  bites every beat), SQUEEZEBOX (`shield`: folds shut, buttons glowing), FULL BELLOWS (ability kind `clamp`: a low
  bellows wall stretches across the ring for a beat, then both boards slam together on the enemy: a ring-wide bite).
- Effects (`reelkit.fx`): attack lights in the fighters' colours on the canvas, ropes and crowd; ink-starburst impacts
  (budgeted); air puffs on every snap and along every flight, tartan shreds on flight hits / BAG BURST / a bagpipes
  K.O.; claw marks where bites land, cracks under flights / body slams / the clamp; shimmer on the waves and the clamp;
  fringing on big hits; ability moments (dim + speed lines) on the five captions. No full-screen flashes.
- Sound: the drone is the bag's air (low at rest, swelling as it inflates, draining in the flight while the chanter
  squeals up an octave); bites are oom (opening, bass button) + pah (the snap's chord stab).

Honest run, seed 1442 (6 fits in 1466; rules frozen after tune.py): the bagpipes build a 20-HP lead by 24 s, FULL
BELLOWS lands at 43.8 s and BAG BURST answers a moment later, the lead flips at 43.8 / 44.5 / 48.0 s, and ACCORDION wins
at 50.43 s with 1.8 HP. Balance over 1358 decided runs: BAGPIPES 52 %, ACCORDION 48 %.

v1 (drone aura vs bellows rings, seed 1581, `ballfight_bagpipes_vs_accordion_v1.mp4`): `*_v1_backup.py` (kept for
reference; the roster has moved to v2, so they no longer reproduce v1; the kinds `aura` / `bellows` are still in the
engine).

## Commands (from this folder)

```bash
python tune.py 80 P.fly_dmg=6.4,6.8 scale=0.72               # rule tuning (done once, frozen in fighters.py + timeline.py)
python seed_search.py 1 10000 --stop 6                       # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 1442                                           # one run's story (cached in check/run_1442.pkl)
python render.py                                             # sprite cache (check/sprites_ballfight4.npz, ~17 s)
python -m reelkit.sheet render.py test/sheet.png 0.0 2.6 3.4 7.1 9.5 15.6 20.3 27.0 43.6 43.95 50.6 53.5
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                        # subject CoC must be 0 px on every frame
python audio.py                                              # audio.wav (2860 AAC frames) + check/stem_*.wav
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # -> ballfight_bagpipes_vs_accordion_v2.mp4
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | The episode: fighters, tempo, count-in, frozen damage scale, story targets, scripted tail, chords, the chanter's shanty line, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine (records RAD: the bagpipes' radius per frame); story analysis + score |
| `tune.py` | Rule grid: `P.<weapon key>`, `A.<weapon key>`, `P.u1.<ability key>`, `scale`; balance, K.O., lead changes, fit rate by window, damage by hit kind, flights + landing rate, bag size |
| `seed_search.py` | #27's early-stopping search |
| `styles.py` | v2 draw functions: the tartan bag (`tartan_rgb`), the flight ribbon, the jaws, the FULL BELLOWS wall, the SQUEEZEBOX collar, WAR CRY's tartan ring |
| `render.py` | #28's renderer, copied, with the v2 attacks and `reelkit.fx` wired in (see projects/ballfight/README.md, "Effects") |
| `audio.py` | Dark sea shanty bed; the bag's drone + the flight squeal, bites as oom-pah chords |
| `mux.py` | Loop-clean encode via `projects/maze_race/mux.py` |

## Rules worth knowing

- Size changes go through guarded attributes (`rad`, `mass`, `vmul`) read by `move()` with getattr defaults, so no other
  fighter's physics changes (R + R == 2 * R exactly; equal masses keep the original clash branch).
- Clash events fire every substep while an inflating ball keeps pushing into the enemy; contact damage has its own
  cooldown (`cool`), and the soundtrack scores contact from the hits, not the clashes.
- The bite's snap resolves on its 16th with a one-substep tolerance (the fire time is up to a substep late).
- The first tune had the accordion winning every run (bites ~60 % on target); the bag needed ~7 damage per landed
  flight. The inflated size reached its cap by mid-fight at +0.02 per hit; +0.015 up to 1.65x keeps it near the
  brief's ~1.6x.
- The seam: the flying "3" now reaches its landing size on the last frame, so frame 0 repeats it (seam 2.97 -> 2.49,
  the calmer v2 fight lowered the median step the check compares against).
