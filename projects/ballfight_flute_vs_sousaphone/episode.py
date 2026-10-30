"""#53 BALLFIGHT: Flute vs Sousaphone (episode kit config: projects/ballfight/README.md, "Making an episode").
Idea + hook: a viewer request ("Now put flute vs a sousaphone"): the smallest vs the biggest in the marching band -
the flutes lead from the front, the sousaphones bring up the rear. FLUTE (left, silver) exactly as in the kit: SNAKE
CHARMER (baskets, cobras striking on the beat), FLUTTER TONGUE, SNAKE DANCE, PICCOLO. SOUSAPHONE (right, marching
crimson), new: the only marcher - DRILL SET (every 2 bars a crisp straight drill path across the ring, 8 steps on the
8ths, trampling the flute - bulldozed on ahead - and its baskets, then a halt and a huge cone bell blast straight
ahead), HORN FLASH (the horn spins, the blast sweeps all round), DOUBLE TIME (marches twice as fast), COMPANY FRONT (a
wall of ghost sousaphones marches rope to rope). The sousaphone model is new, built from Commons photos found with
Claude in Chrome (sousaphone_lowpoly). No episode number on screen.
Music: mood A, dark marching-band fight song (kit/moods/dark_drumline.py, ported from #38: snare line, high quads,
pitched bass drums, cymbal line, marimba + timpani pit; no low brass or flutes of its own). The SOUSAPHONE plays the
bass line (LINES, 8ths: its footfalls while it marches, the beat notes while it marks time, a blat on every blast), the
FLUTE the high melody (LINES, 8ths, while its cobras are up). COMPANY FRONT quotes the trio of Sousa's The Stars and
Stripes Forever (1896, public domain) in the relative major (E-flat), in low unison octaves; PICCOLO plays its usual
shriek over it (no obbligato: user, Oct 30). Original otherwise. C minor."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('FLUTE', 'SOUSAPHONE')       # first named = left corner (screen left)
SEED = None                              # python -m ballfight search flute_vs_sousaphone -> the best fit
DMG_SCALE = 0.7                          # tuned (200 seeds: 52 % / 48 %, median K.O. 48.7 s); the SOUSAPHONE's own
                                         # numbers frozen in the roster
DMG_SIDE = None                          # the FLUTE comes as built; a per-fighter handicap only if needed
RULES = dict()

# ---------------------------------------------------------------- tempo + music
BPM = 126                                # loop_bpm -> 125.90: 32 bars in 61 s, bar 1.906 s, FIGHT! at 3.81 s
MOOD = 'dark_drumline'
KEY, SCALE = 'C', 'minor'                # #38's key: the ported bass-drum tunings apply as they were
LOOP = ['Cm', 'Ab', 'Fm', 'G']
CHORDS = ['Cm', 'G'] + LOOP * 7 + ['Ab', 'G']   # the card on the tonic, the count-in on the dominant; the last bars turn
                                                 # round into the title slam's Cm (frame 0)
ROLES = {'FLUTE': 'melody', 'SOUSAPHONE': 'bass'}
LINES = {
    # the FLUTE's fight song, 8ths (C5-G6): a dark, strutting march tune
    'FLUTE': [79, 79, 84, 79, 75, 79, 84, 86,           # Cm:  G G C G Eb G C D
              87, 84, 80, 84, 87, 89, 87, 84,           # Ab:  Eb C Ab C Eb F Eb C
              77, 80, 84, 80, 89, 87, 84, 80,           # Fm:  F Ab C Ab F Eb C Ab
              79, 83, 86, 83, 91, 89, 87, 83],          # G:   G B D B G F Eb B
    # the SOUSAPHONE's marching bass, 8ths (C2-D3): root-fifth on the march, a turn into the next chord
    'SOUSAPHONE': [36, 43, 36, 43, 36, 43, 39, 41,      # Cm:  C G C G C G Eb F
                   44, 39, 44, 39, 44, 39, 48, 46,      # Ab:  Ab Eb Ab Eb Ab Eb C Bb
                   41, 48, 41, 48, 41, 48, 44, 43,      # Fm:  F C F C F C Ab G
                   43, 50, 43, 50, 43, 47, 50, 47]}     # G:   G D G D G B D B
SOLO = {
    # the flute's victory solo: up the C minor arpeggio to the high G, a run down, the B leading home to C
    'FLUTE': [(0, 84, .5), (.5, 87, .5), (1, 91, 1.0), (2, 89, .5), (2.5, 87, .5), (3, 86, .5), (3.5, 83, .5),
              (4, 84, .5), (4.5, 79, .5), (5, 80, .5), (5.5, 83, .5), (6, 84, 2.0)],
    # the sousaphone's strut: root-fifth-octave, down through Bb and Ab, the dominant's B up to the C, blatted
    'SOUSAPHONE': [(0, 36, .5), (.5, 43, .5), (1, 48, .5), (1.5, 43, .5), (2, 46, .5), (2.5, 44, .5), (3, 43, 1.0),
                   (4, 41, .5), (4.5, 44, .5), (5, 47, .5), (5.5, 50, .5), (6, 48, 2.0)]}
TITLE_HITS = {'FLUTE': (72, 75, 79, 84, 87, 89, 91),   # a shrieking flurry up the C minor arpeggio
              'SOUSAPHONE': (36, 43, 46, 48)}           # a huge low C blat, then a pickup of marching 8ths

# ---------------------------------------------------------------- the post
CAPTION = 'You asked for it 👀 Flute vs Sousaphone. Comment the next match-up 👇'
TAGS = {'instagram': '#ballfight #flute #sousaphone #marchingband #band #drumline #tuba #flutist #musicmemes '
                     '#pixelart #lowpoly #satisfying',
        'tiktok': '#fyp #ballfight #flute #sousaphone #marchingband #bandtok #drumline #musictok #pixelart #satisfying',
        'shorts': ('Flute vs Sousaphone: smallest vs biggest in the band', 'The flute charms snakes, the sousaphone '
                   'marches drill and blasts - every hit in key over a dark drumline. Who wins?',
                   '#shorts #ballfight #flute #sousaphone #marchingband #music #pixelart')}
