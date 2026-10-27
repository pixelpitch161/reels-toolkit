# Reel playbook

**What to read before starting (keep it to this):** this playbook, your row on the board (`docs/IDEAS.md`), your
idea file (`docs/ideas/NN-*.md`), your series' rules if any (`docs/ballfight/RULES.md` for BALLFIGHT), and your own
project's log (`docs/log/<project>.md`). Read another project's code or log only when you reuse it.

## The brief (fixed for every video)

- **Format:** 61.0 s, 1080×1920, 30 fps. Internal render 360×640, 3× nearest-neighbour upscale. 61 s because TikTok
  monetisation only applies to videos of at least 1 minute; the extra second is a safety net (user, Oct 23). Set
  `DUR = 61.0` in the project's `timeline.py` (the template does); the toolkit reads each project's own length, so
  videos made at 60 s before this stay 60 s. Design beat sheets on 61 s: the extra second usually goes to the
  ending's run-up into the loop point.
- **Tempo fits the loop:** pick tempos with a whole number of bars in 61 s, so the groove and anything pulsing on
  the beat are in phase at the loop point: `reelkit.config.loop_bpm(96)` -> 94.43 (24 bars), `loop_bpm(120)` ->
  118.03 (30 bars).
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
  - **consistent loudness from frame 0 to the end** (user, Oct 26: "the videos start out too quiet. The ending
    loudness is fine, but it should be more consistent overall"): the opening sits at the ending's level (within
    1.5 dB), every 2 s window within 3 dB of it (one deliberate dip allowed, e.g. a K.O. hit-stop), spread <= 5 dB.
    Build energy with the arrangement (more parts, density, register, brightness), never by starting quieter: no
    level automation that ramps sections up, master the whole track to one loudness with a limiter;
  - an ending that loops back to the opening image.
- **Loop point = the file boundary.** The last frame flows straight into frame 0, and the moment the video
  "starts over" (reset, crack, rewind, new run, everything snapping back) lands ON frame 0, never seconds
  before the end. The last seconds are the run-up into the reset; frame 0 is the reset itself, with chaos
  following within a second. (User feedback on #20 v1: the next run started at 58.6 s, so the video visibly
  started over ~1.5 s before the file ended and looped.) Every periodic motion (drift, twinkle, orbits, flags,
  searchlights) completes whole cycles in the video (61 s): use `reelkit.config.whole_period(p)`. Check with
  `python -m reelkit.loopcheck render.py audio.wav` before the full render and on the mp4 after.
- **Lighting:** a visible key light (sun or moon) plus warm local lights (fire, lanterns). Use real
  shadows: ground masks plus box occlusion. Brightness should sit roughly at the "lit" level:
  - avoid crushed blacks and heavy vignettes;
  - the user called a dark, moody grade "too dark", so keep `grade()` defaults.
- **Lens:** realistic depth of field with an octagonal aperture, but the focus stays **locked on the
  subject the whole time**:
  - never rack the focus away from it;
  - use a `sharp` band wide enough that the whole subject and its readout (thermometer, plate) stay crisp;
  - verify the subject's CoC is 0 for every frame.
- **Safe zone (text and HUD):** TikTok and Instagram cover the top band, the bottom band and the lower
  right-hand button column with their own interface. Captions, labels, counters, HUD panels and key action
  stay inside `reelkit.config.SAFE` and out of `SAFE_BUTTONS`: internal x 20–340, y 96–500, minus x ≥ 300
  below y 290 (output x 60–1020, y 288–1500, minus x > 900 below y 870). Measured from the user's own TikTok
  and Instagram screenshots of a posted reel (Oct 23); the first version used the ad margins and was far too
  strict. Scenery and the battle itself can fill the whole frame.
  - captions: `draw_captions` defaults put them at the top of the box, centred on the frame (x 180); big size
    only for lines up to 14 chars, small size up to 21;
  - HUD at the bottom (share bar, race lanes, roster): `share_bar` / `race_lanes` / `roster_grid` defaults sit
    at the bottom of the box (ending at y 500), running from x 20 to the button column (x 300), so they cover
    as little of the action as possible. Don't push them up into the arena;
  - the top scrim (`top_scrim()` defaults) sits behind the caption band; projects passing their own scrim
    numbers must move them to match;
  - contact sheets draw the zone (area outside tinted red); `python -m reelkit.safezone render.py` must pass
    before the full render.
- **Sound:**
  - everything is synthesised;
  - music is original or public domain (pre-1929, e.g. Grieg);
  - the science sound (chirp, tone, tick) sits clearly on top of the music, so check its band level;
  - incidental textures (e.g. sand rattle) stay very low or are left out.

## Working efficiently (same quality, less waiting)

Only 2 cores here, so time goes into wasted work, not hardware. #20 took ~2 h 10 min: ~36 min was two full
renders (the second only because the loop seam was found after rendering), ~20 min an 800-run seed search,
and much of the rest writing code other videos already had. Target: a new concept in ~60–75 min, a new episode
of an existing series (e.g. Battle of the Bands) in ~30–40 min.
- **One full render.** Everything checkable on stills is checked before it: contact sheets, safe zone, loop
  check, focus audit, audio levels. A full render is ~18 min; a second one is the biggest single waste.
- **Patch, don't redo.** The full render keeps its lossless raw frames (`frames_raw.rgb`), so a fix that
  touches only some frames re-renders just those: `python -m reelkit.video render.py audio.wav out.mp4
  --frames 41.5:44`. New audio only: `--only-mux`. Same for feedback rounds. Delete the raw (`--discard-raw`
  or by hand) once the video is approved.
- **Searches stop early.** Seed / parameter searches run in parallel and stop once ~5 candidates fit
  (`seed_search.py A B --stop 5`), instead of scanning hundreds; keep the ranked list in `check/` so the next
  episode can reuse it.
- **Reuse before writing.** Use the shared tools instead of copying a project's script: `reelkit.flashcheck`,
  `reelkit.loopcheck`, `reelkit.levels`, `reelkit.safezone`, `reelkit.sheet`, and shared scenery such as
  `reelkit.backdrops.VibrantVoid` (the vibrant void-of-space sky). Before building scenery or an engine, look
  in `projects/` for one to reuse (e.g. the cat in `count_the_thunder`, the stage in `battle_of_the_bands`,
  the battle engine `battle_of_the_bands/battle.py`).
  When you build something clearly reusable, move it into `reelkit/` in its own small commit.
- **Effects: use `reelkit.fx`** (BALLFIGHT v2, user Oct 24): attack lights in the fighter's colour (`Flares` -> the
  scene's `Lights`), 1-2 frame ink-starburst impacts (`Bursts`, bright ones capped at 3 / s), a little `Shake`, heat
  `shimmer` and local chromatic `fringe`, `Particles` with trails (per-instrument presets), fading canvas `Marks`, and
  ability `Moments` (<= 30 % eased dim + speed lines). No full-screen flashes (user rule); `reelkit.flashcheck` must pass.
- **Send stills early** (as now), so redirections happen before the expensive steps.

## Process

0. **Claim the video on the board:** in `docs/IDEAS.md`, set its row to `in production`, note the
   session, then commit and push that one-line change straight away. Ideas are drafted in the
   separate ideas chat, not in production sessions.
1. **Pick the idea:** the phenomenon, the hook line, how it makes music, and a 61 s beat sheet (see
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
   - check every caption, label and panel sits inside the safe box drawn on the sheet, then run
     `python -m reelkit.safezone render.py` (add `--sheet check/safezone.png` to see the worst frames);
   - fix things, then repeat.
7. **Audio:**
   - `python audio.py`;
   - `python -m reelkit.levels audio.wav` must PASS (opening within 1.5 dB of the ending, no window more than 3 dB
     below it except one deliberate dip, spread <= 5 dB, build <= +1.5 dB); the old "+3 dB build" rule is gone;
   - check that the key sound's frequency band sits above the music (`reelkit.audio.band_level`).
8. **Loop check, then the one full render:** `python -m reelkit.loopcheck render.py audio.wav` must PASS
   (seam, no reset in the last 5 s, no early copy of the opening, no audio click). Then
   `python -m reelkit.video render.py audio.wav out.mp4`. Run it in the background and wait for it. Don't
   poll with sleeps. Fixes after this: `--frames A:B` patches, not a second full render.
9. **Verify:**
   - run `ffprobe` (expect 1080×1920, 61.0 s, h264 + aac);
   - `python -m reelkit.loopcheck out.mp4` and `python -m reelkit.flashcheck out.mp4` must PASS;
   - make a contact sheet from the mp4 (`reelkit.video.frames_sheet`) at about 9–14 times;
   - look at it, including that no text or readout sits outside the safe box.
10. **Deliver:** the mp4, plus a short caption and tags for Instagram Reels, TikTok and YouTube Shorts.
11. **Update the board:**
   - set the row in `docs/IDEAS.md` to `review` or `done` with a one-line note and the output file name; details go in
     the idea file's history (`docs/ideas/NN-*.md`);
   - add the lessons to your project's log, `docs/log/<project>.md` (create it if new; `docs/LOG.md` is only the index);
   - commit and push (run `git pull --rebase` first, because other sessions edit the board too).

## Lessons from feedback

- Telephoto lenses make background objects look huge, so push them far back and fog them.
- Shadows only read on surfaces the camera can see. Low cameras see little ground, so put shadow
  drama where it shows: floors seen from above, grass banding, light shafts through silhouettes.
- Bloom must exclude fine detail that has to stay crisp (sand grains).
- Captions and HUD were being covered by the apps' own interface (user's TikTok screenshot, Oct 22: the hook
  sat under "LIVE / Following / For You / search", the timer and counter under the username and
  description). The old rules (captions in the top 150 px, up to 18 chars at the big size, HUD sliding in
  from the bottom) are replaced by the safe-zone rule in the brief.
- HUD panels stay clear of the subject and inside the safe zone; slide them in from the side or fade them,
  not up from the bottom of the frame.
- Safe zone v1 was too strict (user's screenshots of #22 on TikTok and Instagram, Oct 23): the share bar sat
  ~85 px (output) above where the app's UI starts and covered part of the battle, and both sides had unused
  room. The zone is now measured from real screenshots (see the brief).
- Keep backups of each approved version (`*_backup.py`) before a big relight, so you can blend or roll back.
- Openings were too quiet (user, Oct 26): the old levels rule demanded a +3 dB build, so sessions turned the first
  half down. Loudness is now consistent from frame 0 (see "Engaging the whole way"); `reelkit.levels` enforces it.
- The loop point is the file boundary (user, Oct 23, on #20 v1: the restart came ~1.5 s before the file
  ended). See the loop rule in the brief; `reelkit.loopcheck` flags any big jump in the last 5 s.

## Captions and tags

One short hook line, plus a "try it" line if relevant. Keep the tag sets separate:
- **Instagram:** about 12 tags
- **TikTok:** about 10 tags, including #fyp
- **YouTube Shorts:** a title + 1–2 sentence description + about 7 hashtags including #shorts
