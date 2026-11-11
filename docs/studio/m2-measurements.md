# BALLFIGHT Studio: milestone 2 measurements (the preview)

**In plain words.** With the final code, on this PC, the preview plays smoothly when the PC is not busy with other
heavy work: in every such run, 20 s of playback from FIGHT! showed every frame on time at 30 frames a second, both
for the first seed after the app starts and for the seeds after it. Beside a seed search run the way the app runs
it (on half the cores) it stayed at 99.7 to 100 % on time. After you pick a seed the picture is on screen in about
a quarter of a second, and playback starts from FIGHT!'s own frame at that moment; the sound joins about 7 s
later, in step with the picture. The app holds about 200 MB while it plays a take, rising to about 0.8 GB for a few
seconds while a take's sound is built. When other programs keep every core busy, the preview does stutter.
Measured on Nov 11, 2026 on the owner's PC (AMD Ryzen 7 9800X3D, 8 cores / 16 threads, 64 GB, Windows 11), from a
release build: `ballfight-studio --preview-bench` (Task 32), run from `studio/` as
`target\release\ballfight-studio.exe --preview-bench [--seed N] [--seconds S] [--threads T]`. The take is the
starter fight, FLUTE against TROMBONE at handicap 2.0 / 0.4, seed 255 (the first seed from 1 that fits the story),
made by the real engine through `LoadTake`. Every figure is the best and the median of three runs or more, unless
the line says otherwise. "PC busy" is the whole PC's CPU use over the second before the run (every core, every
program), read with Windows' processor counter. Two other workers were reading files on the PC at the time.

## 1. The budget and what was measured

The budget is the table "Performance budget for the preview" in the app's plan
(`2026-11-10-ballfight-studio-plan-m2-app.md`). `workers` is what the app gives the preview: the settings' default,
the PC's threads minus one, **15** on this PC (`default_settings(available_parallelism())`; the plan's table says 7
from an older estimate of 8 cores, but the code counts threads, not cores).

| Quantity | Budget | Measured (best / median) | Met? |
| --- | --- | --- | --- |
| Frames shown on time over 20 s from FIGHT! | at least 99 % | **100 % / 100 %** for the first take and for a later take (601 shown, 0 late, every run with the final code that began on a PC up to 49 % busy and was not loaded by others meanwhile, section 2); 99.7 to 100 % beside the app's yielding search; before ruling M2-R30 98.7 % (section 3) | **yes**, on a PC not saturated by other work |
| Threads the preview needs, `P = ceil(1.25 * frame_ms_p95 / 33.3)` | at most `workers` (15) | **5** from this run's `frame_ms_p95` (109.2 ms); 7 / 9 from milestone 1's 178 / 222 ms; 14 under a full search | **yes** |
| If `P > workers`: every second frame | the scheduler draws every n-th frame, the clock does not wait | not needed on this PC (stride 1 in every run at 15 threads); shown working with `--threads 1`: stride 4, the playhead ends exactly 20 s on | **yes** (the fallback works) |
| Draw-ahead | 12 frames ahead, 64 kept | lead 4 frames (quiet), 5 (search yielding), 11 to 12 (full search) | as designed |
| Scrub: the asked frame on screen | within `frame_ms_p95` + 20 ms | median **100.3 / 100.3 ms**, worst **113.3 / 113.7 ms**, against 108.6 / 109.2 + 20 ms (quiet runs before the ruling; since it, every run within its own `frame_ms_p95` + 20 ms, section 2) | **yes** |
| First picture after choosing a seed | one fight, one picture set-up, one frame | `TakePicture` 0.048 / 0.051 s; first picture given 0.225 / 0.247 s after the take is loaded (it is FIGHT!'s own frame; playback starts then) | **yes** |
| Sound after choosing a seed | joins when built (`soundtrack_build_s`) | `TakeSound` **6.99 / 7.01 s** | as designed |

## 2. The runs

`frame_ms` is the mean of the playback's last 30 draws (what the scheduler paces itself by); `frame_ms_p95` is the
95th percentile of the 50 scrub draws, each drawn alone (one frame on one thread, as milestone 1 measured it);
"play p95" is the 95th percentile of the playback's draws, up to 15 at once. "Late after start" counts only the
frames missed after the first picture. "To picture" and "to sound" are from `LoadTake` to `TakePicture` and to
`TakeSound`.

### With the final code (ruling M2-R30 and its fix round; 20 s, 15 threads)

Since ruling M2-R30 the player keeps the playhead on the start frame after a `Load` with autoplay, asks for that
frame first and then the frames right after it in order, and starts the clock (and the sound, if it has arrived)
when the start frame is given; the bench measures 20 s of playback from the moment the clock starts. Each bench run
now plays two takes: the **first take** (seed 255) on a new player, as the app's first seed after it starts, and
then a **later take** (seed 542, the next fitting seed) on the same player after it has played seed 255 for 3 s, as
every seed after the first: the player lives as long as the app, so its scheduler already knows its draw times
(lead 4 to 12). The fix round closed a gap that only this second case shows: before it, the frames between the
start frame and the lead were never asked for.

Other workers were building on the PC through all of these runs, so it was often fully busy. "Busy before" is the
second before the bench; "busy during" is the mean and the highest of one reading a second while the bench ran,
which includes the bench's own 15 drawing threads.

| Run | Busy before / during | First take: frame_ms, shown / late, on time, first picture, scrub median / worst (`frame_ms_p95`) | Later take: the same |
| --- | --- | --- | --- |
| 1 | 49 % / not read | 121.2 ms, 601 / 0, **100 %**, 251 ms, 107.9 / 125.0 ms (119.6) | 112.4 ms, 601 / 0, **100 %**, 228 ms, 106.3 / 124.9 ms (117.5) |
| 2 | 28 % / not read | 131.2 ms, 601 / 0, **100 %**, 258 ms, 108.6 / 127.6 ms (123.0) | 125.3 ms, 559 / 14, 97.6 %, 234 ms, 107.2 / 119.3 ms (117.3) |
| 3 | 27 % / not read | 175.4 ms, 601 / 0, **100 %**, 248 ms, 179.2 / 353.6 ms (251.1) | 121.2 ms, 598 / 0, **100 %**, 287 ms, 113.1 / 189.0 ms (138.6) |
| 4 | 78 % / 96 % | 641.6 ms, 62 / 540, 10.3 % (stride 2), 331 ms | 299.3 ms, 516 / 16, 97.0 %, 413 ms |
| 5 | 56 % / 81 % | 314.4 ms, 516 / 37, 93.3 %, 268 ms | 170.3 ms, 582 / 4, 99.3 %, 286 ms |
| 6 | 16 % / 40 % (max 70) | 123.6 ms, 601 / 0, **100 %**, 254 ms, 102.0 / 121.2 ms (113.2) | 120.1 ms, 601 / 0, **100 %**, 258 ms, 102.4 / 118.0 ms (112.0) |
| tests | 15 % / 15 % after | 114.6 ms, 601 / 0, **100 %**, 240 ms, 100.2 / 114.2 ms (108.9) | 104.1 ms, 601 / 0, **100 %**, 266 ms, 100.7 / 109.6 ms (105.4) |

- On a PC that was not loaded by others during the run (runs 1, 3, 6 and the tests' run) both takes showed all 601
  frames (108 to 708) on time, and every scrub came within its run's `frame_ms_p95` + 20 ms, except run 3's first
  take, whose scrubs ran while someone else's work started (worst 353.6 ms against 251.1 + 20).
- Run 2's later take (97.6 %) and runs 4 and 5 ran while other workers' builds took most or all of the PC: there
  playback misses, and in run 2 and run 4 even the bench's own 4 ms tick was starved, so some frames went by
  uncounted (shown + late below 601). That is the same limit as a search on every core, below.
- "tests" is `cargo test -p bf-app --release --lib preview:: -- --include-ignored` (80 passed), where
  `playback_meets_the_budget` and `scrubbing_meets_the_budget` check both takes of one shared run.

Beside `ballfight search` on 7 workers (the app's own search yields to half the workers while the preview plays),
re-run after the fix round. The first six tries (busy 64 to 100 % before, 92 to 100 % during, other workers'
builds on top of the search) are left out of the medians but listed; the last three started on a PC 15 to 29 % busy
before the search began:

| Run | Busy before (after the search started) / during | First take: frame_ms, shown / late, on time, scrub median / worst (`frame_ms_p95`) | Later take: the same |
| --- | --- | --- | --- |
| 7 | 56 % / 81 % | 182.1 ms, 601 / 0, **100 %**, 135.9 / 161.0 ms (149.3) | 143.5 ms, 601 / 0, **100 %**, 123.0 / 141.5 ms (129.3) |
| 8 | 60 % / 86 % | 180.5 ms, 601 / 0, **100 %**, 140.5 / 162.7 ms (158.3) | 173.5 ms, 601 / 1, **99.83 %**, 135.9 / 159.8 ms (151.6) |
| 9 | 62 % / 91 % | 192.5 ms, 599 / 2, **99.67 %**, 140.3 / 170.8 ms (157.1) | 180.3 ms, 598 / 0, **100 %**, 143.0 / 161.0 ms (152.8) |
| saturated tries | 64 to 100 % / 92 to 100 % | 68.0 to 100 % | 83.2 to 100 % |

With the search yielding, both takes met the 99 % budget in all three runs, and every scrub came within its
`frame_ms_p95` + 20 ms.

### After ruling M2-R30, before its fix round (first take only)

These runs used a new player for the one take, so the scheduler had no draw times yet and asked from the start
frame with a lead of 1: the fix round's gap could not show in them, and their first-take figures stand.

| Run | PC busy | frame_ms | play p95 | frame_ms_p95 | lead / stride | shown / late | on time | first picture | scrub median / worst | to picture / to sound |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 16 % | 114.9 | 188.6 | 216.4 | 4 / 1 | 601 / 0 | 100 % | 298 ms | 166.8 / 226.8 ms | 0.055 / 8.49 s |
| 2 | 18 % | 110.3 | 116.5 | 109.1 | 4 / 1 | 601 / 0 | 100 % | 247 ms | 99.5 / 112.2 ms | 0.049 / 8.98 s |
| 3 | 11 % | 107.2 | 116.1 | 109.8 | 4 / 1 | 601 / 0 | 100 % | 225 ms | 100.5 / 114.9 ms | 0.051 / 7.00 s |
| tests, together | 7 % | 117.2 | 250.1 | 229.0 | 4 / 1 | 601 / 0 | 100 % | 250 ms | 184.0 / 243.0 ms | |
| `playback_meets_the_budget` by name | 22 % | 136.2 | 195.4 | 150.9 | 5 / 1 | 601 / 0 | 100 % | 250 ms | 121.5 / 167.9 ms | |
| `scrubbing_meets_the_budget` by name | 55 % | 245.2 | 325.8 | 154.5 | 8 / 1 | 530 / 8 | 98.5 % | 245 ms | 116.7 / 171.1 ms | |
| `playback_meets_the_budget` by name | 100 % | 330.2 | 462.2 | 208.3 | 10 / 1 | 372 / 55 | 87.1 % | 470 ms | 151.8 / 256.9 ms | |

The run at 55 % busy showed 530 shown and 8 late: **98.5 % on time, below the 99 % budget** (that run checked only
the scrub budget, which held). At 100 % busy playback missed by much more (87.1 %; the test failed in that run and
passed when rerun at 22 %). Both runs had other workers' builds taking half or all of the PC; in both, the bench's
own 4 ms tick was starved, so frames went by uncounted (530 + 8 and 372 + 55 are below 601).
### Before ruling M2-R30: the clock ran from the `Load` (nothing else running, 20 s, 15 threads)

| Run | PC busy | frame_ms | play p95 | frame_ms_p95 | lead / stride | shown / late | on time | late after start | first picture | scrub median / worst | to picture / to sound |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 5 % | 108.3 | 111.2 | 110.4 | 4 / 1 | 594 / 8 | 98.67 % | 0 | 247 ms | 100.3 / 113.7 ms | 0.051 / 6.99 s |
| 2 | 12 % | 107.4 | 111.5 | 109.2 | 4 / 1 | 594 / 8 | 98.67 % | 0 | 241 ms | 101.1 / 116.4 ms | 0.048 / 7.01 s |
| 3 | 10 % | 106.4 | 111.9 | 108.6 | 4 / 1 | 594 / 8 | 98.67 % | 0 | 243 ms | 100.3 / 113.3 ms | 0.052 / 7.01 s |
| test run | 6 % | 106.3 | 112.1 | 109.8 | 4 / 1 | 594 / 8 | 98.67 % | 0 | 251 ms | 100.3 / 119.6 ms | |

The last row is `playback_meets_the_budget` / `scrubbing_meets_the_budget` (one shared run).

### Beside `ballfight search` (20 s, 15 threads; before ruling M2-R30)

`ballfight search fights\flute_vs_trombone.toml` (the release `ballfight.exe` from `bf-cli`), started 3 s before
the bench. The app's own search runs on half the workers while the preview plays (`engine/search.rs`,
`chunk_workers`), so "yielding" is the search on 7 workers; "full" is the search on all 15, as another program
would run it.

| Run | Search on | PC busy | frame_ms | play p95 | frame_ms_p95 | lead / stride | shown / late | on time | late after start | first picture | scrub median / worst | to picture / to sound |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 7 (yielding) | 51 % | 138.6 | 157.4 | 126.7 | 5 / 1 | 593 / 8 | 98.67 % | 0 | 271 ms | 116.3 / 135.4 ms | 0.053 / 7.51 s |
| 2 | 7 (yielding) | 51 % | 136.0 | 147.1 | 130.3 | 5 / 1 | 594 / 8 | 98.67 % | 0 | 260 ms | 117.8 / 134.9 ms | 0.061 / 7.48 s |
| 3 | 7 (yielding) | 53 % | 141.8 | 171.4 | 126.6 | 5 / 1 | 593 / 9 | 98.50 % | 0 | 294 ms | 116.7 / 134.6 ms | 0.065 / 7.66 s |
| 4 | 15 (full) | 97 % | 343.2 | 421.7 | 184.1 | 11 / 1 | 557 / 78 | 87.72 % | 66 | 372 ms | 162.6 / 222.2 ms | 0.085 / 10.73 s |
| 5 | 15 (full) | 97 % | 371.3 | 414.8 | 218.4 | 12 / 1 | 564 / 72 | 88.68 % | 62 | 314 ms | 167.5 / 266.3 ms | 0.084 / 10.10 s |
| 6 | 15 (full) | 97 % | 355.5 | 408.6 | 182.7 | 11 / 1 | 567 / 55 | 91.16 % | 45 | 341 ms | 164.4 / 188.5 ms | 0.072 / 10.01 s |

With the search yielding the preview meets the budget exactly as on a quiet PC (nothing late after the start;
scrubs within `frame_ms_p95` + 20 ms). Against a search on every core it does not: a draw then takes 340 to 370 ms
on average, its 95th percentile is 409 to 422 ms (12.3 to 12.7 frames), longer than the 12 frames the scheduler draws
ahead, so 45 to 66 frames come late, and the worst scrub (188 to 266 ms) can pass `frame_ms_p95` + 20 ms. `P` is
14 there, still within 15 workers, so the stride stays 1: the frames are late, not skipped.

### Other thread counts and a full play-through (before ruling M2-R30)

| Run | PC busy | frame_ms | lead / stride | shown / late | on time | late after start | first picture | drawn | playhead at the end |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `--threads 7`, 1 | 5 % | 106.9 | 4 / 1 | 597 / 6 | 99.00 % | 1 | 161 ms | 612 | 708 (20 s on) |
| `--threads 7`, 2 | 9 % | 105.8 | 4 / 1 | 597 / 6 | 99.00 % | 1 | 159 ms | 612 | 708 |
| `--threads 7`, 3 | 6 % | 105.7 | 4 / 1 | 597 / 6 | 99.00 % | 1 | 164 ms | 612 | 708 |
| `--threads 1`, 1 | 9 % | 99.5 | 3 / 4 | 184 / 569 | 24.4 % | 564 | 137 ms | 203 | 708 |
| `--threads 1`, 2 | 8 % | 99.4 | 3 / 4 | 183 / 572 | 24.2 % | 567 | 135 ms | 204 | 708 |
| `--threads 1`, 3 | 12 % | 99.2 | 3 / 4 | 186 / 568 | 24.7 % | 563 | 135 ms | 203 | 708 |
| `--seconds 61`, 1 | 15 % | 98.1 | 3 / 1 | 1,820 / 11 | 99.40 % | 3 | 237 ms | 1,841 | 107 (61 s on, looped) |
| `--seconds 61`, 2 | 11 % | 98.8 | 3 / 1 | 1,820 / 10 | 99.45 % | 3 | 232 ms | 1,841 | 107 |
| `--seconds 61`, 3 | 8 % | 98.3 | 3 / 1 | 1,820 / 11 | 99.40 % | 3 | 250 ms | 1,841 | 107 |

- With 7 threads the first picture comes sooner (160 ms against 245 ms with 15), because fewer fresh draws start
  at once, so 6 frames are late instead of 8: 99.0 % on time, just on the line.
- With one thread (`P` = 4 is more than 1) the scheduler draws every 4th frame and holds it, and the playhead still
  ends exactly 600 frames on: the clock never waits for a frame. "Late" counts every frame that was not drawn, which
  is what the stride means; the picture moves at about 7.5 frames a second there.
- Over the whole video (61 s, looping once past the end and through the title card) 3 frames are late after the
  start, in all three runs; where in the loop they fall was not located. A run as long as the video draws every
  frame, so there is nothing left to scrub to and those runs have no scrub figures.

These runs were made before the change, so their late counts include the start's misses ("late after start" is
what playback missed afterwards); the ruling changes only the start.

## 3. The start, and ruling M2-R30 (closed)

**Before the ruling** the clock started when the take was loaded. The take's first draws then take about 245 ms: on a fresh picture the
first 12 to 14 frames are all drawn at once and all finish at about the same moment (in one diagnostic run, 14
draws of frames 109 to 122 finished between 245 and 251 ms after the load, whatever their start), against 100 to
110 ms a frame from then on. In that quarter of a second the playhead passes 8 frames (FIGHT!'s own frame 108 is
never asked for at all: while playing, the scheduler asks from the playhead plus the lead). Every quiet and yielding
run has all of its late frames before the first picture and none after it.

**Ruling M2-R30** settled it the first way proposed: playback starts with the take's first picture (its fix
round added that the frames right after the start frame are asked for in order, whatever the lead).

- After a `Load` with autoplay the transport shows playing, the stage shows nothing yet (`has_picture()` false),
  and the playhead stays on the start frame. The scheduler is asked for that frame first, then the frames right after
  it in order, up to 12 ahead (`Scheduler::want_start`; with the clock stopped no frame is too close).
- When `tick` gives the start frame, the clock starts from it. A soundtrack that arrived meanwhile starts then, at
  the start frame's first sample, so the device's count from there is the playhead and picture and sound are in
  step. Nothing counts late while waiting, and the 500 ms stall rule cannot fire, since the sound has not started.
- The wait is bounded: a start frame whose draw fails ends it at once (the failed-frame status shows), and after
  3 s (`FIRST_PICTURE_WAIT`) the clock starts anyway.
- Pausing and then playing during the wait starts the clock at once, as any play from pause does.
- The result is in section 2: 100 % on time on a PC not loaded by others, for the first take and the later ones,
  and the first frame shown is FIGHT!'s own.

## 4. Frame time against milestone 1

| | Milestone 1 (`bench --only frame`, PC 32 to 83 % busy) | Here, quiet (5 to 12 %) | Here, search yielding (51 to 53 %) | Here, full search (97 %) |
| --- | --- | --- | --- | --- |
| one frame alone, 95th percentile (`frame_ms_p95`) | **178 ms** best, 222 ms median | **108.6 / 109.2 ms** | 126.6 / 126.7 ms | 182.7 / 184.1 ms |
| `P` | 7 / 9 | 5 | 5 | 7 |
| one frame among 15 at once (mean of the last 30) | | 106.4 / 107.4 ms | 136.0 / 138.6 ms | 343.2 / 355.5 ms |

The frames are the same as milestone 1's (the preview shows `Picture::frame_reported`, as the render does); the
difference is how busy the PC was. With 15 threads `P` stays within `workers` in every case measured here, so the
preview never needs its every-second-frame fallback on this PC, and milestone 1's "Speed plan" stages (the encode,
the lens's FFT columns, the two per-pixel passes) are not needed for the preview; they stay the render's.

## 5. Takes and memory

From Task 31 (release test builds, seed 1 at 2.0 / 0.4, 4 engine workers, other lanes building on the PC):

| | Alone | Under load (other engine tests running) |
| --- | --- | --- |
| `LoadTake` to `TakePicture` | 0.06 s | 0.20 to 0.29 s |
| `LoadTake` to `TakeSound` | 6.4 s | 10.0 to 10.9 s |
| Back to one of the last three takes | both events in 0.012 to 0.020 s | |

Here (the bench's own take, seed 255, 15 engine workers): `TakePicture` after 0.048 / 0.051 s and `TakeSound` after
6.99 / 7.01 s on a quiet PC; 0.053 / 0.061 s and 7.48 / 7.51 s with the search yielding; 0.072 / 0.084 s and 10.01
/ 10.10 s beside a full search.

Memory (the process's private bytes, read every 250 ms from outside the bench):

| | Measured |
| --- | --- |
| Three kept takes, 31 frames drawn each (Task 31) | about 170 MB, about 57 MB a take |
| While a take's sound is built | peaks at 804 to 851 MB (every run here; Task 31: 600 to 940 MB) |
| Playing one take for 20 s (the take, 64 kept frames of 0.7 MB, 15 drawing threads) | 160 to 219 MB |
| Playing one take through the whole video, every frame drawn (61 s, three runs) | 164 to 274 MB while playing, 196 to 201 MB at its end; no steady rise |

The bench holds one take (the engine that made it has ended and its kept takes with it), without its sound. In the
app, add the kept takes (about 57 MB each, three at most) and the playing take's soundtrack (23 MB): about 0.4 GB
while playing, about 1 GB for the few seconds a take's sound is built. The picture's caches did not grow over a full
play-through.

## 6. The app on the real engine (Task 35)

Measured on Nov 11, 2026 on the same PC, from release test builds, while three other workers were building and
testing on it. "Busy" is the whole PC's CPU use read just before (and after) the run.

### Test counts

`cargo test --workspace --release --no-fail-fast` (busy 42 % before, 54 % after; 612 s): **1,919 passed, 0 failed,
21 ignored**. For `bf-app`: the unit tests inside each file 493 passed and 7 ignored; the command line 3; the saved
files of older versions (`tests/compat.rs`) 8; the interface program (`tests/ui`) 347 passed and 1 ignored (the
whole-video test). The release engine tests with the ignored ones (`--lib engine:: -- --include-ignored`): 93
passed. The interface program run ten times in a row (release, PC 20 to 41 % busy): ten exits with code 0, 347
passed each time, 157 to 233 s a run.

### The whole video by clicks (`real::a_whole_video_with_the_real_engine`)

Balance (auto-balance from 1.00 / 1.00 to 1.94 / 0.39), search, the first shortlist row whose take passes all three
Music checks (it was row 1, **seed 469**, in every run), Render, the queue, and the finished folder checked
(`flute_vs_trombone.mp4` of 61.01 s by ffprobe, `cover.png`, `caption.txt` starting with the drafted caption, the
fight file with seed 469). 15 engine workers (the settings' default here).

| Run | Busy before | Whole test | Slowest update (`slowest_update_ms`) | Result |
| --- | --- | --- | --- | --- |
| 1, before the interface thread was raised | about 60 % | 123.2 s | 45.8 ms | every file check passed; the 20 ms bound failed |
| 2, the same, while finding where the time went | 50 to 60 % | 116 to 122 s | 103 to 109 ms (the timing printout itself added to it) | as run 1 |
| 3, interface thread raised | 38 % | 134.3 s | 30.5 ms | every file check passed; the 20 ms bound failed |
| 4, the same | 100 % | 124.8 s | 55.1 ms | the same |
| 5, the same | 93 % | 168.0 s | 37.4 ms | the same |

**Where the slow updates are.** Almost all of them are in `Player::tick` while a video renders and the preview
plays at the same time on the Render screen: the preview's 15 drawing threads, the render's 15 and ffmpeg's want
more cores than the PC has, on top of other workers' builds. In a timed run before the change below, 33 of the 84
updates over 8 ms on Render took more than 20 ms (up to 74 ms). Since the interface thread runs above the app's
other threads (`app::interface_first`, Windows `THREAD_PRIORITY_ABOVE_NORMAL`), two timed runs had none and one
such update; what remains is `tick` waiting for the preview scheduler's lock while a drawing thread that holds it waits for a core. The bound was not
loosened. Making the interface never wait on that lock (or giving the drawing threads the same priority) is in
`preview/scheduler.rs`, outside this task, and is proposed for a ruling.

### The preview budgets, run again by name

`playback_meets_the_budget` and `scrubbing_meets_the_budget`, one at a time: at 45 % busy, scrubbing passed and
playback failed (first take 82.5 % on time with `frame_ms` 188 ms; later take 99.8 %); at 100 % busy both failed
(91.0 % / 89.1 %). Section 2 has their passing runs on a quieter PC; nothing they measure changed in this task.
`this_pc_plays_silence` passed.
