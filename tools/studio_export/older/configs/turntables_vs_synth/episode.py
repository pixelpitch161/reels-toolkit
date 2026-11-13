"""#31 BALLFIGHT #5: Turntables vs Synth (v3), as a kit config (plan m4 Task 5, Table O3 row 34). Delivered from
projects/ballfight_turntables_vs_synth/ (left as delivered, only read). Today's roster, the series-standard beat sheet,
the episode's theme (dark breakbeat), tempo, key, scale, chords, both solos, damage scale and seed. #31 v3 already
fights from bar 2 (timeline.py 49), so its seed replays #31's fight. The SYNTH's arpeggio table (timeline.py 88 ARP:
eight notes per chord, not a line over the loop) stays with the SYNTH's part."""

FIGHTERS = ('TURNTABLES', 'SYNTH')     # timeline.py 47
SEED = 1339                             # seed.txt
DMG_SCALE = 0.6                         # timeline.py 58

BPM = 138                               # timeline.py 41: loop_bpm(138) = 137.70, 35 bars
SHARD_SEED, FX_SEED = 271, 31           # render.py 449 (the shatter), 821 (the particles)

MOOD = 'dark_breakbeat'                 # Table O2 (audio.py's bed)
KEY, SCALE = 'F', 'minor'               # timeline.py 76-77: KEY_ROOT 41, SCALE (0, 2, 3, 5, 7, 8, 10)
LOOP = ['Fm', 'Db', 'Bbm', 'C']         # timeline.py 81
CHORDS = ['Fm', 'C'] + LOOP * 8 + ['C']  # timeline.py 81, as it is
SOLO = {
    # timeline.py 110-115
    'SYNTH': [(0, 72, 0.5), (0.5, 75, 0.25), (0.75, 77, 0.25), (1, 80, 0.5), (1.5, 79, 0.5), (2, 77, 0.25),
              (2.25, 75, 0.25), (2.5, 77, 1.0), (3.5, 72, 0.5), (4, 73, 0.5), (4.5, 77, 0.25), (4.75, 80, 0.25),
              (5, 85, 1.0), (6, 84, 0.5), (6.5, 80, 0.5), (7, 82, 0.25), (7.25, 80, 0.25), (7.5, 79, 0.5), (8, 76, 0.5),
              (8.5, 79, 0.25), (8.75, 82, 0.25), (9, 84, 0.75), (9.75, 82, 0.25), (10, 79, 0.5), (10.5, 76, 0.5),
              (11, 77, 1.0)],
    # timeline.py 116-123: (beat, kind, beats), kind a scratch (baby, chirp, trans, flare, tear, scrib, back) or a stab
    'TURNTABLES': [(0, 'stab', 0.5), (0, 'back', 1.0), (1, 'baby', 0.5), (1.5, 'baby', 0.5), (2, 'chirp', 0.25),
                   (2.25, 'chirp', 0.25), (2.5, 'trans', 0.5), (3, 'stab', 0.25), (3.25, 'stab', 0.25),
                   (3.5, 'flare', 0.5), (4, 'stab', 0.5), (4.5, 'tear', 0.5), (5, 'scrib', 0.5), (5.5, 'chirp', 0.25),
                   (5.75, 'chirp', 0.25), (6, 'trans', 1.0), (7, 'stab', 0.25), (7.5, 'stab', 0.25), (8, 'stab', 0.5),
                   (8, 'flare', 0.5), (8.5, 'baby', 0.25), (8.75, 'baby', 0.25), (9, 'tear', 0.5), (9.5, 'chirp', 0.25),
                   (9.75, 'chirp', 0.25), (10, 'scrib', 0.5), (10.5, 'trans', 0.5), (11, 'stab', 1.0),
                   (11, 'back', 1.0)],
}
