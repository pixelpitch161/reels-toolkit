"""#33 BALLFIGHT #7: Ukulele vs Acoustic (v3), as a kit config (plan m4 Task 5, Table O3 row 36). Delivered from
projects/ballfight_ukulele_vs_acoustic/ (left as delivered, only read; the kit's examples/ukulele_vs_acoustic is the
same fight under a stand-in bed). Today's roster, the series-standard beat sheet, the episode's theme (spaghetti
western), tempo, key, scale, chords, both solos (strums as chords: the chord solo of M3-T64), roles, damage scale and
seed. #33 v3 already fights from bar 2 (timeline.py 48), so its seed replays #33's fight. The whistle's theme
(timeline.py 103 WHISTLE) is the bed's."""

FIGHTERS = ('UKULELE', 'ACOUSTIC')     # timeline.py 46
SEED = 1130                             # seed.txt
DMG_SCALE = 0.72                        # timeline.py 56

BPM = 130                               # timeline.py 40: loop_bpm(130) = 129.84, 33 bars
SHARD_SEED, FX_SEED = 271, 33           # render.py 446 (the shatter), 834 (the particles)

MOOD = 'spaghetti_western'              # Table O2 (audio.py's bed)
KEY, SCALE = 'A', 'minor'               # timeline.py 74-75: KEY_ROOT 45, SCALE (0, 2, 3, 5, 7, 8, 10)
LOOP = ['Am', 'G', 'F', 'E']            # timeline.py 78: the descending cadence
CHORDS = ['Am', 'E'] + LOOP * 7 + ['Am', 'F', 'E']   # timeline.py 78, as it is
ROLES = {'UKULELE': 'melody', 'ACOUSTIC': 'rhythm'}  # as the kit's examples/ukulele_vs_acoustic plays #33
# the solos with the strums written out (timeline.py 83 UKE_CHORD, 84 AC_CHORD: a tuple is a strum)
SOLO = {
    # timeline.py 115-122
    'UKULELE': [(0, (69, 60, 64, 69), 0.5), (0.5, 76, 0.25), (0.75, 79, 0.25), (1, 81, 0.5), (1.5, 79, 0.25),
                (1.75, 76, 0.25), (2, (67, 62, 67, 71), 0.5), (2.5, 74, 0.25), (2.75, 76, 0.25), (3, 79, 0.5),
                (3.5, 74, 0.5), (4, (69, 60, 65, 69), 0.25), (4.25, (69, 60, 65, 69), 0.25), (4.5, 77, 0.25),
                (4.75, 76, 0.25), (5, 72, 0.5), (5.5, 76, 0.5), (6, (71, 64, 68, 71), 0.5), (6.5, 80, 0.25),
                (6.75, 83, 0.25), (7, 80, 0.5), (7.5, 76, 0.5), (8, (69, 60, 64, 69), 0.25),
                (8.25, (69, 60, 64, 69), 0.25), (8.5, (69, 60, 64, 69), 0.25), (8.75, 81, 0.25), (9, 84, 0.5),
                (9.5, 81, 0.5), (10, 80, 0.5), (10.5, 76, 0.5), (11, (69, 72, 76, 81), 1.0)],
    # timeline.py 123-131
    'ACOUSTIC': [(0, (45, 52, 57, 60, 64), 0.5), (0.5, 64, 0.25), (0.75, 65, 0.25), (1, 64, 0.25), (1.25, 62, 0.25),
                 (1.5, 60, 0.5), (2, (43, 47, 50, 55, 59, 67), 0.25), (2.25, (43, 47, 50, 55, 59, 67), 0.25),
                 (2.5, (43, 47, 50, 55, 59, 67), 0.25), (2.75, (43, 47, 50, 55, 59, 67), 0.25), (3, 59, 0.5),
                 (3.5, 57, 0.5), (4, (41, 48, 53, 57, 60, 65), 0.5), (4.5, 57, 0.25), (4.75, 60, 0.25), (5, 65, 0.5),
                 (5.5, 64, 0.5), (6, (40, 47, 52, 56, 59, 64), 0.25), (6.25, (40, 47, 52, 56, 59, 64), 0.25),
                 (6.5, (40, 47, 52, 56, 59, 64), 0.25), (6.75, (40, 47, 52, 56, 59, 64), 0.25), (7, 52, 0.25),
                 (7.25, 53, 0.25), (7.5, 56, 0.25), (7.75, 59, 0.25), (8, (45, 52, 57, 60, 64), 0.5), (8.5, 69, 0.25),
                 (8.75, 72, 0.25), (9, 76, 0.5), (9.5, 72, 0.25), (9.75, 69, 0.25),
                 (10, (40, 47, 52, 56, 59, 64), 0.25), (10.25, (40, 47, 52, 56, 59, 64), 0.25), (10.5, 64, 0.25),
                 (10.75, 68, 0.25), (11, (45, 52, 57, 60, 64, 69), 1.0)],
}
TITLE_HITS = {'UKULELE': (69, 72, 76, 81),           # audio.py 518-519: the bright chord blast
              'ACOUSTIC': (45, 52, 57, 60, 64, 69)}  # audio.py 529: the full open chord
