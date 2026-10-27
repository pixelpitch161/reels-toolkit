# #39 BALLFIGHT #13: Bassoon vs Bari Sax (`projects/ballfight_bassoon_vs_barisax`)

The low reeds: the orchestra's clown vs the band's chaos agent. Two new fighters, each with a style no one else has:

- **BASSOON** (left, forest green): the only sneak, where position matters. **TIPTOE**: it doesn't glide, it tiptoes:
  exaggerated staccato hops on the beat (stop-and-go, squash and stretch), each to a spot **behind** the enemy (against
  its direction of travel), each landing a small stomp that hits x2.5 from behind (the backstab).
- **BARI SAX** (right, coral): the only fighter that weaponises the ring. **WINDOW RATTLER**: every bar a huge low
  honk shoves anything close away and makes the **ropes ring** like plucked strings (the side nearest the enemy first,
  spreading to all four as it grows); anything bouncing off a ringing rope is slung back and hurt.

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

Abilities: BASSOON LV2 STACCATO (haste: hops on the 8ths) / LV3 BOCAL HOOK / LOW MOUNTAIN KING; BARI SAX LV2 GROWL
(shockwave, gravelly) / LV3 SKANK / LOW LOW A.

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

## Tuning and the run

Tuned once (`tune.py`, a random search in `tune_search.py` for the shape), frozen in `fighters.py` +
`timeline.DMG_SCALE = 0.45`. The first freeze had MOUNTAIN KING at 1.6 damage a stomp; the hop-timing fix (a landing
due within a substep of its 16th tick was missed, so the dance lost its accelerating schedule) made it decisive, so it
was re-tuned once to 0.4: 200 seeds BASSOON 53 % / BARI SAX 47 %, median K.O. 50.0 s. Honest search with the fight at
2 bars and the usual hard windows (`seed_search.py 1 4000 --stop 8`): 8 fits in 275 runs (56 / 44); seed **190**: the
bari's honks sling the bassoon round the ropes and the bari leads 65-48 at 31 s; BOCAL HOOK at 33.3 s starts the comeback,
LOW A at 42.2 s rings all four ropes and MOUNTAIN KING answers 0.4 s later (Grieg, accelerating), three lead changes
(42.1, 46.3, 50.8 s), and a last hook at 50.2 s: BASSOON wins at 51.16 s with 4.8 HP. Captions GROWL 17.4, STACCATO
22.0, SKANK 27.3, BOCAL HOOK 33.3, LOW A 42.2 (MOUNTAIN KING lights in the HUD list: 4 s rule).

## Music, mood A: dark ska-punk

E minor at 169.18 BPM (43 bars; FIGHT! at 2.84 s). Punk drums, an upstroke guitar skank on every off-beat, an organ
bubble; no horns in the bed. The BASSOON plays a staccato dot per hop landing (timeline.BASSOON_8THS, up high), a
backstab gets a grace; MOUNTAIN KING plays Grieg's *In the Hall of the Mountain King* theme (1875, public domain) in E
minor, one note per hop, accelerating with them. The BARI SAX honks the bass root on every bar, SKANK on the off-beats;
the ropes it rattles are the band's bass strings (one plucked chord tone per ringing side, so the bass fills out as the
rattle spreads); LOW A holds concert C2. One master gain + compressor + soft limiter: `reelkit.levels` PASS.

## Commands (from this folder)

```bash
python tune.py 200                                         # the frozen rules' balance (done once)
python seed_search.py 1 4000 --stop 8                      # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 190                                          # the run's story
python render.py                                           # sprite cache (check/sprites_ballfight13.npz, ~1 min)
python -m reelkit.sheet render.py test/sheet.png 0.0 0.75 5.15 17.5 27.5 33.55 42.3 43.4 50.25 51.16 54.0
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
| `styles.py` | New styles: hop arcs, squash and stretch, stomp rings, backstab slashes, the crook, swirl, honk rings, rope standing waves, jumping tiles, the dance trail |
| `render.py` | #37's ring / HUD / title pipeline with the two new fighters |
| `audio.py` | The ska-punk band + both fighters from the run |
| `band_check.py`, `focus_check.py`, `flash_test.py`, `mux.py` | Checks and the loop-clean encode |
