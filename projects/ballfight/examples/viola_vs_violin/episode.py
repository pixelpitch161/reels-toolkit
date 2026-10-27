"""#42 BALLFIGHT #16: Viola vs Violin, rebuilt as an episode-kit config (the check of the VIOLA and VIOLIN ports,
kit/fighters/viola.py + violin.py, and the orchestral-drill mood, kit/moods/orchestral_drill.py). Delivered from
projects/ballfight_viola_vs_violin/ (left as delivered); this config replays the same fight (seed 8219).

The orchestra's oldest joke: the violin gets every melody, the viola gets the rests and the punchlines. VIOLA (left,
teal) COUNTING RESTS vs VIOLIN (right, violet) as built in #28. Music, mood A: orchestral drill in C minor; the VIOLIN's
bow turns sample its line (16ths), the VIOLA plays the inner harmony a fifth under it until MELODY AT LAST hands it the
lead."""

FIGHTERS = ('VIOLA', 'VIOLIN')
SEED = 8219                              # seed_search 1..6000 --stop 8 (+ a second range): top score of both (8.91)
DMG_SIDE = (0.75, 2.1)                   # tuned once (150 seeds: VIOLA 49 % / VIOLIN 51 %, median K.O. 49.5 s)

BPM = 140                                # loop_bpm -> 141.64: 36 bars, FIGHT! at 3.39 s
SERIES = 'BALLFIGHT #16'                 # (#42 predates the no-tag rule)
FX_SEED = 42

MOOD = 'orchestral_drill'
KEY, SCALE = 'C', 'harmonic minor'       # B natural on G; the viola's lead falls back to Bb over Cm / Ab / Fm
LOOP = ['Cm', 'Cm', 'Ab', 'Ab', 'Fm', 'Fm', 'G', 'G']     # drill's slow two-bar changes
CHORDS = ['Cm', 'G'] + LOOP * 4 + ['Ab', 'G']             # the last two bars fall into the title slam's Cm
ROLES = {'VIOLIN': 'melody', 'VIOLA': 'inner'}
LINES = {
    # the VIOLIN's drill-string line, one note per 16th (a 4-bar phrase with a phrygian Db sting, a third lower where it
    # would clash over Ab / Fm): the bow's quarter turns sample it; the VIOLA harmonises a fifth under it
    'VIOLIN': [72, 72, 72, 75, 75, 75, 74, 74, 72, 72, 71, 71, 72, 72, 67, 67,     # Cm
               75, 75, 75, 79, 79, 79, 77, 77, 75, 75, 74, 74, 73, 73, 72, 72,     # Cm
               68, 68, 68, 72, 72, 72, 75, 75, 80, 80, 77, 77, 77, 77, 75, 75,     # Ab
               73, 73, 73, 70, 70, 70, 73, 73, 77, 77, 77, 77, 82, 82, 84, 84,     # Ab
               72, 72, 72, 75, 75, 75, 73, 73, 72, 72, 70, 70, 72, 72, 65, 65,     # Fm
               75, 75, 75, 77, 77, 77, 77, 77, 75, 75, 73, 73, 73, 73, 72, 72,     # Fm
               68, 68, 68, 72, 72, 72, 75, 75, 80, 80, 79, 79, 77, 77, 75, 75,     # G
               74, 74, 74, 71, 71, 71, 74, 74, 77, 77, 79, 79, 83, 83, 84, 84]}    # G
SOLO = {
    # the viola, at last: a dark, singing line down to its open C string
    'VIOLA': [(0, 67, 1.0), (1, 68, 0.5), (1.5, 67, 0.5), (2, 72, 1.5), (3.5, 70, 0.5), (4, 68, 1.0), (5, 67, 0.5),
              (5.5, 65, 0.5), (6, 63, 1.0), (7, 62, 0.5), (7.5, 59, 0.5), (8, 60, 1.0), (9, 55, 1.0), (10, 48, 2.0)],
    # the violin: a spiccato flurry up the harmonic minor, a cry, sighs down, the low G
    'VIOLIN': [(0, 72, .25), (.25, 74, .25), (.5, 75, .25), (.75, 77, .25), (1, 79, .5), (1.5, 80, .25), (1.75, 83, .25),
               (2, 84, .75), (2.75, 83, .25), (3, 80, .5), (3.5, 79, .5), (4, 77, .25), (4.25, 79, .25), (4.5, 77, .25),
               (4.75, 75, .25), (5, 74, .5), (5.5, 71, .5), (6, 72, .25), (6.25, 74, .25), (6.5, 72, .25), (6.75, 74, .25),
               (7, 75, .5), (7.5, 74, .25), (7.75, 71, .25), (8, 68, .5), (8.5, 67, .5), (9, 71, .25), (9.25, 74, .25),
               (9.5, 77, .25), (9.75, 80, .25), (10, 83, .5), (10.5, 79, .5), (11, 67, 1.0)]}
TITLE_HITS = {'VIOLA': (48, 55, 63),                       # a dark chord on the C string
              'VIOLIN': (67, 70, 72, 75, 77, 79, 84)}      # a flourish up to a high C

CAPTION = "Violists, this one's for you 🎻 Comment the next match-up 👇"
