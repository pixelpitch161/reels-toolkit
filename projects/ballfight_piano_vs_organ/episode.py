"""#45 BALLFIGHT: Piano vs Organ (episode kit config: projects/ballfight/README.md, "Making an episode").
The same keyboard, two different beasts: the living room vs the cathedral, hammers on strings vs air through pipes; prog
rock's keyboard duel as a fight. The debut of PIANO v2 (PIANO ROLL: kit/fighters/piano.py, the engine's `roll` kind);
the ORGAN as built in #35 (kit/fighters/organ.py), on the right this time; the only per-episode lever is the damage
handicap (DMG_SIDE).
Music, mood A: prog-rock keyboard duel (kit/moods/prog_duel.py) in A minor at loop_bpm(132) = 133.77 BPM (34 bars):
a dramatic 70s-prog groove with 3+3+2 pushes inside 4/4 bars, big drums and a driving bass guitar, no keyboards of its
own. The PIANO plays the melody (the roll's rows ARE its line below: what you see is what you hear), the ORGAN the big
chords and the bass pedals (each pipe row = a chord stab + a pedal note); the leader fuller and louder."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('PIANO', 'ORGAN')            # first named = left corner (screen left)
SEED = 638                               # search 1..8000 --stop 8: 8 fits in 1,097 runs (PIANO 49 % of 1,023), ranked first (8.99)
DMG_SCALE = 1.0
DMG_SIDE = (1.0, 0.55)                   # the ORGAN's handicap (its only lever; the PIANO's new numbers are
                                         # frozen in the roster): tune, 300 seeds: 50 % / 50 %, median K.O. 51.0 s
RULES = dict()

# ---------------------------------------------------------------- tempo + music
BPM = 132                                # loop_bpm -> 133.77: 34 bars in 61 s, FIGHT! at 3.59 s
MOOD = 'prog_duel'
KEY, SCALE = 'A', 'minor'                # natural minor; over E7 the scale helpers raise G to G#
LOOP = ['Am', 'F', 'G', 'E7', 'Am', 'F', 'Dm', 'E7']
CHORDS = ['Am', 'E7'] + LOOP * 4         # the card on the tonic, the count-in on the dominant; the loop's last E7 falls
                                         # into the title slam's Am on frame 0
ROLES = {'PIANO': 'melody', 'ORGAN': 'bass'}
_ = None
LINES = {
    # the PIANO's line = the roll's rows (one per 8th, _ = a rest: no row). 3+3+2 accents (8ths 1, 4, 7) push across
    # the 4/4 bar; phrase A climbs to the E7, phrase B answers from the top and falls back to the loop's E.
    'PIANO': [76, _, 72, 81, _, 76, 72, 74,      # Am: E5 . C5 A5 . E5 C5 D5
              77, _, 72, 81, _, 77, 76, _,       # F:  F5 . C5 A5 . F5 E5 .
              79, _, 74, 83, _, 79, 77, 76,      # G:  G5 . D5 B5 . G5 F5 E5
              80, _, 76, 83, _, 80, 81, 83,      # E7: G#5 . E5 B5 . G#5 A5 B5
              84, _, 81, 76, _, 72, 76, 81,      # Am: C6 . A5 E5 . C5 E5 A5
              84, _, 81, 77, _, 72, 77, 81,      # F:  C6 . A5 F5 . C5 F5 A5
              81, _, 77, 74, _, 69, 74, 77,      # Dm: A5 . F5 D5 . A4 D5 F5
              83, _, 80, 76, _, 74, 71, _]}      # E7: B5 . G#5 E5 . D5 B4 .
SOLO = {
    # the piano's victory: a 16th arpeggio up the Am, a turn at the top, then the same climb over F
    'PIANO': [(0, 69, .25), (.25, 72, .25), (.5, 76, .25), (.75, 81, .25), (1, 84, .75), (1.75, 83, .25),
              (2, 81, .5), (2.5, 76, .25), (2.75, 79, .25), (3, 81, 1.0),
              (4, 77, .25), (4.25, 81, .25), (4.5, 84, .25), (4.75, 81, .25), (5, 77, .25), (5.25, 72, .25),
              (5.5, 77, .25), (5.75, 81, .25), (6, 84, .5), (6.5, 83, .5), (7, 81, 1.0)],
    # the organ's victory: a cathedral line, a held A, a falling run to the leading tone and home
    'ORGAN': [(0, 76, .5), (.5, 72, .25), (.75, 74, .25), (1, 76, .5), (1.5, 69, 1.5), (3, 77, .25), (3.25, 76, .25),
              (3.5, 74, .25), (3.75, 72, .25), (4, 71, .5), (4.5, 68, .5), (5, 69, 2.0), (7, 64, 1.0)],
}
TITLE_HITS = {'PIANO': (33, 45, 52, 57),                    # a hammered low A chord (A1 A2 E3 A3)
              'ORGAN': (33, 45, 52, 57, 60, 64, 69, 76)}    # the full organ at once, pedal to mixture

# ---------------------------------------------------------------- the post
CAPTION = 'Same keys, different monster 🎹⛪ Comment the next match-up 👇'
TAGS = {'instagram': '#ballfight #piano #organ #pipeorgan #progrock #keyboards #pianist #organist #pixelart #lowpoly '
                     '#satisfying #music',
        'tiktok': '#fyp #ballfight #piano #organ #progrock #pianotok #organtok #pixelart #satisfying #music',
        'shorts': ('Piano vs Organ: the keyboard duel', 'A player-piano roll that fires its own melody vs pipes that '
                   'erupt from the floor, every hit in key. Same keys, different monster.',
                   '#shorts #ballfight #piano #organ #progrock #pixelart #music')}
