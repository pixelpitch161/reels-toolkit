# BALLFIGHT Studio: milestone 2 measurements (the preview)

**In plain words.** On this PC the preview keeps up with the video: once the first picture is up, every frame
arrives on time, at 30 frames a second, with no frame skipped, also while a seed search runs the way the app runs
it (on half the cores). What misses the target is the start: a fresh take's first picture takes about a quarter of
a second to draw, and the clock is already running, so the first 8 frames after FIGHT! are not shown (98.7 % on
time where the plan asks for 99 %). After picking a seed the picture is ready in about 0.05 s and on screen in
about 0.25 s; the sound joins about 7 s later. The app holds about 200 MB while it plays a take (one take, with
its frames), rising to about 0.8 GB for a few seconds while a take's sound is built.

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
| Frames shown on time over 20 s from FIGHT! | at least 99 % | **98.7 % / 98.7 %** (594 shown, 8 late, every quiet run); 98.5 to 98.7 % with the search yielding | **no**: all 8 late frames are the first 8, before the first picture (section 3); after it, **0 late** in every quiet and yielding run |
| Threads the preview needs, `P = ceil(1.25 * frame_ms_p95 / 33.3)` | at most `workers` (15) | **5** from this run's `frame_ms_p95` (109.2 ms); 7 / 9 from milestone 1's 178 / 222 ms; 14 under a full search | **yes** |
| If `P > workers`: every second frame | the scheduler draws every n-th frame, the clock does not wait | not needed on this PC (stride 1 in every run at 15 threads); shown working with `--threads 1`: stride 4, the playhead ends exactly 20 s on | **yes** (the fallback works) |
| Draw-ahead | 12 frames ahead, 64 kept | lead 4 frames (quiet), 5 (search yielding), 11 to 12 (full search) | as designed |
| Scrub: the asked frame on screen | within `frame_ms_p95` + 20 ms | median **100.3 / 100.3 ms**, worst **113.3 / 113.7 ms**, against 108.6 / 109.2 + 20 ms | **yes** |
| First picture after choosing a seed | one fight, one picture set-up, one frame | `TakePicture` 0.048 / 0.051 s; first picture given 0.241 / 0.243 s after the take is loaded | **yes** |
| Sound after choosing a seed | joins when built (`soundtrack_build_s`) | `TakeSound` **6.99 / 7.01 s** | as designed |

## 2. The runs

`frame_ms` is the mean of the playback's last 30 draws (what the scheduler paces itself by); `frame_ms_p95` is the
95th percentile of the 50 scrub draws, each drawn alone (one frame on one thread, as milestone 1 measured it);
"play p95" is the 95th percentile of the playback's draws, up to 15 at once. "Late after start" counts only the
frames missed after the first picture. "To picture" and "to sound" are from `LoadTake` to `TakePicture` and to
`TakeSound`.

### Nothing else running (20 s, 15 threads)

| Run | PC busy | frame_ms | play p95 | frame_ms_p95 | lead / stride | shown / late | on time | late after start | first picture | scrub median / worst | to picture / to sound |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 5 % | 108.3 | 111.2 | 110.4 | 4 / 1 | 594 / 8 | 98.67 % | 0 | 247 ms | 100.3 / 113.7 ms | 0.051 / 6.99 s |
| 2 | 12 % | 107.4 | 111.5 | 109.2 | 4 / 1 | 594 / 8 | 98.67 % | 0 | 241 ms | 101.1 / 116.4 ms | 0.048 / 7.01 s |
| 3 | 10 % | 106.4 | 111.9 | 108.6 | 4 / 1 | 594 / 8 | 98.67 % | 0 | 243 ms | 100.3 / 113.3 ms | 0.052 / 7.01 s |
| test run | 6 % | 106.3 | 112.1 | 109.8 | 4 / 1 | 594 / 8 | 98.67 % | 0 | 251 ms | 100.3 / 119.6 ms | |

The last row is `playback_meets_the_budget` / `scrubbing_meets_the_budget` (one shared run).

### Beside `ballfight search` (20 s, 15 threads)

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

### Other thread counts and a full play-through

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

## 3. Why the start is late, and what would fix it

The clock starts when the take is loaded. The take's first draws then take about 245 ms: on a fresh picture the
first 12 to 14 frames are all drawn at once and all finish at about the same moment (in one diagnostic run, 14
draws of frames 109 to 122 finished between 245 and 251 ms after the load, whatever their start), against 100 to
110 ms a frame from then on. In that quarter of a second the playhead passes 8 frames (FIGHT!'s own frame 108 is
never asked for at all: while playing, the scheduler asks from the playhead plus the lead). Every quiet and yielding
run has all of its late frames before the first picture and none after it.

Nothing in the scheduler or the player was changed for this (see the report of Task 32): both ways out change what
the player does at a load, which its tests pin, and the choice belongs with the wiring (Task 35) or a ruling:

1. **Start the clock with the first picture** (playback from FIGHT! waits, silent, until the take's first frame is
   on screen; the sound joins at the playhead as now). Every frame after it is then on time on this PC, and FIGHT!'s
   own frame can be the first one shown.
2. **Draw one frame before `TakePicture`** (in `engine/takes.rs`), so the preview's first draws find a warm
   picture. This moves about 0.14 s into the take's own making and would likely leave 4 or 5 frames late.

Fewer drawing threads (7) start faster and land on exactly 99.0 %, but that is too close to the line to rely on, and
it halves the headroom under load.

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
