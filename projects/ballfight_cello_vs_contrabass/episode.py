"""#47 BALLFIGHT: Cello vs Contrabass (episode kit config: projects/ballfight/README.md, "Making an episode").
Idea + hook: the low strings. The cello sings; the double bass holds the orchestra up and never gets thanked. In
Saint-Saens' Carnival of the Animals the cello is the Swan and the double bass is the Elephant. Two new fighters: CELLO
(left, rust) the only grappler (THE EMBRACE: rush, grab, swing, throw into the ropes), CONTRABASS (right, steel blue) the
only juggernaut (WALKING BASS: one heavy step a beat, no knockback while it walks, so the cello must catch it between
steps; SLAP BASS breaks a grab). No episode number on screen.
Music: mood A, dark tango (kit/moods/dark_tango.py: bandoneon marcato, a muted nylon guitar for the strings' chop, golpe
and chicharra, no piano, no bass of its own; the band stops for a beat on every rope slam). The contrabass's footfalls
ARE the walking bass line (LINES, a note a beat, on the beat), the cello sings the melody (LINES, a note a beat; repeats
tie into long bows). Quotes, all public domain: Bach's Cello Suite No. 1 Prelude (PRELUDE), Saint-Saens' The Swan
(SWAN SONG) and The Elephant (THE ELEPHANT), arranged in the episode's key. D minor (both instruments' open D)."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('CELLO', 'CONTRABASS')       # first named = left corner (screen left)
SEED = 2455                              # search 1..3504 (two ranges, 22 fits in 3,504 runs, CONTRABASS 58 %): ranked first;
                                         # no fit captions both desperation moves (they fire ~2 s apart: 4 s rule)
DMG_SCALE = 0.85                         # the shared scale; both fighters are new, so their own numbers are tuned and
DMG_SIDE = None                          # frozen in the roster (fighters.py)
RULES = dict()

# ---------------------------------------------------------------- tempo + music
BPM = 118                                # loop_bpm -> 118.03: 30 bars in 61 s, bar 2.034 s, FIGHT! at 4.07 s
MOOD = 'dark_tango'
KEY, SCALE = 'D', 'harmonic minor'       # C# over the A7: the tango's leading tone
LOOP = ['Dm', 'A7', 'A7', 'Dm', 'Gm', 'Dm', 'E7', 'A7']
CHORDS = ['Dm', 'A7'] + LOOP * 3 + ['Gm', 'Dm', 'E7', 'A7']   # the card on the tonic, the count-in on the dominant; the
                                                               # last bars a cadence into the title slam's Dm (frame 0)
ROLES = {'CELLO': 'melody', 'CONTRABASS': 'bass'}
LINES = {
    # the CONTRABASS's walking line, a note a beat (D1-D3): chord tones with a chromatic approach into every change -
    # each of its footfalls plays the next note
    'CONTRABASS': [38, 40, 41, 44,                       # Dm:  D E F G#  (-> A)
                   33, 37, 40, 43,                       # A7:  A C# E G
                   45, 43, 40, 37,                       # A7:  A G E C#  (-> D)
                   38, 45, 41, 42,                       # Dm:  D A F F#  (-> G)
                   43, 46, 50, 49,                       # Gm:  G Bb D C#  (-> D)
                   50, 45, 41, 39,                       # Dm:  D A F D#  (-> E)
                   40, 44, 47, 46,                       # E7:  E G# B Bb  (-> A)
                   45, 43, 40, 37],                      # A7:  A G E C#  (-> D)
    # the CELLO's singing melody, a note a beat (A3-A4, its tenor register); repeated notes tie into long bows
    'CELLO': [62, 62, 62, 64,                            # Dm:  D - - E
              65, 65, 64, 61,                            # A7:  F - E C#
              64, 64, 64, 57,                            # A7:  E - - A
              65, 64, 62, 62,                            # Dm:  F E D -
              67, 67, 65, 64,                            # Gm:  G - F E
              62, 62, 65, 69,                            # Dm:  D - F A
              68, 68, 65, 62,                            # E7:  G# - F D
              61, 61, 64, 64]}                           # A7:  C# - E -
SOLO = {
    # the cello's tango cadenza over Gm - Dm - E7 - A7: a cry up to the high A, the G# sob, a run down to the leading tone
    'CELLO': [(0, 62, .5), (.5, 65, .5), (1, 69, 1.5), (2.5, 68, .5), (3, 65, 1.0), (4, 64, .5), (4.5, 65, .5),
              (5, 67, .5), (5.5, 69, .5), (6, 70, 1.5), (7.5, 69, .5), (8, 68, .5), (8.5, 65, .5), (9, 64, .5),
              (9.5, 61, .5), (10, 62, 2.0)],
    # the double bass's walking solo: up the D minor arpeggio, a chromatic slide down, slaps on the off-beats
    'CONTRABASS': [(0, 38, .5), (.5, 45, .5), (1, 50, .5), (1.5, 49, .5), (2, 48, .5), (2.5, 47, .5), (3, 46, 1.0),
                   (4, 45, .5), (4.5, 41, .5), (5, 38, .5), (5.5, 40, .5), (6, 41, .5), (6.5, 44, .5), (7, 45, 1.0),
                   (8, 50, .5), (8.5, 49, .5), (9, 45, .5), (9.5, 40, .5), (10, 38, 2.0)]}
TITLE_HITS = {'CELLO': (50, 57, 62),                     # a cello swell: the open D and A, the octave D on top
              'CONTRABASS': (38, 45, 50)}                # a slapped low D, two plucks up the fifth and octave

# ---------------------------------------------------------------- the post
CAPTION = 'The swan vs the elephant 🦢🐘 Comment the next match-up 👇'
TAGS = {'instagram': '#ballfight #cello #doublebass #contrabass #upright #tango #carnivaloftheanimals #orchestra '
                     '#musicmemes #pixelart #lowpoly #satisfying',
        'tiktok': '#fyp #ballfight #cello #doublebass #tango #orchestratok #musictok #pixelart #satisfying #music',
        'shorts': ('Cello vs Double Bass: the swan vs the elephant', 'The grappler that throws you into the ropes vs '
                   'the juggernaut that walks on every beat, every hit in key over a dark tango. Who wins?',
                   '#shorts #ballfight #cello #doublebass #tango #music #pixelart')}
