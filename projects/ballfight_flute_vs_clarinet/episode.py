"""#49 BALLFIGHT: Flute vs Clarinet (episode kit config: projects/ballfight/README.md, "Making an episode").
Idea + hook: the band-room classic: the two biggest sections in every school band, side by side, each sure it's the real
melody. A crossover: FLUTE (left, silver) as built in #32 v2 / drawn in #41 (SNAKE CHARMER), CLARINET (right, lavender)
as built in #37 (TWELFTH JUMP), both straight from the roster; the only lever is the damage handicap (DMG_SIDE). No
episode number on screen.
Music: mood A, minor bossa noir (kit/moods/bossa_noir.py: nylon guitar comping the bossa figure, a soft upright bass,
brushes + rim clicks, a dark Rhodes pad; no woodwinds of its own). The front line is the fight: the FLUTE's cobras sing
the high melody (LINES, 8ths), the CLARINET's jumps the liquid counter-line below it (LINES, one note a beat; CLARION's
8th-jumps add the passing notes). D minor, a minor bossa's ii-V cycle: i7 - i6 - iv7 - bVII7 - bIII maj7 - bVI maj7 -
ii half-dim - V7."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('FLUTE', 'CLARINET')         # first named = left corner (screen left)
SEED = 3864                              # search 1..40000 --stop 8: 8 fits in 6,454 runs (FLUTE 53 %), ranked first
DMG_SCALE = 1.0
DMG_SIDE = (5.3, 0.36)                   # the crossover's only lever (tune, 300 seeds: FLUTE 50 % / CLARINET 50 %,
                                         # median K.O. 49.4 s; ~11 heavy bites vs ~68 cuts a run: the jumps dodge most strikes)
RULES = dict()                           # no lance here, so no swat_cobras: neither fighter's opt-ins apply

# ---------------------------------------------------------------- tempo + music
BPM = 132                                # loop_bpm -> 133.77: 34 bars in 61 s, bar 1.794 s, FIGHT! at 3.59 s
MOOD = 'bossa_noir'
KEY, SCALE = 'D', 'harmonic minor'       # C# over the A7: the noir leading tone
LOOP = ['Dm7', 'Dm6', 'Gm7', 'C7', 'Fmaj7', 'Bbmaj7', 'Em7b5', 'A7']
CHORDS = ['Dm7', 'A7'] + LOOP * 4        # the card on the tonic, the count-in on the dominant; the loop's A7 falls into
                                         # the title slam's Dm7 on frame 0
ROLES = {'FLUTE': 'melody', 'CLARINET': 'line'}
LINES = {
    # the FLUTE's high melody, one note per 8th over the 8-bar loop (C#5-E6): every 8th while its cobras are up
    'FLUTE': [81, 77, 74, 76, 77, 81, 84, 81,          # Dm7:    A F D E F A C A
              83, 81, 77, 74, 77, 81, 83, 86,          # Dm6:    B A F D F A B D
              86, 82, 79, 77, 79, 82, 86, 84,          # Gm7:    D Bb G F G Bb D C
              84, 82, 79, 76, 79, 82, 84, 88,          # C7:     C Bb G E G Bb C E
              88, 84, 81, 77, 76, 77, 81, 84,          # Fmaj7:  E C A F E F A C
              82, 81, 77, 74, 77, 81, 82, 86,          # Bbmaj7: Bb A F D F A Bb D
              86, 82, 79, 76, 74, 76, 79, 82,          # Em7b5:  D Bb G E D E G Bb
              81, 85, 88, 85, 81, 79, 76, 73],         # A7:     A C# E C# A G E C# (leads home to D)
    # the CLARINET's counter-line, one note a beat (D4-E5), long liquid tones below the flute, moving against it; a
    # chromatic fall over the A7 into the loop
    'CLARINET': [65, 67, 69, 72,                       # Dm7:    F G A C
                 71, 69, 65, 62,                       # Dm6:    B A F D
                 70, 69, 67, 65,                       # Gm7:    Bb A G F
                 64, 67, 70, 72,                       # C7:     E G Bb C
                 69, 72, 76, 72,                       # Fmaj7:  A C E C
                 74, 72, 70, 69,                       # Bbmaj7: D C Bb A
                 67, 70, 74, 70,                       # Em7b5:  G Bb D Bb
                 73, 72, 70, 69]}                      # A7:     C# C Bb A
SOLO = {
    # the flute's late-night solo: a breathy leap to a high F, a sighing C# turn, a slow fall, a climb to the high D
    'FLUTE': [(0, 81, .5), (.5, 86, .5), (1, 89, 1.0), (2, 88, .5), (2.5, 86, .25), (2.75, 85, .25), (3, 86, 1.0),
              (4, 82, .5), (4.5, 81, .5), (5, 79, .75), (5.75, 77, .25), (6, 76, .5), (6.5, 77, .5), (7, 79, 1.0),
              (8, 81, .25), (8.25, 84, .25), (8.5, 88, .5), (9, 89, .75), (9.75, 88, .25), (10, 85, .5),
              (10.5, 81, .5), (11, 86, 1.0)],
    # the clarinet's smoky chalumeau solo: low and dark, a sobbing C#, a liquid run up to the high D
    'CLARINET': [(0, 57, .5), (.5, 62, .5), (1, 65, .75), (1.75, 64, .25), (2, 62, .5), (2.5, 60, .5), (3, 62, 1.0),
                 (4, 69, .5), (4.5, 72, .25), (4.75, 70, .25), (5, 69, .5), (5.5, 65, .5), (6, 64, .5), (6.5, 61, .5),
                 (7, 64, 1.0), (8, 62, .25), (8.25, 65, .25), (8.5, 69, .25), (8.75, 72, .25), (9, 74, .5),
                 (9.5, 77, .5), (10, 76, .25), (10.25, 73, .25), (10.5, 69, .5), (11, 74, 1.0)]}
TITLE_HITS = {'FLUTE': (74, 77, 81, 84, 86, 88, 89),          # a flute flurry up the D minor arpeggio to a high F
              'CLARINET': (50, 53, 57, 62, 65, 69, 74)}       # the clarinet's glissando up through the registers
MIX = {'clarinet': (2.4, 0.35),          # +4 dB over its #37 level: its chalumeau line shares 140-700 Hz with the
                                         # upright, the nylon comping and the Rhodes
       'flute': (2.1, 0.45)}             # +1.8 dB: the high melody clear over the nylon guitar's top

# ---------------------------------------------------------------- the post
CAPTION = 'Flutes or clarinets? Band kids, choose 😤 Comment the next match-up 👇'
TAGS = {'instagram': '#ballfight #flute #clarinet #bandkids #schoolband #woodwinds #bossanova #musicmemes #pixelart '
                     '#lowpoly #satisfying #music',
        'tiktok': '#fyp #ballfight #flute #clarinet #bandkid #bandtok #musictok #pixelart #satisfying #music',
        'shorts': ('Flute vs Clarinet: who really plays the melody?', 'Silver cobras vs teleporting twelfth jumps, '
                   'every hit in key over a smoky minor bossa. Band kids, choose.',
                   '#shorts #ballfight #flute #clarinet #bandkid #music #pixelart')}
