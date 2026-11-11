# BALLFIGHT Studio, milestone 6 (packaging): measurements

Measured on the reference PC (AMD Ryzen 7 9800X3D, 16 logical cores, Windows 11), release builds, with the pinned
ffmpeg 9.0.2 (`studio/packaging/ffmpeg.toml`, the copy in `%LOCALAPPDATA%\BALLFIGHT Studio\data\downloads`, named by
`BALLFIGHT_FFMPEG`).

## 1. The faster encoder setting (Task 4, decision D4)

**What was measured.** The whole render of the example episode (FLUTE against TROMBONE, `--rec
reference/fights/flute_vs_trombone/ep392.bfrec`), all 15 checks on, 15 workers, with `ballfight bench ... --only
render --out <folder> --encode <setting> --json`, once with the series' own setting (`best`, x264 `slow`) and with
each candidate preset of the plan: `medium`, `fast`, `faster` and `veryfast`. The candidates ran from a one-off
measuring build in which the Faster setting took its preset from an environment variable; that hook was never
committed. Three rounds, each round all five settings one after the other (so a busy stretch of the PC falls on every
setting rather than on one), 15 whole renders in all, about 21 minutes. Only the preset differs between them; CRF 18,
the profile, `tune animation`, `mbtree=0`, the forced keyframes, the sound and the container are the same.

**How busy the PC was.** Other workers were building and testing on this PC the whole time. `[n %]` is the whole PC's
CPU use over the second before each render (every core, every program), which `bench` measures itself; it was 60 to
100 % before every run, and in round 1 nineteen compiler processes were running when the `slow` run started. So the
times below are **not a quiet-PC figure**, and the run-to-run spread (up to 41 s for `slow`) is larger than some of
the differences between presets. Milestone 1 measured the series' own setting at 81.4 s best and 82.9 s median on a
PC 40 to 69 % busy (`m1-measurements.md`, section 2); this time it took 94.9 s at best.

| Setting (preset) | Whole render, runs 1 / 2 / 3 (s) [PC busy before] | Best | Median | Frames stage, runs 1 / 2 / 3 (s) | mp4 size | Checks |
| --- | --- | --- | --- | --- | --- | --- |
| best (`slow`), the default | 136.1 [100 %] / 94.9 [87 %] / 105.2 [81 %] | 94.9 | 105.2 | 93.6 / 67.4 / 67.3 | 59,332,539 bytes (59.3 MB) | 15 of 15, every run |
| `medium` | 71.9 [70 %] / 76.1 [66 %] / 83.8 [98 %] | 71.9 | 76.1 | 45.7 / 49.0 / 39.8 | 67,069,441 bytes (+13 %) | 15 of 15, every run |
| **`fast`** | **57.6 [60 %] / 64.1 [99 %] / 61.8 [100 %]** | **57.6** | **61.8** | 33.0 / 34.6 / 34.6 | 75,355,837 bytes (+27 %) | 15 of 15, every run |
| `faster` | 54.0 [88 %] / 68.9 [83 %] / 70.0 [65 %] | 54.0 | 68.9 | 27.9 / 39.2 / 39.2 | 74,435,767 bytes (+25 %) | 15 of 15, every run |
| `veryfast` | 69.4 [69 %] / 57.6 [94 %] / 77.8 [100 %] | 57.6 | 69.4 | 37.3 / 29.3 / 41.6 | 84,286,789 bytes (+42 %) | 15 of 15, every run |

Each setting's mp4 had the same SHA-256 in all three of its runs (the default's is `308fb903...c169`, the one
milestone 1 recorded, so the default video is byte for byte what it was). Sizes are exact; the percentages are against
the default.

**The rule** (plan, Task 4): "Faster" is the slowest preset whose median whole render is 60 s or less and which passes
all 15 checks in every run; if none reaches 60 s, the one with the shortest median that passes every check.

**The choice: `fast`.** No candidate's median reached 60 s on the PC as busy as it was, so the second half of the
rule applies, and `fast` has the shortest median (61.8 s). The choice does not rest on noise alone:

- `fast` is the only candidate whose three runs agree closely (57.6 to 64.1 s; its frames stage 33.0 to 34.6 s).
- From `fast` on, the encode no longer holds the frames stage back: the frames stage of `faster` and `veryfast` is
  not reliably shorter (27.9 to 41.6 s), because the drawing (about 23 s on a quiet PC) is then what the stage waits
  for. The quicker presets buy no measurable time and cost file size (`veryfast` +42 %).
- `medium` is clearly slower (76.1 s median) than `fast` in the same rounds.

**The 60 s goal: not met on this measurement.** With Faster (`fast`) the whole render took 57.6 s at best and 61.8 s
median on a PC 60 to 100 % busy; the series' own setting took 94.9 s best and 105.2 s median in the same rounds. On a
quieter PC Faster is expected under 60 s (the default ran about 20 s quicker in milestone 1 than here), but that was
not measured, so this document does not claim it.

**The saving, for the Settings line (Task 13).** Median against median in the same rounds, 105.2 - 61.8 = 43.4 s; best
against best, 94.9 - 57.6 = 37.3 s. Rounded to the nearest 5 s: **about 40 seconds a video** (between the two).
Faster's file is **about a quarter larger** (75.4 MB against 59.3 MB), which is more than "slightly".

**Re-measuring.** At a quiet moment, three runs each of `best` and `fast` (about 8 minutes) would give the quiet-PC
figure and settle the 60 s question; changing the preset later is the constant `FASTER_PRESET` in
`studio/crates/bf-export/src/encode.rs` and a re-bless of the `clip0-90-faster.mp4` line of
`studio/replay/encode_hashes.txt` (`BF_BLESS=1 cargo test --release -p bf-cli --test e2e -- --exact
the_faster_clip_is_pinned` with the pinned ffmpeg).

**Pinned.** The 3 s clip `--frames 0:90 --encode faster`, encoded by the pinned ffmpeg, is
`clip0-90-faster.mp4 7f6d68fa93daa73f2bc75b3d531a6668280dfe8bbd687ff320a1d6c6f4c3b139` in `encode_hashes.txt`; the
default clip's line (`clip0-90.mp4 2a647d29...a07b`) is unchanged.
