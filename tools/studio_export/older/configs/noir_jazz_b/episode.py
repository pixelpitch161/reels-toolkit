"""#29's noir jazz theme under a stand-in pair (plan m4 Task 5, owner default Q2): #29
(projects/ballfight_trumpet_vs_sax/) is TRUMPET vs SAX and the SAX is retired, so the theme is proven with TRUMPET vs
ALTO SAX, #29's lead against a sax that is not retired, at #29's tempo, key and loop, the series-standard beat sheet,
damage scale 1.0 and the starter rule's roles.

At the neutral start (no handicap) no seed from 1 to 5000 fits the kit's search (ALTO SAX won all 120, median K.O. 25.6
s over seeds 1000 to 1119), so the handicap is the kit's one crossover lever, tuned once as kit/tuning.py does (honest
runs on seeds 1000 and up, never the search's seeds) for a median K.O. near 50 s with each side winning about half, then
frozen: TRUMPET 1.1, ALTO SAX 0.38 (TRUMPET 50 % of 159 K.O.s, median K.O. 50.7 s over seeds 1000 to 1159). SEED is the
first seed of the kit's search (search.py's `one`, from seed 1) that fits."""

FIGHTERS = ('TRUMPET', 'ALTO SAX')
SEED = 67                              # the first fit of the kit's search from seed 1
DMG_SCALE = 1.0
DMG_SIDE = (1.1, 0.38)                 # tuned (seeds 1000-1159): TRUMPET 80 of 159 K.O.s, median 50.7 s

BPM = 132                               # timeline.py 34: loop_bpm(132) = 133.77, 34 bars

MOOD = 'noir_jazz'                      # Table O2 (#29's audio.py bed)
KEY, SCALE = 'C', 'minor'               # timeline.py 64: KEY_ROOT 36 (scale_notes 81: C natural minor)
LOOP = ['Cm', 'Ab7', 'Fm', 'G7']        # timeline.py 67: the noir loop i - bVI7 - iv - V7
# timeline.py 67 is ['G7'] + LOOP * 8 + ['Ab7'] from FIGHT! on bar 1; on the 2-bar start the card takes the
# tonic and the count-in the dominant, then the same eight loops: the last G7 falls into the card's Cm
CHORDS = ['Cm', 'G7'] + LOOP * 8
ROLES = {'TRUMPET': 'melody', 'ALTO SAX': 'line'}   # the starter rule's (tools/studio_export/crossovers.py rule)
SOLO = {
    # timeline.py 113-116: #29's TRUMPET solo
    'TRUMPET': [(0, 72, 0.5), (0.5, 75, 0.5), (1, 79, 0.75), (1.75, 77, 0.25), (2, 75, 0.5), (2.5, 74, 0.25),
                (2.75, 72, 0.25), (3, 79, 1.0), (4, 80, 0.5), (4.5, 78, 0.25), (4.75, 75, 0.25), (5, 72, 0.5),
                (5.5, 75, 0.5), (6, 78, 0.75), (6.75, 80, 0.25), (7, 84, 1.0), (8, 82, 0.25), (8.25, 80, 0.25),
                (8.5, 77, 0.5), (9, 80, 0.5), (9.5, 77, 0.25), (9.75, 75, 0.25), (10, 72, 0.5), (10.5, 75, 0.5),
                (11, 79, 0.5), (11.5, 81, 0.5), (12, 83, 1.25)],
}
