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
- Feedback (v2): (pending)

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
