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
  not reliably shorter (27.9 to 41.6 s), because the drawing (about 23 s for the frames of that stage, as measured in
  `m1-measurements.md`, section 2) is then what the stage waits for. The quicker presets buy no measurable time and cost file size (`veryfast` +42 %).
- `medium` is clearly slower (76.1 s median) than `fast` in the same rounds.

**The 60 s goal: not met on this measurement.** With Faster (`fast`) the whole render took 57.6 s at best and 61.8 s
median on a PC 60 to 100 % busy; the series' own setting took 94.9 s best and 105.2 s median in the same rounds. The
default ran 13 to 22 s quicker in milestone 1 than here (best against best 81.4 against 94.9 s; median against median
82.9 against 105.2 s), so Faster may be under 60 s on a quiet PC; that was not measured.

**The saving, for the Settings line (Task 13): no number of seconds yet.** On this busy PC Faster took about 60 % of
the default's time (median 61.8 against 105.2 s, best 57.6 against 94.9 s). That ratio is all these runs support; a
figure in seconds is added only after a quiet-PC re-measure (three runs each of `best` and `fast`, median against
median, rounded to the nearest 5 s), which the controller schedules. Until then the Settings description is: "Best
quality is the series' own setting. Faster makes the video sooner and the file about a quarter larger." Faster's file
is about a quarter larger (75.4 MB against 59.3 MB, +27 %).

**Re-measuring.** That quiet-PC run (about 8 minutes) gives the figure in seconds and settles the 60 s question;
changing the preset later is the constant `FASTER_PRESET` in
`studio/crates/bf-export/src/encode.rs` and a re-bless of the `clip0-90-faster.mp4` line of
`studio/replay/encode_hashes.txt` (`BF_BLESS=1 cargo test --release -p bf-cli --test e2e -- --exact
the_faster_clip_is_pinned` with the pinned ffmpeg).

**Pinned.** The 3 s clip `--frames 0:90 --encode faster`, encoded by the pinned ffmpeg on this PC (24 x264 threads),
is `clip0-90-faster.mp4 threads=24 7f6d68fa93daa73f2bc75b3d531a6668280dfe8bbd687ff320a1d6c6f4c3b139` in
`encode_hashes.txt`; the default clip's line (`clip0-90.mp4 threads=24 2a647d29...a07b`) is unchanged. Each setting
also has a line for 6 threads; section 4 says why.

## 2. The package

Measured on 11 Nov 2026 (Task 11) with `ballfight-pack package --exe studio\target\release\ballfight-studio.exe
--out <a scratch folder>`, from `lane/app-c` at version 0.1.0, the release exe built from that checkout, and the
pinned ffmpeg 9.0.2 from the PC's cache (`%LOCALAPPDATA%\BALLFIGHT Studio\data\downloads\9.0.2-759d0a9831c4`).
Making the package took 5 s (it reads and checks every file, then writes and checks the copies). The tree has no
`studio/packaging/guide` yet (Task 17), so the package holds no `guide\` folder.

| File | Bytes |
|---|---:|
| `ballfight-studio.exe` | 141,795,173 |
| `ballfight.ico` | 7,887 |
| `ffmpeg\ABOUT.txt` | 1,027 |
| `ffmpeg\LICENSE.txt` | 35,147 |
| `ffmpeg\ffmpeg.exe` | 227,823,104 |
| `ffmpeg\ffprobe.exe` | 227,621,376 |
| **Total (6 files, plus `MANIFEST.txt`)** | **597,283,714** (about 570 MB) |

- `MANIFEST.txt` itself is 602 bytes; the install adds `update.toml` (under 1 KB).
- The exe is a release build with line tables (`[profile.release] debug = 1`); ffmpeg and ffprobe are 76 % of the
  package. Without ffmpeg (what a download route would fetch on each update, D1 B) the package is about 142 MB.
- A trial install of this package under a scratch folder (`install --root`, its source copy cloned from the lane's
  local `origin`, no network) took 14 s in all and left 722 MB there (the program folder plus the source copy).

## 3. How long an update takes (Task 12, route A)

Measured on 11 Nov 2026 by the ignored test `update::git::tests::a_real_update_on_this_pc` (run by name, 7 min 51 s
in all), from `lane/app-c` at version 0.1.0, everything under one temp folder
(`%TEMP%\bf-app-update-tests\real-update-<pid>-0`) that the test made and removed. The origin was a bare clone of the
lane (a local path, so the fetch used no network); the app's copy was a `--no-checkout` clone of it, as the install
makes. **The PC was busy:** other lanes were building and testing at the same time (processor 43 % to 100 % busy in
the samples taken just before), so these are upper figures; a quiet PC should do better.

| Step | Time | Notes |
|---|---:|---|
| Look for a newer version (fetch the tags, read them) | 0.5 s | local origin; over the internet add a few seconds |
| Cold build (`cargo build --release --locked -p bf-app`, empty build folder) | 354 s (5 min 54 s) | toolchain and crates already on the PC |
| Warm build after a one-line change in `bf-app` | 74 s | |
| Staging (package into `.next`, 633 MB with ffmpeg) | 4.5 s | ffmpeg files of the real sizes; pin unchanged, no download |
| Checking the staged package (every file's SHA-256) | 0.5 s | plus the exe's `--version`, under 1 s |
| The swap at restart | 1.0 s | |

- The build folder (`data\build`) held **2.2 GB** after both builds; the plan expected 5 to 10 GB, which would only be
  reached with several toolchains or profiles. The release exe was 175.8 MB in this build.
- The first update on the owner's PC is a cold build (about 6 minutes, longer if rustup must fetch a new toolchain or
  cargo new crates); later updates with small changes in the app take 1 to 2 minutes, more when a change reaches the
  crates below `bf-app`. Settings' "It takes about {m} minutes" uses the warm build rounded up: **2**.
- `--version` printed nothing in this build: Task 9 wires it into `main`. Until then a staged real build fails its
  check, in words, as it should.
- The unit tests' stand-in project builds, stages and checks in a few seconds, with a build folder of a few MB.

## Quiet-PC run of the two encoder settings (12 Nov 2026)

Six whole renders of the example episode (`fights/flute_vs_trombone.toml` with `ep392.bfrec`), Best quality and
Faster by turns, three each, with no build or test running beside them. The PC was 23 to 41 % busy before each
run (other programs on the PC, not this build). Times are wall-clock for the whole `ballfight render` command.

| Setting | Runs (s) | Median | mp4 size | Checks |
|---|---|---|---|---|
| Best quality (x264 `slow`) | 77 / 73 / 76 | 76 | 59.3 MB | 15 of 15, every run |
| Faster (x264 `fast`) | 55 / 52 / 55 | 55 | 75.4 MB (+27 %) | 15 of 15, every run |

- Faster meets the 60 s goal on this PC; Best quality does not (73 s at best).
- The saving, median against median, is 21 s: "about 20 seconds a video" is the figure the Settings description may
  state. The three Best quality files are identical to each other, and so are the three Faster files.

## 4. What the pinned encode proves: the same build and the same x264 thread count (12 Nov 2026)

**What happened.** The first CI run of `the_encode_is_pinned` and `the_faster_clip_is_pinned` on GitHub's hosted
Windows runner failed with the pinned ffmpeg (fetched and checked by `ballfight-pack fetch-ffmpeg`): the runner's
clips were `871ced7a...` (Best) and `c587dd19...` (Faster), not this PC's `2a647d29...` and `7f6d68fa...`.
`render_is_reproducible` passed on the runner, so the sound and the frames (blessed in `render_hashes.txt` on this PC)
were the same bytes there.

**The cause: x264's thread count.** No `-threads` is given, so x264 picks its own: 1.5 times the logical processors
the process may use (its affinity mask), and its output bytes depend on that number (frame threads change its
look-ahead and rate-control decisions). The output is the same every time for a given number. Shown on this PC
without changing the product: the same `ballfight render ... --frames 0:90` started with `start /affinity`, which
ffmpeg inherits.

| Logical processors allowed | x264 `threads=` (read from the settings x264 writes into the stream) | Best clip | Faster clip |
|---|---|---|---|
| 16 (all of this PC) | 24 | `2a647d29...a07b` (the blessed line) | `7f6d68fa...b139` (the blessed line) |
| 4 (mask `F`) | 6 | `871ced7adbf1d1ef9a22657d599924501c2be15cc3f14b985e41d42192f3f7ac` = the runner's | `c587dd1978b4f3774291336e3bfc67e2737a17af0ba78ec517c33434f663d60b` = the runner's |
| 2 (mask `3`) | 3 | `eca0a3ee96a8ce815f238f2de33c9d3abfb1967537d7b32539da385dd4553584` (no second machine; not pinned) | not run |

Ruled out:
- **The CPU's instruction sets.** The runner's CPU is not this AMD one, yet this PC limited to 4 processors gives
  the runner's exact bytes.
- **The sound.** The audio stream alone (`-map 0:a -c copy -f md5`) is the same at 4 and at 2 processors; only the
  video stream differs.
- **The container** (a creation time, an encoder string). The files differ only where the video stream differs, and
  the 4-processor file is the runner's file byte for byte.
- **What is fed to the encoder.** `render_is_reproducible` compares the sound and frames 0, 500 and 1700 with
  `render_hashes.txt` (committed in `fb1504b`, blessed on this PC), and it passes on the runner.

**What the pinned encode proves now.** The same ffmpeg build with the same number of x264 threads makes the same
file. `encode_hashes.txt` holds one line per clip for each thread count (`clip0-90.mp4 threads=24 <sha256>`). The
test reads the count x264 used from the clip it made and compares the clip only with that count's line. These lines
are pinned: 24 threads (this PC) and 6 threads (any 4-processor machine, such as the CI runner). The 6-thread lines
are confirmed by two machines: the runner and this PC limited to 4 processors. When a machine's count has no line,
the test prints "not compared: no pinned line for N threads". Under `BF_NEED_PINNED_FFMPEG=1` (CI) that is a failure
whose message starts with the whole line the clip gave, so that line can be checked and added.

**What it does not prove.** A video made on a PC with another number of processor threads is not byte-identical,
even though every frame and every sound sample fed to the encoder is. The default encode arguments are unchanged
(`the_arguments_are_the_kits`), so the series' videos keep their bytes on this PC. Giving x264 a fixed `-threads`
would make the bytes the same on every PC, but it changes the kit's encode arguments and the encode's speed on PCs
with fewer or more processors. It is not done here; it would be the owner's choice.
