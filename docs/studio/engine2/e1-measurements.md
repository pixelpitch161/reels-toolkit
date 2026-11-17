# Engine 2 E1: fight speed and the TEST DUMMY's fights

Measured on 2026-11-17 in the cloud (Intel Xeon @ 2.10 GHz, 4 logical cores, 3 workers, release build), not on
Aedan's PC. The cloud CPU differs from the PC, so the target is judged by the **ratio** of Engine 2 to the current
engine on the same machine. The PC run comes with E2's PC session.

- Engine 2: `cargo run --release -p bf-engine --example engine2_speed` (TEST DUMMY vs TEST DUMMY, seeds 1 to 2000,
  record off; one worker, then all workers but one, as `ballfight bench` counts them).
- Current engine: `ballfight bench fights/flute_vs_trombone.toml --only fights --json` (FLUTE vs TROMBONE, 2000
  seeds), on the same machine just before.

## Speed

| What | Current engine (FLUTE vs TROMBONE) | Engine 2 (TEST DUMMY pair) | Ratio |
| --- | --- | --- | --- |
| Fights per second, 3 workers | 985 to 1040 (three runs) | 1355 to 1551 (2000 seeds), 1440 to 1546 (6000 seeds) | **about 1.45x** |
| Fights per second, one worker | 347 to 362 | 500 to 540 | about 1.45x |

The target from E0 (corrected): all-workers fights per second **at least 0.8x today's**, so the full auto-balance
takes at most 1.25x today's 3.10 s. Engine 2 is about 1.45x today's rate on the same machine: **met**. On the PC
that would be about 6,700 fights per second on 15 workers (E0: 4,661 today); the PC run will confirm it. One cloud
run of 2000 seeds dipped to 950 on 3 workers (the cloud machine is shared); longer runs were steady.

E0's numbers for comparison (Aedan's PC, 15 workers): 4,661 fights per second, 498 on one worker, auto-balance 3.10 s.

### What was fixed to get there

The first measurement gave 2,529 fights per second on one worker and 2,597 on three for short fights (and 540
against 546 once tuned): the workers did not add up. Every fight shared the dummy's one `Arc<FighterDef>`, and the
step clones a definition's `Arc` several times, so all threads fought over one reference count. `World::new` now
copies the two definitions into the world's own `Arc`s (a few microseconds once per fight), and three workers give
about three times one worker. The outcomes of the 1-worker and 3-worker passes are identical (the example asserts
it).

## The TEST DUMMY's fights (seeds 1 to 2000)

| What | Value |
| --- | --- |
| K.O. share | 98.4 % (the rest by decision at 58 s) |
| K.O. time, video seconds: median | 46.15 s |
| K.O. time: 10th to 90th percentile | 40.53 s to 52.14 s |
| K.O. between 40 and 55 s | **88.1 %** (target: at least 60 %) |
| Hits per fight (both fighters, Contact and Block) | 80.8 |

The fight starts at the grid's `t_fight` (3.59 s for FLUTE vs TROMBONE's tempo). The tuning that got there: move
cooldowns of 140 steps (JAB), 360 (HAYMAKER), 540 (GRAB SLAM) and 360 (TOSS), circling at `orbit 0.5`, and
`hp_scale 4.0`, which gives many smaller hits per fight and so a narrow K.O. band. Over 200 seeds before the final
tuning, every move was used and landed (launches, grabs with slams, projectiles, the meter special with its cues and
the low-HP SECOND WIND once a fight).
