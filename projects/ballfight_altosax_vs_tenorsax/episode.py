"""#51 BALLFIGHT: Alto Sax vs Tenor Sax (episode kit config: projects/ballfight/README.md, "Making an episode").
Idea + hook: the sax family feud, a show fight: the alto (most players' first sax, the 80s pop-solo star) owns the
lights, the tenor (the jazz giant, the original rock 'n' roll honker) owns the crowd. Two new fighters: ALTO SAX (left,
orchid) the only light-wielder (SPOTLIGHT SOLO: every bar a spotlight sweeps across the enemy's path and burns it),
TENOR SAX (right, burnt orange) the only fighter that uses the crowd (CROWD PLEASER: every bar a pose and a honk, and
the crowd throws from the stands - they only reach so far in, so the middle of the ring is safe from them). The alto
model is rebuilt from Commons photos (altosax_lowpoly), the tenor's built from them (tenorsax_lowpoly). No episode
number on screen.
Music: mood C, dark disco (kit/moods/dark_disco.py: minor four-on-the-floor, octave bass, open hats on the off-beats,
string stabs, a filtered pad, a mirror-ball shimmer; no saxes of its own). The saxes are the disco leads: the alto plays
the bright lead (LINES, 8ths: every sweep plays the line over the light's run, every burn a squeal a chord tone up),
the tenor the husky, honking low line (LINES, 16ths: a honk on each pose, each throw the next note of its riff, so the
riff grows with the hype). All original. A minor."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('ALTO SAX', 'TENOR SAX')     # first named = left corner (screen left)
SEED = 621                               # search 1..1011 (8 fits, 50 % / 50 % decided): ranked 2nd (648 first), picked
                                         # for the finish the idea hoped for: STAGE DIVE 41.2 s then ENCORE 45.2 s, both
                                         # captioned; ALTO SAX wins at 49.2 s on 2 HP (OCTAVE KEY lights in the HUD only)
DMG_SCALE = 0.75                         # both fighters are new: their own numbers are tuned and frozen in the roster
DMG_SIDE = None
RULES = dict()

# ---------------------------------------------------------------- tempo + music
BPM = 118                                # loop_bpm -> 118.03: 30 bars in 61 s, bar 2.034 s, FIGHT! at 4.07 s
MOOD = 'dark_disco'
KEY, SCALE = 'A', 'minor'                # natural minor; E7's G# comes in by rule (the scale's G raised over E7)
LOOP = ['Am', 'F', 'Dm', 'E7', 'Am', 'F', 'G', 'E7']
CHORDS = ['Am', 'E7'] + LOOP * 3 + ['F', 'G', 'E7', 'E7']   # the card on the tonic, the count-in on the dominant; the
                                                             # last bars turn round into the title slam's Am (frame 0)
ROLES = {'ALTO SAX': 'melody', 'TENOR SAX': 'bass'}
LINES = {
    # the ALTO SAX's bright lead, 8ths (E4-A5): each bar opens on a hook (a sweep plays the bar's first three 8ths,
    # OCTAVE KEY's light the same an octave up)
    'ALTO SAX': [76, 74, 72, 74, 76, 79, 76, 72,         # Am:  E D C D E G E C
                 77, 76, 72, 69, 72, 76, 77, 76,         # F:   F E C A C E F E
                 74, 72, 69, 65, 69, 72, 74, 77,         # Dm:  D C A F A C D F
                 76, 74, 71, 68, 71, 74, 76, 80,         # E7:  E D B G# B D E G#
                 81, 79, 76, 72, 76, 79, 81, 79,         # Am:  A G E C E G A G
                 77, 76, 72, 69, 72, 77, 76, 72,         # F:   F E C A C F E C
                 79, 74, 71, 67, 71, 74, 79, 74,         # G:   G D B G B D G D
                 80, 76, 74, 71, 68, 71, 74, 76],        # E7:  G# E D B G# B D E
    # the TENOR SAX's honking low riff, 16ths (G#2-D#4): slot 0 is the pose's honk, the throws play slots 1, 2, 3 ...
    'TENOR SAX': [45, 45, 48, 50, 52, 50, 48, 45, 57, 55, 52, 50, 48, 50, 52, 55,      # Am
                  53, 53, 57, 60, 57, 55, 53, 48, 53, 55, 57, 60, 57, 55, 53, 52,      # F
                  50, 50, 53, 57, 55, 53, 50, 48, 50, 53, 57, 55, 53, 50, 48, 50,      # Dm
                  52, 52, 56, 59, 62, 59, 56, 52, 50, 52, 56, 59, 56, 52, 50, 47,      # E7
                  57, 57, 55, 52, 48, 50, 52, 55, 57, 60, 57, 55, 52, 50, 48, 45,      # Am
                  53, 53, 52, 48, 45, 48, 52, 53, 57, 53, 52, 48, 45, 48, 52, 53,      # F
                  55, 55, 59, 62, 59, 55, 50, 47, 55, 59, 62, 59, 55, 50, 47, 50,      # G
                  52, 52, 50, 47, 44, 47, 50, 52, 56, 59, 56, 52, 50, 47, 44, 47]}     # E7
SOLO = {
    # the alto's victory solo: an 80s pop-sax cry up to the high A, a run down, the G# leading back up to the A
    'ALTO SAX': [(0, 76, .5), (.5, 79, .5), (1, 81, 1.0), (2, 79, .5), (2.5, 76, .5), (3, 74, .5), (3.5, 72, .5),
                 (4, 74, .5), (4.5, 76, .5), (5, 79, .5), (5.5, 80, .5), (6, 81, 2.0), (8, 79, .5), (8.5, 76, .5),
                 (9, 81, 1.5)],
    # the tenor's honking victory riff: up the A minor arpeggio, a growl down, the low A held
    'TENOR SAX': [(0, 45, .5), (.5, 48, .5), (1, 52, .5), (1.5, 55, .5), (2, 57, 1.0), (3, 55, .5), (3.5, 52, .5),
                  (4, 50, .5), (4.5, 48, .5), (5, 52, .5), (5.5, 50, .5), (6, 45, 2.0), (8, 57, .5), (8.5, 55, .5),
                  (9, 45, 1.5)]}
TITLE_HITS = {'ALTO SAX': (76, 81),      # a bright squeal sliding up to the high A
              'TENOR SAX': (45, 57)}     # a low A honk with its octave on top

# ---------------------------------------------------------------- the post
CAPTION = 'Alto or tenor? Sax players, settle it 🎷 Comment the next match-up 👇'
TAGS = {'instagram': '#ballfight #saxophone #altosax #tenorsax #sax #saxophonist #disco #darkdisco #musicmemes '
                     '#pixelart #lowpoly #satisfying',
        'tiktok': '#fyp #ballfight #saxophone #altosax #tenorsax #saxtok #musictok #disco #pixelart #satisfying',
        'shorts': ('Alto Sax vs Tenor Sax: who owns the stage?', 'The alto owns the spotlights, the tenor owns the '
                   'crowd - every hit in key over a dark disco groove. Alto or tenor?',
                   '#shorts #ballfight #saxophone #altosax #tenorsax #music #pixelart')}
