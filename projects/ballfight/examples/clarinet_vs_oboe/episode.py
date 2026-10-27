"""#37 BALLFIGHT #11: Clarinet vs Oboe, rebuilt as an episode-kit config (the worked example of new fighters' looks and
sounds gathered into kit/fighters/; the bed is kit/moods/baroque_storm.py). Delivered from
projects/ballfight_clarinet_vs_oboe/ (left as delivered); this config replays the same fight (seed 654).

The reed war, single reed vs double reed: CLARINET (left, lavender), the only teleporter (TWELFTH JUMP), vs OBOE
(right, olive), the only debuffer (TUNING A knocks the enemy OUT OF TUNE). Music, mood A: Baroque storm in D minor;
the clarinet plays the lower, liquid line, the oboe the higher, nasal one (its A every bar, 8ths as it stings)."""

FIGHTERS = ('CLARINET', 'OBOE')
SEED = 654                               # seed_search 1..3000 --stop 8: 8 fits in 654 runs, ranked first (7.87)
DMG_SCALE = 0.55                         # tuned once (200 seeds: CLARINET 47 % / OBOE 53 %, median K.O. 50.8 s)

BPM = 140                                # loop_bpm -> 141.64: 36 bars, FIGHT! at 3.39 s
SERIES = 'BALLFIGHT #11'                 # (#37 predates the no-tag rule)
SHARD_SEED, FX_SEED = 371, 37

MOOD = 'baroque_storm'
KEY, SCALE = 'D', 'minor'                # the oboe's A is the dominant: its tuning note IS the storm's pedal
LOOP = ['Dm', 'Gm', 'C', 'F', 'Bb', 'Gm', 'A', 'A']        # circle of fifths
CHORDS = ['Dm', 'A'] + LOOP * 4 + ['Gm', 'A']              # the last two bars fall into the title slam's Dm
ROLES = {'CLARINET': 'line', 'OBOE': 'line'}
LINES = {
    # the chalumeau line, one note per beat (every jump plays the slot it lands on), D3-E4
    'CLARINET': [62, 57, 53, 57, 58, 55, 50, 55, 60, 55, 52, 55, 57, 53, 48, 53,
                 58, 53, 50, 53, 55, 58, 62, 58, 57, 61, 64, 61, 57, 52, 49, 52],
    # the oboe's running 8ths (the stings while the enemy is out of tune), A4-F6
    'OBOE': [74, 77, 81, 77, 74, 76, 77, 81, 79, 82, 86, 82, 79, 77, 74, 77, 76, 79, 84, 79, 76, 77, 79, 76,
             77, 81, 84, 81, 77, 79, 81, 77, 77, 82, 86, 82, 77, 81, 82, 86, 79, 82, 86, 84, 82, 81, 79, 77,
             76, 81, 85, 81, 76, 79, 81, 85, 81, 79, 77, 76, 73, 76, 79, 81]}
SOLO = {
    # a chalumeau run that leaps a twelfth (the register key) and tumbles home
    'CLARINET': [(0, 50, .5), (.5, 53, .5), (1, 57, .5), (1.5, 62, .5), (2, 74, 1.0), (3, 72, .25), (3.25, 70, .25),
                 (3.5, 69, .25), (3.75, 67, .25), (4, 65, .5), (4.5, 64, .5), (5, 61, .5), (5.5, 64, .5),
                 (6, 62, 2.0)],
    # the A, then a nasal descending sequence over the storm, resolving on D
    'OBOE': [(0, 81, 1.0), (1, 82, .5), (1.5, 81, .5), (2, 79, .5), (2.5, 77, .5), (3, 76, .5), (3.5, 77, .5),
             (4, 74, .5), (4.5, 76, .5), (5, 73, .5), (5.5, 76, .5), (6, 74, 2.0)]}

CAPTION = 'Single reed vs double reed 🎵 Comment the next match-up 👇'
