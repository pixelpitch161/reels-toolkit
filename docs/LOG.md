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

