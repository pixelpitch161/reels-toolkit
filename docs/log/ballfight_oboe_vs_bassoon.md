# Log: #52 BALLFIGHT: Oboe vs Bassoon (`projects/ballfight_oboe_vs_bassoon/`)

## Oct 29: made (session_01CzTfqc)
- Episode-kit crossover: the #37 OBOE (left, olive) vs the #39 BASSOON (right, forest green), both exactly as in
  `fighters.py`; sides set in `episode.py` only. No episode number on screen.
- BASSOON wasn't in the kit: ported from #39 v4 into `kit/fighters/bassoon.py` (see `docs/log/ballfight_studio.md`),
  with additive kit hooks for a hopping ball (squash / stretch, airborne depth) and HUD holds for hook / mountain. New
  with the port: under the oboe's OUT OF TUNE its notes play flat against a sour twin, the Grieg quote included.
- New mood `kit/moods/heist.py`: brushes + rim clicks, walking upright, pizzicato, muted vibes, tremolo + clock tick;
  no woodwinds. D minor, loop_bpm(118) = 118.03, 30 bars. Oboe = high nasal melody (LINES), bassoon = low comic
  staccato line (its hops are its notes). MOUNTAIN KING = Grieg's public-domain theme in D minor, a note per hop.
- Tuning (the only lever, frozen): DMG_SIDE = (0.6, 0.85): 300 seeds 50 / 50, median K.O. 49.8 s (the oboe out-hits
  the bassoon ~1.8:1, so the bassoon needs the bigger share; the balance point is steep: x0.6 / x0.4 the oboe won
  100 %, x0.45 / x1.0 the bassoon did).
- Seed 954: BASSOON wins at 51.6 s, 3 lead changes, ILL WIND (41.7 s) then MOUNTAIN KING (42.2 s) out of tune.
- Checks: safezone, focus (CoC 0), levels (spread 1.6 dB), band (oboe +10..+16 dB, bassoon +8..+14 dB over the bed),
  flash, loop all PASS; mp4 1080x1920 61.0 s h264 + aac, loop + flash PASS. Render 351 s + encode 210 s.
