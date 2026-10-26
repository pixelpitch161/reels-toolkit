# #34 BALLFIGHT #8: Trombone vs French Horn (`projects/ballfight_trombone_vs_horn`)

## v5 (user, Oct 25: HORN v3 = ECHO CALL + FINAL FLICK; the HUD ability list)
The HORN's attacks were replaced (IDEAS "HORN v3"; THE HUNT's `pack` kind stays in the engine). New opt-in engine
kinds, each in its own commit, every other finished episode replaying identically: weapon kind **`echo`** (ECHO CALL: a
round blast ring around the ball on every beat; up to 4 echo ghosts trail on the ball's own past path, ghost j where it
was j beats ago, and each repeats the call j beats later from where it was sounded, 0.85x per repeat; +1 ghost per
landed blast; recorded ECHO_G / ECHO_N / ECHO_R) and ability kind **`flick`** (FINAL FLICK: a glove winds up for a beat
tracking the enemy, flicks out up to 2.4 beyond the surface, flings the enemy straight across the ring, +dmg_rope if it
hits the ropes within 0.9 s; recorded FLICK). HORN roster v3: ECHO CALL (dmg 4.3, n0 0 -> 4 ghosts), STOPPED HORN
(shield: a copper hand), HORN RIP (haste x2: the echoes come back on the 8ths), FINAL FLICK (dmg 14 + rope 10). The
TROMBONE is unchanged. Re-tuned once: damage scale 0.60 -> 0.72 (400 seeds: 49 / 51 % decided, median K.O. 51.5 s).
Honest search at 2 bars: 8 fits in 719 seeds, TROMBONE 49 % / HORN 51 % over 591 decided; new seed **140**: the horn's
echoes lead until 38 s (67-54 at 35 s), five lead changes (37.9 / 41.4 / 46.3 / 47.2 / 49.2 s), FINAL FLICK (46.7 s)
flings the trombone into the ropes at 47.2 s (25.7 -> 8.4 HP), PEDAL TONE's bell answers at 48.7 s (22.2 -> 5.0), and
the lance K.O.s the horn: TROMBONE wins at 49.22 s with 6.8 HP. Captions STOPPED HORN 14.8, SAD TROMBONE 20.7, HORN RIP
29.5, POWER SLIDE 35.9, FINAL FLICK 46.7. Look: copper echo ghosts (translucent copies of the horn's ball, newest
brightest, flashing as they repeat) on a fading copper thread along the horn's path; the call a gold-copper ring, each
echo a thinner copper ring (canvas light + a raised ring + heat shimmer); STOPPED HORN an open copper palm pressed into
the bell; FINAL FLICK a giant white cartoon glove on a copper spring (real shadow, its lane lit during the wind-up),
afterimages on the flung trombone. Sound: ECHO CALL is a delay (the call = timeline.HORN_CALL, one note per beat;
every repeat the same note, quieter and darker, panned to its ghost); HORN RIP rips up the harmonic series; FINAL
FLICK = horn fanfare + spring boing + slide whistle, then snap + thud + braam, and a heavy rope hit. HUD ability list
(series rule; `fight_hud(abilities=...)`, #36's helper): LV2 / LV3 / LOW under each bar, grey -> the fighter's colour,
lit on every use. No ring nudge: the balls never come lower than y 403, the taller HUD starts at y 448. Title card,
mood and tempo unchanged. v4 kept as `*_v4_backup.py` and `ballfight_trombone_vs_horn_v4.mp4` (the v4 sources ran on
the THE HUNT roster entry, now replaced).

## v4 (user, Oct 25: the trombone model was inaccurate)
The Pixel Pitch trombone (`series/instrument-history/trombone60_lowpoly.py`, used through pp_bridge) was rebuilt from
reference photos (see that file's notes and the series STATUS): the slide shows both tubes at ~40 deg to the bell plane,
the bell sits ~40 % of the way along a 78 cm slide, nickel crook, no F-attachment loop. Same fight (seed 2200) and
soundtrack; the sprite caches (`check/sprites_ballfight8.npz`, `check/sprites_title.npz`) were rebuilt and the video
re-rendered once. v3 kept as `ballfight_trombone_vs_horn_v3.mp4`.

## v3 (VS title card intro = series rule, Oct 24)
Bar 0 is the shared VS title card (`projects/ballfight/title.py`, moved there from #30 by the #35 session; this episode
added the TROMBONE slide-tube and HORN coil panel patterns in a separate commit, #30's card pixel-identical): TROMBONE,
VS and FRENCH HORN slam in on beats 1-3 over unison band hits (taiko + braam stab + boom + low strings), with the
trombone's slide blast (a low chord gliding up a fifth into D), a thunder crack as VS splits the divider, and the horn's
hunting call (two horns in fifths, D4 -> A4); the doors open on beat 4 over a taiko pickup. Bar 1 = the 3 . 2 . 1
count-in, FIGHT! on the bar-2 downbeat (3.94 s); the top caption and the series-tag line are gone; the HUD and HP
numbers hide while the card is shut. The chords move one bar (bar 0 = Dm under the card, bar 1 = A for the count-in,
the loop from FIGHT!, bar 30 = A resolving on the title slam). The title slam is the frame-0 reset: the doors slide
shut over the last 0.30 s with a riser, the doors' whoosh and a swelling braam into it. The honest search was re-run
with the fight at 2 bars (rules unchanged: v2's seed 894 doesn't even reach a K.O. a bar later): new honest seed
**2200** (8 fits in 2,200; TROMBONE 51 % / HORN 49 % over 1,829 decided): the pack leads 62-40 at 32 s, the lance levels
it by 41 s, TALLY-HO 42.3 and PEDAL TONE 42.8 s, three lead changes in 1.5 s, a 3.7 s standoff at 4.0 vs 1.9 HP,
TROMBONE wins at 50.72 s with 4.0 HP. v2 kept as
`*_v2_backup.py` and `ballfight_trombone_vs_horn_v2.mp4`.

## v2 (BALLFIGHT v2 redesign, user Oct 24): THE HUNT
The HORN no longer shoots: its signature is **THE HUNT** (engine weapon kind `pack`). A horn call every 2 beats brands
the trombone with a copper target mark and releases a ghost hound from the bell while the pack is under its size
(2 -> 5, +0.34 per landed bite). Hounds fan out of the bell, circle to a flank (out of the lance's line) and bite on the
8th grid (horn stab + jaws), heel when the mark fades, circle the horn while it is shielded. STOPPED HORN = shield +
hounds guard; HORN RIP = haste, the pack sprints; TALLY-HO = new ability kind `charge`: the pack, filled to 5, charges
across the ring in one line. The TROMBONE is unchanged; its lance, SAD TROMBONE's shockwave and PEDAL TONE's bell knock
hounds out (embers; the next call releases a new one). Rules re-tuned once (damage scale 0.60, HORN bite 2.8; balance
over 873 decided runs: 50 % / 50 %), new honest seed **894**: the pack leads 83-58 at 26 s, the fully grown lance
levels it by 35 s, TALLY-HO retakes the lead at 42.8 s, PEDAL TONE answers at 43.3 / 44.3 s, TROMBONE wins at 50.72 s
with 4.0 HP (the horn hangs on at 2.2 HP for the last 3.5 s). Hounds are drawn as side-profile 2.5D sprites (long and
low, drop ears, sabre tail, galloping legs, one faint afterimage, a copper aura, no shadow: a copper glow); a top-down
hound silhouette read as a rat at this size. Effects from `reelkit.fx`: brass sparks on lance hits, copper embers on
bites / releases / knock-outs and behind every running hound, attack lights in the fighters' colours, ink-starburst
impacts, bite claw marks + PEDAL TONE's crack and scorch on the canvas, heat shimmer on SAD TROMBONE / the charge front /
the stomp, ability moments (dim + speed lines) on the five captions. v1 kept: `*_v1_backup.py`,
`ballfight_trombone_vs_horn_v1.mp4` (the v1 sources ran on the v1 HORN roster entry, now replaced).

## v1

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
| `styles.py` | v5: echo ghosts, blast rings, the path thread, the STOPPED HORN hand, FINAL FLICK's glove + spring + lane (v2-v4 hounds: `styles_v4_backup.py`). Draw functions for the styles: the `slide` lance (+ thrust streak), `arrow` (+ curving trail), the STOPPED HORN `bellrim`, PEDAL TONE's giant bell (flat-shaded surface of revolution) |
| `render.py` | #28's renderer, copied, with this pair's weapons, abilities, the slowed ball's wobble, the lance's lit line of attack, brass shards and the solo played on the slide (lower note = longer slide) |
| `audio.py` | Brass-less trailer bed; thrusts = the trombone ostinato (glissando into each stab); v5: the horn's calls + their echo repeats (a delay), HORN RIP's harmonic rip, FINAL FLICK's wind-up and THWACK |
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
