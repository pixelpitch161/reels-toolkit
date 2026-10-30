"""#54 BALLFIGHT: Horn vs Trombone, THE REMATCH (episode-kit crossover; projects/ballfight/README.md, "Making an episode").
A viewer request: on #34's reel, "RIGGEDD!! RUN IT BACK!!" and "Horn is superior to trombone bro". The same two fighters
exactly as built, HORN v3 (ECHO CALL, #34 v5) now on the LEFT, TROMBONE (the kit's, as #34 built it) on the right: the
reverse of the first fight, which also reads as round 2. The card says REMATCH instead of VS, diagonally along the crack
in gold (title.py's opt-in MIDDLE). Honest fight; the only lever is the per-fighter damage handicap.
Music, mood A: brass trailer, the sequel. #34's bed (ported as kit/moods/brass_trailer.py) played darker and bigger
(TRAILER sequel: contrabass octave, cello pad, braams + subs every second bar, the taiko ensemble from the start, a
Neapolitan Eb in the loop); no brass of its own. The HORN plays the noble melody (each call one note of LINES['HORN'],
its echo ghosts the quieter delayed repeats), the TROMBONE the punchy low riff and smears (role 'riff')."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('HORN', 'TROMBONE')          # first named = left corner (screen left): HORN left, TROMBONE right
SEED = 630                               # search 1..6000 --stop 8: 8 fits in 729 runs, ranked first (5.91); its story differs from #34 v5 (the bell lands, the echoes finish)
DMG_SCALE = 0.72                         # #34 v5's frozen scale for this pair
DMG_SIDE = (1.04, 1.08)                  # the only lever (tune, 300 seeds: HORN 53 % / TROMBONE 47 % of 266 decided, median K.O. 49.5 s)
RULES = dict()

# ---------------------------------------------------------------- the card
MIDDLE = ('REMATCH', 'divider')          # title.py (opt-in, #54): REMATCH along the diagonal crack instead of VS

# ---------------------------------------------------------------- tempo + music
BPM = 122                                # loop_bpm -> 121.97: 31 bars in 61 s, FIGHT! at 3.94 s (as #34)
MOOD = 'brass_trailer'
TRAILER = dict(sequel=True)              # darker and bigger (kit/moods/brass_trailer.py)
KEY, SCALE = 'D', 'minor'
LOOP = ['Dm', 'Bb', 'Gm', 'A', 'Dm', 'Bb', 'Eb', 'A']                 # the Eb: a Neapolitan shadow over the sequel
CHORDS = ['Dm', 'A'] + LOOP * 3 + ['Dm', 'Bb', 'Gm', 'A', 'A']       # card on the tonic, count-in on the dominant;
                                                                       # the last A falls into the title slam's Dm
ROLES = {'HORN': 'melody', 'TROMBONE': 'riff'}
LINES = {
    # the HORN's noble call, one note per beat (each call of ECHO CALL plays the beat's note; its ghosts repeat it a
    # beat later, quieter and darker, so the echoes stack into the chord): D4-F5
    'HORN': [62, 69, 74, 72,      # Dm: D A D C
             70, 65, 70, 74,      # Bb: Bb F Bb D
             67, 70, 74, 72,      # Gm: G Bb D C
             69, 73, 76, 73,      # A:  A C# E C#
             74, 72, 69, 65,      # Dm: D C A F
             70, 74, 77, 74,      # Bb: Bb D F D
             70, 67, 63, 67,      # Eb: Bb G Eb G (the dark turn)
             69, 64, 61, 64],     # A:  A E C# E (-> D)
    'HORN THEME': [62, 69, 70, 65, 67, 70, 69, 73, 74, 69, 70, 74, 67, 70, 69, 73],   # STOPPED HORN: a note per 2 beats
    # the TROMBONE's punchy low riff, one note per 8th (a thrust a beat plays the beat's note, POWER SLIDE every 8th):
    # root, root, up a fifth, the fourth, root, the octave, the seventh, the fifth: da-da-DAA in the low brass
    'TROMBONE': [50, 50, 57, 55, 50, 62, 60, 57,      # Dm
                 46, 46, 53, 51, 46, 58, 56, 53,      # Bb
                 43, 43, 50, 48, 43, 55, 53, 50,      # Gm
                 45, 45, 52, 50, 45, 57, 55, 49,      # A (the C# leading back to D)
                 50, 50, 57, 55, 50, 62, 60, 57,      # Dm
                 46, 46, 53, 51, 46, 58, 56, 53,      # Bb
                 51, 51, 58, 56, 51, 63, 61, 58,      # Eb
                 45, 45, 52, 50, 45, 57, 55, 49]}     # A
SOLO = {
    # the horn's victory: #34's heroic cadenza, now sounded at last: the call leaps a 4th and 5th, falls through the
    # Neapolitan Eb, climbs to the leading tone and resolves high
    'HORN': [(0, 62, .5), (.5, 69, .5), (1, 74, 1.0), (2, 72, .5), (2.5, 70, .5), (3, 69, 1.0), (4, 63, .5), (4.5, 67, .5),
             (5, 70, .75), (5.75, 69, .25), (6, 67, .5), (6.5, 69, .5), (7, 73, 1.0), (8, 74, 2.0)],
    # the trombone's victory: #34's dark slide solo, low and greasy, down to the pedal D
    'TROMBONE': [(0, 50, .5), (.5, 53, .5), (1, 57, 1.0), (2, 55, .5), (2.5, 53, .5), (3, 50, 1.0), (4, 45, .5),
                 (4.5, 48, .5), (5, 50, .75), (5.75, 53, .25), (6, 57, 1.0), (7, 60, .5), (7.5, 58, .5), (8, 57, .5),
                 (8.5, 53, .5), (9, 50, 1.5)]}
MIX = {'trombone': (2.3, 0.2)}           # the riff clear of the sequel's darker strings (kit default 1.8)
TITLE_HITS = {'HORN': (62, 69),          # a horn call blast: two horns in fifths leaping D4 -> A4
              'TROMBONE': (45, 50)}      # a trombone smear: A2 up into D3

# ---------------------------------------------------------------- the post
CAPTION = 'You said rigged. We ran it back. 📯🎺 Comment the next match-up 👇'
TAGS = {'instagram': '#ballfight #rematch #frenchhorn #trombone #brass #bandkids #musicmemes #musician #pixelart '
                     '#lowpoly #satisfying #music',
        'tiktok': '#fyp #ballfight #rematch #frenchhorn #trombone #brasstok #bandtok #pixelart #satisfying #music',
        'shorts': ('Horn vs Trombone: THE REMATCH', 'You said the first one was rigged, so we ran it back: echo calls vs '
                   'a slide lance, every hit in key.', '#shorts #ballfight #rematch #frenchhorn #trombone #brass '
                   '#pixelart')}
