# #39 BALLFIGHT #13: Bassoon vs Bari Sax (`projects/ballfight_bassoon_vs_barisax`)

The low reeds: the orchestra's clown vs the band's chaos agent. Two new fighters, each with a style no one else has:

- **BASSOON** (left, forest green): the only sneak, where position matters. **TIPTOE**: it doesn't glide, it tiptoes:
  exaggerated staccato hops on the beat (stop-and-go, squash and stretch), each to a spot **behind** the enemy (against
  its direction of travel), each landing a small stomp that hits x2.5 from behind (the backstab).
- **BARI SAX** (right, coral), **v2** (Oct 26, after the user found v1 "really boring"): so heavy it wears a harness.
  **WRECKING BALL**: every other bar a crane cable drops from the spot's rig, hooks the harness, hoists it back and
  swings it across the ring as a wrecking ball in a pendulum arc, smashing what's in front of it (harder the faster it
  is going), then unhooks and rolls on. Each landed smash lengthens the cable (a wider swing that hits harder).
  v1's **WINDOW RATTLER** (the honk that rang the ropes) is frozen in `v1/` (sources, seed 190, `_v1.mp4`).
  **v3** (Oct 27, the user: "Changes have been made to the Bari Sax's model and attack. Re-render this video"): the
  detailed Pixel Pitch bari (#40's detail pass) and the roster's swing out over the ropes (#40 v2: the swing's ends may
  reach past the ropes, never past a corner post; a far end outside drops onto the ropes and is slung back in). v2 is
  frozen in `v2/` (seed 27, `_v2.mp4`).

Both are game rules in the shared engine (`projects/ballfight/fight.py`), added in their own small commits (every
finished episode replays identically, `tools/ballfight_replay_hash.py`):

| Kind | What it does |
|---|---|
| weapon `tiptoe` | a hop every `every` beats (haste divides it), `air16` 16ths in the air, to `gap` behind the enemy where it will be (`lead`), +-`jitter` deg, at most `base` away (+`grow` per landed stomp); airborne = no clash; each landing a stomp within `stomp` (+`stomp_grow`) beyond contact: `dmg`, x `back` within `back_half` deg of straight behind; a hopper only slides when knocked and can't hop while its knock boost is above `stagger` |
| ability `mountain` | MOUNTAIN KING: hops launched `seq16` 16ths apart (accelerating), circling the enemy `step` deg per hop, each a stomp |
| ability `hook` | BOCAL HOOK: the crook winds up at a close enemy (`ready`), hooks it within `reach` / `half` deg: `dmg`, spins it to face away, and the hopper follows with a hop straight behind it |
| weapon `rattle` | WINDOW RATTLER: a honk ring every bar (`push`, `dmg`, `knock`) + the ropes ring (1 side, one more per `side_step` of strength, decaying over `ring` s); a bounce off a ringing rope = a sling (`sling` x amplitude x strength, `sling_knock`) |
| ability `offbeat` | SKANK: an extra honk on every off-beat 8th for `dur_beats` |
| ability `quake` | LOW A: a ring-wide wave + all four ropes held at full ring for `dur_beats` |
| weapon `wrecking` (v2) | WRECKING BALL: every `every` beats a cable drops over `drop16` 16ths; when it bites, the swing is aimed (the bottom `short` x 2R short of where the enemy will stand: where a hop lands, else `lead` x its motion; +-`jitter` deg); the pivot hangs over that point, the cable `base` long (+`grow` per landed smash, up to `max_reach`); hoisted back `theta` deg (less near the ropes) over `lift16`, it swings through in `swing16` 16ths (a pendulum's half period); airborne all the while; the first enemy in front within `hit_r` beyond contact (3D; one mid-hop sails over it, except in its first / last `air_edge`) takes `dmg` x (`dmg_min` + the rest x speed) x (cable / `base`) ^ `len_pow`, knocked along the swing; under SKANK (`offbeat`) a cycle starts every bar on the '&' (`skank_off16`, aimed `skank_short`) so the swings bottom out on the off-beats |
| ability `snap` (v2) | CABLE SNAP: hoisted to `height` and carried over the enemy's predicted spot (fixed when it fires) for `hoist` of `tele16` 16ths; the cable snaps, it falls and craters the spot: `dmg` + `knock` within `radius` beyond contact |

Abilities: BASSOON LV2 STACCATO (haste: hops on the 8ths) / LV3 BOCAL HOOK / LOW MOUNTAIN KING; BARI SAX v2 LV2 GROWL
(shockwave, gravelly; unchanged) / LV3 SKANK (a swing every bar on the off-beats, for 2 bars) / LOW CABLE SNAP (v1:
LOW LOW A).

## The two new instruments (Pixel Pitch folder)

- `series/instrument-history/bassoon_lowpoly.py` (own commits: model, then voice), bridge entry `BASSOON`: the modern
  Heckel bassoon from reference photos found in Claude in Chrome (Commons: the Yamaha YFG-812 II front product shot,
  measured and scaled to 134 cm; a parts layout for the bocal): oval boot with its silver cap and band, long joint,
  bell joint with the bulge and the pale cap ring, wing joint with the silver crown, the bocal arcing out to a
  red-threaded reed, the black hand rest, simplified keywork. Scene `ball` = horizontal distances x2.3 and the crook
  x3.4 thicker so it reads in the ball sprites. Voice: double reed with formants ~480 Hz / ~1150 Hz, a weak low
  fundamental, a tongued "tock", vibrato only on held notes (staccato = the sneaky dot).
- `series/instrument-history/barisax_lowpoly.py` (new file, own commits), bridge entry `BARI SAX`: the modern low-A
  bari adapted from the alto (`altosax60_lowpoly` used read-only: swept tubes, cups, touches, palette), checked against
  the Amati baritone product shot on Commons: ~0.82 m body, the long straight bell (rim at ~80 % of the height), the
  pigtail loop, the big mouthpiece, the low-A cup + rod. Voice: the alto's recipe with frequency-based rolloff, formants
  ~560 Hz / ~1.35 kHz, `honk` adds growl, scoop and the reed slap.

## Tuning and the run (v3)

The roster's BARI SAX is now shared with #40, so its numbers stay as they are; with the swing over the ropes it won
83 % of 400 seeds here. Re-tuned once with the episode's own knobs (`tune.py` keys `hb` / `sc`): the bari's hits x0.8
(`timeline.DMG_SIDE`) at damage scale 0.34: 400 seeds BASSOON 53 % / BARI SAX 47 %, median K.O. 50.8 s. Honest search
at 2 bars: 8 fits in 262 runs (53 / 47); seed **95** (the top score): five early smashes put the bari 97-82 up at
10 s, GROWL 9.2, SKANK 17.7, STACCATO 22.3, BOCAL HOOK 34.8 bring the bassoon back, CABLE SNAP at 42.2 s craters it
at 43.3 s, MOUNTAIN KING answers at 43.3 s; lead changes 42.7, 47.7 and 50.6 s; BASSOON wins at 50.80 s with 5.7 HP.
20 swings (8 of them SKANK's), 13 smashes, 7 times out over the ropes and slung back.

## Tuning and the run (v2, frozen in `v2/`)

The BASSOON is unchanged (its roster entry as v1). The new BARI SAX was tuned once: a random search over its numbers
and the shared damage scale (`tune_search.py`, 120 settings x 60 seeds), refined with `tune.py`, frozen in
`fighters.py` + `timeline.DMG_SCALE = 0.32` (GROWL kept as v1): 400 seeds BASSOON 53 % / BARI SAX 47 %, median K.O.
49.3 s; 61 % of swings smash, CABLE SNAP lands 96 %. Honest search with the fight at 2 bars and the usual hard windows
(`seed_search.py 1 1000 --stop 8`): 8 fits in 166 runs (49 / 51); seed **27** (the top score): the wrecking ball
smashes the bassoon at 3.6, 6.4, 9.0, 11.9 and 14.8 s and the bari leads 98-82 at 10 s; GROWL 9.2, SKANK 17.7 (its
first off-beat swings), STACCATO 22.0 and BOCAL HOOK 34.8 bring the bassoon back; CABLE SNAP at 41.9 s hoists the bari
over the bassoon's spot and craters it at 42.9 s, MOUNTAIN KING answers at 42.9 s; lead changes 41.4, 48.9 and 51.0 s;
BASSOON wins at 50.99 s with 5.3 HP. Captions GROWL 9.2, SKANK 17.7, STACCATO 22.0, BOCAL HOOK 34.8, CABLE SNAP 41.9
(MOUNTAIN KING lights in the HUD list: 4 s rule). 20 swings (8 of them SKANK's), 12 smashes.

v1 (WINDOW RATTLER, seed 190, damage scale 0.45) is kept in `v1/` (its own sources; `v1/sim.py` patches the frozen v1
BARI SAX into the roster) and guarded by the replay hash.

## Music, mood A: dark ska-punk

E minor at 169.18 BPM (43 bars; FIGHT! at 2.84 s). Punk drums, an upstroke guitar skank on every off-beat, an organ
bubble; no horns in the bed. The BASSOON plays a staccato dot per hop landing (timeline.BASSOON_8THS, up high), a
backstab gets a grace; MOUNTAIN KING plays Grieg's *In the Hall of the Mountain King* theme (1875, public domain) in E
minor, one note per hop, accelerating with them. v2: the BARI SAX's bass line rides the swings: each swing one huge
honk on the bar's bass root, swelling to the bottom and bent like a doppler pass (sharp coming in, flat going away),
over a rush of air; SKANK's swings honk up the octave on the off-beats; the winch whirrs as the cable drops and the hook
clanks home; GROWL growls; CABLE SNAP = the winch straining up, the twang, a falling whistle, the crash; v3: a rope
the bari pressed springs back with a low twang, the slung-back return a whoosh and a thud. One master
gain + compressor + soft limiter: `reelkit.levels` PASS.

## Commands (from this folder)

```bash
python tune.py 200                                         # the frozen rules' balance (done once)
python seed_search.py 1 2000 --stop 8                      # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 95                                           # the run's story
python render.py                                           # sprite cache (check/sprites_ballfight13.npz, ~1 min)
python -m reelkit.sheet render.py test/sheet_v3.png 0.0 1.0 6.35 6.75 9.6 12.45 17.75 19.32 26.6 29.6 34.8 42.4 42.9 43.3 50.85 54.0
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                                      # subject CoC must be 0 px on every frame
python audio.py && python band_check.py && python -m reelkit.levels audio.wav
python flash_test.py 41.5 52.0
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## Files

| File | What it is |
|---|---|
| `timeline.py` | Fighters, title, tempo, count-in, frozen damage scale, story targets, scripted tail, chords, the lines, Grieg's theme, both solos |
| `sim.py` | Runs + caches the honest fight on the shared engine; story analysis + score |
| `tune.py`, `tune_search.py` | The tuning grid and the random search over it |
| `seed_search.py` | Early-stopping search (hard windows incl. no 4 s without a hit) |
| `styles.py` | New styles: hop arcs, squash and stretch, stomp rings, backstab slashes, the crook, swirl, honk rings, rope standing waves, jumping tiles, the dance trail; v2: the crane line, hook block and hook, the harness, the swing trail, CABLE SNAP's target |
| `render.py` | #37's ring / HUD / title pipeline with the two new fighters |
| `audio.py` | The ska-punk band + both fighters from the run (v2: the swing honks) |
| `v1/` | v1 as delivered (WINDOW RATTLER, seed 190): re-renderable, replay-guarded |
| `v2/` | v2 as delivered (WRECKING BALL in the ring, seed 27): re-renderable, replay-guarded |
| `band_check.py`, `focus_check.py`, `flash_test.py`, `mux.py` | Checks and the loop-clean encode |
