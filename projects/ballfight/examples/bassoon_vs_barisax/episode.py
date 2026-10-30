"""#39 BALLFIGHT #13: Bassoon vs Bari Sax (v4), rebuilt as an episode-kit config to check the BASSOON port
(kit/fighters/bassoon.py, made for #52 Oboe vs Bassoon). Delivered from projects/ballfight_bassoon_vs_barisax/ (left as
delivered); this config replays the same fight (seed 77: the same replay-guard lines as #39's own sim).

Only the BASSOON is ported: the BARI SAX isn't yet (it draws as a bare ball and is silent), and the bed is a stand-in
(#39's dark ska-punk isn't a kit mood yet), so compare the bassoon's layers and its bus, not whole frames or the mix.
The kit's tail is the series standard (the reassembly two bars before the end; #39's ran half a bar later): compare
frames up to the shatter. The sneak vs the ring rattler: BASSOON (left, forest green) TIPTOE vs BARI SAX (right,
coral) WRECKING BALL."""

FIGHTERS = ('BASSOON', 'BARI SAX')
SEED = 77                                # #39 v4's seed
DMG_SCALE = 0.34                         # #39's frozen damage scale
DMG_SIDE = (1.0, 0.8)                    # #39's: the BARI SAX's hits x0.8

BPM = 170                                # loop_bpm -> 169.18: 43 bars, FIGHT! at 2.84 s
SERIES = 'BALLFIGHT #13'                 # (#39 predates the no-tag rule)
SHARD_SEED, FX_SEED = 371, 39            # #39's own seeds for the shatter and the particles

MOOD = 'big_top'                         # a stand-in: #39's ska-punk is not a kit mood
KEY, SCALE = 'E', 'minor'
LOOP = ['Em', 'C', 'D', 'B']
CHORDS = ['Em', 'B'] + LOOP * 10 + ['B']
ROLES = {'BASSOON': 'melody', 'BARI SAX': 'bass'}
LINES = {
    'BASSOON': [64, 67, 71, 67, 64, 66, 67, 69,      # #39's BASSOON_8THS
                64, 67, 72, 67, 64, 67, 71, 67,
                66, 69, 74, 69, 66, 69, 72, 69,
                66, 71, 75, 71, 66, 69, 71, 63]}
SOLO = {                                 # #39's, unchanged
    'BASSOON': [(0, 64, .25), (.5, 67, .25), (1, 71, .25), (1.5, 69, .25), (2, 67, .25), (2.5, 66, .25), (3, 64, .25),
                (3.5, 71, .25), (4, 76, .25), (4.5, 74, .25), (5, 71, .25), (5.5, 67, .25), (6, 64, .5), (7, 40, .75)],
    'BARI SAX': [(0, 40, .5), (1, 52, .5), (1.5, 50, .25), (2, 47, .5), (3, 45, .25), (3.5, 43, .5), (4, 40, .5),
                 (5, 47, .25), (5.5, 45, .25), (6, 40, 1.5)]}

CAPTION = 'The sneak vs the ring rattler 🎶 Comment the next match-up 👇'
