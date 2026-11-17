# Engine 2 E5: sound, HUD and VS card on the dummy's video

Measured on 2026-11-17 in the cloud (Intel Xeon @ 2.10 GHz, 4 logical cores, lavapipe as the GPU, release build),
not on Aedan's PC. The fight is `fights/engine2/test_dummy.toml` (TEST DUMMY vs TEST DUMMY 2, seed 26, big_top,
Composed). Its K.O. comes at 48.01 s of fight and 48.85 s of video; the director's slow-mo and hitstop add 3.19 s
in all.

- Engine 2's numbers: `cargo run --release -p bf-export --example engine2_e5 -- <folder> --checks`.
- The current engine's soundtrack: `ballfight bench fights/flute_vs_trombone.toml --only sound`, three runs on the
  same machine just after.

## Soundtrack build

| What | Time (best of 3) |
| --- | --- |
| Current engine, FLUTE vs TROMBONE (`build_soundtrack`) | 7.06 s (7.06 to 7.55) |
| Engine 2, `music.slowmo = "steady"`: one build (the video clock) | 7.06 s |
| Engine 2, `music.slowmo = "follow"`: two builds (the video clock, then the fight clock warped to the K.O. and spliced) | 11.50 s |

One build costs what the current engine's does. Follow's second build adds 63 %. Nothing in E5 asked for a target
here. On the PC the current engine's build is 3.57 s (E0), so follow should take about 6 s there.

## UI layer and frames

| What | Value |
| --- | --- |
| UI layer (`Ui2::draw`: count-in, HUD, captions, VS card, the matte at 360 x 640) | 1.38 ms per frame (mean of the 244 safe-zone frames) |
| GPU frame with the overlay, quiet stretches (VS card, count-in) | 0.47 to 0.72 s |
| GPU frame, a special's impact (frame 379) | 3.18 s |
| GPU frame, around the K.O. (debris, motion blur in the slow-mo) | 15 to 60 s (frame 1480: 53 s) |
| The 180 frames the loop check reads (the outro and the opening) | 66.5 s for all the checks, about 0.37 s a frame |
| A 4 s clip (frames 345 to 465, the first special) | 282 s of frames, about 2.3 s a frame |
| A 1.5 s clip across the K.O. (frames 1450 to 1495) | 1,036 s of frames, about 23 s a frame (3 s before the K.O., 30 to 45 s in the slow-mo after it) |

On lavapipe the K.O. stretch dominates: a 10 s clip around the K.O. was started (frames 1380 to 1680) and stopped
after 15 frames in 8 minutes, at a slowing rate of 40 to 65 s a frame. That is why the full video, which took more
than 2 h in E4, belongs on the PC's GPU. The UI layer adds about 0.3 % of a quiet frame.

## The checks before the render (the whole video)

| Check | Result | Numbers |
| --- | --- | --- |
| safezone | pass | 4,419 positions in 244 frames, 0 outside, 0 overlaps |
| levels | **fail** | dips 2 (8 s at −3.2 dB, 50 s at −4.5 dB), spread 6.31 dB (5 allowed), ending −13.49, opening −14.59, build 0.82 |
| band | pass | TEST DUMMY 11.8 to 15.9 dB above the bed while it plays, TEST DUMMY 2 11.2 to 16.3 dB |
| loudness | pass (shown) | intro −11.10 LUFS, fight −11.70 LUFS, +0.60 LU |
| loop_picture | **fail** | a jump 124 times normal 0.27 s before the end (big_jumps 27, seam 0.60, normal 0.50) |
| loop_audio | pass | last and first 20 ms 2.9 dB apart (6 allowed), jump 0.017 |
| focus | pass | both balls in the picture on 100 % of frames, centres in the safe zone on 100 % |
| ko_window | **fail** | K.O. at 48.85 s of video; the window is 49 to 53 s |

There is no flash check on Engine 2. The job refuses to save a whole video when a check fails, as the current
engine's does. **On this seed three checks fail, so `ballfight render` of the whole dummy video saves nothing.**
The review renders below therefore ask for every frame as a clip (`--frames 0:1830`). A clip leaves out the
checks before the render, the cover and the fight file, and it keeps the whole soundtrack.

- `ko_window` and `levels` belong to the seed and the dummy's score. The dummy's K.O. lands 0.15 s early, and its
  music dips around the count-in and the K.O.
- `loop_picture` reads a cut in the last 0.3 s (the outro's reset). E4's director does not yet place it on the loop.

None of these are E5 bugs. They are for the review.

## The sound at the K.O. (from the two soundtracks)

| | Follow | Steady |
| --- | --- | --- |
| Level 46 to 48.8 s | −14.8 dB | −14.3 dB |
| Level 48.85 to 50 s | −13.8 dB | −13.8 dB |
| Level 50 to 52 s | −17.5 dB | −17.6 dB |
| Largest sample step within 10 ms of the K.O. | 0.49 | 0.29 |
| Mean sample step in the second before | 0.027 | 0.058 |

The largest step at the K.O. is the K.O. hit itself in both builds. The splice test (`soundtrack2`) bounds the
seam against the video build's own steps. Follow's music is darker and slower in the second before the K.O. (half
the mean step of steady), which is the slow-mo pitch drop.

## Stills and audio for the review (in `/mnt/project-files/ballfight-studio/engine2/e5/`)

- `e5_vs_card_f43.png`, `e5_count_in_f84.png`, `e5_special_caption_f379.png` and `e5_ko_f1480.png`: 1080 x 1920,
  lossless, with the UI layer.
- `e5_special_clip_f345-465.mp4`: 4 s around the first DUMMY BUSTER, with its sound. The job's clip carries the
  whole 61 s soundtrack from 0 s, so the sound was put back in sync with ffmpeg, starting at 11.5 s.
- `e5_ko_clip_f1450-1495.mp4`: 1.5 s across the K.O. (frame 1465) with its sound (follow), re-synced the same way, starting at 48.33 s.
- `e5_soundtrack_follow.m4a` and `e5_soundtrack_steady.m4a`: the whole soundtrack of each.
- `e5_ko_10s_follow.m4a` and `e5_ko_10s_steady.m4a`: 44 to 54 s, for an A/B listen across the K.O.

## The full-length review renders (on the PC)

From `studio/` on the PC, release build, with the PC's GPU:

```
cargo build --release -p bf-cli
target\release\ballfight.exe render fights\engine2\test_dummy.toml --out <folder> --frames 0:1830
```

For the steady copy, put `slowmo = "steady"` on its own line after `music_seed = 1` under `[music]` in a copy of
the file, saved as `fights\engine2\test_dummy_steady.toml`. Then run:

```
target\release\ballfight.exe render fights\engine2\test_dummy_steady.toml --out <folder> --frames 0:1830
```

Each makes `test_dummy_vs_test_dummy_2_frames0-1830\test_dummy_vs_test_dummy_2_frames0-1830.mp4`. Use a different
`--out` for each copy, because both copies have the same folder name. `--frames 0:1830` is needed only while the
seed fails `levels`, `ko_window` and `loop_picture`. Without it, the render stops at the checks and saves nothing.

## For the review at the end of E6

- Should slow-mo music stay `follow` (the default: the score slows and drops in pitch with the picture, then
  splices back at the K.O.) or default to `steady` (the score plays on at tempo through the slow-mo)? Compare
  `e5_ko_10s_follow.m4a` with `e5_ko_10s_steady.m4a`, and the two full renders.
- The dummy's seed fails three checks (above): a new review seed, or a K.O. window that allows the slow-mo's
  shift?
- The special and the K.O. wash the frame almost white under the effects. The HUD's HP numbers then float on a
  white field.
