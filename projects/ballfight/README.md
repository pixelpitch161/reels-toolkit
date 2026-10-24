# #27 BALLFIGHT #1: Guitar vs Piano (`projects/ballfight`)

New series, inspired by math.vsmath-style ability-ball fights (ours, not a copy). Two glass balls, each holding a
low-poly Pixel Pitch instrument, fight in a boxing ring floating in a dark arena. Each fights with its instrument and
every hit plays it, so the fight is the duet: GUITAR (red corner) swings its own neck like a sword (+1 fret per landed
hit), PIANO (blue corner) fires its keys on the beat (one more key per volley as volleys land). Damage dealt levels each
fighter up (LV2, LV3 unlock two abilities each); below 25 HP each gets one desperation move. Honest simulated fight,
seed 88 picked for pacing, never edited: Guitar leads most of the way, four lead changes, Piano wins at 51.3 s with
2.6 HP left. Mood A: drift phonk (cowbell riff, sliding 808) at 129.84 BPM; the count-in is the band's count.

## Commands (from this folder)

```bash
python tune.py 0.45,0.5,0.55 6,6.5,7 1.0 32   # rule tuning (balance + K.O. time) - done once, rules frozen
python seed_search.py 1 500 --stop 8          # honest runs ranked against the beat sheet -> check/seeds.txt
python sim.py 88                              # one run's story (cached in check/run_88.pkl)
python render.py                              # build the sprite cache (check/sprites_ballfight.npz, ~30 s)
python -m reelkit.sheet render.py test/sheet.png 0.0 1.9 10.3 15.4 27.4 44.7 51.9 56.0
python -m reelkit.safezone render.py --step 0.25
python focus_check.py                         # subject CoC must be 0 px on every frame
python audio.py                               # audio.wav (2860 AAC frames) + check/stem_*.wav
python -m reelkit.loopcheck render.py audio.wav
python -m reelkit.video render.py audio.wav ballfight_tmp.mp4 && python mux.py   # render, then the loop-clean encode
```

## Files

| File | What it is |
|---|---|
| `fighters.py` | **The roster**: every fighter's colour, corner, instrument view, weapon, scaling, 3 abilities and voice in one table |
| `fight.py` | **The reusable engine**: square ring, constant-speed balls, rope walls with seeded give, elastic clashes; weapon kinds `blade` / `volley`, ability kinds `shockwave` / `shield` / `haste` / `arc` / `wave` / `slam`; swept blade hits + parries; records every frame + every event |
| `timeline.py` | The episode: the two fighters, tempo, the countdown, frozen rule scale, story targets, the scripted tail, the music (chords, solos) |
| `sim.py` | Runs and caches the episode's fight; story analysis + score shared by the search, the renderer and the audio |
| `seed_search.py` | Early-stopping search with hard windows (K.O. 49-53 s, >= 2 lead changes, winner HP <= 30, >= 5 ability captions >= 4 s apart) + soft story score; prints the balance |
| `scene.py` | The ring (turned 45 deg: red corner left, blue right), posts + ropes, LED barrier, stands, crowd, the spot |
| `render.py` | Frame renderer (per-pixel canvas with the spot's shadows, rope give, neck, keys, shields, shockwaves, the slam lid, shatter + reassembly, crowd phones / flashbulbs, countdown, HUD, captions) |
| `audio.py` | Drift phonk bed + every hit in the voices of the two instruments (leader fuller, loser thinner and dirtier) + abilities + K.O. tape-stop + victory solo + the build into frame 0 |
| `mux.py` | Loop-clean encode (reuses `projects/maze_race/mux.py`: last-frame keyframe, mbtree off, whole AAC frames) |
| `title.py` | **The VS title card** on bar 0 (series standard): diagonal panels in the fighters' colours + patterns (`PATTERNS`, by fighter), big balls, name A / VS / name B slamming in on beats 1-3, doors opening on beat 4 and shutting into the frame-0 slam at the end; the episode's `timeline` (TITLE, T_DOORS, T_CLOSE) and `render` (NAMES, COL, DARK, CFG) drive it. Built for #30 |

## Making the next episode (~30-40 min)

1. Add the two fighters to `fighters.py` if they're new: a weapon (`blade` or `volley`, or a new kind in `fight.py`),
   `unlock1` / `unlock2` / `low` abilities from the existing kinds, colour, corner, `view` (the instrument's readable yaw /
   swing / roll inside the ball) and `pp` (its Pixel Pitch voice in `which_instrument_wins/pp_bridge.py`).
2. New folder `projects/ballfight_<a>_vs_<b>/` with its own `timeline.py` (`FIGHTERS = ('A', 'B')`, solos, chords),
   importing this folder's engine (append `projects/ballfight` to `sys.path`, as `botb_genre_wars` does with #20).
3. Re-tune once if the new pair is lopsided (`tune.py`: aim for 40-60 % wins and a ~50 s median K.O.), freeze the
   rules, then `seed_search.py --stop 5`.
4. `render.py` draws weapons by `style` (`neck`, `keys`); a new weapon style needs one draw function.
5. The HUD is `reelkit.ui.fight_hud` (two fighting-game HP bars, LV pips, status line), already safe-zone placed.

## Rules worth knowing

- Balance under the frozen rules (313 decided runs of the first search): GUITAR 44 %, PIANO 56 %.
- The piano's volley grows by half a key per landed volley (one more key every second volley that lands): one key per
  landed volley maxed it out by ~25 s and the piano won every run by ~14 s.
- Captions are ability names only, the first use of each, never two within 4 s (here FEEDBACK lands 0.9 s after GRAND
  SLAM, so FEEDBACK is named in the HUD's status line instead). Every use, repeats included, shows in the status line.
- After the K.O.: a 0.35 s hit-stop, the loser shatters in slow motion, the winner takes the centre under the spot and
  plays its solo (bars 29-31); at 58.0 s the loser's pieces fly home, both glide to their corners at 59.15 s (HP refills,
  weapons reset) and rev; the "3" flies in during the last beat and lands on frame 0.

## Effects (reelkit.fx, BALLFIGHT v2) - wiring them into an episode renderer

Worked example: `projects/ballfight_organ_vs_harp/render.py` (search `reelkit.fx`). Build every effect once at import,
from the run's events in time order (so frames render in any order and patch renders match), then:

| Stage of the #27 / #28 renderer | Add |
|---|---|
| module level | `FL = fx.Flares(max_n=6)`, `BU = fx.Bursts()`, `PA = fx.Particles(seed)`, `MK = fx.Marks()`, `MO = fx.Moments()`, `SHK = fx.Shake(cap=2.5)`; loop over `EV` adding hits (`BU.add`, `FL.add`, `PA.emit`, `MK.add`), heavy hits to `SHK`, captioned abilities (`sim.story(RUN)['caps']`) to `MO` |
| `camera(t)` | `dx, dy = SHK.offset(t)` instead of the old `shake(t)` |
| `lights(t, cam)` | `FL.apply(L, t)`: the canvas / floor per-pixel passes and every `shade()` (ropes, posts, stands) pick the attack lights up |
| `arena()` crowd | multiply each person's colour by `1 + 0.9 * FL.tint(heads, t)[j]` |
| canvas pass | `alb = MK.apply(alb, sxp, szp, t, BAYER_FULL[ys, xs])` right after `S.canvas_albedo` (marks in the ring frame) |
| after `sparks()` | `arr = PA.draw(arr, cam, t, light=lambda P: 1 + 0.6 * FL.tint(P, t))` |
| after `dof()` | `fx.shimmer_world(img, cam, centre_world, radius, width, amp_px, t, k)` on shockwave / wave / blast fronts |
| after grade + vignette (float, before the uint8 cast and the UI) | `fx.fringe` on big hits; `img = MO.apply_dim(img, t, keep=[(x, y, rp * 1.7) for both balls])`; `MO.speed_lines(img, t, k, x, y, rp, screen_velocity)`; `img = BU.draw(img, cam, t)` |

Levels that worked for #35 (pearl / wine): row / string lights peak 0.4-0.6, landed hits 0.6-0.9, slams and waves
1.1-1.2 (a pearl or other near-white fighter colour whitens the canvas fast: scale its colour by ~0.9 and keep peaks
low); halve the old per-hit `fr.glow` once the bursts are in. Before the full render, run the flash test on the busiest
stretch (render ~5 s of frames and `reelkit.flashcheck.analyse` the luminance): #35 peaked at 18.6 % of the frame.

