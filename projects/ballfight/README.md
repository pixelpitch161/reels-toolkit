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
| `kit/`, `__main__.py` | **The episode kit** (from Oct 27): `python -m ballfight <step> <episode>` runs any episode from its one config file; the fighters' looks and sounds (`kit/fighters/`), the moods (`kit/moods/`). See "Making an episode" |
| `examples/` | Kit configs that rebuild #41 (crossover) and #37 (new fighters) exactly: the worked examples |

## Making an episode (the episode kit, from Oct 27)

An episode is now **one config file**, `projects/ballfight_<a>_vs_<b>/episode.py`: fighters and sides, SEED, the
handicap, opt-in rules, tempo, mood, key and chords, the fighters' lines and solo motifs, the title-card hits, the post
caption. Everything else is shared and lives in `kit/`: the sim, tuning, seed search, renderer, title card, HUD, audio
framework, checks and the loop-clean mux. Each fighter's look (draw style, effects, shatter parts, VS-card panel) and
sound (voice, idiom) live with that fighter in `kit/fighters/<name>.py`; moods are reusable beds in `kit/moods/`. So a
crossover of ported fighters needs only its config, and its music is part of the config.

Worked examples (both rebuild a delivered episode exactly; their own folders stay as delivered):
`examples/flute_vs_trombone/episode.py` (#41, a crossover: 61 lines vs 2,799 lines of Python in #41's folder; same
fight, same seed, pixel-identical frames, bit-identical audio.wav) and `examples/clarinet_vs_oboe/episode.py` (#37, two
new fighters: 39 lines; same fight, bit-identical audio, 22 of 25 test frames pixel-identical).

**One command per step**, from `projects/` (or `python projects/ballfight <step> ...` from anywhere):

```bash
python -m ballfight new harp_vs_tuba HARP TUBA --mood big_top --bpm 120   # -> projects/ballfight_harp_vs_tuba/episode.py
python -m ballfight preview HARP [--vs TUBA]          # a new fighter: 7 s low-res clip (signature + desperation) -> send it
python -m ballfight tune harp_vs_tuba 300 HARP=1.0,1.2 TUBA=0.8    # balance grid (or scale=..., HARP.weapon.dmg=...)
python -m ballfight search harp_vs_tuba 1 6000 --stop 8            # ranked fits -> check/seeds.txt; set SEED
python -m ballfight sheet harp_vs_tuba [t ...]        # test/sheet.png (default: the run's key moments) -> send early
python -m ballfight audio harp_vs_tuba                # audio.wav + check/stem_*.wav
python -m ballfight check harp_vs_tuba                # safe zone, focus, levels, band, loudness, flash, loop
python -m ballfight render harp_vs_tuba [--frames A:B | --only-mux]   # the ONE full render + loop-clean encode
python -m ballfight verify harp_vs_tuba               # ffprobe, loop + flash on the mp4, test/mp4_sheet.png
python -m ballfight post harp_vs_tuba                 # caption + tags      (also: sim, hash)
```

Measured on #41 (2 cores): tune 300 seeds 52 s, search (968 runs to 8 fits) 2 min 45 s, audio 9 s, full render 7 min
11 s + loop-clean encode 3 min 47 s (one encode: reelkit.video's own is skipped); check (all pre-render checks) 4.5 min,
verify 46 s, a preview clip 75 s. Tune + search + render: about 15 min. Every step pins `PYTHONHASHSEED=0`: `reelkit.ui`'s
caption puff uses Python's salted string hash, so without it the same frame differed run to run (even with #41's own
renderer) and a `--frames` patch would not match the full render.

**What goes where**

| Where | What |
|---|---|
| `<episode>/episode.py` | FIGHTERS (first named = left), SEED, DMG_SCALE / DMG_SIDE (a crossover's only lever), RULES, BPM, MOOD, KEY / SCALE, CHORDS (one per bar; LOOP = the cycle from FIGHT!), ROLES, LINES (a fighter's line over the LOOP: 8 notes a bar = 8ths, 4 = beats), SOLO, TITLE_HITS, CAPTION / TAGS, MIX overrides. A fighter without a line or solo gets a sketch from the chords |
| `fighters.py` | How a fighter fights (the engine's numbers): unchanged, as before |
| `kit/fighters/<name>.py` | `Look` (its layers at fixed points of the pipeline: state, shadows, canvas decals and rings, painter items, effects over the ring, shards, shimmer, glows, crisp overlays, VS-card panel) and `Sound` (voice, title blast, count-in, its part from the run, fall apart, stinger, solo, reassembly, its role in the last bar). The hooks are listed in pipeline order in `kit/fighters/base.py` |
| `kit/moods/<mood>.py` | A bed: tempo and chords in, its sections (title hits, VS, count-in, fight bars + tape stop, K.O., stinger, victory, the build into the seam), its buses, reverb and compressor. `big_top` (#41), `baroque_storm` (#37) |
| `kit/picture.py`, `kit/music.py` | The renderer and the audio framework (the series rules: card, count-in, beat sheet, tail, HUD list, captions, one master) |
| `fight.py` | The engine: new weapon / ability kinds stay additive, own commit, replay guard |

**A new fighter**: its roster entry in `fighters.py` (own commit, replay guard), then its module in `kit/fighters/`
(start from the closest existing one), then `preview` and send the clip to the user **before** tuning and rendering.

**Ported so far: FLUTE, TROMBONE (as #41 drew and played them), CLARINET, OBOE (#37), ACOUSTIC (#33 v3; check: `examples/ukulele_vs_acoustic`), ELECTRIC (new in #44), ORGAN (#35 v4; checked against #35: replay hash identical, frames pixel-identical in a matched harness, organ buses bit-identical; role 'bass' = a pedal under every pipe row), PIANO v2 (PIANO ROLL, new in #45; #27's v1 is not ported), TRUMPET (#29 v3, ported for #46; checked against #29 with `examples/trumpet_vs_sax`: replay hash identical, 12 frames pixel-identical in a matched harness, trumpet bus to 1e-16; a config line `LINES['TRUMPET']` makes its punches play a melody), CELLO and CONTRABASS (new in #47: THE EMBRACE grapple, WALKING BASS juggernaut; the contrabass's Pixel Pitch model is new, from Commons reference photos), EUPHONIUM (#38, ported for #48; checked against #38 with `examples/bassclarinet_vs_euphonium`: replay hash identical, 10 frames pixel-identical in a matched harness, euphonium bus bit-identical; IDENTITY CRISIS's copies are drawn by the enemy's own look module in amber).** Moods: `second_line` (#48: a dark minor New Orleans street groove, parade bass drum with a cymbal on top / snare ghost notes and press rolls / a low sousaphone / tambourine and claps later, no horns; a `press_roll` for a fighter's fanfare), `dark_tango` (#47: bandoneon marcato, a muted nylon guitar for the strings' chop, golpe + chicharra, no piano, no bass of its own; tango stops on the beats fighters ask for via `Sound.stops()`), `hot_jazz` (#46: a dark minor two-beat, tuba / guitar chop / press rolls / brushes, no horns), `prog_duel` (#45), `big_top` (#41), `baroque_storm`
(#37), `stadium_rock` (#44: plugged / unplugged, follows the leader; config `PLUGGED`). Any other fighter draws as a
bare ball until it is ported. To port one: copy its draw functions from its latest episode's `styles.py` and its layers
from that episode's `render.py` into the Look hooks, its part of `audio.py` into Sound (the bed into `kit/moods/` if
it's new), then make a kit config of that episode under `examples/` and compare: `python -m ballfight hash` against
`tools/ballfight_replay_hash.py`, a few frames and `audio.wav` against the episode's own (`PYTHONHASHSEED=0` on both).
Port a fighter when a crossover first needs it.

Kit behaviour worth knowing: no "BALLFIGHT #N" tag unless the config sets SERIES (old episodes only); #41's bowed glide
home applies whenever the two straight glides would touch (zero otherwise); the HUD flash ignores a fighter's
`quiet_hits` (the oboe's stings); `check/` holds the caches (`run_<seed>.pkl`, `sprites_ring.npz`, `sprites_title.npz`).

Before the kit (#27-#42) each episode had its own copies of these scripts; those folders stay as delivered.

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

