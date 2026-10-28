"""#46 BALLFIGHT: Trumpet vs Clarinet (episode kit config: projects/ballfight/README.md, "Making an episode").
Idea + hook: who leads the jazz band? In New Orleans jazz the trumpet blasts the melody and the clarinet weaves around
it; later the clarinet had its swing-era moment as the bandleader's instrument. A crossover: TRUMPET (left, yellow) as
built in #29 v3 (VALVE COMBO), CLARINET (right, lavender) as built in #37 (TWELFTH JUMP), both straight from the roster;
the only lever is the damage handicap (DMG_SIDE). No episode number on screen.
Music: mood A, dark hot jazz (kit/moods/hot_jazz.py: a fast minor two-beat, tuba on 1 and 3, the guitar chopping 2 and
4, snare press rolls, brushed cymbals, no horns or clarinet of its own). The front line is the fight: the TRUMPET's
pistons blast the melody (LINES, 8ths), the CLARINET's jumps weave the counter-line above and around it (LINES, one note
a beat; CLARION's 8th-jumps add the passing notes). D minor, i - iv - bVI7 - V7."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('TRUMPET', 'CLARINET')       # first named = left corner (screen left)
SEED = 8410                              # search 1..30000 --stop 8: 8 fits in 9,267 runs (TRUMPET 46 %), ranked first
DMG_SCALE = 1.0
DMG_SIDE = (0.34, 0.41)                  # the crossover's only lever (tune, 300 seeds: TRUMPET 43 % / CLARINET 57 %,
                                         # median K.O. 49.6 s; balance flips fast: 0.36 / 0.37 = 92 % / 8 %)
RULES = dict()

# ---------------------------------------------------------------- tempo + music
BPM = 140                                # loop_bpm -> 141.64: 36 bars in 61 s, bar 1.694 s, FIGHT! at 3.39 s
MOOD = 'hot_jazz'
KEY, SCALE = 'D', 'harmonic minor'       # C# over the A7: the dark leading tone
LOOP = ['Dm', 'Dm', 'Gm', 'Gm', 'Bb7', 'A7', 'Dm', 'A7']
CHORDS = ['Dm', 'A7'] + LOOP * 4 + ['Bb7', 'A7']     # the card on the tonic, the count-in on the dominant; the last two
                                                     # bars turn round (bVI7 - V7) into the title slam's Dm on frame 0
ROLES = {'TRUMPET': 'melody', 'CLARINET': 'line'}
LINES = {
    # the TRUMPET's melody, 8ths over the 8-bar loop (D4-A5). Each combo plays three of them (8ths 1-2-3 of every
    # half bar, the third the hardest punch: the accent lands on 2 and 4); the 4th 8th sounds only in DOUBLE TONGUE
    'TRUMPET': [69, 74, 77, 76, 74, 72, 69, 67,          # Dm:  A D F (E) | D C A (G)
                65, 69, 74, 72, 69, 65, 62, 64,          # Dm:  F A D (C) | A F D (E)
                67, 70, 74, 72, 70, 67, 62, 65,          # Gm:  G Bb D (C) | Bb G D (F)
                67, 70, 79, 77, 74, 70, 67, 69,          # Gm:  G Bb G5 (F) | D Bb G (A)
                70, 74, 77, 80, 77, 74, 70, 68,          # Bb7: Bb D F (Ab) | F D Bb (Ab)
                69, 73, 76, 79, 76, 73, 69, 67,          # A7:  A C# E (G) | E C# A (G)
                65, 69, 74, 77, 81, 77, 74, 73,          # Dm:  F A D (F) | A5 F D (C#)
                76, 73, 69, 64, 69, 73, 76, 73],         # A7:  E C# A (E) | A C# E (C#)
    # the CLARINET's counter-line, one note a beat (D4-G5): long tones above the trumpet's arpeggios, crossing under
    # it at the turnaround, a chromatic fall over the A7, a climb back up into the loop
    'CLARINET': [77, 76, 74, 76,                         # Dm:  F E D E
                 77, 79, 77, 74,                         # Dm:  F G F D
                 74, 72, 70, 72,                         # Gm:  D C Bb C
                 74, 77, 74, 70,                         # Gm:  D F D Bb
                 72, 74, 72, 68,                         # Bb7: C D C Ab
                 73, 72, 70, 69,                         # A7:  C# C Bb A
                 74, 72, 69, 70,                         # Dm:  D C A Bb
                 69, 73, 76, 79]}                        # A7:  A C# E G
SOLO = {
    # the trumpet's hot lead: a rip up the D minor triad to a shaken A, a cry down through the C#, the climb to a high
    # D falling home
    'TRUMPET': [(0, 74, .5), (.5, 77, .5), (1, 81, 1.0), (2, 79, .5), (2.5, 77, .25), (2.75, 76, .25), (3, 74, 1.0),
                (4, 73, .5), (4.5, 76, .5), (5, 79, .75), (5.75, 77, .25), (6, 76, .5), (6.5, 74, .5), (7, 73, 1.0),
                (8, 74, .25), (8.25, 77, .25), (8.5, 81, .5), (9, 86, .75), (9.75, 84, .25), (10, 81, .5),
                (10.5, 77, .5), (11, 74, 1.0)],
    # the clarinet's swing-era break: a liquid run up from the chalumeau, a sobbing C# turn, a rip to the high D
    'CLARINET': [(0, 62, .25), (.25, 65, .25), (.5, 69, .25), (.75, 74, .25), (1, 77, .75), (1.75, 76, .25),
                 (2, 74, .5), (2.5, 73, .5), (3, 74, 1.0), (4, 81, .5), (4.5, 79, .25), (4.75, 77, .25), (5, 76, .5),
                 (5.5, 73, .5), (6, 70, .5), (6.5, 69, .5), (7, 73, 1.0), (8, 74, .25), (8.25, 76, .25),
                 (8.5, 77, .25), (8.75, 79, .25), (9, 81, .5), (9.5, 86, .5), (10, 85, .25), (10.25, 81, .25),
                 (10.5, 77, .5), (11, 74, 1.0)]}
MIX = {'clarinet': (1.9, 0.35)}         # the clarinet's bus +2 dB over its #37 level: its chalumeau line shares
                                         # 140-700 Hz with the tuba and the guitar's chop
TITLE_HITS = {'TRUMPET': (62, 69, 74, 77, 81),       # a lead trumpet's rip up the D minor triad to a shaken high A
              'CLARINET': (50, 53, 57, 62, 65, 69, 74)}   # the clarinet's glissando up through the registers

# ---------------------------------------------------------------- the post
CAPTION = 'Who really leads the band? 🎺 Comment the next match-up 👇'
TAGS = {'instagram': '#ballfight #trumpet #clarinet #jazz #neworleansjazz #hotjazz #bandkids #musicmemes #pixelart '
                     '#lowpoly #satisfying #music',
        'tiktok': '#fyp #ballfight #trumpet #clarinet #jazz #bandtok #musictok #pixelart #satisfying #music',
        'shorts': ('Trumpet vs Clarinet: who leads the jazz band?', 'Valve punches vs teleporting twelfth jumps, '
                   'every hit in key over a dark hot-jazz two-beat. Who wins?',
                   '#shorts #ballfight #trumpet #clarinet #jazz #music #pixelart')}
