# Reel playbook

## The brief (fixed for every video)

- **Format:** 60.0 s, 1080×1920, 30 fps. Internal render 360×640, 3× nearest-neighbour upscale.
- **Topic:** music + science. The science must *drive* the sound: tempo, pitch, rhythm or
  timbre comes from the phenomenon. Examples: chirp rate → BPM, plate resonance → notes,
  metronome sync → beat.
- **Look:** inspired by @bit_wonder, never a copy:
  - real 3D low-poly scenes (flat-shaded faces, slight per-face tone jitter);
  - a 2.5D diorama feel with gentle camera moves and few hard cuts;
  - vibrant colour, pixel font and dithered banding;
  - a detailed, interesting environment: foreground, midground and background layers, and small moving life.
- **Engaging the whole way:**
  - a hook in the first 2 s;
  - a new visual payoff every 2–4 s;
  - a music payoff in the back half;
  - an ending that loops back to the opening image.
- **Lighting:** a visible key light (sun or moon) plus warm local lights (fire, lanterns). Use real
  shadows: ground masks plus box occlusion. Brightness should sit roughly at the "lit" level:
  - avoid crushed blacks and heavy vignettes;
  - the user called a dark, moody grade "too dark", so keep `grade()` defaults.
- **Lens:** realistic depth of field with an octagonal aperture, but the focus stays **locked on the
  subject the whole time**:
  - never rack the focus away from it;
  - use a `sharp` band wide enough that the whole subject and its readout (thermometer, plate) stay crisp;
  - verify the subject's CoC is 0 for every frame.
- **Sound:**
  - everything is synthesised;
  - music is original or public domain (pre-1929, e.g. Grieg);
  - the science sound (chirp, tone, tick) sits clearly on top of the music, so check its band level;
  - incidental textures (e.g. sand rattle) stay very low or are left out.

## Process

1. **Pick the idea:** the phenomenon, the hook line, how it makes music, and a 60 s beat sheet (see
   `templates/new_video/timeline.py` for the proven shape).
2. **Simulate first** if the visual depends on physics (sand, metronomes). Cache the results per
   frame (`.npz` or memmap) so rendering is fast and deterministic.
3. **Timeline:** one `timeline.py` imported by both `render.py` and `audio.py`. Put all event times there.
4. **Scene:** build geometry in `scene.py` with `reelkit.geometry`. Give each logical object one group
   (`new_group()`) so its parts don't shadow each other.
5. **Render:** start from the template's `render.py`:
   - floor albedo, then per-pixel light with shadow masks and contact AO;
   - objects in painter's order with `Occluders.vis` shadows and rim;
   - then the lens, glows, god rays, bloom, grade, vignette/scrim and UI.
6. **Look before rendering:**
   - make contact sheets at about 6–8 key times (`python -m reelkit.sheet`);
   - check brightness, readability of text and readouts, the focus, and that nothing occludes the subject;
   - fix things, then repeat.
7. **Audio:**
   - `python audio.py`;
   - print the section levels;
   - check that the science sound's frequency band sits above the music (`reelkit.audio.band_level`).
8. **Full render:** `python -m reelkit.video render.py audio.wav out.mp4`. Run it in the background and
   wait for it. Don't poll with sleeps.
9. **Verify:**
   - run `ffprobe` (expect 1080×1920, 60.0 s, h264 + aac);
   - make a contact sheet from the mp4 (`reelkit.video.frames_sheet`) at about 9–14 times;
   - look at it.
10. **Deliver:** the mp4, plus a short caption and tags for Instagram Reels, TikTok and YouTube Shorts.

## Lessons from feedback

- Telephoto lenses make background objects look huge, so push them far back and fog them.
- Shadows only read on surfaces the camera can see. Low cameras see little ground, so put shadow
  drama where it shows: floors seen from above, grass banding, light shafts through silhouettes.
- Bloom must exclude fine detail that has to stay crisp (sand grains).
- Captions sit in the top 150 px over a darkening scrim, with max 18 chars per line at the large size.
- HUD panels slide in from the bottom and stay clear of the subject.
- Keep backups of each approved version (`*_backup.py`) before a big relight, so you can blend or roll back.

## Captions and tags

One short hook line, plus a "try it" line if relevant. Keep the tag sets separate:
- **Instagram:** about 12 tags
- **TikTok:** about 10 tags, including #fyp
- **YouTube Shorts:** a title + 1–2 sentence description + about 7 hashtags including #shorts
