# tools

- `python -m reelkit.sheet render.py out.png t1 t2 ...`: contact sheet from a render module
- `python -m reelkit.sheet render.py out.png --frame T`: one full-size frame
- `python -m reelkit.video render.py audio.wav out.mp4 [--procs N]`: full render + mux
- `reelkit.video.frames_sheet(mp4, times, out)`: contact sheet from a finished video
- `reelkit.audio.section_levels(wav, spans)` / `band_level(wav, lo, hi, spans)`: loudness checks
- `python tools/ballfight_replay_hash.py`: hashes of the finished BALLFIGHT episodes' honest runs (events + recorded
  arrays); run before and after an engine change and `diff` them: every line must match (parallel-production rule)
