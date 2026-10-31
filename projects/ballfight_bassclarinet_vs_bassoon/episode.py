"""#55 BALLFIGHT: Bass Clarinet vs Bassoon (episode kit config: projects/ballfight/README.md, "Making an episode"). A
crossover and a viewer request ("Gotta get bass clarinet and bassoon"): the BASS CLARINET as built in #38 / #40 (ported
into the kit for this episode), the BASSOON as built in #39 / #52, both exactly as in fighters.py; the only per-episode
lever is the damage handicap (DMG_SIDE). Sides: BASS CLARINET left, BASSOON right (its usual side).
Idea + hook: the bottom of the woodwind section. Both hold down the bass line, both get mistaken for something else
("a black saxophone", "a weird oboe"), their players sit side by side. The anchor-and-pivot whirler vs the sneak.
Music: mood horror_trap (horror suspense trap, E minor at loop_bpm(140) = 141.64: 36 bars, bar = 1.694 s, FIGHT! at
3.39 s). The bass clarinet growls the bass line (its whirls play it on the 8ths, the peg the bar's root, slap pops on the
smashes), the bassoon the comic staccato line above it (its hops are its notes); the leader fuller and louder, the loser
thinner and dirtier. LIGHTS OUT drops the bed to the music box + the 808 for its bar; MOUNTAIN KING quotes Grieg's
public-domain theme in E minor over the trap beat, one note per hop, speeding up with the hops."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('BASS CLARINET', 'BASSOON')       # first named = left corner (screen left)
SEED = None                      # python -m ballfight search bassclarinet_vs_bassoon -> the best fit
DMG_SCALE = 1.0
DMG_SIDE = (0.58, 0.70)          # the only lever, tuned once and frozen (300 seeds: BASS CLARINET 51 % / BASSOON 49 % of
                                 # 296 decided, median K.O. 49.5 s, both lows 74 %; grid 0.5-1.0 x 0.5-1.2, then 0.56-0.64 x 0.62-0.70)
RULES = dict()

# ---------------------------------------------------------------- tempo + music
BPM = 140                        # loop_bpm -> 141.64: 36 bars, bar = 1.694 s, FIGHT! at 3.39 s
MOOD = 'horror_trap'
KEY, SCALE = 'E', 'minor'        # the Grieg quote sits in E minor (E4) as in #39
LOOP = ['Em', 'Em', 'C', 'B', 'Em', 'Em', 'Am', 'B']
CHORDS = ['Em', 'B'] + LOOP * 4 + ['C', 'B']             # the last bar's B falls into the title slam's Em
TITLE_LINES = {'BASS CLARINET': ('BASS', 'CLARINET')}    # 13 letters: two lines (as #38 / #40)
ROLES = {'BASS CLARINET': 'bass', 'BASSOON': 'line'}
LINES = {
    # the bass clarinet's growling bass line (its whirls play it on the 8ths), half-time trap feel, creeping
    # chromatically into the next chord, B1-A3
    'BASS CLARINET': [40, 40, 52, 40, 43, 40, 47, 40,   40, 40, 52, 40, 43, 46, 47, 48,      # Em Em
                      36, 36, 48, 36, 40, 36, 43, 42,   35, 35, 47, 35, 39, 42, 45, 39,      # C B
                      40, 40, 52, 40, 43, 40, 47, 40,   40, 40, 52, 47, 46, 45, 44, 44,      # Em Em
                      45, 45, 57, 45, 48, 45, 52, 51,   35, 35, 47, 35, 39, 41, 42, 39],     # Am B
    # the bassoon's comic staccato line above it (a hop landing plays the 8th it lands on), sneaking, E3-E5
    'BASSOON': [64, 67, 71, 67, 64, 63, 64, 67,         64, 67, 72, 71, 70, 69, 68, 67,      # Em Em
                64, 67, 72, 67, 64, 67, 71, 72,         63, 66, 71, 66, 63, 66, 69, 71,      # C B
                64, 67, 71, 67, 64, 63, 64, 67,         76, 75, 74, 73, 72, 71, 70, 71,      # Em Em
                69, 72, 76, 72, 69, 68, 69, 72,         71, 69, 66, 63, 66, 69, 71, 63]}     # Am B
SOLO = {
    # a chalumeau growl climbing out of the dark, a slap-tongue pop at the top, sinking back to the low E
    'BASS CLARINET': [(0, 40, .5), (.5, 43, .5), (1, 47, .5), (1.5, 50, .5), (2, 52, .75), (2.75, 55, .25),
                      (3, 52, .5), (3.5, 47, .5), (4, 45, .5), (4.5, 48, .5), (5, 51, .5), (5.5, 47, .25),
                      (5.75, 42, .25), (6, 40, 1.5)],
    # a sneaky staccato run that tiptoes up, peeks over the top, and lands home with a comic low blat
    'BASSOON': [(0, 64, .25), (.5, 67, .25), (1, 71, .25), (1.5, 69, .25), (2, 67, .25), (2.5, 66, .25), (3, 64, .25),
                (3.5, 71, .25), (4, 76, .25), (4.5, 74, .25), (5, 71, .25), (5.5, 67, .25), (6, 64, .5),
                (7, 40, .75)]}
TITLE_HITS = {'BASSOON': (40, 52)}                         # the bassoon's blat on low E (the bass clarinet: its growl)

# ---------------------------------------------------------------- the post
CAPTION = 'The low woodwinds settle it 🖤 Comment the next match-up 👇'
TAGS = {}
