"""#40 BALLFIGHT #14: Bari Sax vs Bass Clarinet (v2), rebuilt as an episode-kit config to check the BASS CLARINET port
(kit/fighters/bass_clarinet.py, made for #55 Bass Clarinet vs Bassoon). Delivered from
projects/ballfight_barisax_vs_bassclarinet/ (left as delivered); this config replays the same fight (seed 128: the same
replay-guard lines as #40's own sim).

Only the BASS CLARINET is ported: the BARI SAX isn't yet (it draws as a bare ball and is silent), and the bed is a
stand-in (#40's construction site isn't a kit mood yet), so compare the bass clarinet's layers and its bus, not whole
frames or the mix. The kit's tail is the series standard: compare frames up to the shatter. The real one vs the
lookalike: BARI SAX (left, coral) WRECKING BALL vs BASS CLARINET (right, slate blue) ENDPIN ANCHOR."""
from fighters import FIGHTERS as _ROSTER

# #40 v2 was made before the roster's BARI SAX could turn a cramped swing's plane near a corner (weapon option 'steer',
# #39 v4): pinned off for this check process only, exactly as #40's own sim.py does, so the fight replays v2
_ROSTER['BARI SAX']['weapon'] = {k: v for k, v in _ROSTER['BARI SAX']['weapon'].items() if k != 'steer'}

FIGHTERS = ('BARI SAX', 'BASS CLARINET')
SEED = 128                               # #40 v2's seed
DMG_SCALE = 1.0
DMG_SIDE = (0.30, 0.66)                  # #40 v2's frozen handicap

BPM = 126                                # loop_bpm -> 125.90: 32 bars, FIGHT! at 3.81 s
SERIES = 'BALLFIGHT #14'                 # (#40 predates the no-tag rule)
SHARD_SEED, FX_SEED = 401, 40            # #40's own seeds for the shatter and the particles
TITLE_LINES = {'BASS CLARINET': ('BASS', 'CLARINET')}
TITLE_BALLS = [(78, 222, 56), (302, 352, 48)]

MOOD = 'big_top'                         # a stand-in: #40's construction site is not a kit mood
KEY, SCALE = 'G', 'minor'
LOOP = ['Gm', 'Eb', 'Cm', 'D']
CHORDS = ['Gm', 'D'] + LOOP * 7 + ['Eb', 'D']
ROLES = {'BARI SAX': 'bass', 'BASS CLARINET': 'counter'}     # #40: the counter-line an octave over the bari's bass
LINES = {
    'BASS CLARINET': [55, 55, 62, 55, 58, 55, 62, 57,       # #40's COUNTER_8THS
                      51, 51, 58, 51, 55, 51, 58, 55,
                      48, 48, 55, 48, 51, 48, 55, 51,
                      50, 50, 57, 50, 54, 50, 57, 54]}
SOLO = {                                 # #40's, unchanged
    'BARI SAX': [(0, 36, .75), (1, 43, .5), (1.5, 48, .5), (2, 51, .25), (2.5, 48, .5), (3, 46, .5), (3.5, 43, .25),
                 (4, 38, .5), (5, 50, .25), (5.25, 48, .25), (5.5, 45, .25), (5.75, 42, .25), (6, 38, 1.5)],
    'BASS CLARINET': [(0, 48, .5), (.5, 51, .5), (1, 55, .5), (1.5, 58, .5), (2, 60, .75), (2.75, 58, .25),
                      (3, 55, .5), (3.5, 51, .5), (4, 50, .5), (4.5, 54, .5), (5, 57, .5), (5.5, 54, .25),
                      (5.75, 50, .25), (6, 50, 1.5)]}

CAPTION = 'The real one vs the lookalike 🎷 Comment the next match-up 👇'
