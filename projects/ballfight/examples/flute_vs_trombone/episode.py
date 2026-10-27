"""#41 BALLFIGHT #15: Flute vs Trombone, rebuilt as an episode-kit config (the worked example of a crossover: this file
is the whole episode; the fighters' looks and sounds come from kit/fighters/, the bed from kit/moods/big_top.py).
Delivered from projects/ballfight_flute_vs_trombone/ (left as delivered); this config replays the same fight (seed 392)
and renders the same frames.

The front row vs the back row: a real band-room feud (the flutes sit up front, the trombones behind them poke them with
their slides); now the flute answers. FLUTE (left, silver) as built in #32 v2, TROMBONE (right, mint) as built in #34.
Music, mood A: sinister big top in E harmonic minor; the TROMBONE is the bass and the smears, the FLUTE the high
melody. The lance swats cobras (the opt-in rule from #41)."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('FLUTE', 'TROMBONE')         # first named = left corner (screen left)
SEED = 392                               # seed_search 1..6000 --stop 8: 8 fits in 968 runs, ranked first (7.78)
DMG_SCALE = 1.0
DMG_SIDE = (2.0, 0.40)                   # the crossover's only lever (tune: 300 seeds, FLUTE 47 % / TROMBONE 53 %)
RULES = dict(swat_cobras=True)           # a live lance knocks a risen cobra flat (fight.py, #41)

# ---------------------------------------------------------------- tempo, title (only to match the delivered #41)
BPM = 132                                # loop_bpm -> 133.77: 34 bars in 61 s, FIGHT! at 3.59 s
SERIES = 'BALLFIGHT #15'                 # #41 predates the no-tag rule (Oct 27): new episodes leave this out
SHARD_SEED, FX_SEED = 271, 41            # #41's own seeds for the shatter and the particles

# ---------------------------------------------------------------- music: mood A, sinister big top
MOOD = 'big_top'
KEY, SCALE = 'E', 'harmonic minor'       # C natural + D#: the carnival / sideshow colour
LOOP = ['Em', 'Am', 'B7', 'Em', 'C', 'Am', 'B7', 'B7']
CHORDS = ['Em', 'B7'] + LOOP * 4         # the title card on the tonic, the count-in on the dominant; the loop's B7 B7
                                         # falls into the title slam's Em on frame 0
ROLES = {'TROMBONE': 'bass', 'FLUTE': 'melody'}
LINES = {
    # the FLUTE's sideshow line, one note per 8th over the 8-bar loop (E5-F#6; the augmented second C-D# and the D#
    # leading tone give the charmer colour): every 8th while its cobras are up
    'FLUTE': [76, 79, 83, 79, 76, 75, 76, 71,      # Em: E G B G E D# E B
              81, 84, 83, 81, 79, 76, 77, 76,      # Am: A C B A G E F E (the F: a passing phrygian sting)
              75, 78, 81, 78, 83, 81, 78, 75,      # B7: D# F# A F# B A F# D#
              76, 71, 76, 79, 83, 84, 83, 79,      # Em: E B E G B C B G
              84, 83, 79, 76, 72, 76, 79, 84,      # C: C B G E C E G C
              81, 76, 72, 76, 81, 84, 88, 84,      # Am: A E C E A C E C
              83, 87, 86, 87, 90, 87, 83, 81,      # B7: B D# D D# F# D# B A
              78, 75, 71, 75, 78, 81, 83, 75]}     # B7: F# D# B D# F# A B D# (leads home)
SOLO = {
    # the cobras' charmer line, high and ornamented, falling to the E
    'FLUTE': [(0, 83, .5), (.5, 84, .25), (.75, 83, .25), (1, 81, .5), (1.5, 79, .5), (2, 76, .5), (2.5, 75, .25),
              (2.75, 76, .25), (3, 79, .5), (3.5, 83, .5), (4, 88, .75), (4.75, 87, .25), (5, 84, .5), (5.5, 83, .5),
              (6, 76, 1.5)],
    # the clown's victory, 3 bars (Em . C . Am): a big smear up, oom-pah struts, a fanfare over C, the walk down to A
    'TROMBONE': [(0, 40, .5), (.5, 47, .5), (1, 52, 1.0), (2, 55, .5), (2.5, 52, .5), (3, 47, .5), (3.5, 51, .5),
                 (4, 48, .5), (4.5, 55, .5), (5, 60, .75), (5.75, 59, .25), (6, 55, .5), (6.5, 52, .5), (7, 48, .5),
                 (7.5, 47, .5), (8, 45, .5), (8.5, 52, .5), (9, 57, .75), (9.75, 55, .25), (10, 52, .5), (10.5, 48, .5),
                 (11, 45, 1.0)],
}
TITLE_HITS = {'FLUTE': (76, 79, 83, 84, 86, 87, 88),     # a shrieking flurry up to a flutter-tongued high E
              'TROMBONE': (40, 52)}                     # the low E + a big smear up to E3

# ---------------------------------------------------------------- the post
CAPTION = 'The front row vs the back row 🪈🎺 Comment the next match-up 👇'
TAGS = {'instagram': '#ballfight #flute #trombone #bandkids #marchingband #musicmemes #musician #pixelart #lowpoly '
                     '#gamedev #satisfying #music',
        'tiktok': '#fyp #ballfight #flute #trombone #bandkid #musictok #pixelart #satisfying #bandtok #music',
        'shorts': ('Flute vs Trombone: the front row fights back', 'Silver cobras vs a slide lance, every hit in '
                   'key. Who wins the band-room feud?', '#shorts #ballfight #flute #trombone #bandkid #music #pixelart')}
