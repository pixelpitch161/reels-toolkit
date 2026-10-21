# Pixel Pitch playbook

How to make a new instrument-history reel. One session = one instrument.

## Rules (from what worked and what didn't)

1. **Modern instrument first, then the history.** Scene 0 is the modern instrument, shown for 4 bars with a
   hook ("18 FEET OF TUBING, / THE LOWEST BRASS"), then "BUT HOW DID WE / GET HERE?". The history follows,
   oldest to newest, and the outro returns to TODAY.
2. **Every video gets its own background, colours and beat.** Never reuse another video's backdrop or
   groove. Change the backdrop per scene through `THEMES` (each era gets its own palette).
3. **Build the models from real reference photos.** The production session finds them itself in the user's
   open Chrome (Claude in Chrome: image searches, museum collection pages, makers' sites) before modelling.
   Never ask the user to send reference images. The trombone and French horn both needed corrections because
   they were first modelled from memory. Check the proportions, where the bell points, valve or slide
   placement, and how the tubing wraps.
4. **One detailed request per video.** Give the session the instrument, eras, facts and any must-haves up front.
5. **Always turn the lighting on (approved standard since the tuba test):** add `LIGHTING = True` (see README). Check stills mid-scene, not on a scene
   boundary (the instrument is spinning in or out there).
6. **Facts must be checkable.** Each fact line is 2 × ≤ 18 characters, uppercase. Verify every date and claim.
7. **Music:** each scene plays a short riff in that era's style (`credit` names the style). `TRIM` evens out
   scene loudness; the engine masters every video to the same loudness (TARGET_LOUD −11).

## Timeline (60 s format)

`SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1`. At 112–120 BPM that gives an 8 s showcase, about 11 history scenes
of 4 s each, a 1-bar outro and a final chord. Slower tempos (96–100 BPM) need fewer scenes.

## Making one

1. Copy the closest existing script (brass → `tuba60` / `trombone60`; strings → `cello60`; plucked → `harp60`;
   guitars → `electric60`; woodwind → `clarinet60` / `flute60`; percussion → `drums60`).
2. Rename `OUT, TAG`, rewrite `SCENES` (key, theme, year, ys, name, credit, fact, notes, size), the model
   builders, `THEMES`, `background()`, `ped_cols()`, the beat (`KIT`, `drum_events`) and the synth `TONE`s.
3. Research each instrument's look from photos; model it; check `--still` contact sheets per scene.
4. Render with `pp_parallel.py`; watch it all the way through; send.
5. Update [STATUS.md](STATUS.md), commit, `git pull --rebase`, push.

## Captions

Same pattern as the science reels (see `docs/PLAYBOOK.md`): a short hook line plus tags per platform.
