# BALLFIGHT Studio: plan (user, Oct 27)

**Goal (user):** "take what we've designed so far and … create a program that can simulate and render battles with more
user flexibility", running **on the user's own Windows computer**. Fights between existing fighters with existing
moods then cost no Claude tokens; Claude is only needed for new fighters, new moods and new features.

**Built on the episode kit** (`projects/ballfight/README.md`, "Making an episode"): config -> honest sim -> seed search ->
render -> loop-clean mp4. The Studio is a front end + packaging on top of it. Fights stay honest: the same simulation;
the user picks a seed, never edits a fight.

## What the user gets

- `Install BALLFIGHT Studio` (one-time) and a `BALLFIGHT Studio` desktop shortcut that starts a local server and
  opens the browser at `http://localhost:<port>`. Nothing leaves the computer.
- **Pick the fight:** any two fighters from the roster (cards with colour, signature and abilities; retired GUITAR /
  SAX hidden), who's left / right, mood (only moods in `kit/moods/`), tempo (snapped with `loop_bpm`), a balance slider
  (the per-fighter damage handicap, the crossover rules' only lever) with an "auto-balance" button (runs `tune`).
- **Seed browser:** runs the search with progress; a ranked list with each fight's story at a glance (K.O. time, winner +
  HP left, lead changes, ability timeline as a strip). Click one -> an **instant schematic preview** (balls, weapons and
  effects as simple shapes on a canvas, drawn in the browser from the recorded run, scrub-able, with the music if the
  audio is built) and a **real low-res preview** on demand (the kit's preview path, a few seconds of frames).
- **Render:** one button: audio -> all checks (safe zone, focus, levels, band, loudness, flash, loop) -> the one full
  render -> verify; a progress bar with time left; the result: `Videos\BALLFIGHT\<a>_vs_<b>\<a>_vs_<b>.mp4` + `caption.txt`
  (captions + tags for Instagram, TikTok, YouTube Shorts from `post`) + the episode config, so the fight can be rebuilt.
- **History:** past fights listed with their thumbnails; "re-open" loads the config.
- Later (not v1, needs engine work): arenas, 3–4 fighter free-for-alls, tournament brackets.

## Technical shape

- Python backend (FastAPI or Flask; keep dependencies few) wrapping the kit's steps as jobs (one at a time, cancelable,
  progress via polling or SSE). Front end: one static page (vanilla JS + canvas; no Node build step).
- **Windows-safe kit:** the kit uses `multiprocessing.get_context('fork')` (`kit/preview.py`, `kit/search.py`,
  `kit/tuning.py`, `kit/encode.py`); Windows has no fork. Switch to `spawn` (picklable workers, `if __name__ ==
  "__main__"` guards, module-level worker functions), keep results identical (replay guard + the examples' hashes).
  `PYTHONHASHSEED=0` must be set in the environment before Python starts (the launcher sets it; workers inherit it).
  Paths via `pathlib` (no hard-coded `/`), ffmpeg found on PATH or next to the app, temp files under the app folder.
- Render speed scales with cores (pool size = cores - 1 by default, setting in the UI).
- **Windows CI:** a GitHub Actions workflow on `windows-latest` that installs the requirements and runs a smoke test
  (one config: sim + tune 20 seeds + 2 s preview + a 3 s render + verify), so Windows breakage shows up without the
  user's machine. Linux CI runs the replay guard.

## Sessions (in order)

1. **Port everything into the kit** (`docs/studio/PLAN.md` step 1): every fighter in `fighters.py` except the retired
   GUITAR and SAX (kept for #27 / #29 replays only) and those already ported or being ported by an active session
   (check `kit/fighters/` and the board first; at the time of writing #44 is porting ACOUSTIC + adding ELECTRIC and #45
   will add PIANO v2 + port ORGAN), and every episode's mood into `kit/moods/` (named by mood, e.g. `drift_phonk`,
   `balkan_fire`, `noir_jazz`, `sea_shanty`, `dark_breakbeat`, `villain_march`, `spaghetti_western`, `brass_trailer`,
   `gothic_horror`, `brass_house`, `dark_drumline`, `ska_punk`, `industrial`, `orchestral_drill`, …). For each: port
   from its latest episode, then an `examples/` config that rebuilds that episode, verified with the hash, a few frames
   and `audio.wav` (as the README's porting steps say). Small commits; replay guard stays green. Mood beds take tempo +
   chords as inputs, so any fighter pair can use any mood.
2. **Build the Studio** (step 2): the Windows-safe kit (spawn), the backend jobs, the page (fight picker, seed browser,
   schematic preview, real preview, render with progress, outputs, history), the Windows CI smoke test.
3. **Package it for Windows** (step 3): `install.ps1` / `Install BALLFIGHT Studio.bat` (winget or official installers:
   Python 3.11+, ffmpeg, Git; a venv; `pip install -r requirements.txt`), the `BALLFIGHT Studio` shortcut / `.bat`
   launcher (sets `PYTHONHASHSEED=0`, starts the server, opens the browser), **updates** (a button or `Update.bat`:
   `git pull` + reinstall requirements; the repo is private, so the guide covers signing in to GitHub once, e.g. with
   GitHub Desktop or `gh auth login`), and `docs/studio/GUIDE.md`: install, make a fight, where files go, what needs
   Claude (new fighters / moods / features: ask in the ideas chat).

Each session: log in `docs/log/ballfight_studio.md`; keep every finished episode replaying identically.
