# Engine 2 E0: today's speeds

Measured on 2026-11-17 on Aedan's PC with the current engine, so that Engine 2 has numbers to beat.

- Commit measured: `2f426281c0a2f6540b204387b71f0f3c0716b65d` (branch `claude/project-thread-r9s75t`)
- PC: AMD Ryzen 7 9800X3D, 16 logical cores, 15 workers, release build
- Fight: `fights/flute_vs_trombone.toml`, recording `reference/fights/flute_vs_trombone/ep392.bfrec` (seed 392,
  1830 frames = 61 s at 30 fps)
- Raw output of the full bench: [`e0-bench.json`](e0-bench.json)

## The numbers

| What | Today |
| --- | --- |
| Fights per second, all 15 workers (2000 seeds) | **4661** |
| Fights per second, one worker | 498 |
| One frame drawn on one thread: mean / p95 / max | 108.6 ms / 117.4 ms / 127.1 ms |
| Picture setup (once per video) | 28.2 ms |
| Draw all 1830 frames on 15 workers | 27.8 s (65.8 frames per second) |
| Soundtrack build | 3.57 s |
| Whole render with encode (`--only render`, encode best, preset slow) | **63.9 s** (28.6 video frames per second) |
| Auto-balance FLUTE vs TROMBONE (wall time, `Measure-Command`) | **3.10 s** |
| Auto-balance MARIMBA vs TIMPANI (wall time) | 3.57 s |

### The render, stage by stage

| Stage | Seconds |
| --- | --- |
| soundtrack | 3.66 |
| checks (15 of 15 passed) | 10.47 |
| frames | 42.92 |
| encode | 1.37 |
| verify | 4.78 |
| save | 0.16 |
| **total** | **63.89** |

The mp4 was 59.3 MB. The scratch video was deleted afterwards.

### One frame, stage by stage (mean of 15 frames, 107.6 ms)

| Stage | ms | Share |
| --- | --- | --- |
| lens | 50.73 | 47.1 % |
| floor | 15.68 | 14.6 % |
| canvas | 14.26 | 13.2 % |
| bloom | 6.24 | 5.8 % |
| painter | 5.43 | 5.0 % |
| arena | 5.37 | 5.0 % |
| beam | 2.44 | 2.3 % |
| grade | 1.82 | 1.7 % |
| coc | 1.53 | 1.4 % |
| title | 1.24 | 1.2 % |
| particles | 1.09 | 1.0 % |
| over_ring | 1.04 | 1.0 % |
| post_fx, scrim, shimmer, hud, sparks | under 1 each | |

skirt, glows and countdown came out slightly below zero (timing noise: they cost next to nothing).

### Auto-balance results

- FLUTE vs TROMBONE: handicap found **[1.94, 0.39]** (was [2.0, 0.4]). FLUTE wins 45.4 % of 2647 decided fights of
  3000, median K.O. 51.29 s, no K.O. 11.8 %. Even: yes. 29 steps, the tool's own clock said 3.1 s.
- MARIMBA vs TIMPANI: no fight file exists for this pair, so I used a copy of `flute_vs_trombone.toml` outside the
  repo with the fighters renamed and `[pair_rules]` emptied. Handicap found **[0.68, 0.63]**. MARIMBA wins 44.7 % of
  2908 decided fights of 3000, median K.O. 50.72 s, no K.O. 3.1 %. Even: yes. 29 steps, 3.5 s by the tool's clock.

## How busy the PC was

Nothing of Aedan's was closed. Bench reads how busy the CPU already is just before each part (`cpu_busy`, percent):

| Part | CPU busy before it |
| --- | --- |
| fights, all workers | 8.2 % |
| fights, one worker | 7.3 % |
| frame | 7.9 % |
| stages | 11.5 % |
| draw | 12.8 % |
| sound | 18.9 % |
| render | 10.4 % |

So the PC was lightly busy (other programs open) but not loaded. The numbers may be a few percent slower than on an
idle PC.

## Targets for E1 and E2

| Target | Today | Engine 2 must reach |
| --- | --- | --- |
| E1: full auto-balance on a dummy pair | 3.10 s (FLUTE vs TROMBONE) | **at most 3.87 s** (1.25x today) |
| E1: fights per second on all workers | 4661 | **at least 3729** (0.8x today) |
| E2: render of a 61 s video (1830 frames) | 63.9 s, 28.6 fps whole render | **30 fps or faster**, so **61 s or less** end to end |

The fights-per-second line first said "at least 1165 (today / 4)"; it was corrected in E1 to 0.8x today, because the
spec binds auto-balance to about the same time as today, and auto-balance is fights, so 1.25x today's time needs at
least 0.8x today's fights per second.
