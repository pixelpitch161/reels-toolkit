"""#27's drift phonk theme under a stand-in pair (plan m4 Task 5, owner default Q2): #27 (projects/ballfight/) is GUITAR
vs PIANO v1 and neither is offered, so the theme is proven with PIANO vs ELECTRIC at #27's tempo, key, scale and loop,
the series-standard beat sheet, damage scale 1.0 and the starter rule's roles.

At the neutral start (no handicap) no seed from 1 to 5000 fits the kit's search (PIANO won 113 of 120, median K.O. 35.1
s over seeds 1000 to 1119), so the handicap is the kit's one crossover lever, tuned once as kit/tuning.py does (honest
runs on seeds 1000 and up, never the search's seeds) for a median K.O. near 50 s with each side winning about half, then
frozen: PIANO 0.6, ELECTRIC 0.81 (PIANO 48 % of 180 K.O.s, median K.O. 50.8 s over seeds 1000 to 1199). SEED is the
first seed of the kit's search (search.py's `one`, from seed 1) that fits."""

FIGHTERS = ('PIANO', 'ELECTRIC')
SEED = 176                              # the first fit of the kit's search from seed 1
DMG_SCALE = 1.0
DMG_SIDE = (0.6, 0.81)                 # tuned (seeds 1000-1199): PIANO 86 of 180 K.O.s, median 50.8 s

BPM = 130                               # timeline.py 30: loop_bpm(130) = 129.84, 33 bars

MOOD = 'drift_phonk'                    # Table O2 (#27's audio.py bed)
KEY, SCALE = 'E', 'minor'               # timeline.py 59-60: KEY_ROOT 40, SCALE (0, 2, 3, 5, 7, 8, 10)
LOOP = ['E', 'E', 'C', 'B']             # timeline.py 62: E5 . E5 . C5 . B5 (the names #27's bed reads)
# timeline.py 62 is ['E'] + LOOP * 8 from FIGHT! on bar 1; on the 2-bar start the card takes the tonic and the
# count-in the dominant, then seven loops and #27's last three bars (E, C, B)
CHORDS = ['E', 'B'] + LOOP * 7 + ['E', 'C', 'B']
ROLES = {'PIANO': 'melody', 'ELECTRIC': 'riff'}   # the starter rule's (tools/studio_export/crossovers.py rule)
SOLO = {
    # timeline.py 67-70: #27's PIANO motif (the theme's own victory solo)
    'PIANO': [(0, 64, 0.5), (0.5, 67, 0.5), (1, 71, 0.5), (1.5, 76, 0.5), (2, 79, 0.75), (2.75, 78, 0.25), (3, 76, 0.5),
              (3.5, 74, 0.5), (4, 72, 0.5), (4.5, 76, 0.5), (5, 79, 0.5), (5.5, 84, 1.0), (6.5, 83, 0.25),
              (6.75, 81, 0.25), (7, 79, 0.5), (7.5, 76, 0.5), (8, 75, 0.5), (8.5, 78, 0.5), (9, 81, 0.5),
              (9.5, 83, 0.5), (10, 87, 0.25), (10.25, 83, 0.25), (10.5, 81, 0.25), (10.75, 78, 0.25), (11, 75, 1.0)],
}
