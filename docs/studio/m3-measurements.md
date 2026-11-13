# BALLFIGHT Studio: milestone 3 measurements

Milestone 3 of the design (`2026-11-10-ballfight-studio-design.md`, section 14; plan
`2026-11-11-ballfight-studio-plan-m3-kit-roster.md`): the Studio knows the kit's 20 fighters and 14 themes. This file
is what was measured when it closed. Everything was measured on the owner's PC (AMD Ryzen 7 9800X3D, 8 cores / 16
threads, 64 GB, Windows 11), shared with other work. Release build (`cargo build --release -p bf-cli`). Unless a line
says otherwise, the commands ran from `studio/`.

## 1. Speed

(Task 62: `ballfight bench` on one fight of each kit pair against milestone 1's `frame_ms_p95`, `soundtrack_build_s`
and `fights_per_second`; the heaviest looks named.)

## 2. Full renders of every kit pair

Task 61, Nov 12, 2026, with the release `ballfight` built from the build head (be730a8). Every render went through
the render job (`ballfight render`), one at a time, into a scratch folder outside the repo, on a PC shared with other
workers (render times are wall clock and moved with their load: renders took from 74 s in a quiet stretch
to 392 s in a busy one, so they are not speed measurements; section 1 is). A render passes when all 15 checks
pass (the 8 before the frames, then the 7 on the finished mp4); the job refuses the video at the first failed check.
Each mp4 was decoded whole (`ffprobe -count_frames`: 1,830 video frames and 2,860 sound frames every time), its
SHA-256 recorded and the file deleted. Every passing render also wrote its `cover.png` (about 200 KB) and a non-empty
`caption.txt`. The SHA-256 is of the mp4 this PC's ffmpeg (9.0.2, gyan.dev full build) encodes; another ffmpeg gives
other bytes (ruling R42 covers worker counts, not encoders).

### The 18 kit configs, each its kit recording (`render studio/fights/<pair>.toml --rec <recording>`)

The recording is the kit's own fight of the config (`episodes/<pair>/episode.json` names it: the committed
`fights/flute_vs_trombone/ep392.bfrec` for milestone 1's pair, the pinned cache for the others; nine missing cache
files were copied read-only from the main folder, and each one's SHA-256 matches its pin). Seed, winner and K.O. are
the recording's (`ballfight story --rec`), equal to the kit's fight in each pair gate's report.

| Pair | Seed | Winner, K.O. | Checks | Render | mp4 SHA-256 |
| --- | --- | --- | --- | --- | --- |
| flute_vs_trombone | 392 | TROMBONE, 50.02 s | 15 of 15 pass | 74.3 s | `308fb90397d909fd9bf13a704117b5a32eee6716b45a6e79742e79e66012c169` |
| altosax_vs_tenorsax | 621 | ALTO SAX, 49.18 s | 15 of 15 pass | 83.5 s | `ca866d89c232a95fa3360502c1262a62c5cd2dd123b3b684ab78875229680039` |
| bassclarinet_vs_bassoon | 4 | BASSOON, 50.21 s | 15 of 15 pass | 73.7 s | `950126804456666dc17bf0edced4904ebae55d004dc6c8aacf23037c1d9ef322` |
| bassclarinet_vs_euphonium | 1269 | BASS CLARINET, 51.47 s | 15 of 15 pass | 84.7 s | `5ad57cd7aec055430fb387d2d735e31ec250556471841ef09469716a101d06a4` |
| bassdrum_vs_tuba | 132 | BASS DRUM, 51.85 s | 15 of 15 pass | 101.5 s | `009712e5db2c7f4b795ace3848139882421011c0d87b366e542ed24e9f7ba2fc` |
| cello_vs_contrabass | 2455 | CONTRABASS, 50.88 s | 15 of 15 pass | 94.8 s | `b70194cd60249bd7c73baa8af5a55291830eb0bfa2cc318732fc34e6519e01b7` |
| clarinet_vs_oboe | 654 | CLARINET, 52.53 s | 15 of 15 pass | 117.3 s | `bfa8bc7b8a2b40eb1afbb66020a294cb97d0aa97690b79b8df3c3287cde9137f` |
| electric_vs_acoustic | 418 | ACOUSTIC, 51.53 s | 15 of 15 pass | 94.1 s | `116f6eadc52f731809bd62e293626144f84055d1f8d775729e54d4818ca679b3` |
| euphonium_vs_trombone | 4604 | TROMBONE, 51.19 s | 15 of 15 pass | 116.7 s | `44f7423a211ea9cc4bb07c09d37335fd986e8763dec460c2c15c2dee1759bb9f` |
| flute_vs_clarinet | 3864 | CLARINET, 52.03 s | 15 of 15 pass | 109.7 s | `edae5196f4e9edb266b3a07cac5eaaa94a9e34c34c757aaf087771e6c3a6e80c` |
| flute_vs_sousaphone | 146 | SOUSAPHONE, 54.15 s | 15 of 15 pass | 125.4 s | `7293440a0b790e8bdade09a4d6f104f867a314bb112d5578ce6d1903dbeefbb2` |
| horn_vs_trombone_rematch | 630 | HORN, 50.28 s | 15 of 15 pass | 140.4 s | `c573fe75952f54f4dd9c3026df3b619892e997b169874af6f2f195aeb34b2e9b` |
| oboe_vs_bassoon | 954 | OBOE, 52.11 s | 15 of 15 pass | 137.5 s | `41c6e5042e704ef1092276d4e24539e8a0cd33b02fc869822d1f85c78852547d` |
| piano_vs_organ | 638 | ORGAN, 49.57 s | 15 of 15 pass | 173.4 s | `27c198d5ae883e8c4ee9a10194a70ed41b612c70c88aaaf7b31e05bcf775591b` |
| trombone_vs_horn | 140 | TROMBONE, 49.22 s | FAIL (7 of 8 before the frames pass) | 34.9 s | none (refused) |
| trombone_vs_tuba | 358 | TROMBONE, 49.83 s | 15 of 15 pass | 145.2 s | `8afad54c815055ebaa40b04efcf8824ca2d78f4da9601175f8a78c41072917ef` |
| trumpet_vs_clarinet | 8410 | CLARINET, 50.84 s | 15 of 15 pass | 119.2 s | `f0435ce8a24d743ef240a8ac64aca90295d299de60ab188fb483aae01d7aecc1` |
| trumpet_vs_trombone | 1001 | TRUMPET, 50.28 s | 15 of 15 pass | 128.0 s | `dbfa3d7057025832cb7eea4361840fd61dd80d21b4857b3a6675f92cb820fa8a` |

17 of the 18 pass every check. **TROMBONE vs HORN (brass_trailer) fails `levels` on the kit's own fight**, seed 140
(K.O. 49.22 s): the loudest 2 s window is 8.3 dB above the quietest (at most 5 dB), and 2 windows are more than 3 dB
below the ending where one dip is allowed (56 s at -3.1 dB, 58 s at -4.7 dB; the ending is -15.0 dBFS). The other 7
checks before the frames pass, so the job refuses the video and writes no mp4, cover or caption. The kit's own checks
of that fight (`reference/episodes/trombone_vs_horn/checks.json`) fail `levels` with the same numbers (spread
8.2765 dB, dips -3.08 and -4.65 dB), so this is the kit's verdict, not a port difference. (Task 43's report says
"the config's own seed 140 passes Levels in both"; the committed checks document says it does not.)

**Its first ten fits all fail `levels` too** (`search fights/trombone_vs_horn.toml --stop 10 --max 5000`: 10 fits in
983 seeds; then `render --seed N` of each in the search's order; every other check before the frames passes on each):

| Seed | Winner, K.O. | Loudest minus quietest 2 s window (at most 5 dB) | Windows more than 3 dB below the ending (one allowed) |
| --- | --- | --- | --- |
| 820 | HORN, 51.69 s | 11.3 dB | 3: 54 s -4.4, 56 s -3.5, 58 s -5.7 |
| 75 | TROMBONE, 51.69 s | 6.5 dB | at most one |
| 983 | HORN, 50.25 s | 9.5 dB | 3: 52 s -3.1, 56 s -3.2, 58 s -5.5 |
| 203 | TROMBONE, 49.46 s | 8.9 dB | 2: 56 s -3.2, 58 s -4.8 |
| 178 | HORN, 52.29 s | 10.4 dB | 3: 54 s -3.3, 56 s -3.8, 58 s -6.0 |
| 946 | HORN, 50.80 s | 9.1 dB | 2: 56 s -3.5, 58 s -5.9 |
| 300 | TROMBONE, 52.93 s | 5.2 dB | at most one |
| 709 | TROMBONE, 52.67 s | 5.6 dB | at most one |
| 719 | TROMBONE, 49.22 s | 9.0 dB | 2: 56 s -3.3, 58 s -4.8 |
| 927 | TROMBONE, 49.47 s | 8.7 dB | 2: 56 s -3.4, 58 s -5.0 |

These are the seeds and the verdicts of the pair gate (Task 43, ruling M3-R47: the kit's checks of 820, 75 and 983
fail only `levels` as well). The cause is the brass_trailer tail after a late K.O., quiet from about 52 s to 60 s.
So TROMBONE vs HORN under brass_trailer has, today, no video that passes every check among its kit fight and its
first ten fits; its rematch (HORN vs TROMBONE, the same theme) passes. Nothing was changed.

### The six crossovers (the starter of Task 59, then `search --stop 10 --max 5000`, then `render --seed N`)

Each fight file is `bf_content::starter(left, right)` written as TOML (neutral handicap [1, 1], the rule's theme, the
kit's sketch lines); the search checked at most 5,000 seeds (the Studio search's own limit, as the pair gates and
Task 59 use), and the first fit in the search's order that passes every check is rendered.

| Pair | Theme | Seed | Winner, K.O. | Checks | Render | mp4 SHA-256 |
| --- | --- | --- | --- | --- | --- | --- |
| PIANO vs TUBA | prog_duel | 3652 (the only fit in 5,000) | PIANO, 50.13 s | 15 of 15 pass | 278.0 s | `d9610932c6a200fad2dd3329aa556d8433ef17a73d316bf4c8a176c51cb59687` |
| ALTO SAX vs ORGAN | prog_duel | 3656 (the only fit in 5,000) | ALTO SAX, 49.06 s | 15 of 15 pass | 392.0 s | `e3d4d1df93b1f2c613deeaed8d8de0584f6fe18eb0be72c075d58c8edee9be83` |
| CELLO vs FLUTE | dark_tango | no fit in 5,000 seeds | - | not rendered | - | - |
| BASS DRUM vs HORN | dark_drumline | no fit in 5,000 seeds | - | not rendered | - | - |
| EUPHONIUM vs CLARINET | second_line | no fit in 5,000 seeds | - | not rendered | - | - |
| ELECTRIC vs BASSOON | stadium_rock | no fit in 5,000 seeds | - | not rendered | - | - |

The two fits are the kit's first fits of Task 59 (3652 and 3656, the same K.O.s). The four with no fit are ruling
M3-R50's four (the kit finds none either): their handicap stays neutral, as the spec's Q6 says; the app offers
Auto-balance when its search finds nothing.

The native render of each pair gate (a new fit of the search, rendered by the gate's `native_render`) is in that
task's report and is not repeated here.

## 3. The fight: how long a Studio fight stays the kit's fight

(Task 62: the prefix window of every fighter's scenarios and every pair gate: shortest, median, how each ended.)

## 4. Statistics: the Studio's fights against the kit's

(Task 62.)

## 5. Frame and stem exceptions

(Task 62, with images.)

## 6. The reference set

(Task 62: size by folder.)

## 7. Sizes

(Task 62: the models' total size and the exe's size.)

## 8. The controller's rulings in this milestone

(Task 62.)
