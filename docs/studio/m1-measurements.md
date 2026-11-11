# BALLFIGHT Studio: milestone 1 measurements

Milestone 1 of the design (`2026-11-10-ballfight-studio-design.md`, section 14): one match-up, FLUTE against
TROMBONE with the big top theme, end to end from the command line. This file is what was measured when it closed
(Nov 11, 2026): how fast the engine is against the targets of section 7, and how close it is to the Python kit it
was ported from. The app's plan (`2026-11-10-ballfight-studio-plan-m2-app.md`, "Performance budget") reads three
numbers from here, by name: `frame_ms_p95`, `soundtrack_build_s` and `fights_per_second` (section 1).

Everything was measured on the one machine there is: the owner's PC (AMD Ryzen 7 9800X3D, 8 cores / 16 threads,
64 GB, Windows 11), which is also where the Studio is built. Release build (`cargo build --release -p bf-cli`).
Unless a line says otherwise, the commands ran from `studio/`.

## 1. The three numbers the app's plan reads

| Name | Value | What it is | How it was measured |
| --- | --- | --- | --- |
| **`frame_ms_p95`** | **178 ms** (median of six runs 222 ms) | the 95th percentile of one frame drawn on one thread, over every 10th frame of the example episode (183 frames), after the speed-ups of section 3 | `ballfight bench ... --only frame`, six runs on a PC 32 to 83 % busy with other work; the mean frame is 128 ms at best, 145 ms median |
| **`soundtrack_build_s`** | **4.67 s** (best 4.48 s) | building the whole soundtrack of the example episode (score, voices, master), one thread | `bench`'s `sound` part, median of six runs (PC 35 to 87 % busy) |
| **`fights_per_second`** | **6,350** on 15 workers (best 7,416); 663 on one worker (best 791) | fights with the recording off, each with its story and score as the search runs them, seeds 1 to 2,000 | `bench`'s `fights` part, median of six runs (PC 39 to 100 % busy) |

Earlier measurements on a quieter PC (Tasks 16, 22, 24b): one frame about 120 ms with the bare look and 142 ms with
both looks; 821 fights a second on one worker and 7,247 on 16. The app's budget (`P = ceil(1.25 * frame_ms_p95 /
33.3)` threads for live playback) gives **7 threads with 178 ms**, which is the 7 the owner's PC has to spare; on a
PC as busy as it was here (222 ms) it would be 9, so the preview's "every second frame" fallback (app plan, Task 32)
is needed while other heavy work runs.

## 2. Speed against the targets (spec section 7)

The PC was never idle during these measurements: other workers were building and testing the app on it. Each
number is the best and the median of at least three runs; `[PC n % busy]` is the whole PC's CPU use over the second
before the run (every core, every program), which `bench` measures itself. "Before" is the engine as milestone 1
left it; "after" is with the three speed-ups of section 3, which change no bit of any frame or sound.

| Target | Measured before | Measured after | Met? |
| --- | --- | --- | --- |
| Live playback at 30 fps with headroom: one frame under 33 ms on a core, or enough cores | one frame 147.5 ms best, 162 ms median (mean of 183 frames); 95th percentile 209 / 228 ms | 127.7 / 145 ms mean; 95th percentile **178 / 222 ms** | **not on one core** (4 to 5 times over); playback needs 6 to 7 threads drawing ahead, as the app's plan budgets |
| A full 1,830-frame render under 60 s | **94.8 s best, 97.0 s median** (108.7 / 94.8 / 97.0 s at 84 / 45 / 76 % busy) | **81.4 s best, 82.9 s median** (81.4 / 83.3 / 82.9 s at 51 / 40 / 69 % busy) | **no** (see the speed plan) |

Every render here passed all 15 checks and gave the same mp4, byte for byte (SHA-256 `308fb903...c169`, 59.3 MB),
before and after.

### The render by stage (`bench --out`, 15 workers, the example episode)

| Stage | Before (s, runs 1 / 2 / 3) | After (s, runs 1 / 2 / 3) |
| --- | --- | --- |
| soundtrack | 5.9 / 5.9 / 5.2 | 4.9 / 6.2 / 5.2 |
| checks (673 frames drawn, the sound checks beside them) | 17.7 / 17.2 / 14.7 | 13.7 / 15.1 / 13.4 |
| frames drawn and encoded (x264 at 1,080 x 1,920) | 76.3 / 64.9 / 67.8 | 55.0 / 53.9 / 56.6 |
| ffmpeg finishing the file | 2.0 / 1.6 / 2.3 | 1.8 / 1.6 / 1.8 |
| verify (the file decoded and checked) | 6.4 / 4.8 / 6.3 | 5.6 / 6.0 / 5.4 |
| save | 0.2 / 0.2 / 0.5 | 0.3 / 0.4 / 0.3 |
| **whole render** | **108.7 / 94.8 / 97.0** | **81.4 / 83.3 / 82.9** |

Drawing all 1,830 frames on 15 workers without the encode: 43.1 / 38.6 / 37.6 s before, 35.4 / 31.1 / 37.3 s
after (`bench`'s `draw` part). The frames stage takes about 55 s although its frames alone now take about 23 s
(1,157 of them are drawn there): **the encode is what the frames stage waits for**. x264 at the kit's settings
(`slow`, 1,080 x 1,920) shares the 16 threads with the drawing.

### One frame by stage (`bench`'s `stages` part, one thread, every 122nd frame, each the best of three)

Time to reach a stage less the time to reach the one before; small stages can come out a little negative from noise.

| Stage | Before (ms, share) | After (ms, share) |
| --- | --- | --- |
| lens (depth of field: 12 FFT convolutions a frame on average) | 99.5 (59 %) | **68.6 (54 %)** |
| floor (per pixel) | 20.6 (12 %) | 19.5 (15 %) |
| canvas (per pixel, with the shadows) | 14.5 (9 %) | 14.4 (11 %) |
| painter (posts, balls, the looks' items) | 6.7 | 3.9 |
| bloom + grade | about 6 together | 5.5 + 5.0 |
| arena, skirt, over the ring, sparks, particles, circle of confusion, shimmer, glows, beam, scrim, effects, count-in, HUD, title card | each under 5 | each under 5 |
| **a frame** | **169 ms** (PC 87 % busy) | **127 ms** (PC 83 % busy) |

## 3. Speed plan

The render misses its 60 s target, so here is what was done now and what is left.

### Done now (each its own commit; each proven to change no bit)

1. **The lens blur keeps its FFT plans and buffers** (`bf-picture`: `fft.rs`, `lens.rs`). A `Convolver` now keeps
   the plans of both transform lengths, and `dof` gives every convolution of a frame one `Workspace`, so a frame
   asks the system for memory once instead of about 300 MB of fresh pages; the inverse transform also skips the
   rows the crop drops. Measured in thread cycles: `dof` with every level in use **605 to 620 -> 464 to 467
   million cycles (-24 %)**. Building the plans themselves was only 0.2 % of a convolution (50 thousand cycles of
   23 million); the fresh memory was the fifth. Proof: the same bits for `dof` on a full-blur test picture, for the
   lens stage and the frame of every 5th frame of the episode (a hash before and after), and the gates below.
2. **The lens skips a blur level no pixel takes part in** (`lens.rs`). Its convolutions would be of zeros, which
   are zeros to the bit, so its blur is the picture. 134 of the episode's 5,494 levels (2 %). Proof: the lens stage
   and the frame of all 1,830 frames hash the same with and without it; a unit test holds the case.
3. **The render takes the frames the checks drew** (`bf-export`: `pre_render_keeping`, `job.rs`). The checks draw
   673 frames (the safe zone's, the flash stretch and the loop's); the render now hands those to ffmpeg instead of
   drawing them again, and draws only the other 1,157. It holds them in memory while it encodes (about 465 MB).
   Proof: the finished mp4 and cover have the same SHA-256 as before, and the checks' test asserts every kept frame
   is the frame and the results are `pre_render`'s.

Gates run after the changes, all passing: `bf-content` `golden_frames` (43 of 43), `flute_look`, `trombone_look`,
`bf-picture` `ring_stages` (frame and stage hashes), `kit_vectors`, `kit_fx_vectors`, `kit_ui_vectors`, the Pillow
vectors, `title_frames`, `sprites`, `bf-content` `stems`, `bf-export` `checks_parity` and `job`, and the command
line's `render_is_reproducible`.

### The three most expensive stages, and what is proposed for each

| Stage | Cost now | Proposed change | Keeps every bit? | Expected |
| --- | --- | --- | --- | --- |
| **The encode** (x264 `slow` at 1,080 x 1,920 on the frames stage) | the frames stage is about 55 s while its drawing alone is about 23 s | (a) give ffmpeg the 360 x 640 frame and let it scale three times by nearest neighbour (`-vf scale=...:flags=neighbor`) instead of piping 9 times the bytes; (b) a faster x264 preset | (a) must be proven: the same pixels reach the encoder, so the stream should be the same, and a byte compare of the mp4 decides; (b) changes the file: the owner's call | (a) a few seconds of pipe and conversion; (b) could halve the stage |
| **The lens** (54 % of a frame) | 69 ms a frame | transform the FFT columns four at a time in vector registers, as `ducc0` does (each column's arithmetic unchanged; a plain blocking of the columns, tried here, gained nothing measurable) | yes, if each column's operations stay in order | the column passes are about half the transform |
| **The two per-pixel passes** (floor and canvas, 26 %) | 34 ms a frame | for the preview: split a frame's per-pixel passes and its three colour convolutions over threads (no value changes, a frame is drawn sooner); for every frame: write `bloom`, `grade` and the frame's array copies into reused buffers (about 40 MB of fresh memory a frame, measured to cost about 5 ms) | yes | about 5 ms a frame for the buffers; the threads help the preview's latency, not a render's total |

Also proposed, not done now:

- **Build the soundtrack while the checks' frames are drawn** (about 5 s of the render): the frames do not need the
  sound. Left out because it reorders the job's stages and its cancel points, which the app's render queue (app plan,
  Task 33) reads; worth doing together with that task.
- **Verify while encoding**: the file's checks decode the finished mp4 (5 to 6 s); they cannot start before ffmpeg
  finishes, so this stays.
- Caching per-frame-constant buffers in the floor pass: the floor depends on the camera, which moves every frame of
  this episode, so nothing there is constant from frame to frame.

Even with all of these the render would stay near 60 s on this PC while the encode keeps its kit settings; the app
plan should treat about 80 s as today's figure.

## 4. The fight: how long a Studio fight stays the kit's fight

A Studio fight is the kit's fight until the two drift apart in the last bits of a float (spec section 6, "Why not
exact replay"). The bar: positions, HP and every event agree to 1e-6. Measured by `crates/bf-sim/tests/prefix.rs`
(`cargo test -p bf-sim --test prefix -- --nocapture`) over the 21 Python recordings in
`studio/reference/fights/flute_vs_trombone/`, every event and all 19 per-frame values. The floor the test holds is
10 s of fighting; **every one of the 21 agrees for at least 25.6 s of fighting**, and every window that ends, ends
on a float drifting past 1e-6 (`drift`): none on a knife-edge, none on an event only one engine made (a `logic`
break would be a bug).

| Fight | Scenario | Agrees until (s of video) | Of which fighting (s) | The fight lasts (s of fighting) | Frames | Events | What ended it |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ep392 | the episode | 37.05 | 33.47 | 46.43 | 1,004 | 329 | drift: a speed 1.4e-6 apart; same events until 44.02 s |
| seed1 | episode rules | 39.99 | 36.40 | 37.05 | 1,092 | 386 | drift: a speed 1.4e-6; same events to the end |
| seed2 | episode rules | 33.65 | 30.06 | 49.28 | 902 | 260 | drift: a float of event 260 (a `strike`) 1.1e-6 |
| seed3 | episode rules | 40.45 | 36.87 | 47.37 | 1,106 | 383 | drift: a position 1.0e-6; same events until 47.19 s |
| seed500 | episode rules | 41.39 | 37.80 | 52.30 | 1,134 | 351 | drift: a position 1.0e-6; same events until 55.09 s |
| no_ko | episode rules, no K.O. | 43.69 | 40.10 | 54.41 | 1,203 | 275 | drift: a speed 3.4e-6; same events until 54.43 s |
| early_ko_s1 | early K.O. | 14.88 | 11.29 | 11.29 | 340 | 117 | nothing: agrees to the K.O. (largest difference 6.6e-13) |
| early_ko_s2 | early K.O. | 18.00 | 14.41 | 14.41 | 434 | 147 | nothing: agrees to the K.O. (1.8e-11) |
| early_ko_s3 | early K.O. | 22.48 | 18.89 | 18.89 | 568 | 191 | nothing: agrees to the K.O. (8.7e-11) |
| early_levels_s1 | early levels | 37.89 | 34.30 | 35.70 | 1,029 | 384 | drift: a position 1.0e-6; same events to the end |
| early_levels_s2 | early levels | 37.95 | 34.37 | 38.59 | 1,031 | 400 | drift: a position 1.0e-6; same events to the end |
| early_levels_s3 | early levels | 41.29 | 37.70 | 47.10 | 1,131 | 398 | drift: a position 1.0e-6; same events to the end |
| early_low_s1 | early low HP | 36.39 | 32.80 | 42.21 | 984 | 322 | drift: a speed 1.5e-6; same events until 44.35 s |
| early_low_s2 | early low HP | 38.05 | 34.47 | 41.76 | 1,034 | 393 | drift: a speed 5.4e-6; same events to the end |
| early_low_s3 | early low HP | 44.45 | 40.87 | 41.83 | 1,226 | 433 | drift: a speed 1.3e-6; same events to the end |
| plain_s7 | plain | 44.02 | 40.43 | 53.45 | 1,213 | 376 | drift: a speed 1.9e-6; same events to the end |
| plain_s8 | plain | 43.59 | 40.00 | 54.41 | 1,200 | 292 | drift: a speed 1.2e-6; same events to the end |
| plain_s9 | plain | 35.95 | 32.37 | 43.62 | 971 | 279 | drift: a speed 1.3e-6; same events to the end |
| swapped_s1 | corners swapped | 29.15 | **25.57** | 40.89 | 767 | 250 | drift: a position 1.0e-6; same events until 43.70 s |
| swapped_s2 | corners swapped | 39.95 | 36.37 | 47.80 | 1,091 | 348 | drift: a speed 2.1e-6; same events to the end |
| swapped_s3 | corners swapped | 30.99 | 27.40 | 41.00 | 822 | 246 | drift: a position 1.0e-6; same events until 39.65 s |

Inside the 21 windows the events match one for one: 228 `ability` events (all six abilities), 36 shockwave hits
with their 36 slows, 5 wave hits, 5 slams with 4 slam hits, 430 fang hits, 780 blade hits, 273 swats and 3 K.O.s.
Beyond the committed set, 500 more Python recordings (seeds 10 to 109 in five scenarios, compared once and not
kept): 134 agree to the end of the fight; the shortest window of the others is 17.5 s of fighting, the median
31.3 s; all end on a drift. 471 of the 500 have the same winner in both engines.

`crates/bf-sim/tests/coverage.rs`: every kind of event and ability in `coverage.json` (23 of 23) appears in the
Rust fight at the same place, inside the window. `studio/replay/hashes.txt` pins the Studio's own fights (seeds 1,
2, 3, 500 and `1 << 40`, both corner orders): a debug and a release build give the same lines.

Why a window ends at all: numpy's dot product of two 2-vectors is fused on this PC (OpenBLAS), and the Studio's rule
forbids fused multiply-add (ruling R25); `numpy` also takes some transcendentals from the C runtime, one last bit
from `bf_core::dmath` now and then (R16). **Seeds are Studio-owned**: a kit seed is not expected to give the same
fight, and the app never shows one as "the same fight".

## 5. Statistics: the Studio's fights against the kit's

`crates/bf-story/tests/stats_parity.rs` (the 40,000-seed half runs in release only): the kit's statistics of seeds
1 to 4,000 (`studio/reference/fights/flute_vs_trombone/stats.json`) against the Studio's fights of seeds 1 to 4,000
and 1 to 40,000, handicap `[2.0, 0.4]`.

| Figure | Kit, 4,000 seeds | Studio, 4,000 seeds | Studio, 40,000 seeds | Bar |
| --- | --- | --- | --- | --- |
| FLUTE's share of the decided fights | 47.67 % | 47.89 % | 47.49 % | 3 points |
| Median K.O. | 49.863 s | 49.830 s | 49.863 s | 0.7 s |
| Mean lead changes | 0.5693 | 0.5680 | 0.5791 | 0.25 (0.06 in release) |
| Fits the series (fit rate) | 0.750 % | 0.675 % | 0.630 % | 1.5 points (0.5 in release) |
| 10th percentile K.O. | 42.655 s | 42.655 s | 42.880 s | 1 s |
| No K.O. before 58 s | 7.65 % | 7.55 % | 7.64 % | 2 points |

`Stats::of` on the kit's own 4,000 rows gives the kit's pooled figures bit for bit. 3,687 of the kit's seeds 1 to
4,000 end with the same winner in the Studio. Auto-balance from `[1.0, 1.0]` lands on `[1.94, 0.39]` (the kit's
hand-tuned pair is 2.0 / 0.40): FLUTE 45.6 % of 2,662 decided fights, median K.O. 51.38 s, in the band.

## 6. The picture gate

The bar (spec section 7): identical RGB; a frame that cannot be made identical goes in
`studio/reference/frames/exceptions.toml` with a reason, at most 2 levels on at most 0.5 % of its pixels.

| Gate | Test | Result |
| --- | --- | --- |
| Golden frames of the episode, both real looks, through `build_picture` | `bf-content/tests/golden_frames.rs` `all_golden_frames` | **43 of 43 frames are the kit's bytes.** No exception: `exceptions.toml` is the empty list, and no frame had a hull fallback |
| The FLUTE's look (with the bare look for the TROMBONE) | `bf-content/tests/flute_look.rs` | 254 of 254 frames of three fights (124 + 65 + 65), 310 title cards |
| The TROMBONE's look (with the bare look for the FLUTE) | `bf-content/tests/trombone_look.rs` | 181 of 181 frames of two fights (102 + 79) |
| The ring with the bare look | `bf-picture/src/ring/tests.rs`, `tests/ring_stages.rs` | 64 of 64 frame hashes; 51 drawing-stage buffers and the episode's 18 equal to the bit |
| The VS title card | `bf-picture/tests/title_frames.rs` | 18 cards and **249 of 249 card frames** equal (the episode's 61, the fighters swapped 61, 96 BPM without a tag 84, 200 BPM 43) |
| Sprites (the instrument models drawn small) | `bf-picture/tests/sprites.rs`, `flute_look.rs` | all 978 reference mesh sprites equal (12,648 more compared once); the FLUTE look's 503 sprites equal |
| Pillow, numpy, scipy pieces | `bf-picture/tests/{pillow_vectors,pillow_blur_resize,pillow_as_used,kit_vectors,kit_ui_vectors,kit_fx_vectors}.rs` | every drawing case equal; float results within 1e-9 of each value (ruling R34) |
| Other fights of the pair | `golden_frames.rs` `every_fight_of_the_pair_draws` | 1,026 frames of 19 other recordings draw with no panic and no hull fallback |

The frame exceptions list is empty, so there are no images to show for it.

## 7. The sound gate

The bar (spec section 8): every stem within -120 dBFS of the kit's; documented exceptions down to -80 dBFS
(`studio/reference/sound/exceptions.toml`). Measured by `bf-content/tests/stems.rs` and `bf-sound/tests/voices.rs`.

| What | Result |
| --- | --- |
| The finished audio (`audio.wav`, the 16-bit master the video holds) | **identical in every sample**: 0 of 5,857,280 differ; byte for byte the kit's file (`render_is_reproducible` also checks it) |
| Stems, 9 windows of 7 buses, as the float32 references hold them | worst **-162.6 dBFS** (the drums and the FLUTE's bus) |
| Premaster windows | worst -168.6 dBFS |
| Every float64 sample of every bus against the kit's whole buffers (compared once) | premaster -232.5, stems -226.6, master -224.6 dBFS; about a third of all samples have the kit's exact bits |
| 100 ms loudness blocks (`rms.json`) | within 1e-11 dB |
| Voices (`FLUTE.npz`, `TROMBONE.npz`, float32) | -150.5 and -156.5 dBFS: the float32 rounding itself |
| The notes | the 174 first-time notes in the kit's order; the noise generator ends in the kit's state |
| Stem exceptions | none: the list is empty |

## 8. The checks

`bf-export/tests/checks_parity.rs` (14 tests) against `studio/reference/checks/flute_vs_trombone.json`, the kit's
`check` run on the episode. Every verdict is the kit's.

| Check | Against the kit |
| --- | --- |
| levels | all 31 windows, the hit counts and the five summary numbers to the bit |
| loudness | the three numbers to the bit |
| band | 20 numbers, worst 7.1e-14 dB (a moving mean added in another order than numpy's dot product) |
| flash | all 315 areas to the bit |
| loop (picture and sound) | all 178 steps, the seam, the normal step, the ratio and the five sound numbers to the bit |
| focus | worst circle of confusion 0.0 over 39,671 points |
| safe zone | all 5,503 boxes equal (coordinates, kind, text, order) |

The finished mp4 is checked too (`mp4_size`, `mp4_streams`, `mp4_keyframes`, `mp4_length`, `mp4_flash`,
`mp4_loop_picture`, `mp4_loop_audio`): a whole render passes all 15 checks. The mp4's bytes do not depend on the
number of workers (ruling R42; 5 of 5 runs identical for 1 and 8 workers).

**A fight that fits the story can still fail a check.** The story's verdict ("fits the series") does not look at the
sound, and the levels check does: at 120 BPM the fits 204 and 946 fail `levels` (the loudest 2 s window 5.2 and 5.4
dB above the quietest, the bar is 5 dB), at 150 BPM the fit 926 (5.5 dB); at 132 BPM the first three fits (622,
255, 542) pass. The fight file's own seed 392, fought by the Studio (not the kit's episode recording), fails
`levels` too (6.9 dB, and two windows over 3 dB below the ending). So the app must not promise that a fit will
render: it shows the levels result with the take, and a render that fails says so and offers the next seed.

## 9. What needed special handling

Each of these is a place where "do what the Python says" was not enough: the libraries do something the code does
not show. Each is ported and pinned by a test.

| Library | Behaviour | Where |
| --- | --- | --- |
| numpy | `a.sum()` / `a.mean()` of a 1-D float64 array is a pairwise sum (eight running sums up to 128 elements, then halves), not left to right | `bf_picture::np::sum`, `bf_sound::dsp::{sum, mean}` |
| numpy | the dot product of two vectors (`@`, `np.dot`, `np.linalg.norm` of one vector) is fused on this PC (OpenBLAS); the Studio uses the plain sum (ruling R25: no fused multiply-add) | fight engine, camera, sprites |
| numpy | `x ** p` has shortcuts: `-1` is `1 / x`, `0` is `1`, `0.5` is `sqrt`, `1` is `x`, `2` is `x * x`; every other power is the C runtime's `pow`, which differs from `dmath::pow` in the last bit more often than `sin` or `exp` (9 % of one gradient's values) | `post::grade`, the sound helpers |
| numpy | each intermediate keeps numpy's dtype: `float32` stays `float32` (`Frame.array()`, the depth buffer, `coc_map`'s floor, the bloom's `up * k`, vignette and scrim up to `v * 5`, the HUD's backing); `dof` returns float32 when nothing is out of focus and float64 otherwise | `bf_picture` throughout |
| numpy | the normal generator's tail calls the C runtime's `log1p`: one normal in 740,000 is 1 ulp apart; the stream never falls out of step (ruling R20) | `bf_core::rng` |
| numpy | complex division by a real number is Smith's method (`x * (1 / d)`) | `butter` |
| numpy | `np.percentile` (linear rule, two branches), `np.convolve` (swaps the operands), `np.round` (multiply, round half to even, divide) | `bf_sound::dsp`, `bf_core::py` |
| scipy | `fftconvolve` runs the FFT of the float32 plane in float32 (`ducc0`): only the same FFT gives the same picture, so `fft.rs` is `ducc0`'s plans, passes and twiddles; columns four at a time, and for a left-over float32 column above 300 samples the other plan (`cfftp_vecpass`); the angle divided in the x87 unit at 53 bits | `bf_picture::fft` |
| scipy | `ConvexHull(...).vertices` starts where Qhull's facet list starts: Qhull 8.0.2's 2-D build ported step by step; where Qhull would merge, a plain hull is used and counted (ruling R35; none seen in any frame) | `bf_picture::hull` |
| scipy | `butter`, `zpk2sos` (pairing from the last section), `sosfilt` (never reads `a0`), `lfilter`, `resample_poly`, `find_peaks` (ties from numpy's unstable sort, ruling R28), `i0` (Cephes) | `bf_sound::dsp` |
| Pillow | coordinates truncated toward zero; the polygon fill's edge list in C `float`, `ROUND_UP` / `ROUND_DOWN` with a float sum (0.49999997 + 0.5 is 1.0), the lower end of an edge entered twice, discontiguous corners connected, horizontal edges merged | `bf_picture::pil::polygon` |
| Pillow | thin lines without their last pixel (Bresenham), wide lines as quadrilaterals, the ellipse's half-pixel walk (a filled one-pixel ellipse draws nothing), rectangle outlines, blending `((t >> 8) + t) >> 8`, coordinates no `int` holds (NaN, beyond 2^31) | `bf_picture::pil` |
| Pillow | `GaussianBlur` (box passes) and `resize` (bilinear, bicubic) | `bf_picture::pil::{blur, resize}` |
| CPython | `sum()` of floats is Neumaier's compensated sum (from 3.12): the plain sum differs in the last bit for 22.6 % of triples | `bf_picture::ui` (`_hash01`) |
| CPython | `round()` is half to even and `round(x, n)` correctly rounded; `int()` truncates; `//` floors; `%` takes the sign of the divisor (`hash(line) % 997`) | `bf_core::py` |
| CPython | `x ** 2` is the C runtime's `pow`, not always `x * x` (108 of 200,000); the Studio multiplies, and no frame meets the difference | `ui`, the count-in |
| CPython | `math.hypot` is not `np.hypot` (the C runtime's plain square root of the sum of squares): 17 % of pairs differ in the last bit; each port uses the one its line uses | `np::hypot`, `dmath::hypot` |
| CPython | the string hash under `PYTHONHASHSEED=0` (the caption puff's key) | `bf_core::pyhash` |
| C runtime | `sin`, `cos`, `exp`, `tanh`, `log10`, `pow` 1 to 2 ulp from `bf_core::dmath` on some values; the parity bars absorb it (ruling R16) | everywhere |

## 10. The reference set

`studio/reference/`: 249 files tracked, **133.5 MB** (133,478,704 bytes, the git-ignored `.cache/` left out) against
the budget of 150 MB (spec section 12). The sound is the largest part (68 MB), then the DSP vectors (20 MB), the
golden frames (17 MB) and the picture kit vectors (9 MB). `python -m tools.studio_export verify` re-hashes every
file in `manifest.json` (251 files, 0 missing or changed, on Nov 11). Milestone 3 (18 more fighters) cannot add references at this rate: it needs
another storage plan (Git LFS, or references generated on demand), as ruling R24 says.

## 11. The controller's rulings that changed the plan

One line each; the full wording is in the build's ledger.

| | Ruling |
| --- | --- |
| R1 | This Windows PC is the reference platform: every "on Linux" step runs here (later: no Linux at all, by the owner's word) |
| R2 | Milestone 0 ran a Python export lane and a Rust lane side by side, with disjoint paths |
| R3 | `bf_core::show` was built early (task 15a), before the lanes needed it |
| R4 | From milestone 1 each lane works in its own clone on its own branch; the controller merges |
| R5 | The export lane started after the picture and the sound were merged |
| R6 | `.gitattributes` keep reference bytes unchanged on Windows checkouts |
| R7 | Commits carry a plain co-author trailer and no model name; the controller pushes |
| R8 | Numeric ports ran on the most capable workers |
| R9 | The workspace pins the whole tech stack from task 1 |
| R10 | The maths source scan also catches `f64::sin(..)`-style calls |
| R11 | Files the Rust tests bless are adopted into the manifest by the export tool |
| R12 | `Event::who()` is optional (a `clash` has no fighter) |
| R13 | `np_round_nd` follows numpy for infinities and NaN |
| R14 | The ban on the two retired fighters covers them as fighters (roster, names, lists), not a word inside ELECTRIC's ability name |
| R15 | Manifest entries and recordings keep their bytes across unrelated commits |
| R16 | `dmath` stays on the pinned pure-Rust libm; 1 to 2 ulp from Python's maths is accepted under the parity bars |
| R17 | The app's plan was written during milestones 0 and 1 (Block A on fixture data, Block B after milestone 1) |
| R18 | A recording keeps its provenance when written again |
| R19 | A fifth scenario (`early_ko`) so a K.O. falls inside the coverage window |
| R20 | The normal generator's tail uses `dmath::ln(1 - u)`; one draw in 740,000 is 1 ulp apart |
| R21 | An ability option not ported makes that fighter unfightable instead of failing the roster |
| R22 | The Pillow port was split in two (drawing; blur and resize) |
| R23 | Meshes and sprites were ported in their own lane |
| R24 | The reference budget is the overall 150 MB, not the plan's interim 60 MB |
| R25 | No fused dot product: the plain sum, measured to stay within 1e-6 for at least 33 s in core fights |
| R26 | Two more call sites for kinds and abilities besides `schedule.rs` (Python interleaves per fighter) |
| R27 | Fewer `thud` vectors and float32 voice vectors, to stay inside the budget without weakening the bar |
| R28 | `find_peaks`' tied peaks accept scipy's unstable order; the mixer's buses keep creation order |
| R29 | The HUD and the effects were split so two picture workers could run at once |
| R30 | The roster types moved to `bf-core`, so picture and sound depend on the core only |
| R31 | The roster keeps a load-time limit on `frets0` |
| R32 | Task 14 (abilities and parity) was split in two |
| R33 | The big top accepts only the chords with reference stems (Em, Am, B7, C) in milestone 1 |
| R34 | Float results of the kit vectors pass value by value with a floor |
| R35 | A convex hull returns how it was found; each call site does what the Python does with an unusual one |
| R36 | The cover frame is the last shut, still frame, never before the right name has settled |
| R38 | A search keeps every fit and the best 200 others |
| R39 | The ring's Python fixture and stage hashes live in the reference set |
| R40 | The two looks were ported side by side, then the golden-frame gate joined them |
| R41 | A clip is named as a clip and writes no fight file or caption |
| R42 | The encode adds `-max_interleave_delta 0`, so the mp4's bytes do not depend on the workers |

Also by the owner's word (Nov 10): the Linux check was removed from CI and nothing is checked on Linux any more; CI
is one Windows job (it builds `bf-cli`, tests `bf-core`, `bf-sim` and `render_is_reproducible`, renders a 3 s clip).

## 12. Not yet proven, or deferred

- **The render misses its 60 s target** (81.4 s at best, 82.9 s median, after the speed-ups; 94.8 / 97.0 s before): section 3 says what was done and what is left.
- **Other machines.** All numbers are from this PC. "Bit-identical on any PC" rests on the rules (no fused
  multiply-add, `dmath` for every transcendental, no hash-map order, no thread-count dependence) and on CI's one
  Windows runner, not on a second PC of a different make.
- **The kit's replay guard on Windows.** `tools/ballfight_replay_hash.py` drops the kit-episode lines on Windows (it
  compares paths with forward slashes); the kit episodes were checked by running the same child commands one by
  one (on Nov 11 the tool printed its 95 lines exactly as before the milestone, and the 100 kit-episode lines run one by one are unchanged too; `git status --short projects reelkit series` stays empty). Whether to make the guard Windows-safe is a milestone 6 decision.
- **Seeds are Studio-owned**: a Studio fight is the kit's fight for 25.6 s at least, not to the end (section 4).
- **One match-up only.** Every gate here is FLUTE against TROMBONE with the big top. The other 18 kit fighters and
  13 themes are milestone 3; the big top's other chords open when they have reference stems (R33).
- **Qhull's merge step is not ported** (R35): a hull Qhull would merge is drawn in the plain order and counted. No
  frame of 2,000-odd checked met one.
- **The app does not exist yet**: live preview, audio playback, the queue, the Library (milestone 2, its Block A is
  being built). Cancel does not act inside the soundtrack build (about 4 to 5 s).
- **A fit is not a promise of a video** (section 8): the levels check can still fail.
- **CI**: the one Windows job's last result is read by the controller; the third-party ffmpeg install was replaced
  by `choco install ffmpeg`.
- Smaller items left for later are in the build's ledger ("minor (deferred)").
