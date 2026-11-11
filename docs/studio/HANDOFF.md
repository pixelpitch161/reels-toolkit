# BALLFIGHT Studio: handoff brief for a new Claude Code project (Nov 10)

## Status (Nov 11): the Studio's engine exists

**What exists.** The whole engine of BALLFIGHT Studio, rebuilt as a native Windows program: the fight, the story
and seed search, the balance, the picture, the sound, the quality checks and the video encode. It lives in the
`studio/` folder of this repo. Right now it knows one match-up, FLUTE against TROMBONE with the big top music, and
it is driven by one command, `ballfight`. The window (the app with its five steps, preview and queue) is the next
milestone and is being built: its screens (Match-up, Balance, Seeds, Music, Render, Library, Settings) already exist
in the `bf-app` crate, but they still show sample data and are not connected to the engine yet, so nothing in that
window today is a real fight or a real video.

**It makes the same video as the Python kit.** For the example episode (`projects/ballfight/examples/
flute_vs_trombone`, seed 392) the Studio draws all 43 checked frames of the video byte for byte as the kit does,
and its sound file is the kit's sound file sample for sample. Every quality check (loudness, levels, band, flashes,
loop point, safe zone, focus) gives the kit's verdict. A fight the Studio runs itself matches the kit's fight for at
least the first 25 seconds of fighting; after that tiny rounding differences take over, so a Studio seed is its own
fight, not the kit's (that is by design).

**How fast it is** (this PC; details in `docs/studio/m1-measurements.md`):

- A whole 61 s video, from the fight to the checked mp4 with its cover and caption: about **82 seconds** (it was
  about 97 s before the last speed-ups). The goal is under 60 s; most of the remaining time is the video encoder
  itself, and the plan for getting there is in the measurements file.
- Searching seeds: about **6,000 to 7,000 fights a second**, so 200 fitting seeds take about 5 seconds.
- Building the soundtrack: about **4.7 seconds**.
- One picture of the video: about 0.13 to 0.22 seconds on one core, so live playback at 30 frames a second needs
  about 7 cores drawing ahead. The app is planned that way.

**How to run it.** There is no double-click program yet (that comes with the app), so for now this step is for
whoever is developing it, not something the owner has to do. Today it takes one command. Open
PowerShell in the repo's `studio` folder (in File Explorer: open `studio`, type `powershell` in the address bar,
press Enter) and paste:

```
cargo build --release -p bf-cli; .\target\release\ballfight.exe render fights\flute_vs_trombone.toml --rec reference\fights\flute_vs_trombone\ep392.bfrec --out "$env:USERPROFILE\Videos\BALLFIGHT"
```

The first build takes a few minutes; after that it renders the example episode into
`Videos\BALLFIGHT\flute_vs_trombone\` (the mp4, the cover picture, the caption and the fight file). It never
replaces a video that is already there: a second render goes into `flute_vs_trombone_seed392\`. Other commands
(find seeds that fit, balance the pair, draw one frame, build only the sound, run only the checks, measure the
speed) are listed in `studio/README.md`.

**What comes next.** Milestone 2 is the app: the window with the five steps, live preview with sound, the shortlist
of seeds, the render queue and the Library of finished videos, all on FLUTE against TROMBONE. Then the other 18 kit
fighters and 13 themes (milestone 3), the older ten fighters (4), the music system (5), and packaging with the
Update button (6).

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
- **In the kit (usable in the app now), 20 fighters:** ACOUSTIC, ALTO SAX, BASS CLARINET, BASS DRUM, BASSOON, CELLO,
  CLARINET, CONTRABASS, ELECTRIC, EUPHONIUM, FLUTE, HORN, OBOE, ORGAN, PIANO, SOUSAPHONE, TENOR SAX, TROMBONE, TRUMPET,
  TUBA. **14 moods:** baroque_storm, big_top, bossa_noir, brass_trailer, dark_disco,
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
