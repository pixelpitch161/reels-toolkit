"""#44 BALLFIGHT: Electric vs Acoustic (episode kit config: projects/ballfight/README.md, "Making an episode").
The guitar family feud, plugged vs unplugged. ELECTRIC (left, neon green, new: LIGHTNING RIFF, every riff note a bolt
from its headstock or from the corner post nearest the enemy) vs ACOUSTIC (right, wood tan, exactly as built in #33:
SIX-STRING RAKE). No episode number on screen.
Music, mood B: minor stadium rock (kit/moods/stadium_rock.py) in E minor at loop_bpm(126) = 125.90 BPM (32 bars, FIGHT!
at 3.81 s), one original song in two arrangements that follow the leader: plugged while the ELECTRIC leads, unplugged
while the ACOUSTIC leads, both before anyone leads and on the card. The ELECTRIC plays the riffs (its bolts are the riff
notes), the ACOUSTIC the strums and picked lines; the winner's solo is shredded or fingerpicked."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('ELECTRIC', 'ACOUSTIC')      # first named = left corner (screen left)
SEED = 418                               # search 1..15000 --stop 6: 6 fits in 1251 runs (ELECTRIC 50 %), ranked first (7.64)
DMG_SCALE = 1.0
DMG_SIDE = (0.92, 1.33)                  # the per-fighter handicap (crossover rule): tuned once (tune 160 seeds
                                         # a cell), frozen
RULES = dict()

# ---------------------------------------------------------------- tempo + music: mood B, minor stadium rock
BPM = 126                                # loop_bpm -> 125.90: 32 bars in 61 s
MOOD = 'stadium_rock'
PLUGGED = 'ELECTRIC'                     # its lead = the band plugged in; the ACOUSTIC's = unplugged
KEY, SCALE = 'E', 'minor'
LOOP = ['Em', 'C', 'G', 'D', 'Em', 'C', 'Am', 'B7']
CHORDS = ['Em', 'B7'] + LOOP * 3 + ['Em', 'C', 'G', 'D', 'Am', 'B7']   # the card on the tonic, the count-in on the
                                                                     # dominant; bar 31's B7 falls into the slam's Em
ROLES = {'ELECTRIC': 'riff', 'ACOUSTIC': 'rhythm'}
LINES = {
    # the ELECTRIC's riff, 8 notes a bar over the loop (E3-E5): each bolt plays the note of its slot (the 3-3-2 notes
    # first: the 8ths 1, 4 and 7, then the fills), so the riff grows with the bolts
    'ELECTRIC': [64, 64, 67, 64, 69, 67, 64, 62,       # Em
                 64, 64, 67, 64, 72, 71, 67, 64,       # C
                 67, 67, 71, 67, 74, 71, 67, 66,       # G
                 66, 66, 69, 66, 74, 72, 69, 66,       # D
                 64, 64, 67, 64, 71, 69, 67, 64,       # Em
                 67, 67, 72, 67, 76, 74, 72, 71,       # C
                 69, 69, 72, 69, 76, 74, 72, 69,       # Am
                 71, 71, 75, 71, 74, 75, 66, 63]}      # B7 (D# leading home)
EM, C, G, D = (40, 47, 52, 55, 59, 64), (48, 52, 55, 60, 64), (43, 47, 50, 55, 59, 67), (50, 57, 62, 66)
SOLO = {
    # shredded: a pentatonic run down, a bend on the high E, a tapped arpeggio, the dive home to E
    'ELECTRIC': [(0, 76, .25), (.25, 74, .25), (.5, 71, .25), (.75, 69, .25), (1, 67, .25), (1.25, 69, .25),
                 (1.5, 71, .25), (1.75, 74, .25), (2, 76, 1.0), (3, 79, .25), (3.25, 76, .25), (3.5, 74, .25),
                 (3.75, 71, .25), (4, 72, .25), (4.25, 76, .25), (4.5, 79, .25), (4.75, 84, .25), (5, 79, .25),
                 (5.25, 76, .25), (5.5, 72, .25), (5.75, 76, .25), (6, 74, .25), (6.25, 78, .25), (6.5, 81, .25),
                 (6.75, 78, .25), (7, 75, .5), (7.5, 71, .5), (8, 76, 2.0)],
    # fingerpicked: the open shapes, thumb on the bass, a strum to close each chord, home on a full Em
    'ACOUSTIC': [(0, 40, .5), (.5, 55, .5), (1, 59, .5), (1.5, 64, .5), (2, 52, .5), (2.5, 59, .5), (3, EM, 1.0),
                 (4, 48, .5), (4.5, 55, .5), (5, 60, .5), (5.5, 64, .5), (6, C, .5), (6.5, 67, .5), (7, 64, .5),
                 (7.5, 60, .5), (8, 43, .5), (8.5, 55, .5), (9, 59, .5), (9.5, 67, .5), (10, D, .5), (10.5, 66, .5),
                 (11, 62, .5), (11.5, 57, .5), (12, EM, 2.0)]}
TITLE_HITS = {'ELECTRIC': (40, 64, 67, 71, 76),          # a power-chord blast on E + a quick riff flurry up
              'ACOUSTIC': EM}                            # the open E minor rung out

# ---------------------------------------------------------------- the post
CAPTION = 'Plugged or unplugged? Pick a side 🎸⚡ Comment the next match-up 👇'
TAGS = {'instagram': '#ballfight #electricguitar #acousticguitar #guitar #guitarist #rockmusic #musicmemes #pixelart '
                     '#lowpoly #gamedev #satisfying #music',
        'tiktok': '#fyp #ballfight #electricguitar #acousticguitar #guitartok #rock #pixelart #satisfying #musictok '
                  '#guitar',
        'shorts': ('Electric vs Acoustic: plugged or unplugged?', 'Lightning riffs vs a six-string rake, every hit in '
                   'key. Which guitar wins the family feud?', '#shorts #ballfight #electricguitar #acousticguitar '
                   '#guitar #rock #pixelart')}
