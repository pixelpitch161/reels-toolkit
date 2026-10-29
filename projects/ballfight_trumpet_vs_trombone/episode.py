"""#50 BALLFIGHT: Trumpet vs Trombone (episode kit config: projects/ballfight/README.md, "Making an episode").
Idea + hook: the brass section's two loudest seats: the lead trumpet (the star: high notes, ego) vs the trombones (the
back-row clowns with the slides). A crossover: TRUMPET (left, yellow) as built in #29 v3 / #46 (VALVE COMBO), TROMBONE
(right, mint) as built in #34 / #41 / #48 (SLIDE LANCE), both straight from the roster; the only lever is the damage
handicap (DMG_SIDE). A reach battle: the lance outranges the pistons. No episode number on screen.
Music: mood A, big-band mambo (kit/moods/mambo.py: piano montuno, contrabass tumbao, congas + timbales + cowbell, a low
sax-section pad, no trumpets or trombones of its own). The fighters are the brass section: the TRUMPET's combos are the
shout chorus up top (LINES, 8ths: each combo plays three, DOUBLE TONGUE the fourth too), the TROMBONE's thrusts punch the
low riff with a smear into each stab (LINES, 8ths: POWER SLIDE plays every one). G minor, i - iv - V7 - i - i - iv -
bVI7 - V7. All original."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('TRUMPET', 'TROMBONE')       # first named = left corner (screen left)
SEED = 1001                              # search 1..30000 --stop 8: 8 fits in 1,096 runs (TRUMPET 51 %), ranked first:
                                         # TRUMPET wins 50.28 s on 1 HP, both lows fire (HIGH C vs PEDAL TONE)
DMG_SCALE = 1.0
DMG_SIDE = (2.8, 0.40)                  # the crossover's only lever (tune, 200 seeds: TRUMPET 51 % / TROMBONE 49 %,
                                         # median K.O. 51.2 s; 2.4 / 0.40 = 32 %, 3.2 / 0.40 = 66 %). Big gap because
                                         # the lance outreaches the pistons: ~16 trumpet hits land vs ~63 thrusts
RULES = dict()

# ---------------------------------------------------------------- tempo + music
BPM = 132                                # loop_bpm -> 133.77: 34 bars in 61 s, bar 1.794 s, FIGHT! at 3.59 s
MOOD = 'mambo'
KEY, SCALE = 'G', 'harmonic minor'       # F# over the D7: the dark leading tone
LOOP = ['Gm', 'Cm', 'D7', 'Gm', 'Gm', 'Cm', 'Eb7', 'D7']
CHORDS = ['Gm', 'D7'] + LOOP * 4         # the card on the tonic, the count-in on the dominant; the last bar's D7 falls
                                         # into the title slam's Gm on frame 0
ROLES = {'TRUMPET': 'melody', 'TROMBONE': 'riff'}
LINES = {
    # the TRUMPET's shout chorus, 8ths over the 8-bar loop (F#4-Bb5). Each combo plays three of them (8ths 1-2-3 of
    # every half bar, the third the hardest punch); the 4th 8th sounds only in DOUBLE TONGUE
    'TRUMPET': [74, 77, 79, 77, 79, 77, 74, 70,          # Gm:  D F G (F)    | G F D (Bb)
                72, 75, 79, 77, 79, 75, 72, 74,          # Cm:  C Eb G (F)   | G Eb C (D)
                74, 78, 81, 78, 81, 78, 74, 72,          # D7:  D F# A (F#)  | A F# D (C)
                79, 82, 79, 77, 74, 70, 67, 70,          # Gm:  G Bb G (F)   | D Bb G (Bb)
                67, 70, 74, 77, 79, 74, 70, 74,          # Gm:  G Bb D (F)   | G D Bb (D)
                72, 75, 79, 82, 80, 79, 75, 72,          # Cm:  C Eb G (Bb)  | Ab G Eb (C)
                75, 79, 82, 79, 77, 73, 70, 73,          # Eb7: Eb G Bb (G)  | F Db Bb (Db)
                78, 81, 78, 74, 72, 69, 66, 67],         # D7:  F# A F# (D)  | C A F# (G)
    # the TROMBONE's low riff, 8ths over the loop (A2-D4): a thrust a beat stabs the on-beat notes (smeared up into
    # them), POWER SLIDE plays every 8th: walking riffs, a chromatic lead into the turnaround
    'TROMBONE': [55, 50, 58, 55, 53, 55, 50, 46,         # Gm:  G3 . Bb3 . F3 . D3 .
                 48, 51, 55, 51, 58, 55, 51, 48,         # Cm:  C3 . G3 . Bb3 . Eb3 .
                 50, 54, 57, 54, 60, 57, 54, 50,         # D7:  D3 . A3 . C4 . F#3 .
                 55, 58, 62, 58, 55, 53, 50, 49,         # Gm:  G3 . D4 . G3 . D3 .
                 46, 50, 55, 50, 58, 55, 50, 53,         # Gm:  Bb2 . G3 . Bb3 . D3 .
                 48, 55, 51, 48, 60, 58, 55, 51,         # Cm:  C3 . Eb3 . C4 . G3 .
                 51, 55, 58, 61, 58, 55, 51, 49,         # Eb7: Eb3 . Bb3 . Bb3 . Eb3 .
                 50, 54, 57, 54, 60, 57, 54, 45]}        # D7:  D3 . A3 . C4 . F#3 . (A2 pickup)
SOLO = {
    # the lead trumpet's victory: a rip up the G minor triad to a held G, the F# turn, a scream up to the high Bb, home
    'TRUMPET': [(0, 67, .5), (.5, 70, .5), (1, 74, .5), (1.5, 79, 1.0), (2.5, 77, .25), (2.75, 74, .25), (3, 78, 1.0),
                (4, 79, .5), (4.5, 82, .75), (5.25, 81, .25), (5.5, 79, .5), (6, 77, .5), (6.5, 74, .5), (7, 78, 1.0),
                (8, 74, .25), (8.25, 79, .25), (8.5, 82, .5), (9, 81, .5), (9.5, 78, .5), (10, 74, .5), (10.5, 78, .5),
                (11, 79, 1.0)],
    # the trombone section's answer: a smear up from the low Bb, a held G, a growl down through the F#, the walk home
    'TROMBONE': [(0, 46, .5), (.5, 50, .5), (1, 55, 1.0), (2, 58, .5), (2.5, 57, .25), (2.75, 55, .25), (3, 54, .5),
                 (3.5, 50, .5), (4, 51, .75), (4.75, 50, .25), (5, 48, .5), (5.5, 51, .5), (6, 54, .5), (6.5, 57, .5),
                 (7, 55, 1.0), (8, 58, .5), (8.5, 62, .5), (9, 61, .5), (9.5, 58, .5), (10, 57, .5), (10.5, 54, .5),
                 (11, 55, 1.0)]}
MIX = {'trombone': (2.4, 0.2),            # the trombone's low riff over the band: it shares 120-1500 Hz with
       'bass': (1.5, 0.08), 'sax': (0.7, 0.35)}   # the tumbao, the sax pad and the montuno's low octaves
TITLE_HITS = {'TRUMPET': (67, 74, 79, 82),          # a lead trumpet's rip up the G minor triad to a shaken high Bb
              'TROMBONE': (46, 58)}                 # the low Bb + a big smear up an octave

# ---------------------------------------------------------------- the post
CAPTION = 'Lead trumpet or trombone section? Choose wisely 🎺 Comment the next match-up 👇'
TAGS = {'instagram': '#ballfight #trumpet #trombone #brass #mambo #latinjazz #bigband #bandkids #musicmemes #pixelart '
                     '#satisfying #music',
        'tiktok': '#fyp #ballfight #trumpet #trombone #brass #mambo #bandtok #musictok #pixelart #satisfying',
        'shorts': ('Trumpet vs Trombone: who rules the brass section?', 'Valve punches vs a slide lance, every hit in '
                   'key over a dark big-band mambo. Lead trumpet or trombone section?',
                   '#shorts #ballfight #trumpet #trombone #mambo #music #pixelart')}
