# #50 BALLFIGHT: Trumpet vs Trombone (`projects/ballfight_trumpet_vs_trombone`)

- **Oct 29, v1 (session_01CzTfqd).** Episode-kit crossover: TRUMPET (left, yellow) vs TROMBONE (right, mint), both as in
  `fighters.py`; the only lever is DMG_SIDE = (2.8, 0.40). The reach gap is large (the lance outreaches the pistons:
  ~16 trumpet hits land to ~63 thrusts), so the trumpet needs ~7x the per-hit scale; damage alone still balanced it
  (51 %, median K.O. 51.2 s). Seed 1001 (8 fits in 1,096 runs): TRUMPET wins 50.28 s on 1 HP, HIGH C vs PEDAL TONE.
- New mood `kit/moods/mambo.py`: piano montuno (Pixel Pitch grand), contrabass pizz tumbao (fifth on the and of 2,
  next root on 4), congas, timbales (cascara front half, mambo bell back half, abanico into every phrase and a fill into
  every ability via `M.EV`), cowbell, 2-3 claves, a low alto-sax pad. No brass of its own.
- Lesson: the first `check` failed `band` for the trombone (its low riff shares 120-1500 Hz with the tumbao, sax pad and
  low montuno; the loser's thinning took it to -0.1 dB at the end). MIX trombone 2.4 / bass 1.5 / sax 0.7 fixed it
  (+2.6 dB worst section) with levels and loudness still passing.
- Timings (2 cores): tune 3 grids ~7 min, search 104 s, sheet 33 s, audio 11 s, check 220 s, render 367 s + encode
  220 s, verify 46 s. Replay hashes of earlier episodes identical (diff vs a pre-change checkout).
