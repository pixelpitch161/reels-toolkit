"""#42 BALLFIGHT #16: Viola vs Violin, as a kit config (plan m4 Task 5, Table O3 row 41). Delivered from
projects/ballfight_viola_vs_violin/ (left as delivered, only read). Today's roster (the VIOLIN as #28 built it), the
series-standard beat sheet, the episode's theme (orchestral drill), tempo, key, scale, loop, the violin's line, both
solos, handicap, damage scale and seed. #42 already fights from bar 2 (timeline.py 54), so its seed replays #42's fight.
The viola's inner voice (timeline.py 156 viola_inner) and MELODY AT LAST's lead (166 VIOLA_LEAD) are the VIOLA's part's
own."""

FIGHTERS = ('VIOLA', 'VIOLIN')         # timeline.py 52
SEED = 8219                             # seed.txt
DMG_SCALE = 1.0                         # timeline.py 63
DMG_SIDE = (0.75, 2.1)                  # timeline.py 64: the frozen handicap

BPM = 140                               # timeline.py 46: loop_bpm(140) = 141.64, 36 bars
SHARD_SEED, FX_SEED = 271, 42           # render.py 427 (the shatter), 951 (the particles)

MOOD = 'orchestral_drill'               # Table O2 (audio.py's bed)
KEY, SCALE = 'C', 'harmonic minor'      # timeline.py 105-106: KEY_ROOT 36, SCALE (0, 2, 3, 5, 7, 8, 11)
LOOP = ['Cm', 'Cm', 'Ab', 'Ab', 'Fm', 'Fm', 'G', 'G']   # timeline.py 110
CHORDS = ['Cm', 'G'] + LOOP * 4 + ['Ab', 'G']           # timeline.py 111, as it is
LINES = {
    # timeline.py 130-153: melody_at on each 16th of the 8-bar loop (the 4-bar MELODY twice, fitted to the Ab / Fm bars)
    'VIOLIN': [72, 72, 72, 75, 75, 75, 74, 74, 72, 72, 71, 71, 72, 72, 67, 67,
               75, 75, 75, 79, 79, 79, 77, 77, 75, 75, 74, 74, 73, 73, 72, 72,
               68, 68, 68, 72, 72, 72, 75, 75, 80, 80, 77, 77, 77, 77, 75, 75,
               73, 73, 73, 70, 70, 70, 73, 73, 77, 77, 77, 77, 82, 82, 84, 84,
               72, 72, 72, 75, 75, 75, 73, 73, 72, 72, 70, 70, 72, 72, 65, 65,
               75, 75, 75, 77, 77, 77, 77, 77, 75, 75, 73, 73, 73, 73, 72, 72,
               68, 68, 68, 72, 72, 72, 75, 75, 80, 80, 79, 79, 77, 77, 75, 75,
               74, 74, 74, 71, 71, 71, 74, 74, 77, 77, 79, 79, 83, 83, 84, 84],
}
SOLO = {
    # timeline.py 182-185
    'VIOLA': [(0, 67, 1.0), (1, 68, 0.5), (1.5, 67, 0.5), (2, 72, 1.5), (3.5, 70, 0.5), (4, 68, 1.0), (5, 67, 0.5),
              (5.5, 65, 0.5), (6, 63, 1.0), (7, 62, 0.5), (7.5, 59, 0.5), (8, 60, 1.0), (9, 55, 1.0), (10, 48, 2.0)],
    # timeline.py 186-191
    'VIOLIN': [(0, 72, 0.25), (0.25, 74, 0.25), (0.5, 75, 0.25), (0.75, 77, 0.25), (1, 79, 0.5), (1.5, 80, 0.25),
               (1.75, 83, 0.25), (2, 84, 0.75), (2.75, 83, 0.25), (3, 80, 0.5), (3.5, 79, 0.5), (4, 77, 0.25),
               (4.25, 79, 0.25), (4.5, 77, 0.25), (4.75, 75, 0.25), (5, 74, 0.5), (5.5, 71, 0.5), (6, 72, 0.25),
               (6.25, 74, 0.25), (6.5, 72, 0.25), (6.75, 74, 0.25), (7, 75, 0.5), (7.5, 74, 0.25), (7.75, 71, 0.25),
               (8, 68, 0.5), (8.5, 67, 0.5), (9, 71, 0.25), (9.25, 74, 0.25), (9.5, 77, 0.25), (9.75, 80, 0.25),
               (10, 83, 0.5), (10.5, 79, 0.5), (11, 67, 1.0)],
}
