"""#33 BALLFIGHT #7: Ukulele vs Acoustic Guitar (v3), rebuilt as an episode-kit config to check the ACOUSTIC port
(kit/fighters/acoustic.py, made for #44 Electric vs Acoustic). Delivered from projects/ballfight_ukulele_vs_acoustic/
(left as delivered); this config replays the same fight (seed 1130: the same replay-guard lines as #33's own sim).

Only the ACOUSTIC is ported: the UKULELE draws as a bare ball and is silent, and the bed is a stand-in (#33's spaghetti-
western bed isn't a kit mood), so compare the acoustic's layers, not whole frames or the mix. Little brother vs big
brother: UKULELE (left, pink) SURF'S UP vs ACOUSTIC (right, wood tan) SIX-STRING RAKE."""

FIGHTERS = ('UKULELE', 'ACOUSTIC')
SEED = 1130                              # #33 v3's seed (fight at 2 bars)
DMG_SCALE = 0.72                         # #33 v2's frozen damage scale

BPM = 130                                # loop_bpm -> 129.84: 33 bars, FIGHT! at 3.70 s
SERIES = 'BALLFIGHT #7'                  # (#33 predates the no-tag rule)
SHARD_SEED, FX_SEED = 271, 33            # #33's own seeds for the shatter and the particles

MOOD = 'big_top'                         # a stand-in: #33's bed is not a kit mood
KEY, SCALE = 'A', 'minor'
LOOP = ['Am', 'G', 'F', 'E']             # the descending cadence
CHORDS = ['Am', 'E'] + LOOP * 7 + ['Am', 'F', 'E']
ROLES = {'UKULELE': 'melody', 'ACOUSTIC': 'rhythm'}
AM, G, F, E = (45, 52, 57, 60, 64), (43, 47, 50, 55, 59, 67), (41, 48, 53, 57, 60, 65), (40, 47, 52, 56, 59, 64)
SOLO = {
    # the acoustic: rasgueado bursts and bass runs down the cadence, ending on a full Am (#33's, unchanged)
    'ACOUSTIC': [(0, AM, .5), (.5, 64, .25), (.75, 65, .25), (1, 64, .25), (1.25, 62, .25), (1.5, 60, .5),
                 (2, G, .25), (2.25, G, .25), (2.5, G, .25), (2.75, G, .25), (3, 59, .5), (3.5, 57, .5), (4, F, .5),
                 (4.5, 57, .25), (4.75, 60, .25), (5, 65, .5), (5.5, 64, .5), (6, E, .25), (6.25, E, .25), (6.5, E, .25),
                 (6.75, E, .25), (7, 52, .25), (7.25, 53, .25), (7.5, 56, .25), (7.75, 59, .25), (8, AM, .5),
                 (8.5, 69, .25), (8.75, 72, .25), (9, 76, .5), (9.5, 72, .25), (9.75, 69, .25), (10, E, .25),
                 (10.25, E, .25), (10.5, 64, .25), (10.75, 68, .25), (11, (45, 52, 57, 60, 64, 69), 1.0)]}
TITLE_HITS = {'ACOUSTIC': (45, 52, 57, 60, 64, 69)}

CAPTION = 'Little brother vs big brother 🪕🎸 Comment the next match-up 👇'
