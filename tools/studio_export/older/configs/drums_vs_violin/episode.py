"""#28 BALLFIGHT #2: Drums vs Violin, as a kit config (plan m4 Task 5, Table O3 row 32). Delivered from
projects/ballfight_drums_vs_violin/ (left as delivered, only read). Today's roster, the series-standard beat sheet (the
VS card on bar 0, the count-in on bar 1, FIGHT! on bar 2, the kit's tail), the episode's theme (balkan fire), tempo,
key, scale, loop, the violin's line, both solos, damage scale and seed.

#28 fought from bar 1 (no VS card: timeline.py 41 `T_FIGHT = BAR`), so this config's fight at its seed is the kit's run
under the 2-bar start, not #28's fight (plan question Q-E: the kit's run is the reference recording here)."""

FIGHTERS = ('DRUMS', 'VIOLIN')         # timeline.py 39
SEED = 568                              # seed.txt (the replay guard's #28 seed)
DMG_SCALE = 1.3                         # timeline.py 45

BPM = 140                               # timeline.py 33: loop_bpm(140) = 141.64, 36 bars
SHARD_SEED = 271                        # render.py 332 (#28 draws no particles: FX_SEED is the kit's default)

MOOD = 'balkan_fire'                    # Table O2 (audio.py's bed)
KEY, SCALE = 'D', 'harmonic minor'      # timeline.py 63-64: KEY_ROOT 38, SCALE (0, 2, 3, 5, 7, 8, 11)
LOOP = ['Dm', 'Dm', 'Gm', 'A']          # timeline.py 67: the loop i - i - iv - V
# timeline.py 67 is ['A'] + LOOP * 8 + ['Dm', 'Gm', 'A'] from FIGHT! on bar 1; on the 2-bar start the card takes
# the tonic and the count-in the dominant, then the same eight loops and the episode's last two bars (Gm, A)
CHORDS = ['Dm', 'A'] + LOOP * 8 + ['Gm', 'A']
LINES = {
    # timeline.py 89-100: MELODY, one pitch per 16th over the loop (the bow samples it)
    'VIOLIN': [74, 74, 73, 74, 76, 76, 77, 77, 76, 76, 74, 74, 73, 73, 70, 70,
               69, 69, 70, 69, 67, 67, 69, 69, 70, 70, 73, 73, 74, 74, 76, 76,
               77, 77, 79, 79, 77, 77, 74, 74, 82, 82, 81, 81, 79, 79, 77, 77,
               76, 76, 77, 76, 73, 73, 76, 76, 79, 79, 81, 81, 82, 82, 85, 85],
}
SOLO = {
    # timeline.py 111-118
    'VIOLIN': [(0, 74, 0.25), (0.25, 76, 0.25), (0.5, 77, 0.25), (0.75, 79, 0.25), (1, 81, 0.5), (1.5, 82, 0.25),
               (1.75, 85, 0.25), (2, 86, 0.75), (2.75, 85, 0.25), (3, 82, 0.5), (3.5, 81, 0.5), (4, 79, 0.25),
               (4.25, 81, 0.25), (4.5, 79, 0.25), (4.75, 77, 0.25), (5, 76, 0.5), (5.5, 73, 0.5), (6, 74, 0.25),
               (6.25, 76, 0.25), (6.5, 74, 0.25), (6.75, 76, 0.25), (7, 77, 0.5), (7.5, 76, 0.25), (7.75, 73, 0.25),
               (8, 70, 0.5), (8.5, 69, 0.5), (9, 73, 0.25), (9.25, 76, 0.25), (9.5, 79, 0.25), (9.75, 82, 0.25),
               (10, 85, 0.5), (10.5, 81, 0.5), (11, 74, 1.0)],
    # timeline.py 119-125: (beat, piece, velocity), piece one of K S T1 T2 T3 C (audio.py 41 PIECE)
    'DRUMS': [(0, 'C', 1.0), (0, 'K', 1.0), (0.5, 'S', 0.8), (0.75, 'S', 0.6), (1, 'K', 0.9), (1.5, 'T1', 0.8),
              (1.75, 'T1', 0.7), (2, 'T2', 0.9), (2.25, 'T2', 0.7), (2.5, 'T3', 0.9), (2.75, 'T3', 0.8), (3, 'K', 1.0),
              (3.25, 'S', 0.7), (3.5, 'S', 0.8), (3.75, 'S', 0.9), (4, 'C', 0.9), (4, 'K', 1.0), (4.5, 'K', 0.8),
              (4.75, 'S', 0.8), (5, 'K', 0.9), (5.25, 'T1', 0.7), (5.5, 'T2', 0.8), (5.75, 'T3', 0.9), (6, 'S', 0.9),
              (6.25, 'S', 0.5), (6.5, 'S', 0.7), (6.75, 'S', 0.9), (7, 'K', 1.0), (7.25, 'T3', 0.8), (7.5, 'T2', 0.8),
              (7.75, 'T1', 0.8), (8, 'C', 1.0), (8, 'K', 1.0), (8.5, 'S', 0.9), (9, 'K', 0.9), (9.25, 'K', 0.8),
              (9.5, 'S', 0.9), (10, 'T1', 0.8), (10.25, 'T2', 0.8), (10.5, 'T3', 0.9), (10.75, 'T3', 0.9),
              (11, 'C', 1.0), (11, 'K', 1.0)],
}
