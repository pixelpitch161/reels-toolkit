"""#29 BALLFIGHT #3: Trumpet vs Sax (v3), rebuilt as an episode-kit config to check the TRUMPET port
(kit/fighters/trumpet.py, made for #46 Trumpet vs Clarinet). Delivered from projects/ballfight_trumpet_vs_sax/ (left as
delivered); this config replays the same fight (seed 191: the same replay-guard lines as #29's own sim).

Only the TRUMPET is ported: the SAX is retired (Oct 27) and stays unported, so it draws as a bare ball and is silent,
and the bed is a stand-in (#29's noir-jazz bed isn't a kit mood), so compare the trumpet's layers, not whole frames or
the mix. #29 predates the VS title card: FIGHT! lands on bar 1 (FIGHT_BAR), so the kit's card overlaps the count-in
here. Jazz's oldest argument: TRUMPET (left, yellow) VALVE COMBO vs SAX (right, teal) STRAP FLAIL."""

FIGHTERS = ('TRUMPET', 'SAX')
SEED = 191                               # #29 v3's seed
DMG_SCALE = 0.6                          # #29 v3's frozen damage scale
FIGHT_BAR = 1                            # #29: count-in on bar 0, FIGHT! on the bar-1 downbeat

BPM = 132                                # loop_bpm -> 133.77: 34 bars, FIGHT! at 1.79 s
SERIES = 'BALLFIGHT #3'                  # (#29 predates the no-tag rule)
SHARD_SEED, FX_SEED = 271, 29            # #29's own seeds for the shatter and the particles

MOOD = 'big_top'                         # a stand-in: #29's bed is not a kit mood
KEY, SCALE = 'C', 'minor'
LOOP_START = 1
LOOP = ['Cm', 'Ab7', 'Fm', 'G7']         # the noir loop i - bVI7 - iv - V7
CHORDS = ['G7'] + LOOP * 8 + ['Ab7']     # the count-in on G7, the last bar Ab7 -> G7 -> Cm on FIGHT!
ROLES = {'TRUMPET': 'melody', 'SAX': 'melody'}
SOLO = {                                 # #29's, unchanged
    'TRUMPET': [(0, 72, .5), (.5, 75, .5), (1, 79, .75), (1.75, 77, .25), (2, 75, .5), (2.5, 74, .25), (2.75, 72, .25),
                (3, 79, 1.0), (4, 80, .5), (4.5, 78, .25), (4.75, 75, .25), (5, 72, .5), (5.5, 75, .5), (6, 78, .75),
                (6.75, 80, .25), (7, 84, 1.0), (8, 82, .25), (8.25, 80, .25), (8.5, 77, .5), (9, 80, .5), (9.5, 77, .25),
                (9.75, 75, .25), (10, 72, .5), (10.5, 75, .5), (11, 79, .5), (11.5, 81, .5), (12, 83, 1.25)],
    'SAX': [(0, 60, .5), (.5, 63, .25), (.75, 65, .25), (1, 66, .5), (1.5, 67, .5), (2, 70, .75), (2.75, 67, .25),
            (3, 63, 1.0), (4, 56, .75), (4.75, 60, .25), (5, 63, .5), (5.5, 66, .5), (6, 68, .5), (6.5, 66, .25),
            (6.75, 63, .25), (7, 60, 1.0), (8, 65, .25), (8.25, 67, .25), (8.5, 68, .25), (8.75, 70, .25), (9, 72, .5),
            (9.5, 75, .5), (10, 72, .25), (10.25, 70, .25), (10.5, 68, .5), (11, 67, .5), (11.5, 66, .5),
            (12, 67, 1.25)]}

CAPTION = "Who's the lead horn? 🎺🎷 Comment the next match-up 👇"
