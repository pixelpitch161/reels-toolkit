"""#52 BALLFIGHT: Oboe vs Bassoon (episode kit config: projects/ballfight/README.md, "Making an episode"). A crossover:
the OBOE as built in #37, the BASSOON as built in #39 (ported into the kit for this episode), both exactly as in
fighters.py; the only per-episode lever is the damage handicap (DMG_SIDE). Sides for this episode only: OBOE left,
BASSOON right (both swapped from their own episodes; the roster is untouched).
Idea + hook: the double-reed family feud. The orchestra's only two double reeds, both forever carving reeds; the
bassoon is the oboe's big, sneaky older sibling (high and nasal vs low and comic).
Music: mood heist (sneaky heist caper, D minor at loop_bpm(118) = 118.03: 30 bars, FIGHT! at 4.07 s). The oboe plays
the high nasal melody (its tuning A is the key's dominant), the bassoon the low comic staccato line (its hops are its
notes): a double-reed duet that starts arguing, the leader fuller and louder, the loser thinner and sour (the oboe's
detune is audible on the bassoon, the Grieg quote included). MOUNTAIN KING quotes Grieg's public-domain theme in D
minor, one note per hop, speeding up with the hops."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('OBOE', 'BASSOON')       # first named = left corner (screen left)
SEED = 954                           # search 1.. --stop 8: 8 fits in 1,787 runs, ranked first (9.19): BASSOON wins at 51.6 s,
                                     # 3 lead changes; ILL WIND (41.7 s) lands just before MOUNTAIN KING (42.2 s): Grieg out of tune
DMG_SCALE = 1.0
DMG_SIDE = (0.6, 0.85)               # the only lever, tuned once and frozen (300 seeds: OBOE 50 % / BASSOON 50 % of 296
                                     # decided, median K.O. 49.8 s; x0.6 / x0.4 the oboe won 100 %, x0.45 / x1.0 the bassoon)
RULES = dict()

# ---------------------------------------------------------------- tempo + music
BPM = 118                            # loop_bpm -> 118.03: 30 bars, bar = 2.034 s, FIGHT! at 4.07 s
MOOD = 'heist'
KEY, SCALE = 'D', 'minor'            # the oboe's A is the dominant
LOOP = ['Dm', 'Dm', 'Bb', 'A7', 'Dm', 'Gm', 'Bb', 'A7']
CHORDS = ['Dm', 'A7'] + LOOP * 3 + LOOP[:4]               # the last bar's A7 falls into the title slam's Dm
ROLES = {'OBOE': 'melody', 'BASSOON': 'line'}
LINES = {
    # the oboe's high, nasal caper melody (its stings play it, 8ths), creeping chromatically, A4-F6
    'OBOE': [74, 77, 81, 80, 81, 77, 74, 73,     74, 76, 77, 79, 81, 79, 77, 76,      # Dm Dm
             77, 82, 86, 82, 77, 74, 77, 82,     81, 79, 77, 76, 73, 76, 79, 81,      # Bb A7
             86, 84, 81, 77, 74, 77, 81, 84,     82, 79, 74, 79, 82, 86, 82, 79,      # Dm Gm
             77, 82, 86, 89, 86, 82, 77, 74,     73, 76, 79, 82, 81, 79, 76, 73],     # Bb A7
    # the bassoon's low comic staccato line (a hop landing plays the 8th it lands on), octave jumps, G2-Bb3
    'BASSOON': [50, 53, 57, 53, 50, 52, 53, 55,  50, 53, 57, 58, 57, 53, 50, 49,      # Dm Dm
                46, 50, 53, 50, 46, 48, 50, 53,  45, 49, 52, 55, 57, 55, 52, 49,      # Bb A7
                50, 57, 53, 57, 50, 57, 53, 52,  43, 46, 50, 46, 55, 50, 46, 50,      # Dm Gm
                46, 53, 50, 53, 58, 53, 50, 48,  45, 52, 49, 52, 57, 52, 55, 49]}     # Bb A7
SOLO = {
    # the A, then a sly nasal descent that peeks back up and lands home on D
    'OBOE': [(0, 81, 1.0), (1, 82, .5), (1.5, 81, .5), (2, 77, .5), (2.5, 74, .5), (3, 73, .5), (3.5, 76, .5),
             (4, 79, .5), (4.5, 82, .5), (5, 81, .5), (5.5, 76, .5), (6, 74, 2.0)],
    # a sneaky staccato run that tiptoes up, peeks, and lands home with a comic low blat
    'BASSOON': [(0, 50, .25), (.5, 53, .25), (1, 57, .25), (1.5, 55, .25), (2, 53, .25), (2.5, 52, .25), (3, 50, .25),
                (3.5, 57, .25), (4, 62, .25), (4.5, 60, .25), (5, 57, .25), (5.5, 53, .25), (6, 50, .5),
                (7, 38, .75)]}
TITLE_HITS = {'OBOE': (69, 81), 'BASSOON': (38, 50)}     # the oboe's A; the bassoon's blat on low D

# ---------------------------------------------------------------- the post
CAPTION = 'The double reeds settle it 🪈 Comment the next match-up 👇'
TAGS = {}
