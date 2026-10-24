# #30 BALLFIGHT #4: Bagpipes vs Accordion (`projects/ballfight_bagpipes_vs_accordion`)

**v3 intro test (user, Oct 24; this episode only):** bar 0 is a VS title card (`title.py`): the frame splits diagonally
into the fighters' colours (tartan green top-left, magenta bottom-right, panels filling the screen) with each ball big
and close and a small BALLFIGHT #4 tag; BAGPIPES (pixel scale 6), VS (scale 12, white-gold) and ACCORDION (scale 5) slam
in on beats 1-3 at the biggest sizes that fit the safe zone (ACCORDION left of the button column), each with an
overshoot, motion streaks, a shake, a dust burst and a local light; VS cracks the divider (sparks) and the doors split
along the crack on beat 4, sliding apart to reveal the ring. Sound: a unison band hit per word (stomp, boom, choir
"hey") with a drone blast / a thunder crack / an accordion chord blast on top, a pickup fill into the doors. Bar 1 = the
usual 3 . 2 . 1, FIGHT! at 3.94 s; the old top caption is gone; the HUD is skipped while the card is shut.
The fight starts one bar later (`T_FIGHT = 2 * BAR`). Seed 1442 did not replay identically one bar later (float drift
from ~31 s: K.O. 51.41 instead of ~52.4), so the honest search was re-run with the fight at 2 bars (rules unchanged):
it ranked 1442's new run first (6 fits in 1444; BAGPIPES 57 % / ACCORDION 43 % over 1212 decided). Same story shape
as v2: the bagpipes lead by 20, FULL BELLOWS lands at 45.8 s, BAG BURST answers, 5-5 at 50 s, ACCORDION wins at 51.41 s
with 0.24 HP. Loop: the title slam is the frame-0 reset; the doors slide shut over the last 0.30 s with a riser + a
whoosh, and BAGPIPES falls in as a ghost that lands exactly on frame 0's word (seam 0.9x; audio step 3.9 dB).
v2 render: `ballfight_bagpipes_vs_accordion_v2.mp4` (fight at 1 bar, seed 1442's v2 run).

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
python -m reelkit.sheet render.py test/sheet.png 0.0 0.5 1.0 1.5 2.0 3.967 9.5 17.6 22.3 29.0 45.6 51.5
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                        # subject CoC must be 0 px on every frame
python audio.py                                              # audio.wav (2860 AAC frames) + check/stem_*.wav
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # -> ballfight_bagpipes_vs_accordion_v3.mp4
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | The episode: fighters, tempo, count-in, frozen damage scale, story targets, scripted tail, chords, the chanter's shanty line, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine (records RAD: the bagpipes' radius per frame); story analysis + score |
| `tune.py` | Rule grid: `P.<weapon key>`, `A.<weapon key>`, `P.u1.<ability key>`, `scale`; balance, K.O., lead changes, fit rate by window, damage by hit kind, flights + landing rate, bag size |
| `seed_search.py` | #27's early-stopping search |
| `title.py` | v3 intro test: the VS title card (panels, big balls, word slams, crack, doors, the closing run-up into frame 0) |
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
