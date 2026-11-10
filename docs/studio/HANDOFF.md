# BALLFIGHT Studio: handoff brief for a new Claude Code project (Nov 10)

The user is building the BALLFIGHT software themselves in a Claude Code project. **Start here, then read the files
below in this order.** Everything about how a fight is simulated, drawn, scored and rendered already exists in this repo
as the episode kit; the software is a front end + packaging on top of it, not a rewrite.

## 1. Read these (in order, ~600 lines total)

1. `docs/studio/PLAN.md`: the product spec (what the app does, technical shape, Windows notes, build steps).
2. `projects/ballfight/README.md`: the episode kit: the commands (`python -m ballfight new|tune|search|preview|sheet|
   audio|check|render|verify|post <episode>`), the config format, where fighters and moods live, porting steps, timings.
3. `docs/ballfight/RULES.md`: the series rules every video must follow (title card, HUD, captions, colours table,
   crossover rules, retired fighters, no episode number, no approval waits).
4. `docs/PLAYBOOK.md`: the video rules behind them (61.0 s, 1080x1920, 30 fps, safe zone, loop point, loudness, flashes).
5. One finished kit episode end to end: `projects/ballfight/examples/flute_vs_trombone/` (a ~60-line config) and its
   episode folder, to see a config → mp4 run.

Don't read the other ~55 idea files, logs or old episode folders unless a task needs one (they're history).

## 2. What exists today

- **Engine:** `projects/ballfight/fight.py` (honest, deterministic 2D sim), `fighters.py` (the roster: numbers per
  fighter), `projects/ballfight/kit/` (picture, sound, HUD, title card, tune, search, check, encode), `reelkit/` (the
  shared renderer, effects, UI, checks).
- **In the kit (usable in the app now), 22 fighters:** ACOUSTIC, ALTO SAX, BASS CLARINET, BASS DRUM, BASSOON, CELLO,
  CLARINET, CONTRABASS, ELECTRIC, EUPHONIUM, FLUTE, HORN, OBOE, ORGAN, PIANO, SOUSAPHONE, TENOR SAX, TROMBONE, TRUMPET,
  TUBA (+ the base / shared reed helpers). **14 moods:** baroque_storm, big_top, bossa_noir, brass_trailer, dark_disco,
  dark_drumline, dark_tango, heist, horror_trap, hot_jazz, mambo, prog_duel, second_line, stadium_rock.
- **Not in the kit yet (in old episode folders only):** fighters ACCORDION, BAGPIPES, BARI SAX, DRUMS, HARP, SYNTH,
  TURNTABLES, UKULELE, VIOLA, VIOLIN; their moods (drift phonk, Balkan fire, noir jazz, sea shanty, dark breakbeat,
  villain march, spaghetti western, gothic horror, brass house, ska-punk, industrial, orchestral drill). Porting steps:
  the kit README; progress: `docs/log/ballfight_studio.md`. **GUITAR and SAX are retired**: keep their roster entries
  (old episodes replay), never offer them in the app.
- **Checks that must keep passing:** `tools/ballfight_replay_hash.py` (every finished episode replays identically), the
  kit's `check` (safe zone, focus, levels / consistent loudness, band, flash, loop) and `verify`.

## 3. Non-negotiables for the software

- **Honest fights:** the app runs the real simulation; the user picks a seed from the search, never edits a fight.
  Balance only through the per-fighter damage handicap (the crossover rules).
- **Determinism:** `PYTHONHASHSEED=0` before Python starts (the launcher sets it); same config + seed = same video.
- **Don't change finished episodes:** engine / kit / title changes are additive and opt-in; the replay guard stays
  green; every earlier title card and HUD draws exactly as before.
- **Video spec:** 61.0 s, 1080x1920, 30 fps, h264 + aac; loop point at the file boundary; the safe zone; consistent
  loudness from frame 0 (`reelkit.levels`); no full-screen flashes (`reelkit.flashcheck`); no episode number on screen;
  colours from the table in `docs/ballfight/RULES.md` (one unique colour per instrument).
- **Windows:** the kit's process pools use `fork` (`kit/preview.py`, `kit/search.py`, `kit/tuning.py`, `kit/encode.py`);
  Windows needs `spawn` (module-level workers, `if __name__ == "__main__"` guards), with identical results. Paths via
  `pathlib`; ffmpeg on PATH or bundled. This dev container has 2 cores; the user's PC will be faster.

## 4. Ask the user (things only they can decide)

- PC specs: Windows version, CPU cores, RAM, free disk (a render keeps ~1.2 GB of raw frames), GPU (not needed).
- Where outputs go (default `Videos\BALLFIGHT\<a>_vs_<b>\`) and what each export includes (mp4, caption.txt, config).
- How much control they want in v1: fighter pick, sides, mood, tempo, handicap slider / auto-balance, seed browser,
  schematic preview, history (PLAN.md's list), and anything later (arenas, 3–4 fighter brawls, tournaments, the
  "all instruments vs the band director" boss fight from `docs/ideas/requests.md`).
- Whether the app stays in this repo (recommended: it imports the kit directly) and how they'll get updates (git pull).
- Whether new fighters / moods will still come from Claude sessions (yes so far: the ideas chat drafts, a session builds,
  the app picks them up after an update).

## 5. Useful references

- Viewer requests and feedback: `docs/ideas/requests.md` (e.g. the TUBA is the most criticised fighter).
- Fighter redesign history: `docs/ballfight/redesign-2026-10-24.md`.
- Each episode's design: `docs/ideas/NN-*.md` (linked from the board, `docs/IDEAS.md`).
