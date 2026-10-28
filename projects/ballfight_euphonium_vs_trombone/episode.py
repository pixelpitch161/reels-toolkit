"""#48 BALLFIGHT: Euphonium vs Trombone (episode kit config: projects/ballfight/README.md, "Making an episode").
Idea + hook: valves vs slide. The same notes, the same band parts, the same register (half of all euphonium players get
handed a trombone at some point): identical range, opposite mechanics. A crossover: EUPHONIUM (left, amber) as built in
#38 (BELL MORTAR; ported into the kit for this episode), TROMBONE (right, mint) as built in #34 / #41 (SLIDE LANCE), both
straight from the roster; the only lever is the damage handicap (DMG_SIDE). The lance swats shells out of the air
(the opt-in rule swat_shells, added for this episode). No episode number on screen.
Music: mood A, dark New Orleans second line (kit/moods/second_line.py: the parade bass drum with a cymbal on top, the
snare's ghost notes and press rolls, a low simple sousaphone, no horns of its own). Same register for both fighters, so
they split the jobs: the EUPHONIUM sings the warm melody (every salvo = its beat's note; MARCHING ORDERS = all of it),
the TROMBONE riffs and smears around it (every thrust = a riff stab, POWER SLIDE every 8th). E minor, i - iv - i -
bVI7 - V7 - i - iv - V7."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('EUPHONIUM', 'TROMBONE')     # first named = left corner (screen left): swapped from #38 for this episode
SEED = 4604                              # search 1..40000 --stop 7: 7 fits in 4,606 runs (EUPHONIUM 56 %), ranked 2nd:
                                         # its captioned IDENTITY CRISIS copies SAD TROMBONE (the wah-wah in amber)
DMG_SCALE = 1.0
DMG_SIDE = (0.44, 0.35)                 # the crossover's only lever (tune, 400 seeds: EUPHONIUM 52 % / TROMBONE 48 %,
                                         # median K.O. 51.2 s; 0.44 / 0.37 = 38 %, 0.46 / 0.35 = 64 %)
RULES = dict(swat_shells=True)           # a live lance knocks a falling shell out of the air (fight.py, #48)

# ---------------------------------------------------------------- tempo + music
BPM = 122                                # loop_bpm -> 121.97: 31 bars in 61 s, bar 1.968 s, FIGHT! at 3.94 s
MOOD = 'second_line'
KEY, SCALE = 'E', 'harmonic minor'       # D# over the B7: the dark leading tone
LOOP = ['Em', 'Am', 'Em', 'C7', 'B7', 'Em', 'Am', 'B7']
CHORDS = ['Em', 'B7'] + LOOP * 3 + LOOP[:5]     # the card on the tonic, the count-in on the dominant; the last bar's B7
                                                # falls into the title slam's Em on frame 0
ROLES = {'EUPHONIUM': 'melody', 'TROMBONE': 'riff'}
LINES = {
    # the EUPHONIUM's melody, one note a beat over the 8-bar loop (B3-C5): a salvo every 2 beats plays beats 1 and 3
    # (the singing line on its own), MARCHING ORDERS fires every beat and fills in the steps between
    'EUPHONIUM': [67, 66, 64, 59,              # Em:  G F# E B      (1 + 3: G  E)
                  60, 64, 69, 67,              # Am:  C E A G       (C  A)
                  67, 71, 64, 67,              # Em:  G B E G       (G  E)
                  70, 69, 67, 64,              # C7:  Bb A G E      (Bb G)
                  66, 63, 59, 63,              # B7:  F# D# B D#    (F# B)
                  64, 67, 71, 69,              # Em:  E G B A       (E  B)
                  72, 71, 69, 64,              # Am:  C B A E       (C  A)
                  63, 66, 69, 66],             # B7:  D# F# A F#    (D# A)
    # the TROMBONE's riff, 8ths over the loop (D#3-E4): a thrust a beat stabs the on-beat notes (smeared up into
    # them), POWER SLIDE plays every 8th: blue notes (the flat fifth Bb over Em), a walk down the B7
    'TROMBONE': [52, 55, 55, 57, 58, 59, 55, 52,    # Em:  E3 . G3 . Bb3 . G3 .
                 57, 60, 60, 61, 64, 63, 60, 57,    # Am:  A3 . C4 . E4 . C4 .
                 59, 58, 55, 52, 55, 57, 58, 59,    # Em:  B3 . G3 . G3 . Bb3 .
                 60, 58, 55, 52, 58, 57, 55, 52,    # C7:  C4 . G3 . Bb3 . G3 .
                 59, 57, 54, 51, 57, 54, 51, 47,    # B7:  B3 . F#3 . A3 . D#3 .
                 52, 55, 59, 62, 64, 62, 59, 55,    # Em:  E3 . B3 . E4 . B3 .
                 57, 60, 64, 60, 57, 55, 52, 57,    # Am:  A3 . E4 . A3 . E3 .
                 59, 61, 63, 61, 59, 57, 54, 51]}   # B7:  B3 . D#4 . B3 . F#3 .
SOLO = {                                 # the solo starts on bar 27 (Am), bar 28 Em, up to the reassembly
    # the tailgate trombone: a smear up from the low A, a held A3, a fall through the blues, a sly flat fifth, home on E
    'TROMBONE': [(0, 45, .5), (.5, 52, .5), (1, 57, 1.0), (2, 60, .5), (2.5, 59, .25), (2.75, 57, .25), (3, 55, .5),
                 (3.5, 52, .5), (4, 59, .75), (4.75, 58, .25), (5, 55, .5), (5.5, 52, .5), (6, 47, .5), (6.5, 50, .5),
                 (7, 52, 1.0)],
    # the euphonium's hymn: up the A minor triad to a held B, down to G, a turn through the D# leading tone to E
    'EUPHONIUM': [(0, 64, 1.0), (1, 69, .5), (1.5, 72, .5), (2, 71, 1.0), (3, 69, .5), (3.5, 67, .5), (4, 67, 1.0),
                  (5, 66, .5), (5.5, 64, .5), (6, 59, .5), (6.5, 63, .5), (7, 64, 1.0)]}
MIX = {'trombone': (2.4, 0.2), 'euphonium': (1.6, 0.35),       # both fighters over the street band: the trombone's
       'sousa': (1.45, 0.10), 'drums': (1.25, 0.14)}            # riff shares 120-700 Hz with the sousaphone
TITLE_HITS = {'EUPHONIUM': (64, 71, 52),           # a euphonium blast: E4, a fat fifth, the E under it
              'TROMBONE': (40, 52)}                # the low E + a big smear up to E3

# ---------------------------------------------------------------- the post
CAPTION = 'Same notes, different weapon. Valves or slide? Comment the next match-up 👇'
TAGS = {'instagram': '#ballfight #euphonium #trombone #lowbrass #brassband #secondline #neworleans #bandkids '
                     '#musicmemes #pixelart #satisfying #music',
        'tiktok': '#fyp #ballfight #euphonium #trombone #lowbrass #bandtok #musictok #brassband #pixelart #satisfying',
        'shorts': ('Euphonium vs Trombone: valves or slide?', 'The same notes and the same register, opposite '
                   'mechanics: brass mortar shells vs a slide lance, every hit in key over a dark New Orleans second '
                   'line. Who wins?', '#shorts #ballfight #euphonium #trombone #brassband #music #pixelart')}
