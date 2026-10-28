"""#38 BALLFIGHT #12: Bass Clarinet vs Euphonium, rebuilt as an episode-kit config to check the EUPHONIUM port
(kit/fighters/euphonium.py, made for #48 Euphonium vs Trombone). Delivered from
projects/ballfight_bassclarinet_vs_euphonium/ (left as delivered); this config replays the same fight (seed 1269: the
same replay-guard lines as #38's own sim).

Only the EUPHONIUM is ported: the BASS CLARINET isn't yet (it draws as a bare ball and is silent), and the bed is a
stand-in (#38's dark drumline isn't a kit mood yet), so compare the euphonium's layers and its bus, not whole frames or
the mix. The two instruments nobody can name: BASS CLARINET (left, slate blue) ENDPIN ANCHOR vs EUPHONIUM (right,
amber) BELL MORTAR."""

FIGHTERS = ('BASS CLARINET', 'EUPHONIUM')
SEED = 1269                              # #38's seed
DMG_SCALE = 0.68                         # #38's frozen damage scale

BPM = 126                                # loop_bpm -> 125.90: 32 bars, FIGHT! at 3.81 s
SERIES = 'BALLFIGHT #12'                 # (#38 predates the no-tag rule)
SHARD_SEED, FX_SEED = 381, 38            # #38's own seeds for the shatter and the particles
TITLE_LINES = {'BASS CLARINET': ('BASS', 'CLARINET')}   # #38's card: 13 letters on two lines at scale 6
TITLE_BALLS = [(74, 250, 54), (282, 396, 56)]

MOOD = 'big_top'                         # a stand-in: #38's drumline is not a kit mood
KEY, SCALE = 'C', 'minor'
LOOP = ['Cm', 'Ab', 'Fm', 'G']
CHORDS = ['Cm', 'G'] + LOOP * 7 + ['Ab', 'G']
ROLES = {'BASS CLARINET': 'bass', 'EUPHONIUM': 'melody'}
LINES = {
    'BASS CLARINET': [36, 36, 43, 36, 46, 36, 43, 39, 44, 44, 51, 44, 39, 44, 48, 51,     # #38's BASS_8THS
                      41, 41, 48, 41, 44, 41, 48, 44, 43, 43, 50, 43, 47, 43, 50, 47],
    'EUPHONIUM': [67, 68, 67, 63, 60, 63, 68, 72, 72, 68, 65, 68, 67, 62, 59, 62]}       # #38's MELODY (beats)
SOLO = {                                 # #38's, unchanged
    'BASS CLARINET': [(0, 41, .5), (.5, 48, .5), (1, 53, .5), (1.5, 56, .5), (2, 55, .75), (2.75, 53, .25), (3, 51, .5),
                      (3.5, 48, .5), (4, 47, .5), (4.5, 50, .5), (5, 55, .5), (5.5, 53, .5), (6, 50, .5), (6.5, 47, .5),
                      (7, 43, 1.0)],
    'EUPHONIUM': [(0, 65, 1.0), (1, 68, .5), (1.5, 72, .5), (2, 70, 1.0), (3, 68, .5), (3.5, 65, .5), (4, 67, 1.0),
                  (5, 62, .5), (5.5, 65, .5), (6, 67, 2.0)]}

CAPTION = 'Name these two instruments 👀 Comment the next match-up 👇'
