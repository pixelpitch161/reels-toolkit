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
- Feedback: (pending)

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
- Feedback: (pending)

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
