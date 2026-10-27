# Video log

## 100 Metronomes (`projects/100_metronomes`)
- Coupled-metronome sync (Kuramoto mean-field), a board on rollers, a holdout that finally locks in.
- Ticks drive the beat. The user's brief: not exactly Bit Wonder, subtle low-poly, engaging throughout.
- Fix: draw HUD text after shapes (it was being covered).

## Sand That Draws Sound (`projects/sand_draws_sound`)
- Chladni plate in a Moroccan desert ruin, from night to sunrise. Resonance sweep (12 patterns),
  slow-motion explanation, and Grieg's *Morning Mood* played as plate patterns.
- v2 (`render2.py`):
  - per-pixel floor lighting with moon, sun and fire shadows, AABB occlusion, sand shadowed per grain;
  - zellige floor, bunting, bulbs, lanterns, props, camels;
  - locked DOF, sun god rays, bloom that excludes the sand.
- Feedback: the sand rattle was too loud, so it was cut by about 24 dB.

## Crickets Are Thermometers (`projects/crickets_open`, `projects/crickets_diorama`)
- Dolbear's law: chirps in 15 s + 40 = °F, so BPM = 4(T − 40). Each chirp is the hi-hat. Seven genre
  bands as the night warms, then a cold front tape-stops the song.
- Open-environment version first. Then a Bit Wonder–like 2.5D diorama with a near-static camera.
- Lighting iterations:
  - the user asked for stronger shadows, then for a moody, dark relight;
  - that read as too dark, so the final version is the brighter middle ground (`drender.py`);
  - the moon in frame is the key light, there are long shadows from poplars, god rays and bokeh;
  - the focus is locked on the cricket and the thermometer, because a rack focus read as an error.
- Chirps sit 10–21 dB above the music in the 2.2–3.8 kHz band.

## The Lowest Note in the Universe (`projects/lowest_note`)
- Perseus cluster as a floating low-poly island: a basin of glowing hot gas, the black hole (dark sphere
  + hot disc) with jets above it, bubbles that launch pressure-wave rings, observatory + lamp, dishes,
  crystals, orbiting member galaxies, a Chandra-style satellite. First video built on `reelkit`.
- Octave counter -57 -> 0 on a slide-up HUD. One list of crest launches drives both the rings on screen
  and the kicks in the mix: 55 -> 109 -> 219 BPM, flutter 7.3 -> 14.6 Hz, then the same pulse train at
  29.1 Hz is heard as B-flat0; B-flat1..4 stack (colour band per octave) under an original melody at
  109.3 BPM (= the note 8 octaves down). Ends on "HUM A B-FLAT", rewinds and loops (frame 0 vs 60 s
  differ by 0.4/255 on average).
- What worked: additive glows sit on top of everything, so a "black hole" glow must be painted *before*
  the dark sphere or the dark centre disappears. Galaxies read better as dithered glow balls / arm-only
  spirals than as solid emissive meshes (those looked like popcorn or pizzas).
- Phone check: sub kicks vanish on phone speakers, so each crest carries a short pitched B-flat4 "thock"
  and a click; short-term peaks above 250 Hz then land within ~4.5 dB of the climax.
- Focus audit over all 1800 frames: black hole + inner rings CoC = 0. Orbit phases chosen so no
  foreground galaxy crosses the black hole on screen.
- Runtime glyphs (`♭`, `≈`, `→`) added to `reelkit.font.GLYPHS` from the project, not in reelkit.
- Feedback on v1: looks cool, neat fact, but near-silent for most of it (no beat until 29 s), so it won't hold viewers.
- v2 re-cut (`lowest_note_v2.mp4`, v1 kept as `*_backup.py`): cold open on the climax, tape-stop to real speed,
  beat-synced spin-up (year/day/hour/minute slams + calendar/clock widget), drop at 11.7 s, then the v1 arc
  compressed; "SPED UP ...×" HUD line + log-frequency ruler with a "YOUR HEARING" window; camera shake, a
  screen-space ripple per crest, crystals/dishes/dust hop on the kicks; last bar slams into the cold open.
- Lessons:
  - One grid for everything: 1 beat = the hole's note 8 octaves down, so octaves -9..-4 are exactly 2 beats,
    1, 1/2, 1/4, 1/8, 1/16. Placing the drop 88 beats before 60 s makes the loop point a bar line.
  - A scene clock (`timeline.clock`) separate from video time handles "freeze" and "time-lapse": it stops in
    the tape-stop and whips round in the spin-up, adding exactly one extra lap over 60 s so orbits still loop.
  - Never-quiet check on phones: RMS above 250 Hz per 2 s window, not just full-band RMS. Fixes: open the
    "muffled" filter from ~340 Hz (not 160), crackle clicks over rumble, a longer B-flat4 thock on sparse crests.
  - A few sub hits (thud, slams) set the peak and pulled the whole mix down 4 dB: a fast peak limiter at the
    99.5th percentile of 2 ms peaks before normalising fixed it (sections now -13 to -16 dBFS RMS).
  - Loop-aware mastering: wrap anything past 60 s onto the start and give the limiter/reverb a pre-roll of the
    song's end; render the cold-open B-flat stack with t0 = 60 s so its phase continues from the last bar.
  - Sparse-click checks: measure the band level at each crest onset (15 ms), not a p98 of 10 ms frames, which
    misses kicks that are 1% of the time.
- Feedback: (pending)

## Mozart's Dice Waltz (`projects/dice_waltz`)
- The 1792 waltz dice table (Simrock, Berlin; sold as Mozart's). Each roll's sum picks a bar, which lands on a
  parchment staff while a self-playing fortepiano plays it. Back half: ten cups' waltzes stack on different
  timbres and the wall histogram locks into the 2-12 triangle.
- Note data: 2-of-3 vote over three transcriptions (13 bars disputed), stored in our own format in `waltz_table.py`.
  Column harmony verified from the table (every column shares one harmony), so stacking stays consonant.
- Look: cut-away salon. The room shell (floor, walls, window) is ray-traced per pixel. The key light is the moon
  through an arched window; the moon is pinned in the arch as a painted backdrop, because a high camera can't see
  the real sky direction.
- Lessons:
  - A portrait frame only shows about 0.43 x distance in width, so keep rooms narrow (3.4 units here).
  - Use the camera's `sy` (screen y of the target) to keep readouts below the caption band.
  - A wall overlay must sort nearer than its board in painter order, or the board covers it.
  - A volumetric beam at 0.55 strength washed every frame with diagonal bands; 0.2 reads as atmosphere.
  - Seamless loop: reset everything before 60 s (staff clears, chart rolls back up, histogram sinks).
  - Audio: carve about -8 dB at 2.5-6 kHz in the music bus and keep high layers out of that band, so the dice
    clacks sit about 10 dB above even ten stacked waltzes.
- Feedback: (pending)

## Dinosaur Brass Band (`projects/dinosaur_brass_band`)
- Parasaurolophus crests as closed air tubes: crest air length sets each animal's pitch (f = 1/L), five sizes
  = five registers of an original G-major chorale. Honk synth = odd-harmonic closed-tube voice with a lip-buzz
  scoop and fixed passage resonances; honks sit 10-15 dB above the timpani/pad bed in 150-1500 Hz.
- Built the real dinosaurs (backlog fallback not needed): lofted rigid parts, neck/head pivots for raise and
  honk, leg swing for walks. X-ray crest = navy faces + cyan edges; CT slices drawn as hollow ring cards
  (outer ring + dark air hole), which reads as "slices" and "hollow" at once, unlike solid cards.
- Lessons:
  - Portrait vs a row of long animals: recede the row toward the low sun (depth/lat layout frame) and
    auto-fit wide shots from the posed geometry instead of hand-tuning targets.
  - A huge far volcano (150 m radius) read as a pink band across every sky and hid the sun: keep big far
    shapes small in angle and off the sun's azimuth; clear the forest silhouette where the sun sets.
  - The sun disc only reads when the sky around it is orange, not near-white: narrow the sky glow, small sprite.
  - Walk paths: straight along the heading went through the animal ahead in a column; walking straight off
    passed beside the camera. Use curved entries/exits and check "removed while on screen".
  - Locked focus: size the sharp band from every vertex and face-mean depth of faces touching the screen
    (on-screen vertices alone left 1.3 px edge blur); `focus_check.py` verifies all frames without rendering.
- Feedback: (pending)

## The Perfect Loop (`projects/perfect_loop`)
- 15 uncoupled pendulums (n = 51..65 swings per minute, exact analytic motion), so the 60 s video is one
  physics cycle and frame 1800 == frame 0. Clockmaker's attic at sunset: a cut-away gable with a round window
  behind the rig, so the low sun throws every bob's shadow forward across the floor (the wave is drawn twice).
- One trigger list (t = 60 m / n_k) drives the flashes, the coloured floor light and the plucks: 3 octaves of
  C pentatonic, longest = lowest = red. The middle pendulum (n = 58) is exactly the 58 BPM beat, so bass, pad
  and kick lock to the physics; groups become eighths/triplets/sixteenths by themselves.
- Lessons:
  - Window glazing bars cast stripes across whatever the beam is aimed at: a rose window with a clear centre
    (ring + spokes outside it) keeps the subject's light clean and still draws a pattern on the floor.
  - A top beam directly above a row of bobs hides them from high cameras near the row axis: keep high shots
    at yaw 30-55 (the end-on view only for "2 teams", where the halves sit left and right of the beam).
  - First sun/beam pass was one orange wash: cool ambient (blue-violet) + warm key keeps sunset colour with range.
  - Group tie-lines read as extra strings until dashed, 2 px and tinted per group.
  - `reelkit.audio.pad()` is ~20 dB quieter than a pluck at equal gain; measure per-bus RMS before balancing.
    Plucks sit 11-12 dB above the music in 300-3000 Hz in the back half.
  - Loop audit: seam diff 1.4 with the frame-0 flash disabled vs 3.4 for a normal frame step; the only jump is
    the intended unison flash. Focus audit: 0 blur on every bob in all 1800 frames (`focus_check.py`).
- Feedback (v1): scale the concept up (more pendulums) -> v2.
- v2 re-cut (`perfect_loop_v2.mp4`, v1 kept as `*_backup.py`, which import each other so v1 still rebuilds). Feedback on
  v1: scale it up. Now 100 pendulums (n = 51..150, 34.4 -> 4.0 cm, 10 050 plucks ~167/s) in a long gallery lit by four round
  windows in the left wall; physics = video + 8 s, so it opens mid-pattern (15 groups at frame 0), the ladder lands every 2-5 s
  and all 100 line up at 52 s; frame 1800 = physics 68 = 8 = frame 0. Every milestone is exact and on the 120 BPM grid
  (pendulum n = 120 plucks on every beat); hat subdivision = current group count. HUD counter races 60 / (52 - t): 8 -> 100 -> 1.
- Lessons (v2):
  - A 2.5 m row in portrait reads best near end-on (yaw 5-10, pitch 40-47): row up the frame, swing sideways. At yaw ~20 the
    swing partly runs along the row on screen and group structure vanishes; side views turn the swing into depth.
    (v1's yaw 30-55 rule was for 15 big bobs; with 100 the swing direction matters more than beam occlusion.)
  - 100 overlapping glow sprites + white-hot cores + additive trails sum to white: saturated cores, glows only on a pluck,
    and trails composited "strongest sample wins" with a screen blend keep the rainbow.
  - Groups in a 100-bob row are faint in stills: bold 2 px tie-lines in fixed group colours plus matching markers on the
    HUD phase-ring clumps make the count readable; the ring alone already shows clean clumps.
  - Wider release at the long (far) end (8.5 -> 2.0 cm, capped 30 deg) evens out the on-screen swing in perspective.
  - Focus audit only over on-screen bobs (a glide past the front end puts bobs behind the lens).
  - Audio: the in-phase 100-voice strum sets the peak normalisation and left the mix ~4 dB quiet; a 24 ms lookahead limiter
    (+4.5 dB drive) brought it to -18 -> -11 dBFS with the strum still loudest. A saw pad is what eats the pluck band:
    dropping it an octave, lp 700 Hz and -4 dB took the margin from 4 to 9-10 dB (17-20 dB before the band enters).
  - The rolled last-pluck sweeps (60 - j*T_k) converge visibly in the spectrogram: the physics makes its own riser.
  - Seam diff 3.2/255 vs 3.0 for a normal frame step; subject blur 0 on all 1800 frames.
- Feedback (v2): the VS title card intro became a series rule (user, Oct 24, after #30's test).
- v3: bar 0 = the shared VS title card (`projects/ballfight/title.py`; #35's session moved it there minutes before this one
  would have; this episode added TROMBONE slide-tube + HORN coil panel patterns, #30's card checked pixel-identical on 12
  frames), 3-2-1 on bar 1, FIGHT! at 3.94 s, top caption dropped, HUD hidden while the card is shut; card audio: a unison
  band hit per word + slide blast / thunder crack / two-horn hunting call, taiko pickup as the doors open; chords moved a
  bar (Dm under the card, A for the count-in, loop from FIGHT!, A into the seam). Honest search at 2 bars: v2's 894
  didn't reach a K.O. (the drift the rule warns about); new seed 2200 (8 fits in 2,200, 51 / 49): TROMBONE wins at 50.72 s
  with 4.0 HP after three lead changes in 1.5 s and a 3.7 s standoff at 4.0 vs 1.9 HP.
- v3 lessons: check origin right before moving a shared file (another session had just moved title.py; adopt theirs,
  add only what's missing). With the rules unchanged the median K.O. moves ~2 s later, so fewer runs land in 49-53 s
  (8 fits took 2,200 runs vs 974 at 1 bar); the solo now stops at the reassembly if a late K.O. leaves less room. The
  title slam makes a very clean seam (0.5x render, 1.2x mp4). Render 11.1 min.
- Feedback (v3): the trombone model is inaccurate (user, Oct 25).
- v4: the Pixel Pitch trombone rebuilt from reference photos found in Chrome (Yamaha YSL-891Z side view, measured from
  the image's pixels at ~0.41 mm/px using its 20 cm bell; Jupiter student tenor 3/4 view): the old slide plane was
  turned ~100 deg so the slide read as one rod, the bell sat 65 % of the way along a short slide (real: 40 %), and an
  F-attachment ellipse hung below the instrument. Now: slide ~40 deg out of the bell plane about the shared lower line,
  bell rim 31 cm / crook 78 cm ahead of the slide lock, bow 38 cm behind, Bessel bell fitted to the photo, nickel crook
  + bumper + stockings, tuning-slide braces + medallion, bell brace at the lock, a 4 cm mouthpiece, plain straight
  tenor. Every series scene still builds; the posted Pixel Pitch video is unchanged. Same fight + soundtrack; one full
  render (the trombone is on screen in every frame, so no patch render).
- Lessons (v4): read a reference photo's pixels in the page (canvas getImageData on the upload.wikimedia.org image
  tab) for real measurements instead of eyeballing a screenshot; check an instrument model in a plain orthographic
  side view next to the product photo before judging it at sprite size.
- Feedback (v4): (pending)

## Prime-Number Cicadas (`projects/prime_cicadas`)
- Brood XIX (13-yr) + Brood XIII (17-yr), together in 2024 for the first time since 1803 (next 2245). Golden-hour
  Illinois woodland cut away like an ant farm: soil face with strata, roots, tunnels and nymph chambers in front,
  XIX trees left / XIII right of a split-rail fence, lantern hanging over the cut, farmhouse porch lamp, cedars.
- Music = calendar: 1 year = 1 sixteenth at 120 BPM, so 1803 -> 2024 = 221 sixteenths and 2024 lands on a downbeat
  (51 s). XIX calls every 13 sixteenths, XIII every 17 (synth "Pharaoh" calls, C#6 vs E6) over an original A-major
  groove; each hit also plucks a chord tone. Ghost 12/18 broods at 36 yr/s = a 3:2 that meets every second.
  One emergence list drives bursts, crown flashes, HUD lane ticks and the calls.
- Lessons:
  - Low cameras in front of a cut sit below ground: test "ray reaches z = 0 below y = 0" for the face, and restore
    face pixels that objects behind the cut painted over (depth > face depth).
  - With a 6.5 degree sun, near crowns shadow the void in front of the cut; field stripes need low casters further
    back (cedars, tree line: only parts lower than distance-to-cut / 8.8 land on the field).
  - Glows need an occlusion test against the depth buffer (a porch lamp and fireflies leaked through the soil).
  - An orange crown flash over green leaves reads yellow-green: use a deeper red-orange (255, 104, 24) for XIX.
  - Calls sit 30+ dB over the music in 980-1450 Hz after carving 880-1750 Hz out of the music + drum buses.
  - Focus audit: subject CoC = 0 on all 1800 frames; frame 0 vs last frame differ by 1.9/255.
- Feedback: (pending)


## Count the Thunder (`projects/count_the_thunder`)
- Flash-to-bang counting: seconds ÷ 5 = miles (NWS), ÷ 3 = km (ECCC). Night prairie, cut-away log cabin, the
  cat watches from the sill (indoors throughout; ends on "TRY IT: COUNT FROM INSIDE"). One strike list drives
  flashes, bolts, a sound ring crawling across the prairie (reaches the cabin exactly at the thunder), the HUD,
  captions and the mix. First half: pitched count ticks + real-time thunder (10, 7.5, 5, 3 s). Back half at
  120 BPM: snare on each flash, the thunder is the kick, late by 2 s, 1, ½, ¼, ⅛ s, fusing at the barn-rod
  strike (50 s). Thunder low-pass by distance: centroid ~130 Hz (3.4 km) -> ~4.9 kHz (overhead).
- Lessons:
  - Portrait cut-away + sky is an angle budget: seeing the floor needs ~20°+ down, the storm needs the horizon
    in frame. fov 60 / pitch 21 fit horizon (y≈185), room (y≈265-527) and the HUD; a numeric probe of key points
    (floor corners, wall tops, rod, strikes) found it far faster than test renders.
  - Behind a cut-away wall the ground is hidden for ~8-10 units; storytelling on the prairie (ring, markers)
    needs a high wide shot (used as the 6-9 s establishing move).
  - Fireplace on a side wall is seen edge-on; moving it to the back wall made the fire (and the chimney above
    the wall) read. Black cat vs a dark window vanished; ginger reads.
  - A far storm must be squat to fit the sky band; lift it as it nears (cell_lift) so near bolts drop from the
    top of frame.
  - Flash envelope must peak ON the strike frame (a 1-frame ramp put the picture 33 ms behind the snare).
  - Smoke as opaque polys read as floating rocks at night; banded GLOW-kernel sprites blended after the lens work.
  - Photosensitivity (`flash_check.py`, Harding-style on the mp4): 0 general flashes; max 4 % of frame per strike.
  - Mix: high-passed "bright" snare (280 Hz) + deep bass ducking keep the thunder-kick 8-15 dB over the music.
- v1 feedback: not engaging enough, especially at the start (first 30 s ~1 hit/s at ~-25 dBFS, whole mix 6-9 dB under
  the other reels).
- v2 re-cut (`count_the_thunder_v2.mp4`, same scene/renderer, new timeline + audio): cold open on the climax (rod strike,
  fused clap, full groove), a 2 s rewind back to the horizon, then ~1 flash/s of storm chaos at 1-5 km: every ring
  reaches the cabin when its rumble sounds, so the rumbles land out of step. A tagged far flash is counted on the beat
  (15 s = 3 MI ≈ 5 KM), delays shrink (5 s, 3 s), then v1's lock-in, and a second cell (3 s, 2 s, 1 s) whose rod strike
  is frame 0. One strike list with kinds (pre / chaos / count / lock / rod / cell2) drives everything.
- Lessons:
  - v1 sat low because the big claps set the peak normalisation: a 3 ms lookahead limiter master (drive +5 dB) gets
    -12 LUFS (other reels ~-11) and lets section gains shape the build (chaos -17…-19 dBFS -> climax -12).
  - A 2 s-window "hits/s" probe (`engagement_check.py`: >= 6 dB energy jumps per band, within 20 dB of the loudest)
    matched the ~1 hit/s measured on v1. Sustained pads/rumbles mean the 250-2500 Hz band never registers a hit; a
    beat-synced sidechain pump on pads, rain and smear fixed that. Rain hiss masks hats: keep it down under a groove.
  - Count ticks starting at D4 vanished under the full groove (no energy in 1.5-4 kHz); start at D5, one octave per
    5 s (= 1 mile), and pull the flash snare back on downbeats that carry a tick.
  - Ring clutter: ~1 flash/s with 4-17 s delays keeps <= 10 rings on screen; chaos rings at 0.6 strength so the amber
    counted ring reads.
  - Rewind: a "phase" clock that runs 10 s backwards inside the 2 s rewind, so the whole video spans 48 s of phase (a
    multiple of the 3/4/6 s motion periods) and periodic motion still loops; windmill speed integrated and scaled to
    whole blade steps. A 2/4 bar (27-28 s) moves the downbeats from odd to even seconds after the rewind.
  - Loop: the second cell ends exactly where the main cell starts; seam diff 3.9 vs a normal 3.6 frame step (with the
    intended frame-0 flash removed). Focus CoC 0 on all 1800 frames; flash check 0 general flashes, <= 5.1 % per strike.
- Feedback: (pending, v2)

## Chaos Makes a Perfect Curve (`projects/galton_board`)
- Galton board (12 rows, 4,096 paths, 13 bins) as a giant brass-and-walnut machine on the stage of a sunlit
  Victorian lecture hall: three arched plate-glass windows (low sun from front-left = key light, painted sun pinned
  in the first window), gallery with standing spectators and a pigeon, chalkboard ("GALTON, 1889"), bookcase,
  gas lamps, tiered benches with pawn spectators. Built to the new engagement rule: cold-open torrent with sound
  from frame 0, never quiet, lock-in at the drop.
- One cached simulation (`sim.py` -> `balls.npz`, `motion.py`) drives picture and sound: honest seeded runs (main
  1,000 balls seed 1889, cold-open torrent seed 4096) plus one staged all-left golden ball shown as a "what if",
  never counted, dissolved after landing. Real run: 0, 8, 9, 48, 119, 194, 229, 198, 116, 58, 20, 0, 1 vs ideal
  0.2 ... 225.6 ... (one real ball reached the right edge bin; kept, it's honest).
- Sound: each contact pings its column on C pentatonic (C5 centre), so every ball plays a 12-note random walk;
  decay and gain scale with ping density (melody -> rain -> hail, ~1,400 pings/s at the flood); landings = tock +
  bin note; each bin holds one note of a wide C6/9 chord and is as loud as the bin is full; after the drop a
  marimba arpeggio picks bin notes with probability = the real counts; Pascal rows ping left to right as they light.
- Lessons:
  - Raking sun on a deep board: full-depth bin dividers shadow most of each bin and long pegs streak. Shallow
    pegs (3.4 cm), balls 2.1 cm off the backplate, dividers only as deep as the balls, sun az 226 (not 238).
  - A light maple backplate + warm lamps washed out the subject; a mid cherry-walnut board with the lamps kept
    small (R ~1 m, no wrap light on planes facing away) makes rainbow balls and their shadows read.
  - Painter order with a ray-traced backplate: draw "behind" objects, then the backplate pass, then the machine.
    A base box drawn after the balls hid the bottom rows (its top face behind the backplate): draw base first,
    and hide draining balls once they pass the bin floor.
  - Tiny balls need a 1 px dark outline to separate from pegs; the hopper pile reads better without it.
  - Path trails (contact to contact, cool = left, warm = right) explain "coin flips" and the golden staircase better
    than arrows alone.
  - Hook caption fully on at frame 0: start it at -0.5 s and re-type it at 59.5 s, so the loop seam is clean.
  - Checks: focus CoC 0 on all 1800 frames (`focus_check.py`); science band 300-3000 Hz 9-25 dB over the music in
    every section with balls; quietest 0.5 s window -22.7 dBFS (never quiet).
- Feedback: (pending)

## Fireflies That Sync (`projects/fireflies_sync`)
- *Photinus carolinus* at Elkmont: moonlit low-poly cove (tulip poplars, creek, ferns, red-lit trail, owl, layered
  ridges). One flash schedule (`sim.py`) drives glows, flash light on the litter, the HUD raster and the bells: ~60
  random flashers (sparkle, ~60 notes/s), a Kuramoto lock-in in the beat-grid frame (spread 274 -> 8 ms over burst A =
  smear -> flam -> unison), burst 2 as a lit band moving at 0.5 m/s (each fly flashes while the front is within
  0.69 m = 5 flashes; pitch follows x so the chord climbs and pans), burst 3 as two fronts meeting + a 2500-fly
  valley. Females answer with kalimba notes in the gaps. 109 BPM from the 0.55 s flash spacing; video = 109 beats.
- Lessons:
  - A 100-150 ms flash is only ~2 bright frames at 30 fps: random sheets miss it. Check sheets at flash-aligned times
    (`T.grid(b)`), and draw glows with a perceptual curve (I^0.55) so flashes linger without changing their timing.
  - Big zooms (7 m -> 9 cm macro): interpolate distance in log space AND slide the look-at target in proportion to
    the distance, or the subject passes behind the lens mid-push.
  - Populations scattered "around the clearing" mostly landed off-frame; place them in each shot's view wedge.
  - Thousands of far flashes as halos merged into a glowing hedge on the horizon; below ~0.1 px size draw a 1 px core.
  - DOF: `sharp = max(need, min(0.28, 1/zf))` (template-style) keeps wides crisp and still gives macro bokeh;
    flashes with CoC > 1.6 px become octagonal bokeh discs (beautiful behind the macro hero).
  - Photosensitivity: splatted ground light from a synchronous valley flash crossed 25 % area (28 %); lowering the
    light map gain and bloom brought it to 22 % (0 general flashes on the mp4).
  - Audio: cap the summed gain per 12 ms pitch bin (thousands of bells otherwise set the peak and push the opening to
    -25 dBFS); run the compressor envelope around the loop seam; fold the >60 s tail onto the start.
- Feedback: (pending)

## Marble Waterfall Organ (`projects/marble_waterfall_organ`)
- Satisfying-first (user direction), abstract void (no room, no ground). A brass-and-glass marble machine floats on a
  stone plinth: central spiral-screw lift in a cage, 8 tiers (kick drum + toms, snare + 3 glass discs, 10 tuned pipes,
  7 glass bowls, 7 tuned bars, a 1.25-turn 16-bar spiral ramp, crash + ride, 10 golden bell bars), basin ring of ~640
  marbles. Opens with every gate open (~120 hits/s of random D-major notes, escapees streak away weightless); gates
  shut one by one on the downbeat (6, 10, 14, 18, 20, 24, 32 s), the melody gate sticks (35-40 s, two failed attempts,
  half-beat silent gap), snaps shut = drop at 40 s; lever at 57 s springs every gate open back into frame 0.
- One cached list (`sim.py` -> `marbles.npz`) drives marbles, element flashes, basin splashes and every note: routed
  marbles roll down a chute then take an exact ballistic hop (v0 = d/T + g T/2) onto the element at the note time;
  sprayed marbles take 1-3 random contacts (each contact = that element's note) then escape. Spiral: one marble per bar
  rings all 16 ramp bars on 16ths. Full song ~33 marble hits/s. Music original, 120 BPM, | Bm | G | D | A | from bar 8
  so the drop and the final chord both land on Bm.
- Lessons:
  - A lat-long nebula texture has a seam: index it with az/2pi + 0.5 so the seam sits behind the camera (it showed as a
    hard vertical band right behind the machine at az 0).
  - Peak-normalising the master flattened the build (wash = drop at -9 dBFS). Set the drop's RMS to a target (-10.5) and
    only limit peaks: wash -15, kick -13.5, bass -12.6, drop -10, back to -15 at the seam.
  - Loop-seam click: the reverb tail of the last seconds was added twice at the start. Fold every bus into a periodic
    60 s buffer first, then circular reverb (`reverb(concat(send, send))[NS:]`) and circular filters (1 s wrapped pre-roll).
  - Simultaneous chord notes on one chute rolled as one blob: stagger their hop times (x(1 + 0.32 j)).
  - Visible orb vs shadows: draw the orb low beside the crown (el 7) but shade with a higher key (el 28) so the machine's
    shadows land on the plinth top.
  - Holdout framing: from the front over the bell arc (pitch ~16) so the instrument arc doesn't hide the stuck flap;
    red open flaps + lamps, sparks on the failed attempts, a light burst on every gate shut.
  - Flash check: 9 general flashes on the first render. Causes: bar-line void bursts with a white core, a nebula pulse on
    every bar, and fast camera moves over the textured nebula (fly-down, an 84 deg whip pan). Chord-tinted burst ring with a
    3-frame rise, palettes of similar luminance, pulse 5 %, gentler pans -> 2 (motion only, worst 1 per second).
  - Speed: 2,195 faces + ~250 moving marbles + pile + lift, ~0.47 s/frame, ~7 min on 2 cores.
- Feedback: (pending)

## Domino Symphony (`projects/domino_symphony`)
- Satisfying-first reel: one paw tap -> 64-ray rainbow fan (out of step) -> onion-dome funnel locks into 16ths -> bell lane
  (every 4th rings) -> 4 chord lanes -> 4,096-domino field reveals a pixel music note in a chevron sweep -> 2-lane spiral
  (16ths -> triplets -> 32nds) -> red holdout on a plank ledge teeters -> 1.5x staircase booms D C# B A G F# E -> 1.28 m giant
  slams on the drop -> 3+3 frame lanes race round the layout playing the song on chime bars -> rewind into the loop.
- No physics sim: `layout.py` gives every box a strike time from the grid and `boxes.py` tips it about its front edge; the
  same table drives picture, clicks, chimes and the counter. The funnel's domino count is solved so the total is exactly 10,000.
- Lessons:
  - A 1 -> 64 binary fan that funnels back: route the funnel as polar "meridians" around the fan origin whose lateral spread
    (r * h(r)) halves between merge rows; adjacent lanes never cross and pairs merge where they would get too close. Offset the
    merging pair across the PARENT lane's own direction (outer lanes curve sideways). A cheap footprint SAT audit plus a
    mm-scale relaxation pass took standing overlaps to 0; fallen overlaps at Y junctions are normal.
  - Painted-face reveals: a domino's painted back faces the camera while it stands whenever the camera is on that side (all
    the views from the start of the layout), so keep field dominoes cream on every face until they tip.
  - 10k boxes at ~0.5 s/frame: vectorise tilt/corners/normals/shading for all boxes, splat anything under ~2.4 px as pixels
    (depth-tested), draw the rest as PIL polygons in painter chunks of <= 12 per lane; give every big/special box (holdout,
    staircase) its own chunk or same-lane order puts the step behind over the holdout. Sun shadows: project sun-facing faces
    to the floor into a screen-space mask; tiny dominoes cast dotted streaks.
  - Exposure: window-beam volumetrics fill the whole room when the windows are tall, so keep the density very low
    (acc/46, x0.28) or every frame goes yellow-hazy; floor sun x1.7, bloom threshold 236.
  - Audio: 1,000 clicks/s are peaky; a fast compressor on the click bus stops the master limiter pumping the whole field
    section down. reelkit pad() needed ~+16 dB (music bus gain 4.5) to be heard at all. Full-band RMS undersells a click-heavy
    section; check approximate K-weighted loudness (`loudness_check.py`). A tom-led build keeps the 2-6 kHz band for the clicks.
  - Rewind: one scene clock tau (56 -> -0.3 s eased, then real time) rewinds dominoes, the cat and the counter together; a
    tape-reversed final chord fills the gap before tau reaches the last dominoes.
- Checks: subject CoC 0 on all 1800 frames; domino band +8..+43 dB over the music wherever clicks play; seam diff 1.9.
- Feedback: (pending)

## The Metronome Choir (`projects/metronome_choir`)
- Sequel to #1 on `reelkit`: 199 metronomes on 5 tiered risers (4 x 10, one empty seat = the soloist's) + a golden
  soloist on a podium, low-poly concert hall lit by a big round skylight (sun = key), organ facade, stage wings, warm
  truss spots, footlights, a ginger cat on the grand piano, timpani, pawn listeners. #1's metronome model, tip colours
  and soda cans kept for series recognition; caps and board edges in section colours.
- Physics: two-layer Kuramoto. Each riser couples its own 40 once it settles on its cans; the platform on giant cans
  couples everyone (and surges, as in #1) and drags in the soloist (15 % fast) last. Calibration is exact because the
  model is rotation invariant: per-riser rotations set the 5 sections' flams before the platform (+-70-150 ms), a
  global rotation puts left-end ticks on whole seconds (bar lines on even seconds, drop at 38.0 = a downbeat), the
  soloist's start phase is searched (it barely moves the crowd, so it is integrated alone against the crowd's mean
  field) so it joins at 37.8 s, and the jolt resets every phase to theta(0) - omega(60 - 58): frame 1800 = frame 0.
- Sound: every tick is a bell, built as impulse trains (fractional sample positions) convolved once per note: ~24k
  ticks in seconds. Section pairs by swing side stack into D major (left) / Asus2 (right); unsynced = short quiet
  bells, locked = long bells at 1/N gain so a locked riser's line rises out of the wash. The soloist's carousel steps
  one melody note per own tick, slot 0 on the drop, so the last 2 bars are a pickup that drags against the beat.
- Lessons:
  - Bell partials: the classic 2.76 partial turned 200 stacked bells into a clang; vibraphone-like 1/2/3/4 (+ a hint
    of 2.76) keeps the chords clean. Avoid C# in melody/stabs over the ringing D-major ticks (A chords kept sus2).
  - Locked sections get ~6 dB louder when the global coupling surges (spread 22 ms -> 0.5 ms = fully coherent sum):
    a natural build into the drop; balance the soloist above it.
  - Photosensitivity: a fast camera sweep across bright boards / dark steps flagged 30 % frame changes for 2.5 s
    (worst 3/s, at the limit). A static push with each section lighting up in turn reads better and flags 0.
  - A stage truss at y 13.6 sat right in front of the wide frontal cameras (a black band across the frame): keep rigging
    above the cameras' top rays (y 16.7 here).
  - LED show on lit cream bodies washed out to pastel; darken "off" pixels (body front * 0.32) and paint "on" pixels
    with the LED colour over the whole front of the body: the 10 x 20 display (tiers stack in a frontal view) reads.
  - Per-pixel sun tests against ~40 AABBs were 55 % of frame time: test each surface only against the boxes that can
    shadow it and evaluate the floor on a 2 x 2 grid (1.8 -> 1.3 s/frame).
- Feedback: (pending)

## The Mountain King Escape (`projects/mountain_king_escape`)
- Satisfying-first "can it escape before it ends?" reel: a glowing ball in 12 counter-rotating crystal rings inside the
  Mountain King's cavern (cut-away diorama: moon shaft through a roof crack, torches, crystal clusters, troll statues,
  stalactites). Every melody note of Grieg's theme is one wall hit; phrase endings are escapes (ring shatters into shards
  that pile up on the floor). Ring 12 breaks on the final chord, the ball goes up the shaft and out of the snow-capped
  summit, the camera pulls back: the floating mountain sits inside a giant ring with a gap (BONG, "TRAPPED AGAIN?"); the
  shards fly back into rings, all 12 gaps line up, the ball drops through into ring 1 = frame 0.
- Music: transcribed from the CC0 Mutopia engraving of Grieg's own piano arrangement (University Society 1918), parsed from
  its LilyPond source (`verify_score.py` diffs it bar by bar). The engraving's right hand in bars 63-64 repeats bars 59-60
  (G natural, D natural) while its left hand has D#: read as a copy-paste slip, so B# uses D# and the run uses G#.
- One timeline: 11 phrases (A A B B A A A B B# A coda), tempo 152 for 32 beats then 152 -> 265 (8.8 bounces/s at the end);
  the instrument climbs pizz -> bassoon -> strings -> octaves -> brass -> tutti -> prestissimo -> coda stabs.
- Lessons:
  - Ball path = a dynamic programme per ring over wall angles on a 0.5 deg grid anchored on the escape angle: long chords
    (138/158/172 deg for eighth/quarter/longer gaps), one circulation direction per ring, every hit clear of the gap at
    that note's time, near-misses restricted to a skim window, last chord ends exactly in the gap. Exact and instant
    (0.4 s for 267 notes), unlike random search.
  - Hand-off rule: if ring k+1's gap is near ring k's gap at the escape, the ball would fly through both. Pick each ring's
    spin (outermost first) as close to plan as gives >= 0.9 rad between them; ring 12's spin is an integer number of turns
    so its gap is on top at the final chord (straight up the shaft).
  - "Two near-misses in a row" can't be consecutive hits (the gap barely moves between them): skim, cross, skim the other
    edge (hits i and i+2).
  - Loop with a scripted tail: rings get their phase from music time m = t - 60 once they re-form, and every gap is set to
    point up at the moment the falling ball passes it, so the drop-in threads all 12 gaps and frame 0 continues it.
  - The cut-away face (per-pixel strata) dominated wide shots with wide wavy bands; finer, closer-toned strata, a raised
    front ceiling and a snowline under a jagged silhouette made it read as a mountain.
  - Glows need an occlusion test even for the hero: the ball's glow showed through the rock while it was inside the chimney.
  - 15 Hz camera shake over bright ring bands risks flash-test failures; ~6 Hz, ~1.5 px is enough.
  - The ring-1 break in close-up (all ring hues stepping at once + burst) is the only general flash (28 %); a slower hue step
    would remove it if needed.
  - Checks: CoC 0 on all frames; bounce voice 10-26 dB over the accompaniment (1.5-6 kHz); -9.9 LUFS; seam 4.8 vs 5.6.
- Feedback: (pending)

## Battle of the Bands (`projects/battle_of_the_bands`)
- Satisfying-first territory battle, inspired by @algorithmbattles (4-team battles) but ours: four physical-process teams
  (SAND pours/piles/avalanches = drums, BALL bounces and smashes tiles = lead, RIPPLE one ring per beat = bass, VINE grows,
  branches, burrows = keys) fight over a round LED wall on a floating low-poly festival stage; legend pinned to each team's
  base, territory bar under the wall, one speaker stack per team. Every move = a note, stage share = volume, knocked-out
  bands power down and drop out, the winner solos; at 58 s the wall cracks and the next run's burst loops into frame 0.
- Reusable engine (`battle.py`): arena shape, team line-up + balance, seed and bell time are parameters; it records owner
  grids, claims, attempted moves, knockouts and each process's per-frame state. `seed_search.py` ranks honest seeds by story
  (knockouts near 31/41/50 s, the bell leader falls, underdog winner, lead changes, a real duel, no lingering). Seed 82 of 800
  (VINE leads at the bell, three lead changes, SAND wins from tied-last 17 %); never edited.
- Rules added beyond the backlog, and why:
  - Balanced per-cell churn never eliminates anyone (it needs drift), per-cell sand avalanches snowball, and a Pong-wars
    ball is anti-snowball (a small pocket flips tiles fastest, so it lingered at 2-4 % for 20 s). Fix: "volume = power"
    (attack odds and process speed scale with stage share, i.e. what you hear), a heat ramp that restarts after each
    knockout (rounds, so knockouts space ~10 s instead of cascading), a Go-style capture (7 of 8 neighbours) that cleans up
    speckle, and a knockout below 2.5 % of the stage instead of at 0 tiles.
  - FILL left 15-30 % enclosed gaps (ball arcs and the vine web box regions in), so the last 4 s of FILL close up (empty
    tiles next to a team fill in, accelerating) and the bell snaps the rest.
- Music: original A minor, 120 BPM, | Am | F | C | G |. Gating on claims alone left RIPPLE's bass nearly silent (boxed in
  for ~10 s while alive), so the engine also logs attempted moves: a landed move plays accented, a failed one a ghost note;
  a stalled team (no move at all) still stutters. Volume = share of the claimed stage (FILL counts like FIGHT); a slow band
  leveler (max +3.5 dB over 1.5 s) keeps energy without hiding stutters. Opening chaos: every 6th claim rings a random chord
  tone off-grid, locking to the grid by 4 s.
- Look lessons: a crowd standing on the stage read as a field of posts; a pit below the stage edge shows heads and raised
  hands and frees the boards for moon shadows, moving-head pools and a glossy reflection of the LED wall (sampled by
  mirroring the eye in the boards). LED wall = per-pixel ray/plane hit into a 4x texture of the grid (grains, ball + trail,
  ring bands from each ring's geodesic distance field, vine tips, claim flashes, knock-out embers, note sparkles).
- Checks: safe zone OK (bar lifted 3 px: the panel's drop shadow crossed y 416); LED wall + props CoC 0 on all 1800 frames;
  flash check 0 general flashes (max 14 % at the crack); audio -14 dBFS fill -> -12 fight -> -10.8 solo, quietest 0.5 s
  -16.9 (the breath before the bell); first render's loop seam jumped 4.6 (nebula drift, twinkle, pennants, spot pools,
  a searchlight term not whole cycles over 60 s) -> every periodic motion made whole-cycle, seam 2.1 vs 1.1 normal step.
- Render ~0.6 s/frame/core (18 min on 2 cores); seed search ~3 s per run.
- Feedback (v1): start the video when the stage is empty, so the loop starts and ends where the video does.
- v2: same run (seed 82), video time = run time: frame 0 is the empty stage as the four seeds drop and the crack ends at
  exactly 60 s. Kept the run by keeping its bell (21.4 s) and ring phase and moving the music grid to bar lines at 1.4 + 2k s
  (so the bell is still a bar line); every event moved 1.4 s later. Seam lessons: the stage was still lit (and reflected) by
  the full winner wall during the crack (fade glow + reflection with the tiles that have fallen); the 7 s hopper swing isn't a
  divisor of 60 (slide it home during the crack, when no sand falls); the win push-in and crowd pose must release before
  60; a bar sliding out crosses the safe-box edge (fade it in, cut it on the crack).
- Feedback (v2): the VS title card intro became a series rule (user, Oct 24, after #30's test).
- v3: bar 0 = the shared VS title card (`projects/ballfight/title.py`; #35's session moved it there minutes before this one
  would have; this episode added TROMBONE slide-tube + HORN coil panel patterns, #30's card checked pixel-identical on 12
  frames), 3-2-1 on bar 1, FIGHT! at 3.94 s, top caption dropped, HUD hidden while the card is shut; card audio: a unison
  band hit per word + slide blast / thunder crack / two-horn hunting call, taiko pickup as the doors open; chords moved a
  bar (Dm under the card, A for the count-in, loop from FIGHT!, A into the seam). Honest search at 2 bars: v2's 894
  didn't reach a K.O. (the drift the rule warns about); new seed 2200 (8 fits in 2,200, 51 / 49): TROMBONE wins at 50.72 s
  with 4.0 HP after three lead changes in 1.5 s and a 3.7 s standoff at 4.0 vs 1.9 HP.
- v3 lessons: check origin right before moving a shared file (another session had just moved title.py; adopt theirs,
  add only what's missing). With the rules unchanged the median K.O. moves ~2 s later, so fewer runs land in 49-53 s
  (8 fits took 2,200 runs vs 974 at 1 bar); the solo now stops at the reassembly if a late K.O. leaves less room. The
  title slam makes a very clean seam (0.5x render, 1.2x mp4). Render 11.1 min.
- Feedback (v3): the trombone model is inaccurate (user, Oct 25).
- v4: the Pixel Pitch trombone rebuilt from reference photos found in Chrome (Yamaha YSL-891Z side view, measured from
  the image's pixels at ~0.41 mm/px using its 20 cm bell; Jupiter student tenor 3/4 view): the old slide plane was
  turned ~100 deg so the slide read as one rod, the bell sat 65 % of the way along a short slide (real: 40 %), and an
  F-attachment ellipse hung below the instrument. Now: slide ~40 deg out of the bell plane about the shared lower line,
  bell rim 31 cm / crook 78 cm ahead of the slide lock, bow 38 cm behind, Bessel bell fitted to the photo, nickel crook
  + bumper + stockings, tuning-slide braces + medallion, bell brace at the lock, a 4 cm mouthpiece, plain straight
  tenor. Every series scene still builds; the posted Pixel Pitch video is unchanged. Same fight + soundtrack; one full
  render (the trombone is on screen in every frame, so no patch render).
- Lessons (v4): read a reference photo's pixels in the page (canvas getImageData on the upload.wikimedia.org image
  tab) for real measurements instead of eyeballing a screenshot; check an instrument model in a plain orthographic
  side view next to the product photo before judging it at sprite size.
- Feedback (v4): (pending)

## Falling Sand Wars (`projects/falling_sand_wars`)
- Battle-family reel in mood A (dark, hypnotic): four sands with their own physics (IRON sinks, PUMICE floats and rolls,
  CLAY piles steep and clings to walls, GLASS runs flat) poured into a glass box set in a turning brass ring, hung on
  chains under a cracked lintel in a night desert ruin (low full moon, two braziers, dunes, palms). Falling-sand engine
  (`sands.py`, 128 x 128 in the box's own frame, gravity snapped to 8 directions); the plug pops at bar 8 (20.33 s) in a
  seeded bottom corner; 45 deg turns on downbeats; the drained sand builds a strata dome on the dune. Seed 176: IRON out
  32.7, PUMICE 44.7, GLASS 51.1, CLAY wins keeping 388 grains; the closing roll pours the winner out and lands the empty
  box upright on 61.0 = frame 0 (the inlet springs open). Soundtrack: D Phrygian minimal techno at 94.43 BPM, drone +
  heartbeat from frame 0, each team's 16th-slot activity steps its part, turn hits, avalanche toms, drain tones, a stark
  D minor(b9) at the win.
- New reelkit pieces: `ui.share_bar` (fixed name slots + leader lines that slide with the segments, OUT state) and
  `ui.draw_captions(..., style='impact')` (drop, scale punch, text-only shake, sand puff) + `audio.thud()`.
- Lessons (sand sim):
  - Four separate streams make four side-by-side columns: different colours never touch, so nothing sorts. One shared
    inlet (a braided stream) mixes them, then density swaps sort the pile (iron core, clay on top, glass wide, pumice
    outside) - the "IRON SINKS / PUMICE FLOATS" shot needs that.
  - Draining a layered pile through any hole cuts across the layers (the crater exposes every layer), so all teams drain
    in proportion and the last ones go out together with almost nothing left. Selective elimination doesn't emerge.
    A knockout threshold (12 % of the start; the team's last grains go dark and still drain) gives pacing and a winner
    you can see. The physics doesn't depend on the threshold until the win, so one run per seed can be re-scored at
    many thresholds (`check/ko_sweep_*.txt`) instead of re-running the search.
  - Mid-wall holes leave pockets in the corners that no tilt drains; a corner notch is the one hole a tilt can make the
    lowest point. The turn rule "tip the hole lower with p 0.7" keeps the drain going; pure random turns stall for 10 s+.
  - A pile peaking at the inlet starves whichever hopper injects last: shuffle the hopper order every substep.
  - Single-file grains along a wall drain one per two passes: the closing pour opens the whole side wall and runs 3x
    substeps (the rattle), so the box is empty by ~59.5 s.
  - Balance is not even under the final rules: across 90 runs clay and pumice win most, glass a few, iron almost never
    (it sinks onto the bottom-corner notch). An earlier rule set had all four winning but no story. Next episode: add top
    corners to the seeded hole positions or lower the turn greed.
- Lessons (look): a big sky glow around a low moon washes the disc to white; a long lens (fov 36) compresses the sky, so
  the lintel's underside sat exactly at the moon's height until the ruin was raised. The strata pile reads as bands only
  when built ring by ring (lathe), not from a square mound grid.
- Time: ~4 h, far over the 60-75 min target; nearly all of it went into sim rules and balance, not the picture.
- Feedback: (pending)

## Rock Paper Scissors Orchestra (`projects/rock_paper_scissors_orchestra`)
- Battle-family reel in mood A (ominous cinematic orchestra): 120 low-poly tokens (40 stones, 40 folded sheets, 40 pairs of
  scissors) chase on the round stage of a low-poly opera house (curtains, gilded boxes with pawn patrons, crystal chandelier
  key light, footlights, orchestra pit with a conductor beating time, the stalls). A catch converts the caught piece and
  steps the hunter's section (ROCK timpani/taiko, PAPER pizzicato + tremolo, SCISSORS brass stabs); volume = headcount.
  Seed 60: ROCK crashes to 5 pieces at 30 s, swells to 110, wipes out SCISSORS at 38.5 s and dooms itself; PAPER wins at
  46.6 s. The twist caption ("ROCK KILLED / SCISSORS... / AND DOOMED ITSELF") lands its last line on a low brass sting.
- Loop: frame 0 = the clump in three colour wedges bursting out; from 57.2 s the winner spirals into the same slots (Hungarian
  assignment) and turns to face outward, arriving at exactly 61.0 s. Only the colours change at the seam (picture seam 2.2x).
- Engine (`rps.py`): seek nearest prey / flee nearest predator (inside a sight radius, sliding along the rim when cornered),
  wander, damping, speed cap, elastic disc collisions, conversion on touch with a short immunity. Plain seek/flee eliminated
  a team by ~15-20 s with 2-4 s mop-ups; halving the speed got the timing right and a "panic sprint" (+30 % cap with a
  predator in sight, for everyone) produced the near-death comebacks (teams dip to 1-6 pieces and recover). Fits are rare
  (~3 % of seeds), so `seed_search.py` logs every run's counts (`check/raw_*.jsonl`) to re-score without re-running.
  Balance over 102 runs: ROCK 24, PAPER 23, SCISSORS 26 wins.
- Lessons:
  - With catches on from frame 0 the packed clump converts instantly; starting catches at bar 1 makes the hook the three
    wedges fanning out (one word per beat, each team lit as its word lands).
  - A rich caption line only has base / hi / alt colours: the three-word hook is three one-line captions with their own y0,
    each landing on its beat (draw_captions(..., y0=...), ui.IMPACT_STAGGER set per caption for the twist).
  - The win "blaze" washed the boards to pale yellow and hid the white paper tokens: blaze the chandelier, not the floor.
  - Section automation before the compressor was flattened (build 0 dB); applying it after the normalisation gave +3 dB.
  - AAC frames are 1024 samples: 61.0 s at 48 kHz pads the last frame with 640 samples of silence. ffmpeg-based decoders
    play it, so `loopcheck out.mp4` sees a level step at the seam even though the wav passes (Pixel Pitch avoids it with
    1840-frame / 61.333 s files). Open question for the user.
- Render 0.25 s/frame/core (7.5 min on 2 cores). Time: ~45 min to v1.
- Feedback: (pending)

## Bounce Royale (`projects/bounce_royale`)
- Battle-family reel #3, mood A (dark synthwave): four balls (RUBY drums, JADE bass, COBALT arp, AMBER lead) fight
  over a 48-tile wall of a holographic ring above a rain-soaked neon rooftop (magenta moon, stairwell door lamp, a
  neon sign, projector under the ring, city towers with lit windows, banners, beacons and drones). Every wall bounce
  splashes paint and plays its colour's instrument (pitch from how high on the ring, pan from the side, volume =
  share, repaint = accent, pop = cut-out). Ring shrinks from 16 s; the winner floods the ring and solos (its bounces
  step through a motif); the ring closes to a point that frame 0 snaps open.
- Engine `royale.py` (pure-Python billiards, ~40 ms per run) + `seed_search.py` (story ranking, early stop).
  Seed 25672 of ~12,700: RUBY leads early, JADE takes over 13.5 s, COBALT and AMBER both at 1 segment, COBALT pops
  24.85 while AMBER claws back 1 -> 21, RUBY hangs on 1-2 segments for 12 s (pops 39.63), JADE vs AMBER swings four
  times (24-24 at 45 s), JADE wins 47.58. Captions are placed by the run's events (a story beat every 2-4 s).
- Rule changes vs the backlog, and why:
  - One tile per bounce never eliminates anyone: hits spread evenly round a circle, so each colour sits at 12 +- 3
    of 48 (Ehrenfest-like restoring force; 0 pops in 24 runs). Splash width proportional to share makes each
    repaint a fair coin for both colours (neutral drift, like the voter model), so pops happen and comebacks too.
    Exponent 1.5 snowballed (all pops by ~20 s); ring-scaled splashes cascaded the pops together.
  - Equal masses with equal speeds don't keep equal speeds after an elastic collision: keep the elastic directions,
    reset both to the shared speed (the "same speed for every ball" flag).
  - A ball can end up gliding along the wall (in a stepped sim: thousands of bounces/s, which also let the literal
    rule pop colours by sweeping): a 12 deg bevel on grazing bounces fixes it.
  - Launch one ball per beat so the instruments enter in turn (simultaneous launches all hit the wall at 0.37 s).
  - Fit needed "close duel" + "near-death before pop 1" in the search, or the top seeds had one-sided duels.
  - Winner choice: drums won #20 and #21, so a seed with a non-drum winner and an equally strong story was searched
    for (second batch); JADE (bass) wins.
- Lessons:
  - Loop seam lighting: a collapsing ring's lights fading to 0, then the snap-open at full strength jumped the roof
    lighting (seam 3.2x). Let the closing point charge up to the same light (blend both ends to one lilac) -> 1.0x.
  - Audio build: section gains before the compressor got flattened (+1.3 dB); apply them after the compressor and
    lower the target so the limiter doesn't cap the loud half (+3.1 dB).
  - With fov 50 portrait the horizontal view is only +-15 deg: props near the camera must sit within |x| < 1.4 m.
  - City windows as 1 px points disappear under DOF; 2x2 px dots and a smaller aperture (K 3.8) keep the skyline lit.
- Feedback: (pending)

## Maze Race (`projects/maze_race`)
- Four explorers race one 20x20 maze with four-fold rotational symmetry (one quadrant grown from the goal door, rotated;
  symmetric links between quadrants and braids add loops) from the corners to a brass goal inlay, on a carved wooden
  board on a scholar's desk at night (green banker's lamp as the visible key, two candles, mantel clock ticking on the
  beat, moth, dust in the beam, moon in the window, each explorer's prop by its corner). FLOOD = BFS ink (strings),
  DIVER = random DFS with a red thread that winds back (bass), SEEKER = greedy best-first gold light (marimba), MARBLE =
  real circle-vs-wall physics under the board's tilt (drums: every wall hit is a hit). One fair budget: the paint
  explorers uncover the same cells per beat, each on its own 16th. Mood B heist chase, D minor, 121.97 BPM.
- Run (maze 12382, tilt 1094; two-stage `seed_search.py`, never edited): MARBLE leads early, FLOOD, DIVER, FLOOD the
  tortoise from 32 s, SEEKER's dead end next to the centre at 34.7 s, SEEKER wins 44.4, FLOOD 46.5, DIVER 49.9, marble
  lost. Captions are written from the run's events (lead changes, traps, the tortoise).
- Lessons (sim):
  - Under a fair cell budget breadth-first is the slowest: over 30k mazes SEEKER wins 74 %, DIVER 25 %, FLOOD 1 %. At 2
    cells per beat everyone finished in 7-25 s; 1 per beat (2 in the final stretch) and the hard-maze tail give a 44-46 s
    finish. Quadrants grown from a random cell gave short routes (~18 cells); growing from the goal door and a 20x20
    board gives 35-50.
  - A slowly sweeping tilt leaves a marble parked in pockets (median progress 0.15-0.3, ~1 wall hit/s); a new seeded
    direction on every beat gives ~3 hits/s (real drum fills) but it still never reached the centre in ~10k tilts over
    the top 4 mazes (best 0.64). Treat it as the wildcard; a marble that can win needs a different board (or bowl).
  - The search is cheap (paint race 1 ms, marble 0.15 s): rank mazes on the paint story first, then search tilts.
- Lessons (loop/encode):
  - `reelkit.loopcheck` passed on render.py but failed on the toolkit mp4 (3.4x seam, 7.8 dB audio step). Causes: frame 0
    is a keyframe and the last frame a P-frame, and x264's mbtree gives frame 0 a finer quantiser (on a dense dithered
    board that's 1.2-1.8 of seam diff); AAC pads its last 1024-sample frame with zeros (61.0 s = 2859.4 frames), so the
    decoded file ends in 13 ms of silence. `mux.py` re-encodes the kept raw with mbtree off and a keyframe on the last
    frame (1.6x), and the soundtrack is built as a loop exactly 2860 AAC frames long and muxed untrimmed (file 61.01 s).
    Worth moving into reelkit.video if other dithered videos hit it.
  - Captions and labels that fade in on frame 0 add a real seam jump (0.81 vs 0.29 normal): start the hook caption on
    frame 1 and ramp labels from zero; GO still lands on frame 0.
- Render ~0.45 s/frame/core (6.6 min on 2 cores); `mux.py` ~2.5 min.
- Feedback: (pending)

## Battle of the Bands: Genre Wars (`projects/botb_genre_wars`)
- BotB episode on #20's engine, imported unchanged (`episode.py` appends `battle_of_the_bands/` to `sys.path` so the
  episode's own `sim.py`, `scene.py`, ... win): ORCHESTRA = SAND, ROCK = BALL, EDM = RIPPLE, JAZZ = VINE, new names +
  colours only. One original song (A minor, | Am | F | Dm | E |, 118.03 BPM, 8-bar melody) arranged four ways
  (drums / bass / harmony / lead per genre: timpani + galloping cellos + violin pad/brass + horns; kit + pick bass +
  3-3-2 power chords + distorted lead; four-on-the-floor + off-beat sub + supersaw stabs + saw lead; swung ride/brush +
  walking upright + rootless comping (Am9 Fmaj9 Dm9 E7b9) + tenor sax). Every layer gated by its genre's moves (#20
  rule); a stinger in the new leader's style on each lead change; the stage mood (wash, ROCK pyro on each bar, EDM
  lasers stepping on the beat, JAZZ blue haze + spot cones, ORCHESTRA gold motes) follows the leader.
- Seed: the 61 s search (bell on bar 10 = 20.33 s, ring period = one beat, hopper sweep 61/9 s) fits less often than
  #20 (5 in 192); RIPPLE/EDM wins most runs (98 of 164 decided). Seed 189 picked over the top score (63) because three
  genres lead (the song changes genre three times) and its first knockout isn't early.
- Lessons (sound):
  - "The song follows the leader" needs per-layer curves: bass + harmony follow share gently (everyone audible), drums
    share^2.2 and the melody share^4 (the groove and the tune belong to the leader). Normalise gains equal-power
    (sum of squares = 1): the #20-style (1/sum)^0.55 went above 1 when one genre dominated, so the win dropped in level.
  - Calibrate layers on a phone-weighted band (150 Hz - 8 kHz): on raw RMS the EDM stem won with sub energy phones
    can't play. The opening's free-note voices need their own calibration (the jazz piano came in ~10 dB hot).
  - Gating density differs per genre (rings claim in bursts, the ball in streaks), so a leading ROCK sounded quieter
    than EDM. A per-genre up-only leveler (toward the level its share calls for, ~1.5 s smoothing) makes the loudest
    genre the leader again while stutters stay audible.
  - The tape-stop into the crack left the file's last 20 ms silent (-40 dBFS) against the boom on frame 0: a riser +
    snare roll that builds right up to the seam (6 ms tail, no fade) passes level and click.
  - mp4 audio seam: same AAC padding as #22 (640 samples). The container signals the audio as exactly 61.000 s
    (duration_ts 2928000); only ffmpeg's decoder appends the padding, which `loopcheck out.mp4` reads.
- Lessons (look): pyro cannons at x +-2.8 were off-frame (the ground stacks sit at the frame edges): +-1.95 on the
  front edge. A series-title line inside the scoreboard panel clipped the bottom of the wall; it became an impact
  caption right after the hook. Caption scheduler: event captions with priorities and minimum holds, lead changes
  wait for a more important caption, fillers fill fight stretches with a blank > 2.4 s or a hold > 5.2 s.
- Checks: safe zone OK (0.25 s steps); wall + props CoC 0 on all 1830 frames; flash 0 general flashes; opening -14.3
  dBFS, build +3.1 dB; loopcheck (render) PASS. Render 0.35 s/frame/core (10.8 min on 2 cores). Time: ~55 min to v1.
- Feedback: (pending)

## Which Instrument Wins? (`projects/which_instrument_wins`)
- Trend-format reel (#26, "Who Will Win? 16 entrants"), mood A dark tango: 16 glass balls, each holding a Pixel Pitch
  instrument model, drop into a crimson-and-gold ring turning over a pit in a sunset colosseum (stands collapsed on
  the sunset side so the sun sits low behind the ring, two braziers, podium torches, a crowd of pixel dots bobbing on
  the beat, pennants, pigeons, far ridges and an aqueduct). Fall out through the gap = out: the ball drifts toward the
  camera, drops into the pit on one last dull note and the 4 x 4 roster dims it with its place. Seed 406 (2 fits in
  600 honest runs): DRUMS out first at 3.6 s after rattling in the gap, BAGPIPES stuck in the gap 6.5-8.9 s, PIANO and
  TRUMPET out together at 15.4/15.7, UKULELE 9th after three near-misses, VIOLIN survives five edge hits, final 3 at
  40.3, VIOLIN vs CELLO from 43.1 with 7 close calls, CELLO wins at 49.41. The ring seals on the win, the cello floats
  out to the spotlight and plays an original 4-bar solo in its own voice while the stands light up; at 58.4 s the 15
  lift out of the pit (each plays one note of a rising cascade) and everyone eases into formation, at rest on 61.0 =
  frame 0 (they drop again).
- Music: every wall bounce plays the ball's own Pixel Pitch voice (pitch = chord tone by where it hit, left low ->
  right high, in its register; loudness = impact speed; 32nd grid), gap-edge hits ring a bell (rattles), glass clinks
  once fewer than 12 are left; bed = D-minor tango at 118.03 BPM (habanera bass, staccato strings in quarters then
  eighths from bar 10, dominant pedal + tremolo + roll for the duel, the win resolves it with a tonic hit).
- New: `reelkit.ui.roster_grid` (4 x 4 elimination scoreboard; columns sized to their longest name, places in the chip slot).
- Lessons:
  - Pixel Pitch models reuse cleanly read-only: import the video modules, run pp_framework.setup() for their events,
    call build()/synth() with a fake scene. Their display stands must go (runtime patches during the build: the stand
    helpers become no-ops, floor-anchored rods and floor discs are skipped); with stands the instrument filled a third
    of the ball. Thin instruments read only with a fixed best view: per-instrument yaw centre, +-swing and a screen
    roll (violin, guitar, flute tilted ~30 deg), never a full spin (a spinning cello read as a stick). A light glass
    interior is what makes black models (piano, clarinet, accordion) readable.
  - Ring pacing: with gravity, a gap barely wider than a ball still drains piles fast (a ball wedged between the two end
    caps is squeezed outward). What paces it is how briefly the gap sits under the pile: 20 turns in 61 s (2.06 rad/s,
    also loop-safe), gap 0.303 -> 0.48 rad, wall restitution 1.02 with a speed cap. Rules were tuned once on median
    elimination times over 10 seeds (40 random rule sets, `check/rulesearch.py`), then frozen for the seed search.
  - Duels with only two balls rarely end on their own (exits need the pile pushing): most runs had 10-17 s duels; the
    window "3rd-last out 41.5-46.5 s, win 48-50.6 s" kept only ~0.3 % of runs.
  - A white-hot sun under top_scrim() turns grey: draw the disc after the grade on sky pixels only (banded rings).
    A low visible sun means grazing light: shadows of the ring and balls fall behind the camera, so the floor drama
    comes from the stands' long shadows; the stands and the podium must be broken low on the sun's side or the arena
    sits in shade.
  - The ball names hidden during the scripted fly-back popped back on frame 0 (seam 3.0x); showing them from 60.25 s
    (balls nearly home) -> 1.5x.
  - Loop encode: the toolkit mux trims the soundtrack to 61.0 s, AAC pads the last frame with silence and
    `loopcheck out.mp4` failed (7.7 dB step) although the render check passed. Building the audio as 2860 AAC frames
    and re-muxing through projects/maze_race/mux.py (imported, not copied) -> PASS (seam 1.8x, 2.5 dB). Two projects
    now need it: worth moving into reelkit.video.
- Checks: safe zone OK; subject CoC 0 on all 1830 frames; flash 0 general flashes; bounces 7-15 dB over the bed in
  400-4000 Hz; opening -18.7 dBFS, build +4.1 dB. Render ~0.26 s/frame/core (7.8 min on 2 cores) + 3 min mux.
  Time: ~3.5 h, mostly the ring-pacing rules and the colosseum look.
- Feedback (v1): no "ITS STORY: / ON OUR PAGE" at the end; move the roster to the loosened safe zone.
- v2: the cross-promo caption is gone ("WHO DID YOU / PICK?" holds to 58.25 s); roster_grid called with its new
  defaults (bottom of the zone, left of the button column); ball names stay above the roster and out of the button
  column (safe_overshoot); ring recentred on x 180 so it sits under the frame-centred captions. Full re-render + mux.
- Feedback (v2): the VS title card intro became a series rule (user, Oct 24, after #30's test).
- v3: bar 0 = the shared VS title card (`projects/ballfight/title.py`; #35's session moved it there minutes before this one
  would have; this episode added TROMBONE slide-tube + HORN coil panel patterns, #30's card checked pixel-identical on 12
  frames), 3-2-1 on bar 1, FIGHT! at 3.94 s, top caption dropped, HUD hidden while the card is shut; card audio: a unison
  band hit per word + slide blast / thunder crack / two-horn hunting call, taiko pickup as the doors open; chords moved a
  bar (Dm under the card, A for the count-in, loop from FIGHT!, A into the seam). Honest search at 2 bars: v2's 894
  didn't reach a K.O. (the drift the rule warns about); new seed 2200 (8 fits in 2,200, 51 / 49): TROMBONE wins at 50.72 s
  with 4.0 HP after three lead changes in 1.5 s and a 3.7 s standoff at 4.0 vs 1.9 HP.
- v3 lessons: check origin right before moving a shared file (another session had just moved title.py; adopt theirs,
  add only what's missing). With the rules unchanged the median K.O. moves ~2 s later, so fewer runs land in 49-53 s
  (8 fits took 2,200 runs vs 974 at 1 bar); the solo now stops at the reassembly if a late K.O. leaves less room. The
  title slam makes a very clean seam (0.5x render, 1.2x mp4). Render 11.1 min.
- Feedback (v3): the trombone model is inaccurate (user, Oct 25).
- v4: the Pixel Pitch trombone rebuilt from reference photos found in Chrome (Yamaha YSL-891Z side view, measured from
  the image's pixels at ~0.41 mm/px using its 20 cm bell; Jupiter student tenor 3/4 view): the old slide plane was
  turned ~100 deg so the slide read as one rod, the bell sat 65 % of the way along a short slide (real: 40 %), and an
  F-attachment ellipse hung below the instrument. Now: slide ~40 deg out of the bell plane about the shared lower line,
  bell rim 31 cm / crook 78 cm ahead of the slide lock, bow 38 cm behind, Bessel bell fitted to the photo, nickel crook
  + bumper + stockings, tuning-slide braces + medallion, bell brace at the lock, a 4 cm mouthpiece, plain straight
  tenor. Every series scene still builds; the posted Pixel Pitch video is unchanged. Same fight + soundtrack; one full
  render (the trombone is on screen in every frame, so no patch render).
- Lessons (v4): read a reference photo's pixels in the page (canvas getImageData on the upload.wikimedia.org image
  tab) for real measurements instead of eyeballing a screenshot; check an instrument model in a plain orthographic
  side view next to the product photo before judging it at sprite size.
- Feedback (v4): (pending)

## BALLFIGHT #1: Guitar vs Piano (`projects/ballfight`)
- New series (#27), inspired by math.vsmath ability-ball fights: two glass balls with Pixel Pitch instruments inside
  fight in a boxing ring (turned 45 deg so the red corner is screen-left and the blue corner screen-right, matching the
  HUD) in a dark arena: hard spot overhead (can at the top of frame + a soft beam), red / blue corner-post lights, an
  LED barrier chasing on the beat, ten tiers of crowd silhouettes with rim light, phones and flashbulbs on big hits.
  GUITAR swings its neck (5 -> 24 frets, +1 per landed hit; parries keys), PIANO fires keys on every second beat
  (volley grows half a key per landed volley). Abilities: POWER CHORD (shockwave) / SHRED (spin x2), SUSTAIN (shield,
  a blocked neck bounces and reverses) / GLISSANDO (15-key arc), desperation FEEDBACK (wave) / GRAND SLAM (the lid
  slams on the predicted spot after 2 beats).
- Honest run, seed 88 (8 fits in 336 runs; balance GUITAR 44 % / PIANO 56 % over 313 decided): Guitar leads from 5 s,
  the maxed piano volleys catch up, lead changes at 31.6 / 36.0 / 44.4 / 48.0 s, GRAND SLAM lands at 44.4 and FEEDBACK
  fires 0.9 s later, PIANO wins at 51.3 s with 2.6 HP. Captions (first uses, >= 4 s apart): POWER CHORD 10.2, SUSTAIN
  15.3, SHRED 20.3, GLISSANDO 27.3, GRAND SLAM 43.4 (FEEDBACK named in the HUD status line).
- Reusable: `fight.py` (engine) + `fighters.py` (one table per fighter: weapon kind, scaling, 3 abilities, look, voice)
  + `reelkit.ui.fight_hud` (two fighting-game HP bars draining from the middle, damage trail, LV pips, status line).
- Rules lessons: a volley growing one key per landed volley maxed out by ~25 s and won every run by ~14 s; half a key
  per landed volley + one volley every 2 beats balanced it. Balance is touchy on blade damage (5 -> 7 swung guitar wins
  17 % -> 67 %): tune on a small grid (`tune.py`, 32-40 seeds per cell, ~1 min). Levels from damage dealt at
  14 / 36 put the unlocks near the beat sheet's 10-30 s targets. Fit rate ~2-3 % with the hard windows, so an
  early-stopping search finds 5+ fits in about a minute.
- Loop lessons: beat pulses (LED boards, corner glows, crowd hops) snap on at frame 0 because frame 0 is a downbeat;
  `timeline.beat_pulse` ramps each pulse up over the last 70 ms before the beat (seam 5.9x -> 2.2x together with an
  ease-out fly-in for the "3"). Audio: a riser + roll that build through the seam (12 ms past it, no fade) and the
  frame-0 count hit 4 ms in -> 4.2 dB step, no click (was 7.8 dB + click).
- Sound: section automation after the compressor (build +3.4 dB); every hit snapped to the nearest 16th in the
  attacker's own voice; a fighter's part high-passes and saturates as its HP drops, the leader gets an octave double.

## BALLFIGHT #2: Drums vs Violin (`projects/ballfight_drums_vs_violin`)
- Second BALLFIGHT episode (#28) on #27's engine, ring, HUD and count-in (imported; new folder with its own timeline,
  sim, tune, seed search, renderer copy and audio). DRUMS pulse a drumhead ring out on every beat (new weapon kind
  `pulse`: reach grows per landed pulse; haste rolls it on the 16ths), VIOLIN spins its bow (blade + new `spin_grow`:
  2.4 -> 14 rad/s). Abilities: DRUM ROLL / CRASH CYMBAL (the bow bounces off) / DOUBLE KICK (beater taps an 8th early,
  then stomps); PIZZICATO / TREMOLO / SCREECH. Mood B dark Balkan fire: D harmonic minor, 141.64 BPM (36 bars).
- Honest run, seed 568 (6 fits in 932; DRUMS 57 % / VIOLIN 43 % over 767 decided): Drums edge the opening, the bow
  takes over at 18.2 s (62-26 at ~36 s), DOUBLE KICK at 37.7 starts the comeback, SCREECH at 43.6, DRUMS win on the
  last hit at 50.91 s with 2.8 HP (the second lead change is the K.O. itself).
- Engine lessons: keep additions opt-in so #27 replays bit-identically (hash every recorded array + the event list
  before and after: seed 88 + four others). A new 16th-grid loop inside `run()` first shadowed the loop's `step`
  counter (events out of order, K.O. at 4 s) - name new locals carefully in shared loops.
- Rules lessons: both weapons grow on landed hits, so leads snowball; lead changes stay rare (~0.5 per run, #27 ~0.7)
  and smaller growth didn't help. With the hard windows the fit rate is ~0.6 %, still fine for an early-stopping
  search (~1 min for 932 runs). `tune.py` here takes generic overrides (`D.dmg=..`, `V.spin_grow=..`, `scale=..`).
- Sound lessons: the bow's quarter turns sampling a 64-slot melody give the "slow strokes -> frantic spiccato" arc for
  free (1 note/s at the start, a full 16th line at 14 rad/s). First balance had the violin ~24 dB over the bed
  (accordion inaudible); target the fighters ~8-13 dB over the bed in 300-5000 Hz. One transient set the master's
  normalisation (early fight at -21 dBFS): normalise on the 99.97th percentile and let the soft clipper take the rest.
- Mid-production direction: the ideas chat gave every BALLFIGHT instrument its own colour (DRUMS orange, VIOLIN
  violet). Applied by patching the imported scene at runtime (corner cushions, middle rope per side, neutral skirt band,
  corner lights, LED sides, crowd tints) so the shared scene.py is untouched; full re-render (7 min) + mux.
- Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames; loopcheck PASS render + mp4 (seam 1.8x);
  flash 0 general; opening -17.8 dBFS, build +3.2 dB. Render 0.23 s/frame/core (7 min) + 3 min mux.
- Feedback: (pending)

## BALLFIGHT #6: Tuba vs Flute (`projects/ballfight_tuba_vs_flute`)
- Sixth BALLFIGHT episode (#32), produced in parallel with #29-#35 on the shared engine. TUBA (indigo, left) blasts a
  slow, huge brass wave every bar (#28's `pulse`, every=4) and pulls the flute in all the time (new weapon option
  `pull`: gravity that only steers, speed stays shared; +0.08 per landed wave); FLUTE (silver, right) fires tiny fast
  darts every beat (volley with the new option `on16`, so FLUTTER TONGUE's haste can double it). Abilities: OOMPAH /
  BRASS WALL / SUBSONIC (a giant tuba bell hovers, then slams); FLUTTER TONGUE / TRILL / PICCOLO. Villain march: C
  minor, 125.90 BPM (32 bars), i-VI-N6-V with the B natural over Db as the villain's augmented-sixth snarl.
- Honest run, seed 1097 (6 fits in 1099; TUBA 49 % / FLUTE 51 % over 1057 decided): neck and neck, the flute's chip
  damage leads until 44.9 s, flute back ahead at 48.6, both desperation moves on the same beat (48.1), the bell lands
  at 49.1, TUBA wins at 51.54 s with 4.5 HP. Captions: FLUTTER TONGUE 14.3, OOMPAH 19.5, TRILL 24.8, BRASS WALL 29.5,
  SUBSONIC 48.1 (PICCOLO in the status line, same beat).
- Engine (shared, additive, two small commits): weapon option `pull` (+ `pull_grow` / `pull_max`, grows with a landed
  main pulse) and volley option `on16`. #27 (seed 88) and #28 (seed 568) replay identically
  (`tools/ballfight_replay_hash.py`), re-checked after every rebase onto the other sessions' engine commits (beam,
  orbit, rewind, aura, bellows). Roster commit conflicted with #30's (both appended at the dict's end): keep both.
- Rules lessons: without the pull the tuba landed ~5-8 waves a fight and never won (a once-a-bar weapon needs reach
  or help getting there); reach 1.8 + pull 1.2 balanced it in one grid (`tune.py`, 20 seeds per cell).
- Look lessons: the flute's Pixel Pitch model is a 1 px line at ball size; a pixel-art flute (silver tube, lip plate,
  keys, ink outline) in `styles.flute_sprite` reads, and a light (silver) fighter needs a cooler, darker glass tint.
- Sound: the tuba fighter IS the march's bass (main waves = root "oom", OOMPAH's beat-3 waves = the fifth, landed waves
  = a walk-up), the flute's volleys are runs up the chord (n darts = n notes); bed kick turned down and the pad's low
  note dropped so the tuba sits +7-9 dB over the bed in 40-260 Hz; flute +7-13 dB in 0.9-6 kHz.
- Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames; loopcheck PASS render + mp4 (seam 2.2x);
  flash 0 general (max 6.6 % at the bell slam); opening -18.9 dBFS, build +3.1 dB. Render 0.19 s/frame/core (6 min) + mux.
- Feedback: (pending)
- v2 (BALLFIGHT v2 redesign, user Oct 24: attacks the instrument's body / playing, more visual character, no
  full-screen flashes). TUBA: OOM-PAH SWALLOW (new kind `swallow`: oom = the bell inhales for a beat and steers the
  enemy in, touching it = swallowed and held inside, pah = spat out like a cannonball, a rope hit within 0.9 s = extra
  damage; the reach grows per landed swallow); BRASS WALL = a brass dome, SUBSONIC = the floor ripples like water.
  FLUTE: SNAKE CHARMER (new kind `striker`: a basket every bar, up to 2, a cobra per basket striking on the 16th grid;
  new ability kind `dance`: SNAKE DANCE sends the cobras slithering after the enemy for a bar); PICCOLO adds glass-crack
  marks. Engine commits additive and alone; the swallowed ball is skipped in `move()` via a `held` attribute.
- Parallel production: the shared fight.py took 10+ other sessions' commits during this work; every rebase conflicted
  in the same elif chains (setup / beat / step / trigger). Resolving by hand (and moving #32's setup into its own loop)
  and re-checking all 45 guarded runs against upstream origin/main before each push kept #27, #28 and #29-#35
  identical. A helper (rebase, keep-both on additive conflicts, verify vs a worktree of origin/main, push, retry on a
  push race) saved several rounds.
- Rules lessons: the cobra bite is the touchiest number (1.05 -> 1.25 moved the tuba 53 % -> 17 %); 32-48 seeds per cell
  misled once (21 % in the search), 90-120 seeds got it right. The first v2 opening was slow (first hit 5-6 s, 5 s
  stretches without a hit): a basket every bar, a longer starting reach and a hard "no 4 s without a hit" window fixed
  the pacing. Final: TUBA 53 % / FLUTE 47 % over 515 decided, damage scale 1.06.
- Honest run, seed 514: tuba leads to 33 s (four swallows by 26 s), flute takes over, tuba back ahead at 47.6 s,
  FLUTE wins at 49.37 s on a double cobra strike with 5.7 HP.
- Look lessons: thin cobras vanish on the lavender canvas; an ink outline, a 0.16-unit body and a hood turned partly
  toward the camera made them read at ball scale. Additive canvas light doesn't show on a bright canvas: the suction
  cone reads as darker wedge + bright dashes racing in. `reelkit.fx` wired per projects/ballfight/README.md (flash
  probe on the busiest stretch peaked at 15.8 % of the frame).
- Loop lesson: less motion than v1 lowered loopcheck's "normal step", so the same beat-pulse snap at frame 0 failed
  (2.6x); a longer pre-ramp on `beat_pulse` (120 ms) brought the seam to 1.5x.
- Sound: oom = the tuba's root held through the inhale (+ air sucked in), pah = the fifth as a blast (+ brass blast and
  whoosh on a spit), gulp = a low bubble pop; the flute plays an original charmer line in C harmonic minor (augmented
  2nd, grace notes, phrygian-dominant turn over G) while its cobras are up, flutter-tongued in FLUTTER TONGUE, an
  octave up in SNAKE DANCE, with a hiss-click per strike; the music box went to quarter notes so the charmer line leads.
- v2 checks: safe zone OK; subject CoC 0 on all 1830 frames; loopcheck PASS render (1.5x) + mp4 (1.9x); flash 0
  general (max 15.6 %); opening -17.9 dBFS, build +3.7 dB. Full render 313 s on 2 cores + mux. Feedback: (pending)
- v3 (VS title card intro, series rule Oct 24): bar 0 = the shared title card (projects/ballfight/title.py, already moved
  there by #35; this episode added opt-in CLOSEUP / GLASS registries so the flute's close-up is the pixel-art flute in cool
  glass; #30's card checked pixel-identical on 13 frames). Panels: indigo bell-mouth rings / steel-silver key cups (a
  near-white silver panel read as white: scaled to ~0.7). 3-2-1 on bar 1, FIGHT! at 3.81 s, top caption + series tag
  code dropped, HUD hidden while the card is shut; chords Cm (card) . G (count-in) . the march loop . Ab . G into the
  slam; the charmer line and music box re-anchored to bar 2.
- Seed search re-run at 2 bars (rules unchanged): 6 fits in 753, TUBA 53 % / FLUTE 47 %. Seed 514 ranked first again;
  unlike #30's 1442, its run at 2 bars is bit-identical to v2's shifted by a bar (no float drift for this roster), so
  the fight is v2's (FLUTE 51.28 s, 5.7 HP).
- Loop: seam 0.4x render / 1.1x mp4 (the title slam as the reset beats v2's beat-pulse snap), audio step 2.0 dB; flash
  check PASS with the single doors pair (1.9 s), max 51.9 % of the frame changing as the doors shut (one transition).
  Full render 5.5 min + mux. Feedback: (pending)


## BALLFIGHT #4: Bagpipes vs Accordion (`projects/ballfight_bagpipes_vs_accordion`)
- Fourth BALLFIGHT episode (#30), produced in parallel with #29–#35 on the shared engine. Two new weapon kinds, both on
  their own 16th counter (additive: `grid16_fire`, `aura_tick`, `fire_bellows`, `suction_step`): BAGPIPES' `aura` (a
  drone ring that hurts on every 8th while the opponent is inside; +reach per contact) and ACCORDION's `bellows` (a
  stroke every 2 beats alternating a push ring and a collapsing suction ring that yanks the opponent in). Abilities: WAR
  CRY / DOUBLE DRONE (ticks on the 16ths) / BAG BURST; POLKA (a stroke every beat) / SQUEEZEBOX / FULL BELLOWS (a giant
  accordion with its keyboard on top, squeezed shut on the predicted spot). Mood: dark sea shanty, A dorian, 121.97 BPM.
- Honest run, seed 1581 (6 fits in 1854; BAGPIPES 51 % / ACCORDION 49 % over 1637 decided): the accordion leads from
  16.9 s, the bagpipes retake it at 28.8 s, FULL BELLOWS misses at 39.9 s, BAG BURST at 44.3, 8-7 at 48 s, BAGPIPES
  win on the last tick at 50.55 s with 2.1 HP.
- Parallel-production lessons: added `tools/ballfight_replay_hash.py` (hash every finished episode's honest runs before
  and after an engine change; other sessions picked it up). Engine commits rebased onto five other sessions' engine
  commits; every conflict was "keep both" (new kinds side by side in `__init__`, `run()`'s grid blocks and step calls).
  Give each new grid weapon its own counter instead of widening the shared pulse condition: no existing line changes.
- Rules lessons: a small aura (0.35 beyond the ball) almost never bit; base 0.9, growth per contact (not per tick) and a
  1.9 cap keep it from snowballing. The brief's "push one bar, suck the next" left the accordion idle ~2 s at a time;
  every 2 beats reads as breathing (flagged to the user with the early sheet). A longer suction (pull_k 1.7) raised lead
  changes 0.5 -> 0.8 a run, because every suck can drag the accordion into the drone. Captions are the tight window
  (first uses cluster): ~27 % of runs have 5 captions >= 4 s apart.
- Look lessons: the aura as added canvas light read as a pale white ring; painting it as a tartan band (albedo blend, lit
  like the canvas) reads as the bagpipes at a glance. A round stacked-disc "bellows" read as a cake; a square pleated box
  with the keyboard on top reads as an accordion instantly.
- Sound lessons: the drone is synthesised at frequencies nudged (< 0.01 Hz) to whole cycles in the file, so it loops
  sample-perfectly, and its level is automated: under the mix, swelling only while the aura lands (a leaky integrator of
  the landed ticks), back to the opening level before the seam. Chanter via the Highland module's own `chanter()` (the
  bridge's `voice()` adds the whole drone to every note); accordion chords via `reed()` with the 'golden' tone (the
  showcase 'morino' scene adds a dotted-8th echo).
- Feedback: (pending)

## BALLFIGHT #8: Trombone vs French Horn (`projects/ballfight_trombone_vs_horn`)
- Episode #34 on #27's engine, ring, HUD and count-in (imported; renderer copied from #28). TROMBONE (left, mint) aims its
  slide and shoots it out like a lance on every beat (new blade `mode='thrust'`: reach 0.95 -> 2.5, +0.08 per landed hit,
  swats arrows it passes through); HORN (right, copper) fires curving hunting-call arrows every 2 beats (#29's `curve` +
  new `curve_grow`: 52 -> 137 deg/s; +half an arrow per landed volley). Abilities: SAD TROMBONE (shockwave + new `slow`:
  0.6x for 1 s) / POWER SLIDE (thrusts on 8ths) / PEDAL TONE (a giant flat-shaded brass bell drops in and stomps);
  STOPPED HORN (copper bell-rim shield) / HORN RIP (arc of 7 curving arrows, new arc `curve=True`) / TALLY-HO (chevron
  wave). Mood: cinematic brass trailer, D minor at 121.97 BPM; the bed has no brass (strings, braams, taiko).
- Honest run, seed 1261 (8 fits in 2,848; TROMBONE 51 % / HORN 49 % over 2,776 decided): lance leads early, arrows drag
  it level by 32 s, 4 lead changes, PEDAL TONE lands 45.3, TROMBONE wins at 50.21 s with 4.8 HP.
- Parallel-production lessons: six other sessions pushed engine additions while this one was tuning; two rebases with
  conflicts (keep both sides, then verify with tools/ballfight_replay_hash.py against the upstream file before pushing).
  #29 had already added a `curve` volley (homing with a lifetime), so this episode dropped its own homing arrays and only
  added growth + curving arcs on top: check the latest engine for an existing option before adding a parallel one. New
  recorded arrays (EXT, SLW, CRV) only exist when a fighter uses the options, so the shared hash guard stays identical.
- Rules lessons: homing vs a lance is touchy both ways (strong homing won every run by ~20 s; slow arrows + a parrying
  lance lost every run); the lance's parry is the counterplay that balances them. Desperation moves are damage-scaled too:
  at dmg_scale 0.47 they needed raw 24 / 20 to swing anything. 80 seeds read 49 % but 812 read 61 %: check balance on a
  few hundred seeds (tune.py now uses seeds 1.. like the search; it runs at scale 1.0 unless `scale=` is passed).
- Look lessons: the giant bell as stacked translucent discs read as a mushroom; a flat-shaded surface of revolution
  (painter-sorted quads) reads as a bell, but brass under the spot needs a base ~0.5x BRASS or bloom turns it white.
- Loop lesson: this fight changes little per frame (median step 1.15-1.4), so the downbeat on frame 0 (corner lights,
  LEDs, ropes) stood out at 2.6x; a 120 ms pre-beat ramp in timeline.beat_pulse (was 70 ms) -> 1.5x (render), 1.9x (mp4).
- Sound: thrusts = the ostinato (glissando into each stab via a time-warp), landed lances = accents with a 5-semitone
  glide, volleys = the horn's call (horn fifths when leading); fighters 8-13 dB over the bed after giving the trombone
  bus 1.5x (its voice is quieter in 300-5000 Hz than the horn's).
- Checks: safe zone OK; subject CoC 0 on all 1830 frames; loopcheck PASS render + mp4; flash 0 general (max 8 %);
  opening -19.0 dBFS, build +3.8 dB. Render 0.27 s/frame/core (4.1 min) + mux; ~8.5 min total.
- Feedback (v1): the BALLFIGHT v2 redesign (user, Oct 24): attacks more unique, more visual character; the trombone's
  lance is kept, the horn gets THE HUNT.
- v2: HORN = THE HUNT (engine `pack` + ability `charge`): a horn call every 2 beats brands the trombone (copper target
  mark) and releases a ghost hound (pack 2 -> 5); hounds flank, bite on the 8ths, heel, guard (STOPPED HORN), sprint
  (HORN RIP), charge in one line (TALLY-HO). Re-tuned (damage scale 0.60, bite 2.8; 50 / 50 over 873 decided runs), new
  seed 894: the pack leads 83-58 at 26 s, the lance levels by 35 s, TALLY-HO 42.8, PEDAL TONE answers, TROMBONE wins
  50.72 s with 4.0 HP. Effects from reelkit.fx (brass sparks, copper embers, lights, bursts, marks, shimmer, moments).
- v2 lessons: hounds spawned toward the enemy ran straight into the lance's line (50 knocked out a fight, pack never
  grew); fanning them out of the bell and flanking (approach from 110 deg off the lance line) fixed it (~2.5 hounds
  alive). Engine pushes raced six other sessions: a script (rebase with --autostash, keep-both on adjacent insertions,
  replay guard vs the upstream files, push, retry) - a plain `git pull --rebase` silently fails on an unstaged file and
  every push then looks like a race. Hounds as top-down silhouettes read as rats; 2.5D side-profile sprites (long and
  low, drop ears, sabre tail, gallop) read as hounds. The new attack lights blew PEDAL TONE's brass bell out again:
  base 0.42x BRASS and the stomp's flare at canvas height. The flash test needs `rel_lum` on 0..255 frames.
- Checks (v2): safe zone OK; CoC 0 (hounds included); loopcheck PASS render 1.5x / mp4 1.8x; flash 0 general (max
  15.5 %); opening -19.3 dBFS, build +4.3 dB. Render 10.8 min on 2 cores (fx ~2.6x v1).
- Feedback (v2): the VS title card intro became a series rule (user, Oct 24, after #30's test).
- v3: bar 0 = the shared VS title card (`projects/ballfight/title.py`; #35's session moved it there minutes before this one
  would have; this episode added TROMBONE slide-tube + HORN coil panel patterns, #30's card checked pixel-identical on 12
  frames), 3-2-1 on bar 1, FIGHT! at 3.94 s, top caption dropped, HUD hidden while the card is shut; card audio: a unison
  band hit per word + slide blast / thunder crack / two-horn hunting call, taiko pickup as the doors open; chords moved a
  bar (Dm under the card, A for the count-in, loop from FIGHT!, A into the seam). Honest search at 2 bars: v2's 894
  didn't reach a K.O. (the drift the rule warns about); new seed 2200 (8 fits in 2,200, 51 / 49): TROMBONE wins at 50.72 s
  with 4.0 HP after three lead changes in 1.5 s and a 3.7 s standoff at 4.0 vs 1.9 HP.
- v3 lessons: check origin right before moving a shared file (another session had just moved title.py; adopt theirs,
  add only what's missing). With the rules unchanged the median K.O. moves ~2 s later, so fewer runs land in 49-53 s
  (8 fits took 2,200 runs vs 974 at 1 bar); the solo now stops at the reassembly if a late K.O. leaves less room. The
  title slam makes a very clean seam (0.5x render, 1.2x mp4). Render 11.1 min.
- Feedback (v3): the trombone model is inaccurate (user, Oct 25).
- v4: the Pixel Pitch trombone rebuilt from reference photos found in Chrome (Yamaha YSL-891Z side view, measured from
  the image's pixels at ~0.41 mm/px using its 20 cm bell; Jupiter student tenor 3/4 view): the old slide plane was
  turned ~100 deg so the slide read as one rod, the bell sat 65 % of the way along a short slide (real: 40 %), and an
  F-attachment ellipse hung below the instrument. Now: slide ~40 deg out of the bell plane about the shared lower line,
  bell rim 31 cm / crook 78 cm ahead of the slide lock, bow 38 cm behind, Bessel bell fitted to the photo, nickel crook
  + bumper + stockings, tuning-slide braces + medallion, bell brace at the lock, a 4 cm mouthpiece, plain straight
  tenor. Every series scene still builds; the posted Pixel Pitch video is unchanged. Same fight + soundtrack; one full
  render (the trombone is on screen in every frame, so no patch render).
- Lessons (v4): read a reference photo's pixels in the page (canvas getImageData on the upload.wikimedia.org image
  tab) for real measurements instead of eyeballing a screenshot; check an instrument model in a plain orthographic
  side view next to the product photo before judging it at sprite size.
- Feedback (v4): the user replaced the HORN's attacks (Oct 25): "HORN v3" = ECHO CALL + FINAL FLICK, plus the HUD
  ability list (series rule).
- v5: new opt-in engine kinds, each in its own commit (replay guard identical for the other 9 episodes; THE HUNT's
  `pack` stays): weapon `echo` (a blast ring on every beat; the ball's path is kept every substep and up to 4 ghosts sit
  j beats back on it, each repeating the call j beats later, so the repeat lands where the call was sounded: a delay in
  space; +1 ghost per landed blast) and ability `flick` (a glove winds up a beat tracking the enemy, flicks out up to 2.4
  beyond the surface, flings it straight across the ring; a rope hit within 0.9 s adds damage, like the tuba's spit).
  HORN roster v3 in its own commit; TROMBONE unchanged. Re-tuned once (damage scale 0.72, echo 4.3, start with no
  ghosts: 400 seeds 49 / 51 % decided); honest search at 2 bars: seed 140 (8 fits in 719): TROMBONE wins at 49.22 s with
  6.8 HP after FINAL FLICK flings it into the ropes at 47.2 s and PEDAL TONE answers at 48.7 s.
- Lessons (v5): starting with a ghost (n0 = 1) let the echoes front-load damage (the horn led at 15 s in ~86 % of
  runs); starting with none evened the opening (the trombone led at 15 s in 20-40 %) and gave more lead changes (1.0-1.4
  per run vs 0.3-1.1). A ring that hits "when its front crosses the enemy" misses an enemy already inside a ghost's starting radius, so
  the echo kind hits when the ring reaches the enemy's near surface instead. The cartoon glove reads best flat on the
  balls' plane (back of the hand up, three stitch lines, ink outline) on a zig-zag copper spring; at 1x it looked
  small next to the ball, so it is 1.45x (~1.9 units cuff to fingertip). Render 4.4 min on 2 cores (the effects are cheaper than v4's hounds).
- Checks (v5): safe zone OK; CoC 0 (ghosts + glove included); loopcheck PASS render 0.5x / mp4 1.2x (audio 2.1 dB, no
  click); flashcheck PASS (1 = the doors opening; FINAL FLICK's impact changes <= 10 % of the frame); fighters 9-15 dB
  over the bed in 600-2500 Hz.
- Feedback (v5): (pending)

## BALLFIGHT #5: Turntables vs Synth (`projects/ballfight_turntables_vs_synth`)
- #31, produced in parallel with #29-#35 on the shared engine. TURNTABLES throw vinyl frisbees that ricochet off the
  ropes (new volley option `bounce`, up to 2; `on16` so SCRATCH's haste x4 fires them on the 8ths); SYNTH is an
  arpeggiator: note gems orbit the ball and hit what they touch (new weapon kind `orbit`, 2 -> 8 satellites on fixed
  seats, they also swat records). REWIND is a new ability kind: the ball keeps its own 2 s record, HP goes back to its
  value 2 s ago (`heal` event) while it zips back along that path in one beat, intangible. Mood: dark breakbeat,
  F minor, 137.70 BPM (35 bars).
- Honest run, seed 129 (5 fits in 666; TURNTABLES 47 % / SYNTH 53 % over 631 decided): satellites edge the opening, the
  records take over at 21.2 s, two REWINDs (7.2 / 8.8 HP undone), 8 satellites take the decks from 51 to 4 HP in four
  seconds, WHITE NOISE and BASS DROP both fire, TURNTABLES win at 49.72 s with 1.5 HP. All six abilities get captions.
- Parallel-production lessons: seven sessions edited fight.py / fighters.py at once; every engine commit hit a merge
  conflict in the same spots (new methods before the abilities section, the roster's closing brace). Both sides were
  pure insertions, so keeping both and re-running tools/ballfight_replay_hash.py before the push was enough. Reusing
  recorded fields (`orbit`: TH = angle, NV = satellites, BL = radius) kept the recorder and every renderer untouched.
- Bridge: appending to pp_bridge.ENTRANTS changes #26's 16-entrant roster (its run, roster grid and sprites iterate
  NAMES); later instruments now go in EXTRA (entrant() searches both; ACOUSTIC moved there). The organ / synth /
  turntables modules cache notes by event index, so voice() clears that cache (otherwise every note = the first one).
- Rules lessons: the first orbit (0.4 beyond the surface) barely reached the decks (12 hits vs 200 records); at 0.9 it
  balances. +1 satellite per landed hit snowballed, so +1 per two. Lead changes ~0.6 per run, fit rate ~0.75 %.
- Look lessons: flat instruments need tipping toward the 3/4 camera; the episode renders sprites with an extra pitch
  (deck 45 deg, synth 28) and lifts the black deck (+30) so it reads through the lime glass. Satellites at 0.15 were
  specks; at 0.24 with a facet, a core and an orbit line they read.
- Sound lessons: the satellites crossing the top of the orbit = the arpeggio gives a figure that densifies by itself
  (1 note/s with 2 satellites, ~8/s with 8 under LFO). Scratches use the Pixel Pitch deck's own motion curves on a
  vocal-free F-minor stab (the deck's own 'fresh' source imitates a voice). The synth's voice has a built-in echo at
  its own tempo: notes are cut at 0.34 s before it.
- Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames; loopcheck PASS render + mp4 (seam 2.4x);
  flash 0 general (max 8.6 %); opening -16.4 dBFS, build +3.4 dB; leads 2-12 dB over the bed in 400-4000 Hz. Render
  9.9 min + mux.

## BALLFIGHT #3: Trumpet vs Sax (`projects/ballfight_trumpet_vs_sax`)
- Third BALLFIGHT episode (#29), produced in parallel with #30-#35 on the shared engine. TRUMPET (yellow) fights with a
  new weapon kind `beam`: the bell charges for a beat with the aim locked on the sax's predicted spot (`lead` 0.7, a
  dashed sight on screen), then a straight blast that hits once, pierces smoke rings, and grows longer + wider per landed
  blast; the trumpet sprite rolls inside its ball to point along the aim (side-on, valves up). SAX (teal) blows smoke
  rings (volley + new `curve` option: homing at 110 deg/s, 2.6 s life, own hit radius). Abilities: FANFARE / DOUBLE
  TONGUE / HIGH C; SMOKE SCREEN / BEBOP RUN / HONK (a giant sax bell hovers over the predicted spot, then blasts).
- Honest run, seed 337 (18 fits in 899; TRUMPET 43 % / SAX 57 % over 893 decided): Trumpet up 16 at 15 s, the rings
  snowball back, lead changes at 40.6 / 44.4 / 45.9 / 49.3 s, HIGH C 42.6 + HONK 43.5, TRUMPET wins at 49.34 s, 5.4 HP.
- Engine lessons: a beam's aim locked at charge start with linear prediction lands ~2/3 of shots (walls, clashes and
  knockbacks make it miss); keep beam state opt-in (no RNG draws, no new snap keys: charges + blasts are events, the bell
  angle rides in `TH`). Parallel sessions all appended to fighters.py: resolve rebase conflicts by keeping every entry,
  then re-run tools/ballfight_replay_hash.py (and compare your own chosen seed with its cached run) before pushing.
- Rules lessons: balance is touchy on beam damage (4.0 -> 4.4 = 28 % -> 68 % trumpet wins at scale 0.5). Lower the
  weapons' damage rather than the global scale, so desperation moves keep their punch: lead changes 0.55 -> 0.8 per run.
- Look lessons: a smoke ring standing across its flight is edge-on from this camera (a sliver): draw it as a donut turned
  three-quarters to the camera. An effect hovering above the ring (the honk bell) leaves the sharp band above ~2.9 units
  near the front corner: keep it low (focus audit caught a 1.2 px CoC).
- Loop lessons: corner-light beat pulses light the whole floor, so a beat whose 70 ms ramp falls inside one frame is a
  whole-frame step; the mp4 seam measured 2.7x (raw 2.4x). Ramping only the frame-0 downbeat over 0.14 s (4 frames)
  fixed it with a 4-frame patch render: seam 1.9x.
- Sound: horns as call and response on the engine's own grid (volleys on beats 1 + 3, blasts on 2 + 4); horn buses
  needed +6.4 dB (trumpet) / +3.2 dB (sax) to sit 3-12 dB over the band in 300-5000 Hz.
- Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames; loopcheck PASS render + mp4 (seam 1.9x); flash
  0 general; opening -17.5 dBFS, build +3.5 dB. Render ~0.19 s/frame/core (~6 min) + 3.5 min mux.
- Feedback v1 (Oct 24): the trumpet model was inaccurate (made before the series used Chrome references), and the
  laser had a hard cutoff at the ball. v2: the Pixel Pitch modern trumpet rebuilt from a Yamaha YTR-8335LA side view +
  top view (the old one had the leadpipe on top and a small low bell; a real trumpet has the bell pipe on top, the
  leadpipe in front of the valves, the main slide looping down at the bell end, the bell bow at the mouthpiece end).
  Lesson: a light effect should start at its source inside the subject (the bell, inside the glass) with blurred
  halos, not at a hard polygon edge on the ball's rim. Full re-render (the model is in every frame).
- v3 (BALLFIGHT v2 redesign, Oct 24): new signatures, the instrument's body as the weapon. TRUMPET VALVE COMBO (engine
  kind `combo`: three pistons punch 1-2-3 on the 8ths, aimed at the enemy, out / hold / back, the third harder; reach
  grows) and SAX STRAP FLAIL (kind `flail`: the head spins up toward `spin_max`, damage grows with momentum, a landed hit
  wraps the enemy, reels it in to 2.3 until a beat two on, then yanks); HIGH C became an ability kind `beam` (v2's blast as
  the finisher). Three small engine commits + a roster commit, each checked with tools/ballfight_replay_hash.py (all nine
  episodes; #29's own entry moved to the new seed).
- v3 rules lessons: reeling the trumpet to 1.6 put it inside its own piston reach and it won 100 % by 27 s; reeling to
  2.3 (just outside the starting reach) makes the growing reach the trumpet's way back in. Balance flips fast on the
  flail's damage (7 -> 8 = 83 % -> 22 %). Close fights level both fighters together, so unlocks pair up and ~1 % of runs
  fit the hard windows; the search still took a minute. Seed 24 had a later lead change but a 4.3 s hitless opening,
  so seed 191 (longest gap 2.6 s) won.
- v3 look lessons: pistons need ~5 px heads to read at ball size; smoke trails need short lives or they carpet the
  ring; a wide beam's halo must not scale with its width (cap it) or the finisher blinds the frame; the giant beam's
  visible length ends at the ropes to stay in the sharp band. Flash test on the busiest stretch: 16.6 % peak.
- Feedback v3: (pending)

## BALLFIGHT #9: Organ vs Harp (`projects/ballfight_organ_vs_harp`)
- Ninth BALLFIGHT episode (#35), produced in parallel with #29-#34 on the shared engine. ORGAN erupts rows of organ pipes
  out of the canvas toward the harp on every second beat (new weapon kind `line`: the row runs out at 6 u/s, each pipe up
  0.55 s, one hit per row, grows its length); HARP stretches tripwire strings rope to rope across the ring (new kind
  `trap`: crossing an enemy string cuts, the owner passes freely, 1 -> 7 strings, the oldest fades). Abilities: CATHEDRAL
  (dome shield) / PULL ALL STOPS (haste) / TOCCATA (slam: a pipe forest); PLUCK (arc) / ANGEL WINGS (wings + halo shield)
  / 47 STRINGS (wave). Mood: gothic horror, D minor, 125.90 BPM (32 bars). ORGAN bridge entry = organ60_lowpoly's
  Saint-Sulpice case + reed voice (the modern "cage" organ reads as a frame in an acrylic box at ball size).
- Honest run, seed 1181 (10 fits in 1,179 runs; ORGAN 52 % / HARP 48 % over 1,042 decided): organ leads early, the harp's
  web takes over at 33.4 s, lead flips at 38.1 / 43.9 s, the organ wins on its last hits at 51.66 s with 4.5 HP.
- Engine lessons: hazards reported as events (`line`, `string`, `string_off`; hits carry the hazard id) need no new
  recorded arrays, so the change stays tiny and replays of #27 / #28 are untouched (tools/ballfight_replay_hash.py before
  and after every commit, and again after merging the other sessions' engine commits: rebases conflict in the same
  places - Fight.__init__ state, the on-beat branch, the per-substep calls, the roster tail - keep both sides).
- Rules lessons: fast, wide rows landed ~80 % and won by ~28 s; slower, thinner rows with a short start make the organ an
  early, close-range bully, and a one-string harp that grows per landed crossing makes the natural "bully vs web" story
  (~0.6-0.8 lead changes per run). Fit rate ~1 % with #27's hard windows: an early-stopping search is ~2 min.
- Loop lesson: the pearl corner light is bright, so its beat pulse moved enough of the frame to fail the seam (2.7x with
  #27's 70 ms pre-ramp); a 0.16 s pre-ramp in this episode's `beat_pulse` -> 1.3x. The balls' sprite yaw ignores the
  corner rev shake (no 6-degree yaw-step flicker at the seam).
- Sound: first balance had the organ stabs only +2-5 dB over the bed (short stabs every 2 beats read low on average);
  organ bus x2 -> +8-11 dB, harp +8-14 dB. The organ synth caches notes by (scene t0, event index): the bridge clears
  that cache per voice() call (added by #31's session), or every organ note comes back as the first one.
- Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames (pipe tops and strings included); loopcheck PASS
  render + mp4 (seam 1.7x); flash 0 general (max 7.6 % at the TOCCATA); opening -18.5 dBFS, build +3.8 dB. Render 716 s
  on 2 cores + mux (19 min total).
- v2 (BALLFIGHT v2 redesign, user Oct 24): built the shared effects kit `reelkit/fx.py` first (five small commits, pushed
  before the re-render so #29-#34 could start): Flares (point lights in the fighter's colour fed into the scene's
  Lights, so the canvas / floor passes, ropes, posts and stands light up; `tint()` for the crowd), Bursts (1-2 frame ink
  starbursts; FlashBudget keeps bright cores to 3 / s), Shake (capped 2.5 px), shimmer / fringe (local warps),
  Particles (analytic drag + gravity + wobble, trails = the same particle sampled earlier, so any frame renders alone),
  Marks (per-pixel floor marks in the caller's floor coords), Moments (eased <= 30 % dim with the fighters kept lit +
  speed lines). Everything is built once from the run's events, so patch renders match the full render.
- v2 lessons: a pearl (near-white) fighter's light whitens the canvas fast: scale its colour by ~0.9 and keep peaks
  <= 0.75; first pass at peaks 1.0-1.8 plus the old per-hit glows blew out the hit spots. The flash test on the two
  busiest stretches before the full render (render 5-7 s of frames, `flashcheck.analyse`) matched the final mp4
  (18.6 % vs 18.5 % max). Same audio (the events didn't change). Render 18 min.
- v3 (VS title card intro, series standard Oct 24): moved #30's `title.py` into `projects/ballfight/` (its own commit)
  and generalised it: the word layout comes from the names (biggest scale <= 7 that fits; #30 reproduces exactly),
  panel patterns by fighter (`PATTERNS`; other sessions have since added theirs, plus opt-in CLOSEUP / GLASS
  registries), the sprite cache in the episode's own check/. Verified by rendering 10 of #30's title frames with the
  old and new file: pixel-identical. Bar 0 = the card, bar 1 = 3-2-1, FIGHT! on bar 2; chords and sections moved a bar
  (title on the tonic, count-in on the dominant, bar 31's dominant resolves on the title slam).
- v3 seed: the search re-run with the fight at 2 bars (rules frozen) ranked seed 237 first (6 fits in 999); old caches
  (1-bar runs) moved aside first so `sim.load` couldn't return a stale run. Flash: the doors pair is the one counted
  flash (worst 1 / s), as #30. Render 17 min.
- v4 (HUD ability list, series rule Oct 25; the user asked for #35's re-render): #36's `fight_hud(abilities=...)` with
  #36's row builder (same hold conventions, so the list behaves the same in every episode). Measured the overlap first
  (screen extents of balls, pipe tops, ring corners over the whole video): the HUD top moved 466 -> 448 and cut across
  the front corner post (444-450) while the fight itself stayed above 438, so a small 8 px camera nudge (SY 288 -> 280)
  was enough; the back post top lands at 122, just under the one-line captions (100-121). Pearl (near-white) unlocked
  rows still read apart from the grey locked ones. Same audio.wav (no events or captions changed). Render 21 min.
- Feedback: (pending)

## BALLFIGHT #7: Ukulele vs Acoustic Guitar (`projects/ballfight_ukulele_vs_acoustic`)
- Seventh BALLFIGHT episode (#33) on #27's engine, ring, HUD and count-in. Little brother vs big brother: UKULELE
  (pink, left) body-slams on every 2nd beat (new weapon kind `dash`: a speed burst that decays like a knockback, so
  power = speed + distance; contact within 0.5 s hits once; +0.08 power per landed dash), ACOUSTIC (wood tan, right;
  "ACOUSTIC" everywhere, never GUITAR) strums a 26 deg cone of sound out of its sound hole on every beat (new kind `cone`:
  +0.06 length per landed strum). Abilities CHUNK / SPEED STRUM / UKE-NADO (a spinning whirl); CAPO (a capo clamps
  across the ball, cones on the 8ths) / FINGERPICK / BIG STRUM (a giant tortoiseshell pick's downstroke). The uke
  sprite sits smaller in its ball (0.70 vs 0.86 of the radius). Spaghetti-western standoff: A minor, Am - G - F - E,
  129.84 BPM (33 bars).
- Honest run, seed 874 (6 fits in 974; UKULELE 57 % / ACOUSTIC 43 % over 894 decided): the uke bullies early, it stays
  within ~10 HP from 20 s, BIG STRUM 43.9 and UKE-NADO 44.8 swing the lead twice (44.9 / 46.5 s), 9.4-9.4 at 50 s,
  UKULELE wins at 51.87 s with 6.2 HP. Captions CHUNK 7.9, CAPO 13.9, SPEED STRUM 21.7, FINGERPICK 27.3, BIG STRUM 43.9
  (UKE-NADO 0.9 s later goes to the HUD status line, 4 s rule).
- Engine lessons: seven sessions edited fight.py / fighters.py within the same hour; every push hit a conflict, always
  adjacent additions (new list attributes in __init__, new blocks in run()). Keep both sides, re-run
  tools/ballfight_replay_hash.py after every rebase, then push. A new kind on its own 16th counter with its own local
  names (`dc_*`) merged cleanly next to others' blocks. Comparing cached runs: the `K` array is NaN-padded, so compare
  with equal_nan (a plain array_equal reports a false mismatch).
- Rules lessons: the first tune on 64 seeds said 55 % for the uke; the search (826 runs) said 64 %. Re-tuned on 160
  seeds a cell (dash damage 5.2 -> 4.8). Balance swings ~8 points per 0.4 dash damage.
- Sound lessons: the fighters are the rhythm section (uke strums on its dashes, acoustic boom-chick on its cones), so
  the bed has no rhythm guitar of its own; the whistle at the first level sat level with the fighters - at -8.5 dB and
  the twang -3 dB the fighters sit 7.5-12.5 dB over the bed in 300-5000 Hz.
- Loop lesson: the mp4 seam failed at 2.6x (render 2.4x): the "3" flying in on the last frame was 97.4 px against 97.5 at
  frame 0, so its nearest-neighbour rows shifted. The fly-in now lands on exactly frame 0's size one frame early;
  patched the last beat into the kept raw (patch_frames.py) and re-muxed: 2.3x PASS. No second full render.
- Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames; loopcheck PASS render + mp4 (seam 2.3x, 2.7 dB,
  no click); flash 0 general; opening -19.7 dBFS, build +3.5 dB, no quiet windows. Render 16.5 min + 5 min mux.
- Feedback: (pending)

### v2 (BALLFIGHT v2 redesign, user Oct 24): SURF'S UP vs SIX-STRING RAKE
- New signatures on new engine kinds (additive, small commits; #27 / #28 replay identically; replay guard moved to the
  new seed): `surf` (a speed burst + a wave front that crashes once and pushes the enemy along it; height grows), `rake`
  (six string lines sweeping sideways across the front, alternating down / up; one hit per stroke flicks the enemy
  along the stroke; length grows), abilities `whip` (FINGERPICK), `sweep` (BIG STRUM, ring-wide), `spout` (UKE-NADO, a
  homing waterspout, 4 ticks; its path is logged in out['X33_SPOUT'] for the renderer).
- Rules lessons: aimed dead on, both signatures landed ~100 % (uke 71 hits, acoustic 129): a deterministic damage race
  with no variance and no lead changes. Option `aim_k` (the attack goes partly where the ball is heading) brought hit
  rates to ~55 % / ~64 %; slower growth (0.07 -> 0.03 both) raised lead changes from ~0.3 to ~0.5-0.7 per run. The
  tuner now prints the share of decided runs (timeouts had been counted as acoustic wins).
- Look lessons: from the 3/4 top-down camera a wave reads by its white foam crest, a bowed front and a turquoise body;
  dark blue flat faces read as a box. Strings need one wide warm glow under the bundle to read on the lilac canvas.
  Claw marks from every stroke (life 2.6 s) keep the canvas busy without noise.
- Honest run, seed 1248 (6 fits in 1248; UKULELE 54 % / ACOUSTIC 46 % over 1104 decided): UKULELE wins at 52.33 s with
  4.2 HP after BIG STRUM flips the lead at 41.9 s. Checks: flash 0 (max 16.4 %); loopcheck PASS render 1.8x, mp4 2.1x;
  CoC 0; safe zone OK; opening -19.4 dBFS, build +3.4 dB. Render 23 min CPU (~12 min wall) + 4 min mux.
- Feedback: (pending)

### v3 (series standard, Oct 24): the VS title card intro
- Bar 0 = the shared VS title card (`projects/ballfight/title.py`; #35's session moved it from #30 a minute before my
  own move, so I dropped mine and used theirs, then added UKULELE / ACOUSTIC panel patterns in a small commit; #30's card
  stays pixel-identical). 3-2-1 on bar 1, FIGHT! on bar 2 (3.70 s); top caption, series-tag strip and the end-of-file
  '3' fly-in removed; HP numbers + HUD hidden while the card is shut. Chords re-laid so the card sits on Am, the count-in
  on E, and bar 32 (E) resolves into the slam.
- Seed 1248 at 2 bars is a different fight (K.O. 54.2 s), so the honest search was re-run with the rules unchanged:
  seed 1130 (UKULELE wins at 50.97 s with 4.3 HP, two lead changes, UKE-NADO captioned).
- Audio: the louder card lifted the first half, so the early fight is pulled back 0.4 dB to keep the build at +3.3 dB.
  (A comment pasted mid-line had swallowed five GAIN_PTS points; caught by the levels re-check.)
- Checks: loopcheck render 0.9x / mp4 1.5x; flashcheck 1 doors pair (1/s); CoC 0; safe zone OK. Render ~9.5 min wall
  + 3.5 min mux.
- Feedback: (pending)

## BALLFIGHT #4 v2: Bagpipes vs Accordion (BALLFIGHT v2 redesign)
- v2 of #30 on the user's approved redesign: BAG INFLATE (the bag swells to ~1.6x size and mass over 2 beats, then
  rockets around like a released balloon, zig-zagging and squealing) vs BELLOWS BITE (the end boards fly apart with the
  pleats as jaws and snap shut); WAR CRY / HIGHLAND FLING / BAG BURST; POLKA / SQUEEZEBOX / FULL BELLOWS (a ring-wide
  clamp). Effects on reelkit.fx. Hook, captions, colours, mood and tempo unchanged.
- Engine (small separate commits, all additive): guarded per-ball radius / mass / speed + clash hooks + a RAD record in
  move() / snap(); weapon kinds `inflate` and `bite`; ability kind `clamp`. Proof of "no replay changes": hash every
  guarded episode with the upstream fight.py and with mine (45 runs identical), as well as #27 / #28 vs the morning's
  snapshot. Four rebases onto other sessions' engine commits; every conflict was keep-both, except one where the
  keep-both landed my `elif` after another session's new `if` block (a syntax error caught by `ast.parse` before
  committing): always parse after resolving.
- Rules: the accordion first won every run (bites ~60 % on target, the bag rarely landed); ~7 damage per landed flight
  balanced it; size growth +0.015 per hit up to 1.65x keeps the brief's ~1.6x. Seed 1442 (6 fits in 1466, 52 / 48 %):
  the bagpipes lead by 20, FULL BELLOWS lands at 43.8 s, three late lead changes, ACCORDION wins on 1.8 HP.
- Look: the tartan stretched over the swelling glass reads as "the bag" at a glance; a full-height FULL BELLOWS wall
  along the axis hid both fighters (the axis runs through them by construction), so it is now low (0.32) and drawn
  under the balls, with tall end boards. The flight ribbon needed width 7 (internal px) to read under the puffs.
- Seam: v2's calmer fight lowered the median frame step (1.56 -> 1.18), so the same count-in seam failed (2.5x); the
  flying "3" now reaches its landing size on the last frame (frame 0 repeats it): 2.1x render, 2.4x mp4.
- Feedback: (pending)

## BALLFIGHT #5 v2: Turntables vs Synth (THE RING IS THE RECORD vs PATCH CABLES)
- BALLFIGHT v2 redesign (user, Oct 24): new attacks for both fighters, same hook, captions, colours, mood and tempo.
  Engine (additive, three small commits): `scratch_floor` (the floor jerks forward then back every bar; each jerk is a
  knock around the ring centre; the first rope hit within 0.8 s is a slam scaled by impact speed, detected from the
  engine's own 'rope' events so move() is untouched), `tether` (plugs -> cables -> pulses on their own 16th counter; a
  leash that bounces the enemy off its end or snaps under a hard fling) and `lash` (SAWTOOTH's saw-tooth sweep; the name
  `whip` was already taken by #33's straight flicks, and no two fighters share an attack kind). All three are rebuilt
  from events in the renderer, so the recorder is unchanged.
- Tuning lessons: v2 slams at 4.0 ended fights by 15 s. At slam 1.6 / pulse 0.2 the balance flipped from 55 % decks to
  0 % as the damage scale dropped: longer fights let the synth reach 4 cables, a full leash never snapped (the knock has
  decayed by the time the ball reaches the leash) and it also stopped the decks' rope slams. snap 0.15 x cables + pulse
  0.17 + scale 0.6 = 50 / 50 with ~1.1 lead changes per run (v1 ~0.6).
- Picture: the floor angle (a slow spin, 7 whole turns per loop, + each scratch's baby-scratch curve) drives the record's
  label, strobe dots and dust specks (grooves alone are rotation-invariant, so the spin needs marks that turn); the same
  curve is the record position the scratch audio plays, so picture and sound move together. Effects from reelkit.fx as in
  the ballfight README recipe; a landed BASS DROP record must go under the balls (it painted over them at first); jerk
  streak particles need a direction (the default is straight up, which read as rain).
- Checks: safe zone OK; CoC 0 on all frames; loopcheck PASS render + mp4; flash peak 14.8 % (BASS DROP); opening
  -16.7 dBFS, build +3.0 dB; leads 4-11 dB over the bed in 400-4000 Hz. Render 8.8 min + mux.
- Feedback: (pending)

## BALLFIGHT #4 v3: the VS title card intro test (#30 only)
- User direction (Oct 24): a better hook than the bare 3-2-1. Bar 0 is now a fighting-game VS card (`title.py`):
  diagonal panels in the fighters' colours with each ball big and close, BAGPIPES / VS / ACCORDION slamming in on
  beats 1-3 (overshoot, streaks, shake, dust, local light; drone blast / thunder crack as VS splits the divider /
  accordion chord), doors sliding apart along the crack on beat 4; bar 1 = the 3-2-1, FIGHT! at 3.94 s; the old top
  caption is gone. Words at the biggest pixel scales that fit the safe zone (6 / 12 / 5; ACCORDION left of the button
  column); only VS pulses on beat 4 (a name pulsing by 5 % went 1 px out of the zone).
- Seed: 1442 did not replay identically with the fight one bar later (positions drift ~1e-7 from float rounding of
  the absolute substep times, and the chaos amplifies it from ~31 s: K.O. 51.41 instead of ~52.4). Re-ran the honest
  search at 2 bars (rules unchanged): 1442's new run ranked first anyway (same first 30 s, a closer ending: ACCORDION
  wins on 0.24 HP). Lesson: shifting t_fight is not a pure time shift for this engine; test, don't assume.
- Loop: the title slam is the frame-0 reset. The doors slide shut over the last 0.30 s (each step ~1/9 of the change, so
  loopcheck reads them as rhythm, not a reset) and BAGPIPES falls in as a ghost that lands exactly on frame 0's word.
  Two seam traps: the slam's light / streaks / ball glow / shake started ON frame 0 (seam 5.2x) - they now rise from the
  next frame; and a size factor of 2.2500000000000044 vs 2.25 rounded the word one pixel taller (banker's rounding of
  94.5) - snap the ghost to the exact frame-0 value. Seam 0.9x render, 1.5x mp4; audio: the first 20 ms (the slam) was
  7.7 dB over the last 20 ms until the doors' whoosh ramps to full into the seam (3.9 dB).
- Flash: one up/down pair (doors shut / doors open), worst 1 per second; the panels are never white.
- The HUD is skipped while the card is shut (its notes overlapped the caption in reelkit.safezone even though hidden).
- Feedback: (pending)

## BALLFIGHT #5 v3 (#31): the VS title card intro (series rule)
- Shared card, no fork: `projects/ballfight/title.py` (#35's session moved #30's card there minutes before my own move;
  my push conflicted, so I dropped my version and built on theirs). The episode only registers its panel patterns in
  `TT.PATTERNS` (lime record grooves + label, a cyan circuit board) and pre-fills `TT._SPR` with its own pitched sprites,
  so the shared module and #30 stay untouched.
- Seed: re-ran the honest search with the fight at 2 bars (rules unchanged): 6 fits in 1339 runs, TURNTABLES 617 /
  SYNTH 665 of 1282 decided. Seed 1339 (v2's 781 is not a time shift): the decks lead for 24 s, the lead swaps back and
  forth from 28 to 42 s, a 9.9 HP REWIND keeps the decks ahead to the wire, and the cables' pulses take it at 50.05 s:
  SYNTH wins at 50.79 s with 2.8 HP.
- Git: a rebase stripped a commit subject that began with '#' (git reads it as a comment on `rebase --continue`); start
  subjects with a word ("IDEAS 31 v3: ...").
- Flash test before the render: decimating frames (`[::2, ::2]`) counted 3 "flashes" in the card (the grooves / circuit
  textures shaking on the slams); `reelkit.flashcheck` area-downscales to 360x640, and with the same downscale the card
  is 0 flashes (textures 15-18 %, the doors 79 % as one transition). Pre-render flash tests need the checker's own scaling.
- Safe zone: FIGHT!'s slam (punch 0.25) poked 7 px into the button column for one frame; the punch is 0.12 now.
- Checks: safe zone OK; CoC 0 on all frames; loopcheck PASS render 0.6x / mp4 1.1x (audio step 3.2 dB, no click); flashcheck PASS (one doors pair at 1.77 s, worst 1/s); opening -16.4 dBFS, build +3.1 dB. Render 11 min + 10 min mux.
- Feedback: (pending)

## BALLFIGHT #10 (#36): Trombone vs Tuba, the first crossover + the HUD ability list
- Crossover rule in practice: the fighters come straight from `fighters.py` and the renderer imports both episodes'
  draw functions (`styles.py` loads #34's and #32's styles modules by path) instead of copying them. The engine only had
  the shared `dmg_scale`, so `Rules(dmg_side=(left, right))` went in as its own opt-in commit (replay guard identical
  for all 9 finished episodes). The pair is very lopsided in hit count (lance ~70 hits, swallow ~12), so the handicap is
  big: TROMBONE x0.35, TUBA x1.55 (49 % / 51 % over 261 decided runs, median K.O. 50.8 s). Damage alone balanced it.
- HUD ability list: `reelkit.ui.fight_hud(..., abilities=(left_rows, right_rows))`, rows = dict(tag, name, state, glow,
  sweep), in its own commit. Checked "draws exactly as before" by hashing 353 HUD input combinations (image + the
  safe-zone notes) before and after. Two look fixes from the first test: the tag needs a real space ("LV2SAD" read as one
  word), and the dark text outline filled the 1 px gaps between letters, chopping the pill into pieces: while lit, the
  outline shifts to a deep shade of the fighter's colour and all pills draw before any text. The episode builds the
  rows from the run's `level` / `ability` events (`render.ability_rows`): held lit while a shield / haste lasts, a slam
  until it lands; frozen through the hit-stop; back to grey on the glide home. The 18 px taller HUD stayed clear of the
  action (the balls never got below internal y 417), so no ring nudge.
- Seed pick: the top-scoring fit (106) had its K.O. at 52.94 s, which leaves ~1 s for COMMENT THE NEXT MATCH-UP and a
  5-beat solo; took the second (358, K.O. 49.10 s). Worth adding "K.O. + 3.65 s caption gets >= 3 s" to the score.
- Merging two renderers: the slam kind is shared (PEDAL TONE and SUBSONIC are both `slam`), so every slam consumer
  (the giant bell, its target decal, the "under the bell" HP hide, shimmers, fx, the focus audit) filters by owner.
  #32's TUBA title panel was centred on the left ball; it's recentred on this episode's right-hand ball. PEDAL TONE's
  bell is lit mint (a flare during the hover + a halo under the lip) so it reads apart from the tuba's own bell.
- Focus audit: the hovering PEDAL TONE bell (up to 8 units above the canvas) is 4.8 px soft, as it was in #34; the audit
  now checks its rim once it stomps (0 px), and the subject stays 0 px on every frame.
- Music (mood C, brass house): with no bass synth the kick had to make room for the tuba: a 0.16 s body high-passed at
  48 Hz and a lower send lifted the tuba from +5 to +8 dB over the bed in 40-260 Hz.
- Checks: safe zone OK; CoC 0; loopcheck PASS render 0.5x / mp4 1.2x (audio 1.2 dB, no click); flashcheck PASS (1, the
  doors); opening -16.4 dBFS, build +3.2 dB. Render 9 min + 3 min mux.
- Feedback: (pending)

## BALLFIGHT #11 (#37): Clarinet vs Oboe, two new fighters + the first levels rule episode
- Reference photos: Chrome wasn't reachable from the session, so the built-in browser (the desktop app's pane) found
  them on Wikimedia Commons. The Yamaha YOB-831 product shot is a straight-on 863 x 6937 px strip: measured with a
  canvas script in the page (same-origin image) plus scaled screenshots, at 590 mm body length. At ball-sprite size
  the true oboe is a 1-2 px line (and the clarinet nearly the same), so `oboe_lowpoly.py` has a `ball` scene with every
  radius x1.6 about the bore (lengths true); the true model stays the default for a future Pixel Pitch video.
- Engine: a teleport needs no per-frame state (positions jump in P; events carry both spots). The jump / beam timing
  decides everything: the beam locked after the beat's jump hit ~80 % (oboe ran away with it), locked before it hit
  ~0 % (clarinet 100 %); lock 3 / fire 3 sixteenths after the bar line with no lead gives ~64 % and real dodges when
  CLARION jumps on the 8ths. Lead changes stayed low (~0.6 a run) until OUT OF TUNE's weakening was strong (x0.4 at
  full depth) so detuned stretches swing the fight; fit rate 1.7-2.5 %, close to #36's.
- Stun is general: every weapon dispatch skips a stunned fighter (getattr default, so old episodes are untouched);
  blades already spinning keep spinning.
- Picture: the zip line in lavender vanished on the lilac canvas under the spot: it needed an ink edge, a white core
  and a luminous halo, and an arc (it reads as a leap). DOUBLE REED's blades draw over both balls while they snip
  (the enemy is always close then, so painter order hid them). Beams stop at the ropes. OUT OF TUNE = a doubled, warped
  olive outline + a sour glass tint + a seasick sprite roll in 4-degree steps (cached: 1672 sprites).
- Music: consistent loudness from frame 0 with no automation at all: one master gain + compressor + soft limiter, the
  storm already full under the title card, building with parts (violin II, violas, arpeggios, timpani on 3). First
  levels run passed (spread 4.8 dB); trimming the title tutti and thickening bars 34-35 brought it to 4.3 dB.
- Checks: levels PASS; band check +7..+12 dB; safe zone OK; CoC 0; loopcheck PASS render 0.5x / mp4 1.0x; flashcheck
  PASS (the doors). Render 7 min + 4 min mux.
- Feedback (Oct 26): "in the last couple of seconds of the fight, the clarinet stops making noise". Cause: the HP-based
  thinning (drop some of a hurt fighter's notes) used #36's `h01`, which is linear in its input: on consecutive slots
  its values creep up by ~0.007, so all 15 of CLARION's 8th jumps after 48.9 s (clarinet at ~10 HP) fell under the
  threshold and the line went silent; the oboe's stings had 1 s holes too. Fix: a splitmix64 hash + never two dropped
  notes in a row (max 30 %). Re-muxed only. Lesson: check each fighter's stem has no silent stretch while it's acting
  (0.5 s windows), not just the mix levels; #36 and older episodes use the same hash (posted, not re-rendered).
- Feedback: (pending)


## BALLFIGHT #12 (#38): Bass Clarinet vs Euphonium, two new fighters on the levels rule
- Reference photos in the user's Chrome (Claude in Chrome worked this time): Commons category pages read as text for
  the file lists + sizes, then each candidate drawn into a canvas with a 50 px grid (same-origin via `crossOrigin`) and
  screenshotted in zoomed slices to measure. Bass clarinet: Yamaha YCL-622II front shot (no official dimensions, so
  scaled to a 40 mm body), checked against a Buffet Prestige 3/4 view. Euphonium: Yamaha YEP-321 front shot, scaled with
  Yamaha's published 280 mm bell / 664 mm height (the two agree within 4 %). A bass clarinet is 26:1 long:thin, so the
  `ball` scene fattens radii x2 (neck + mouthpiece x2.4, the bell rim x chunk^0.75 so it doesn't balloon).
- Engine (5 small commits + a per-arm cooldown fix, all 11 finished episodes bit-identical each time): `anchor` (the
  whirl drives the ball's position directly and `move()` skips it; the clash still pushes the enemy out), `mortar` +
  `barrage` (shells with flight times; `predict()` follows an anchored enemy round its circle), `twin`, `copy` (new
  optional `trigger(k, slot, ab=, quiet=)` args + `Ball.last_ab`; a copied snip steps through `extra_snip`),
  `lights_out` (a `shadow` attribute read by `damage()`, `_stunned()` and the clash test; absent = as before).
  The #39 session pushed six engine kinds in parallel: no conflicts (all additive), my seed replays the same after the
  rebase.
- Tuning: first pass the anchor won 95 % (1.5 turns hit twice per whirl); a 0.8 s rest per arm (the MULTIPHONICS ghost
  arm has its own) + exact-lead shells fixed it. Lead changes stay rare (~0.7 a run: both weapons snowball), so fits
  are 0.4 % (8 in 2,171). SLAP TONGUE reuses #37's `snip` kind (no new kind needed).
- Title card: BASS CLARINET is the longest name yet (one line only fits at scale 4), so `title.py` got opt-in
  `TITLE_LINES` (BASS / CLARINET at scale 6) + `TITLE_BALLS` (the left ball moved down); every older card hashes the same.
- Picture: the endpin only reaches 0.35-0.7 beyond the glass at the base orbit, so it reads as a bright silver rod over
  the ball's edge (drawn after the balls), not a black arm under them. Shell landings that miss only kick dust: scorches
  for every shell turned the canvas into a carpet of splats. LIGHTS OUT = an eased 40 % dim with keep circles on both
  balls (flash test 0) + the ball sinking into a dark pool. IDENTITY CRISIS shows an amber bass clarinet sprite in the
  euphonium's ball for a bar.
- Music: the bass clarinet (140-600 Hz) first sat only -2..+2 dB over the drumline: tenors retuned up (294-494 Hz,
  hp 260), softer bass-drum beaters and timpani overtones, a held growl between whirls (it was silent half of every
  bar), +7..+9 dB after. Post-K.O. bars were 3 dB under the fight (the fighters carry the loudness, not the cadence):
  both fighters play their lines while revving in the corners, levels spread 3.1 dB. The seam needed a louder roll +
  riser (first 20 ms -8.7 vs last -13.1 dBFS). #37's lesson checked: no 0.5 s silent windows in either fighter's stem
  while it's fighting (a thinned note on the 2-beat salvo grid is now clipped, not dropped; the bass clarinet growls
  from FIGHT! to its first plant).
- Checks: levels PASS (wav + mp4); band check +7..+12 dB; safe zone OK; CoC 0; loopcheck PASS render 0.3x / mp4 0.9x;
  flashcheck PASS (the doors). Render 7.7 min + mux.
- Feedback: (pending)


## BALLFIGHT #13 (#39): Bassoon vs Bari Sax, the sneak vs the ring rattler
- Reference photos in the user's Chrome (Claude in Chrome): Commons' search API from a Commons page (same origin) for the
  file lists, then the candidates drawn into a canvas (`crossOrigin`) and measured per row (red-wood spans, silver /
  ivory bands). Bassoon: the Yamaha YFG-812 II front product shot, scaled to the usual 134 cm (0.815 mm/px): boot 425 mm
  (an oval 73 -> 63 mm), long joint to 1012 mm, bell 47 mm with a bulge and a 62 mm pale cap ring, wing joint 34 mm with
  its crown at 849-894 mm; "Bassoon parts.jpg" for the bocal's shape. Bari: the Amati baritone product shot, for what
  changes from the alto (long straight bell with the rim at ~80 % of the height, the pigtail loop, big mouthpiece,
  low-A key). At ball size a 1.34 m bassoon is a 3-px stick: the `ball` scene widens it x2.3 about its centre line and
  thickens the crook x3.4, so the hooked bocal is what reads (it vanished into dots at 16 px until then).
- The bari is a new file adapted from the alto (`altosax60_lowpoly` imported read-only for its swept tubes, cups,
  touches, palette); its voice reuses the alto's recipe, but the alto rolls harmonics off by number (`exp(-k/3)`), which
  at 65 Hz left a dull hum: the bari rolls them off by frequency through its own formants (~560 Hz / ~1.35 kHz).
- Engine (4 small commits, then a timing fix; all finished episodes bit-identical each time, 12 once #38 landed):
  `tiptoe` + `mountain`, `hook`, `rattle` + `offbeat`, `quake`. A hopper is a new kind of mover: `move()` skips it
  mid-hop (`air`) and moves it only by its knock boost on the ground (`hopper`), the clash skips airborne balls. The
  rattle reads new 'rope' events from the event list to find slings (no change to `move()`'s bounce).
- Timing bug: a hop landing a fraction of a substep after its 16th tick was missed at the tick, so the next launch
  slipped a 16th: MOUNTAIN KING's accelerating schedule came out as eight 3-16th hops then a ragged mix. Landing a hop
  due within one substep at the tick fixed it, and the on-time dance became decisive (82 % wins): re-tuned once
  (MOUNTAIN KING 1.6 -> 0.4 a stomp) and re-searched (the first pick, seed 668, no longer replays).
- Tuning: smooth damage streams (a stomp every beat landing ~95 %) gave ~0.2 lead changes a run. What helped: a
  small, growing stomp radius with an imperfect lead (about half the stomps land early, ~95 % late), backstabs x2.5, and a rattle
  that needs ~10 landed hits to reach all four ropes, so the bari's slings ramp up mid-fight while the bassoon ramps up
  later: lead changes ~0.7 a run, fits ~1-2 %, the search is cheap (275 runs for 8 fits). A random search over the
  grid (`tune_search.py`) was a waste here: the caption rule (>= 5 first uses 4 s apart) and the lead changes are
  structural, not numeric.
- Picture: the ropes ringing only read once they were tinted coral with a single standing-wave mode per side (three
  ropes in different modes looked like a glitch). Squash and stretch about the ball's bottom point; the hop's shadow
  stays on the canvas, which sells the height.
- Music: the honk sat under the bed in its own band until the downbeat kick stepped back under it (the honk IS the
  downbeat) and the incidental thuds were thinned; more ringing ropes = a fuller bass chord scaled by 1/sqrt(n), not a
  louder one (levels: 6.2 -> 3.6 dB spread). The bassoon's thinning only drops notes when its line is dense (a tumbling
  bassoon was going silent for 1.75 s).
- Checks: levels PASS (wav + mp4); band check +5..+17 dB; safe zone OK; CoC 0; flash 0 on the busiest stretch; loopcheck
  PASS render 0.4x / mp4 1.0x. Render 10 min + 3.5 min mux.
- Feedback (user, Oct 26): "The bari sax is really boring." From four attack sets the user picked WRECKING BALL: v2.

### #39 v2: the bari sax as a wrecking ball
- v1 kept first: its sources copied to `v1/` (paths one level deeper; `v1/sim.py` patches the frozen v1 BARI SAX into
  the roster at import), the guard repointed at `v1/` (seed 190 replays bit-identically), the mp4 renamed `_v1.mp4`.
- Engine (additive, one commit each, the guard identical on all 65 runs every time): weapon `wrecking`, ability
  `snap`, then `len_pow` (a longer cable hits harder). The pendulum is a real half period (phi = -theta cos(pi u)), the
  ball airborne all the way (the clash and `move()` already skip `air`), hoisted back first so it swings *through* the
  enemy instead of starting on top of it.
- The first wrecking ball hit 100 % of its swings: the bassoon hops behind the bari every beat, so it is always within
  reach of anything that sweeps the bari's surroundings. What made dodges possible: hits only on what is in front of
  the swing, an enemy mid-hop sails over it (the clash's rule; except the first / last tenth of the hop), the aim reads
  where a hop will land, and the bottom of the arc aimed a little short of the enemy so the contact comes at full
  speed. The hit rate then hangs on the beat grid (the bassoon is airborne on 16ths 0-2 of every beat): normal swings
  bottom out on beat 3 and land 84 % (200 seeds), SKANK's off-beat swings 23 % (they meet the bassoon in mid-hop; they
  aim further short to catch it landing). So SKANK reads as a flurry the bassoon mostly hops over.
- Tuning: lead changes were the hard part again (the bari leads early, the bassoon's growing stomps take over). A
  random search (`tune_search.py`, 120 x 60 seeds) did find the shape this time: a lower damage scale (0.32) for a
  longer back-and-forth, a cable that grows fast and hits harder with length (`len_pow` 2.5), a big CABLE SNAP as the
  bari's comeback. Fits 3.5 % over the tuning seeds; the honest search took 166 runs for 8.
- Picture: at the 58 deg view the pendulum's real height (0.7 units) read flat, so the crane system is drawn 1.6x
  taller (pivot and ball together; ground positions exact). The cycle is rebuilt from the run's events (cable, swing,
  smash) and the WRECK record, so the ball is placed exactly between frames (smear ghosts, trail). Hops that sail over a
  swing are drawn as leaps (1.45 units) with the crane drawn under them: at the normal hop height the bassoon sat on
  top of the bari and the cable seemed to hook the bassoon.
- Music: no honk on every bar any more; the bass line is one long honk per swing, swelling to the bottom and bent like
  a doppler pass, the kick stepping back on the beat where it bottoms out. Levels PASS without changes (spread 4.3 dB).
- Checks: levels PASS (wav + mp4); band check bari +3.6..+7.0 dB over the bed while sounding; safe zone OK; CoC 0;
  flashcheck PASS; flash test 0 on 41.5-52 s; loopcheck PASS render 0.4x / mp4 1.0x. Render 7 min, then patch renders for the leaps
  (67 frames, the crane drawn under a leaping ball), mux. Guard: 65 runs identical + v2's seed 27 added (70).
- Feedback (user, Oct 27): "Changes have been made to the Bari Sax's model and attack. Re-render this video": v3.

### #39 v3: the detailed bari, swinging out over the ropes
- What changed came from #40 v2 (session_01CzTfqe): the Pixel Pitch bari's detail pass and the wrecking kind's opt-in
  `overhang` (made the roster's BARI SAX standard, with #39's sim pinning it off). v2 first went to `v2/` with its
  BARI SAX frozen in full (the #39 v1 pattern) and its sprite caches: its frames re-render byte-identical to the
  delivered raw, the guard runs seed 27 there.
- Balance: with full swings almost anywhere the bari won 83 %. The roster's numbers now serve #40 too, so the re-tune
  used the episode's own knobs, as #40 did: a per-fighter damage handicap (`Rules.dmg_side`, the bari x0.8) and the
  shared scale (0.34). Fits 2 %, lead changes ~0.6 a run; 262 honest runs for 8 fits.
- Sprite caches (`check/sprites_ballfight13.npz`, `check/sprites_title.npz`) must be deleted when a model changes; the
  title card builds its big sprites lazily, so render a frame with both balls on the card once in one process before the
  parallel render (otherwise the workers race to write the cache).
- Ported #40 v2's picture for the swing over the ropes (pressed ropes, spring-back wobbles as synthetic rope events,
  drawn after the front ropes) and its sounds. Its slung-back whoosh read its length from the wrong event fields
  (`e[7] - e[6]` is t0 minus a position, so whooshes seconds long, growing through the fight); here it is
  `e[8] - e[7]` (0.27 s). #40's audio.py still has the old line.
- Checks: levels PASS (wav + mp4, spread 4.3 dB); band check bari +3.4..+7.0 dB while sounding; safe zone OK; CoC 0;
  flashcheck PASS; flash test 0 on 41.5-52 s; loopcheck PASS render 0.4x / mp4 1.1x. Render 7.2 min + mux.
- Feedback: (pending)


## #40 BALLFIGHT #14: Bari Sax vs Bass Clarinet (session_01CzTfqe, Oct 26)
- Second crossover after #36: both fighters straight from the roster, sides reversed vs their own episodes. No engine
  option was needed for the sides: `Fight(a, b)` seats a on the left, b on the right, and nothing reads the roster's
  `corner`. Spin is mirrored by side (`spin_dir`), so a right-corner anchor whirls clockwise.
- New pairings can expose engine assumptions: the crane (#39 v2) predicted its target along the enemy's velocity, which
  was fine against a hopper but, against an anchored whirler, meant extrapolating the spin (~10 units/s). CABLE SNAP
  landed 3 % (96 % in #39). Measured it, flagged it (crossover rule), the user chose the fix: lead an anchored enemy
  with the engine's existing `predict()`. Gated on an anchorer being in the fight, so the guard stayed identical.
  Lesson for crossovers: before tuning, measure each signature's hit rate against the new opponent vs its own episode.
- Tuning was quick and sensitive (old engine: the bari won 0 % at its #39 / #38 scales x0.32 / x0.68, 46 % at x0.6 /
  x0.6, 95 % at x0.9 / x0.6). With the fix: x0.35 / x0.68. The honest search took 617 runs for 8 fits. Picked 617 over the top score (309), whose CABLE SNAP
  misses and LIGHTS OUT falls in the 4 s caption gap.
- Title card on swapped sides: the shared two-line option only covered name A (added for name B, opt-in); the panel
  patterns are drawn for one side of the divider, so mirror them (`styles.mirrored`, idempotent: title.py imports the
  renderer by name, so the episode's module-level patch ran twice and un-mirrored itself the first time).
- Renderer = #39's + #38's bass clarinet (styles imported, not copied). LIGHTS OUT's dim keeps a band along the crane
  line. Focus: the bari hoisted 3 up for CABLE SNAP needed sharp 0.19 (0.17 gave 0.10 px).
- Music: the bari as the whole bass line needed two things: a pump between swings (swing honks alone leave a bar in
  every two without bass), and capping the "loser thins" high-pass for a bass fighter (~110 Hz, not ~240): while the
  bari trailed 17-50 s the rule was removing its fundamental. The bed's every-beat kick (late density) sat on the pump:
  replaced by a high anvil ghost. The tail was 2-3 dB under the fight: thickened (hold bar, solo bars at density 0.85,
  bigger pump honks into the seam) rather than raised. Levels spread 4.7 -> 3.3 dB; bari over the bed +1..+4 -> +5..+7.
- Checks: levels PASS (wav + mp4); band check bari +5..+7, bass clarinet +4..+7 dB while sounding; safe zone OK; CoC 0;
  flash test 0 on 38.4-51.6 s, flashcheck PASS; loopcheck PASS render 0.4x / mp4 1.1x. Render 5.7 min + mux. Guard: 70
  runs identical + #40 added (75).
- Feedback: (pending)

### #40 v2: a detailed bari, swings over the ropes, a quieter intro
- "Completely smooth" was a sprite-scale bug as much as a lack of detail: the `ball` scene fattens every tube x1.35 for
  small sprites, but the keys were placed on the unscaled body, so they sat inside it; gold keys on gold also gave no
  contrast, and rods / guards / engraving were lines, which `pp_bridge.model_faces` drops. Lesson for new Pixel Pitch
  models used in BALLFIGHT: place details on the scaled surface, make them faces, and build contrast in value (dark
  pad rings, white pearls, deeper-gold rods), not hue. Checked by rendering the sprite at 20 / 44 / 120 px.
- Swing over the ropes: an engine opt-in (`overhang`, a corner-post cap, a slung-back `return`), made the bari's roster
  standard, with #39's sim pinning it off (the #39 v1 pattern). The swing's ends were clipped to the ring, which cut 41 %
  of swings below 25 deg; now 6 %. Frame limit: the ring's left / right corners sit at the frame edges, hence the
  |x| + |z| cap. The stronger swing needed a re-tune (bari 75 % at the v1 handicap -> x0.30).
- Picture: ropes pressed down under the ball's underside where it passes over them, a spring-back wobble (synthetic
  rope events) + a low twang when it lifts off; drawn after the front ropes when over them.
- Intro loudness: RMS (reelkit.levels) can't see brightness. The card measured +0.7 LU K-weighted over the fight (peaks
  2-3 LU over) with three stacked sheet-metal crashes; thinning by arrangement first overshot to -3.5 LU (the old "too
  quiet" problem), settled at -1.4 LU, level with the ending. `loudness.py` (BS.1770 K-weighting) is worth reusing for
  any bright, percussive bed.
- Focus: this run's CABLE SNAP hoists the bari 3 up near the camera: sharp band 0.26.
- Checks: levels PASS (wav + mp4, spread 3.3 dB); band bari +5..+7, bass clarinet +4..+7 dB; safe zone OK; CoC 0; flash
  test 0 on 36.5-51.2 s, flashcheck PASS; loopcheck PASS render 0.4x / mp4 1.1x. Render 10.7 min (the guard ran
  alongside) + mux.
- Feedback: (pending)
