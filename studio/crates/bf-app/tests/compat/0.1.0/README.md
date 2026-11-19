# Files saved by version 0.1.0

These three files are exactly what BALLFIGHT Studio 0.1.0 writes. They were made once, on Nov 11, 2026, by the app's
own code (`make_this_versions_files` in `tests/compat.rs`, run with `BF_BLESS=1`), and are frozen: their SHA-256s are
written in that test file, and `the_frozen_files_are_unchanged` fails if one changes.

- `settings.toml`: the settings file (`config\settings.toml`), saved by the store's `Writer`: videos go to
  `D:\Vidéos du studio\BALLFIGHT`, 3 threads.
- `queue.json`: the render queue (`config\queue.json`), saved by the store's `Writer`: Flute vs Trombone seed 20418
  finished (all ten checks done, 82.25 s), and Flute vs Trombone seed 18433 waiting. Both fights were made by the
  engine's `Enqueue` from a new match-up (the roster's starting pair, its first theme, the drafted captions, the seed).
- `fight.toml`: the fight file a render leaves in the finished video's folder (`flute_vs_trombone\flute_vs_trombone.toml`):
  the queued fight file of seed 20418, read and written out again, as `bf-export` writes it.

`tests/compat.rs` reads them with the code as it is now and checks that they mean the same: the settings' values with
no notice, both queued videos (with the "saved by another version" warning once the app is not 0.1.0), and the fight
file listed in the Library and reopened by the engine as the same fight.

**The rule: every release adds a folder for its own version, and no folder is ever rewritten.** At a release, run
`make_this_versions_files` with `BF_BLESS=1` once (it refuses a folder that already holds files), add the new folder's
SHA-256s to `FROZEN` in `tests/compat.rs`, and write a README like this one in the new folder.
