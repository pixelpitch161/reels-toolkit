# BALLFIGHT Studio: handoff brief (updated Nov 15)

## Status (Nov 15): 30 fighters, 26 themes, every pairing starts

**What exists.** BALLFIGHT Studio is a native Windows program in the `studio/` folder of this repo: the engine (the
fight, the story and seed search, the balance, the picture, the sound, the quality checks and the video encode)
and the app's window on top of it, with its five steps (Match-up, Balance, Seeds, Music, Render), a live preview
with sound, the shortlist of seeds, the render queue, the Library of finished videos and Settings. Everything it
shows and saves is made by the real engine. The Studio is now the source of truth for BALLFIGHT: the Python kit in
`projects/ballfight/` is left exactly as it is and its README is not updated.

**The roster today.**

- **30 fighters, all offered in the app.** The 20 kit fighters (ACOUSTIC, ALTO SAX, BASS CLARINET, BASS DRUM,
  BASSOON, CELLO, CLARINET, CONTRABASS, ELECTRIC, EUPHONIUM, FLUTE, HORN, OBOE, ORGAN, PIANO, SOUSAPHONE, TENOR SAX,
  TROMBONE, TRUMPET, TUBA) are proven exact against the Python kit. The 10 older fighters (ACCORDION, BAGPIPES,
  BARI SAX, DRUMS, HARP, SYNTH, TURNTABLES, UKULELE, VIOLA, VIOLIN) are ported "close enough for the eye test", as
  the owner decided on Nov 14: they fight, look and sound like themselves, but are not checked bit for bit against
  their old episodes.
- **26 themes.** The 14 kit themes are exact (baroque storm, big top, bossa noir, brass trailer, dark disco, dark
  drumline, dark tango, heist, horror trap, hot jazz, mambo, prog duel, second line, stadium rock). The 12 older
  themes are ported by ear (Balkan fire, brass house, dark breakbeat, drift phonk, gothic horror, industrial, noir
  jazz, orchestral drill, sea shanty, ska-punk, spaghetti western, villain march).
- **Every one of the 870 pairings starts** (30 fighters, each against each of the other 29, either side). Each
  older fighter starts on its own home theme. A pair is offered only the themes whose music has every hit its
  fighters play.
- **The seed finder balances.** When a pairing is uneven, Auto-balance sets the handicap so the fight is fair, and
  the search then finds seeds that fit the story. GUITAR and SAX stay retired and are never offered.

**What the owner can make now.** Any of the 870 pairings, on any theme the pair is offered: pick the two fighters,
balance, pick a seed from the shortlist with its picture and sound, and render a checked 61 s video with its cover
and caption into `Videos\BALLFIGHT`.

**Proof that it matches the kit** (details in `docs/studio/m3-measurements.md`). Every kit pairing passed its pair
gate (17 pairs plus milestone 1's FLUTE vs TROMBONE). Rendered from the kit's own fights, 17 of the 18 kit episodes
pass all 15 checks; the 18th is TROMBONE vs HORN (see the limits below). Of the six crossover pairings, the two the
kit can fit also render with all 15 checks.

**How fast it is** (milestones 1 and 2, this PC): a whole 61 s video takes about 82 seconds (goal under 60 s; most of
it is the video encoder); the seed search runs about 6,000 to 7,000 fights a second.

**How it is started today.** There is no installer yet (milestone 6). The program is built from the code:
`cargo build --release -p bf-app` in the `studio` folder makes `studio\target\release\ballfight-studio.exe`, which is
the app. Its settings and queue live in `%APPDATA%\BALLFIGHT Studio`, its shortlist cache in
`%LOCALAPPDATA%\BALLFIGHT Studio`. It needs ffmpeg to save videos (Settings says whether it found it). The
`ballfight` command line (`studio/README.md`) runs every step without the window.

**What is left.**

1. **Milestone 5, the music system: waiting for the owner's choice.** The three options are: skip it for now
   (recommended), a lean story layer on top of today's themes, or a full composer. Nothing is built until the owner
   picks one.
2. **Milestone 6, the guide and the release:** the installer, the Update button and the owner's guide (plan:
   `docs/studio/2026-11-11-ballfight-studio-plan-m6-packaging.md`, tasks 17 to 19 still to do).
3. **Milestone 3's last two tasks** (62 and 63): the long measurement runs and the tidy list. The measurements file
   says which sections are still empty and why.

**Known limits** (from the milestone 3 and 4 ledgers):

- **TROMBONE vs HORN on brass trailer has no seed that passes every check.** The kit's own fight (seed 140) and its
  first ten fits all fail the loudness check: after a late K.O. the brass trailer ending is too quiet. The kit gives
  the same verdict. The rematch (HORN vs TROMBONE, same theme) passes. Pick another theme for that pair. The app does
  not yet suggest another theme by itself.
- **The seed search does not listen to the soundtrack**, so a seed from the shortlist can still fail the loudness
  check at render time. The render job then refuses it and says why; pick the next seed.
- **The FIGHT! caption fails the safe zone at 138 BPM**, for any pairing. Avoid 138 BPM until it is tidied.
- **The band check ignores an early K.O.** It looks at 32 to 44 s, so a fight with a K.O. at 31 s would fail it. The
  seed finder only offers K.O.s between 49 and 53 s, so this does not affect real videos.
- **Four kit crossovers have no fit at an even handicap** (CELLO vs FLUTE, BASS DRUM vs HORN, EUPHONIUM vs CLARINET,
  ELECTRIC vs BASSOON; the kit finds none either). Use Auto-balance.
- **Some older fighters need a big handicap** against some kit fighters (VIOLIN against TROMBONE, for example).
  Auto-balance handles it.
- **A very late K.O. can leave the winner no solo.**
- **A fight the Studio runs itself is its own fight**: it matches the kit's fight for at least the first 25 seconds,
  then tiny rounding differences take over (by design).
- **The preview's speed tests can fail when the PC is busy**; they pass on a quiet PC.

The owner is building the BALLFIGHT software in its own project. **Start here, then read the files
below in this order.** Everything about how a fight is simulated, drawn, scored and rendered already exists in this
repo as the episode kit; the software is a front end + packaging on top of it, not a rewrite.

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
- **Not in the kit (old episode folders only), but in the Studio since milestone 4:** fighters ACCORDION, BAGPIPES,
  BARI SAX, DRUMS, HARP, SYNTH, TURNTABLES, UKULELE, VIOLA, VIOLIN; their moods (drift phonk, Balkan fire, noir jazz,
  sea shanty, dark breakbeat, villain march, spaghetti western, gothic horror, brass house, ska-punk, industrial,
  orchestral drill). They live only in the Studio; the kit is not changed for them. Progress:
  `docs/log/ballfight_studio.md`. **GUITAR and SAX are retired**: keep their roster entries
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
- Whether new fighters / moods will still come from building sessions (yes so far: the ideas chat drafts, a session builds,
  the app picks them up after an update).

## 5. Useful references

- Viewer requests and feedback: `docs/ideas/requests.md` (e.g. the TUBA is the most criticised fighter).
- Fighter redesign history: `docs/ballfight/redesign-2026-10-24.md`.
- Each episode's design: `docs/ideas/NN-*.md` (linked from the board, `docs/IDEAS.md`).
