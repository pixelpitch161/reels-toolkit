# #56 BALLFIGHT: Bass Drum vs Tuba (`projects/ballfight_bassdrum_vs_tuba`, session_01CzTfqd, Oct 31)

A viewer request ("Base drum vs tuba", twice), made with the episode kit, straight through (no approval waits).

**Made**
- **BASS DRUM (new, gunmetal, left).** Model from Commons reference photos found with Claude in Chrome ("Camden Muster
  bass drum.jpg" + the concert / marching drums of the "concert bass drum tension rods" search):
  `series/instrument-history/bassdrum_lowpoly.py`, a marching 28 x 14 in drum, gunmetal shell, plain white heads (no
  logos), dark hoops, 10 chrome T-rods a side, a felt mallet; voice in the same file (boom / mallet / roll; own commit);
  bridge entry `BASS DRUM`. Look + sound `kit/fighters/bass_drum.py`.
- **Engine (additive, own commits, replay guard identical for every finished episode):** weapon kind `roll_charge`
  (STEAMROLLER: tip onto its side with the line telegraphed, roll like a wheel, unstoppable while rolling, first contact
  = BOOM; speed + size grow per landed boom; recorded as RCHG), ability kinds `heartbeat`, `drum_roll`, `cannons`
  (slams with an optional hit kind), and the opt-in rule **`Rules(swallow_rollers=True)`**: swallowing a rolling
  fighter wasn't defined - by default the swallow bell can't catch a rolling drum; with the rule it can and the gulp
  ends the roll. This episode turns it on (the tuba gulps the drum mid-roll ~9x a fight).
- **TUBA** exactly as built, ported into the kit from #36 (see `ballfight_studio.md`).
- **Mood** `dark_drumline` with the new opt-in `DRUMLINE = dict(bass_drums=False, after_ko=True)`: the bed's own bass
  drums are off in the fight (the fighter is the kick and the booms), back from the K.O. on (#53's audio bit-identical
  without the option). The tuba plays oom-pah bass (LINES). 1812 OVERTURE's five cannons are five tuned booms playing the
  opening of "God Save the Tsar!" (Lvov 1833; scale degrees 5 6 6 5 3 from hymnary.org's RUSSIAN HYMN incipit), the
  anthem the 1812 finale sets under its cannons, in E-flat (the 1812's key, relative major of C minor), the pit doubling
  it and tubular bells ringing, resolving to the tonic.
- **Tuning** (300 seeds x 6 cells): DMG_SCALE 0.66, DMG_SIDE (0.86, 1.0): BASS DRUM 58 % / TUBA 42 %, median K.O. 53.7 s.
  **Search** at 2 bars: 8 fits in 341 runs; seed **132** (7.59): the drum leads, 1812 OVERTURE at 42.4 s, the tuba
  comes back (lead changes 43.2 / 48.1 s) and wins at 49.84 s on 0.1 HP.
- **Fixes on the way:** the levels check failed on the victory (two dips: the fight's kick-every-beat makes a loud
  floor) - the bed's bass drums return after the K.O. (`after_ko`), the solo bus +5 dB (MIX), both solos run to the
  reassembly: PASS. A roll whose first stroke started a hair before 0 gave an empty slice (fixed in the voice).
