"""#27's drift phonk theme under a stand-in pair (plan m4 Task 5, owner default Q2): #27 (projects/ballfight/) is GUITAR
vs PIANO v1 and neither is offered, so the theme is proven with ELECTRIC vs TUBA at #27's tempo, key, scale and loop,
the series-standard beat sheet, damage scale 1.0 and the starter rule's roles.

At the neutral start (no handicap) no seed from 1 to 5000 fits the kit's search (ELECTRIC won 112 of 120, median K.O.
36.5 s over seeds 1000 to 1119), so the handicap is the kit's one crossover lever, tuned once as kit/tuning.py does
(honest runs on seeds 1000 and up, never the search's seeds) for a median K.O. near 50 s with each side winning about
half, then frozen: ELECTRIC 0.6, TUBA 1.15 (ELECTRIC 59 % of 145 K.O.s, median K.O. 51.1 s over seeds 1000 to 1159).
SEED is the first seed of the kit's search (search.py's `one`, from seed 1) that fits."""

FIGHTERS = ('ELECTRIC', 'TUBA')
SEED = 12                              # the first fit of the kit's search from seed 1
DMG_SCALE = 1.0
DMG_SIDE = (0.6, 1.15)                 # tuned (seeds 1000-1159): ELECTRIC 86 of 145 K.O.s, median 51.1 s

BPM = 130                               # timeline.py 30: loop_bpm(130) = 129.84, 33 bars

MOOD = 'drift_phonk'                    # Table O2 (#27's audio.py bed)
KEY, SCALE = 'E', 'minor'               # timeline.py 59-60: KEY_ROOT 40, SCALE (0, 2, 3, 5, 7, 8, 10)
LOOP = ['E', 'E', 'C', 'B']             # timeline.py 62: E5 . E5 . C5 . B5 (the names #27's bed reads)
# timeline.py 62 is ['E'] + LOOP * 8 from FIGHT! on bar 1; on the 2-bar start the card takes the tonic and the
# count-in the dominant, then seven loops and #27's last three bars (E, C, B)
CHORDS = ['E', 'B'] + LOOP * 7 + ['E', 'C', 'B']
ROLES = {'ELECTRIC': 'riff', 'TUBA': 'bass'}   # the starter rule's (tools/studio_export/crossovers.py rule)
