"""#28 BALLFIGHT #2: Drums vs Violin, rebuilt as an episode-kit config (the check of the DRUMS port, kit/fighters/
drums.py, and the Balkan-fire mood, kit/moods/balkan_fire.py). Delivered from projects/ballfight_drums_vs_violin/ (left
as delivered); this config replays the same fight (seed 568).

#28 predates the VS title card: FIGHT_BAR = 1 keeps its count-in on bar 0 and FIGHT! on bar 1 so the honest run is the
same one (the kit draws no card for it). It also predates the v2 effects, the HUD ability list and the one-master-gain
loudness rule, so whole frames and the mastered mix differ from #28's by design; the DRUMS' layers and its 'kit' bus,
and the bed's buses, are the parts to compare. The VIOLIN here is the one last delivered (#42: rosin dust, attack
lights, its #42 part). Brute force vs finesse: DRUMS (left, orange) pulses on the beat vs VIOLIN (right, violet).
Music, mood B: dark Balkan fire in D harmonic minor, no drums of its own: the DRUMS fighter is the kit."""

FIGHTERS = ('DRUMS', 'VIOLIN')
SEED = 568                               # seed_search 1..2000 --stop 6: 6 fits in 932
DMG_SCALE = 1.3                          # tuned once (DRUMS 57 % / VIOLIN 43 % of 767 decided)

BPM = 140                                # loop_bpm -> 141.64: 36 bars
FIGHT_BAR = 1                            # (#28's layout: count-in on bar 0, FIGHT! at 1.69 s, no VS card)
REASSEMBLE_AT = (34, 1)                  # (#28: the pieces fly back on bar 34 beat 2)
SERIES = 'BALLFIGHT #2'

MOOD = 'balkan_fire'
KEY, SCALE = 'D', 'harmonic minor'
LOOP = ['Dm', 'Dm', 'Gm', 'A7']
CHORDS = ['A7'] + LOOP * 8 + ['Dm', 'Gm', 'A7']           # the count-in on the dominant; bar 35 builds into the "3"
ROLES = {'DRUMS': 'kit', 'VIOLIN': 'melody'}
LINES = {
    # the VIOLIN's line, one note per 16th over the 4-bar loop: the bow's quarter turns sample it
    'VIOLIN': [74, 74, 73, 74, 76, 76, 77, 77, 76, 76, 74, 74, 73, 73, 70, 70,     # Dm
               69, 69, 70, 69, 67, 67, 69, 69, 70, 70, 73, 73, 74, 74, 76, 76,     # Dm
               77, 77, 79, 79, 77, 77, 74, 74, 82, 82, 81, 81, 79, 79, 77, 77,     # Gm
               76, 76, 77, 76, 73, 73, 76, 76, 79, 79, 81, 81, 82, 82, 85, 85]}    # A7
SOLO = {
    'VIOLIN': [(0, 74, .25), (.25, 76, .25), (.5, 77, .25), (.75, 79, .25), (1, 81, .5), (1.5, 82, .25), (1.75, 85, .25),
               (2, 86, .75), (2.75, 85, .25), (3, 82, .5), (3.5, 81, .5), (4, 79, .25), (4.25, 81, .25), (4.5, 79, .25),
               (4.75, 77, .25), (5, 76, .5), (5.5, 73, .5), (6, 74, .25), (6.25, 76, .25), (6.5, 74, .25), (6.75, 76, .25),
               (7, 77, .5), (7.5, 76, .25), (7.75, 73, .25), (8, 70, .5), (8.5, 69, .5), (9, 73, .25), (9.25, 76, .25),
               (9.5, 79, .25), (9.75, 82, .25), (10, 85, .5), (10.5, 81, .5), (11, 74, 1.0)],
    # DRUMS: kit/fighters/drums.py's DEFAULT_SOLO (#28's), in drum pieces
}
CAP_TARGETS = {'DRUM ROLL': 10, 'PIZZICATO': 15, 'CRASH CYMBAL': 22, 'TREMOLO': 28, 'DOUBLE KICK': 40, 'SCREECH': 40}

CAPTION = 'Comment the next match-up 👇🥁🎻'
