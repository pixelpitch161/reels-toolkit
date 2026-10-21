# Finished projects

These are frozen source snapshots of the delivered videos. They predate `reelkit`, so each is
self-contained: it has its own copy of the font and helpers. They are kept for reference and
for lifting scenes, simulations and music. New videos should start from `templates/new_video`.

| folder | build order | output |
|---|---|---|
| `100_metronomes` | `python sim.py` → `python audio.py` → `python make_video.py` | 100_metronomes.mp4 |
| `sand_draws_sound` | `python sandsim.py` (sand cache + drive.npz) → `python audio.py` → `python make_video.py out.mp4` (uses `render2.py`; `render_v1.py` is the first version) | sand_draws_sound_v2.mp4 |
| `crickets_open` | `python audio.py` → `python make_video.py` | cricket_thermometer.mp4 |
| `crickets_diorama` | `python audio.py` → `python make_video.py out.mp4` (uses `drender.py` + `dscene.py`) | cricket_diorama_v4.mp4 |
| `lowest_note` | `python audio.py` → `python -m reelkit.video render.py audio.wav lowest_note.mp4` (built on `reelkit`, see its README) | lowest_note.mp4 |
| `count_the_thunder` | `python audio.py` → `python -m reelkit.video render.py audio.wav count_the_thunder.mp4` (built on `reelkit`; checks in its README) | count_the_thunder.mp4 |

Generated files (`*.wav`, `*.mp4`, `*.rgb`, `*.f32`, `*.f16`, `*.npz`, `test/`, `check/`) are git-ignored.
