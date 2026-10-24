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
- Feedback (v2): (pending)

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
- Feedback (v2): (pending)

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
