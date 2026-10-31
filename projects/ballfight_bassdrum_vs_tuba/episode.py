"""#56 BALLFIGHT: Bass Drum vs Tuba (episode kit config: projects/ballfight/README.md, "Making an episode").
Idea + hook: a viewer request ("Base drum vs tuba", twice): who owns the BOOM - the marching band's two biggest low
sounds. BASS DRUM (left, gunmetal), new: the only roller - STEAMROLLER (every bar it tips onto its side and rolls across
the ring like a wheel at the tuba, a dust trail telegraphing the line, every impact a huge BOOM; roll speed and drum
size grow per landed boom), HEARTBEAT (two quick booms, lub-dub), DRUM ROLL (rolls nonstop for a bar, zig-zagging),
1812 OVERTURE (five cannon booms land across the ring). The bass drum model is new, built from Commons photos found with
Claude in Chrome (bassdrum_lowpoly). TUBA (right, indigo) exactly as built (#32 v3 / #36, ported into the kit): OOM-PAH
SWALLOW, OOMPAH, BRASS WALL, SUBSONIC; the only per-episode lever is the damage handicap. Swallowing a rolling drum
wasn't defined: opt-in Rules(swallow_rollers=True) here - the bell can gulp the drum mid-roll, which ends the roll.
No episode number on screen.
Music: mood A, dark drumline (kit/moods/dark_drumline.py) with its own bass drums off (DRUMLINE): snares, quads,
cymbals and the pit stay; the BASS DRUM is the kick and the booms (a kick on every beat on the root, a roll into every
boom), the TUBA the bass line (LINES: oom = the root, pah = the fifth, as in #32). 1812 OVERTURE fires its cannons as
the opening of "God Save the Tsar!" (Lvov, 1833), the anthem Tchaikovsky's 1812 Overture (1880) sets under the cannons
in its finale (both public domain), in E-flat - the 1812's own key, the relative major of the episode's C minor - with
the pit doubling it and tubular bells ringing; nothing else quoted. C minor at 125.90 BPM."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('BASS DRUM', 'TUBA')         # first named = left corner (screen left)
SEED = 132                               # search 1..341 at 2 bars (8 fits, 58 % / 42 % decided): ranked first (7.59):
                                         # the drum leads, 1812 OVERTURE at 42.4 s, the tuba comes back (lead changes 43.2 /
                                         # 48.1 s) and wins at 49.84 s on 0.1 HP
DMG_SCALE = 0.66                         # tuned once (300 seeds x 6 cells): BASS DRUM 58 % of 251 decided, median K.O.
                                         # 53.7 s, fit rate 3.0 % (the best cell); frozen
DMG_SIDE = (0.86, 1.0)                   # the crossover lever (TUBA as built): (left, right), from `tune`, frozen
RULES = dict(swallow_rollers=True)       # #56 opt-in: the bell can gulp a rolling drum (the gulp ends the roll)

# ---------------------------------------------------------------- tempo + music
BPM = 126                                # loop_bpm -> 125.90: 32 bars in 61 s, bar 1.906 s, FIGHT! at 3.81 s
MOOD = 'dark_drumline'
DRUMLINE = dict(bass_drums=False, after_ko=True)   # the fighter IS the bass drum: the bed's own bass drums are
                                                   # off in the fight, back from the K.O. on (it lies in pieces)
KEY, SCALE = 'C', 'minor'
LOOP = ['Cm', 'Ab', 'Fm', 'G']
CHORDS = ['Cm', 'G'] + LOOP * 7 + ['Ab', 'G']   # the card on the tonic, the count-in on the dominant; the last bars turn
                                                 # round into the title slam's Cm (frame 0)
ROLES = {'BASS DRUM': 'drums', 'TUBA': 'bass'}
LINES = {
    # the TUBA's oom-pah bass (8ths over the LOOP: oom = root on the beat, pah = fifth on the off-beat, C2-G3)
    'TUBA': [36, 43, 36, 43, 36, 43, 36, 43,      # Cm:  C G
             44, 39, 44, 39, 44, 39, 44, 39,      # Ab:  Ab Eb
             41, 48, 41, 48, 41, 48, 41, 48,      # Fm:  F C
             43, 50, 43, 50, 43, 50, 47, 50]}     # G:   G D (B on the way home)
SOLO = {
    # the bass drum's victory cadence: booms on the beats, mallet taps between, rolls into the big ones
    # (beat, midi, beats); midi < 24 = a roll on midi + 24; a beat or longer = a boom
    'BASS DRUM': [(0, 36, 1), (1, 36, .5), (1.5, 36, .5), (2, 12, 1), (3, 36, 1), (4, 36, .5), (4.5, 36, .25),
                  (4.75, 36, .25), (5, 36, 1), (6, 12, 1.5), (7.5, 36, 1), (8.5, 36, .5), (9, 36, 1), (10, 12, 1),
                  (11, 36, .5), (11.5, 36, .5), (12, 36, 2)],
    # the tuba's strut: oom-pah up the C minor arpeggio, down through Bb and Ab, the dominant's B up to the C
    'TUBA': [(0, 36, .5), (.5, 43, .5), (1, 48, .5), (1.5, 43, .5), (2, 46, .5), (2.5, 44, .5), (3, 43, 1.0),
             (4, 41, .5), (4.5, 44, .5), (5, 47, .5), (5.5, 50, .5), (6, 48, 1.5), (7.5, 43, .5),
             (8, 36, .5), (8.5, 43, .5), (9, 44, .5), (9.5, 43, .5), (10, 41, .5), (10.5, 43, .5), (11, 47, .5),
             (11.5, 50, .5), (12, 48, 2.0)]}         # (a second bar of oom-pah to the reassembly)
MIX = {'solo': (2.7, 0.4)}               # the winner's solo + stinger +5 dB (kit 1.5): the fight's kick-every-beat and booms
                                         # make a loud floor, and the solo has the stage alone after the K.O. (levels check)
TITLE_HITS = {'BASS DRUM': (36,),                 # a huge boom on the low C
              'TUBA': (36, 43, 36, 43)}           # a tuba blast, then oom-pah

# ---------------------------------------------------------------- the post
CAPTION = 'Who owns the low end? 🥁📯 Comment the next match-up 👇'
TAGS = {'instagram': '#ballfight #bassdrum #tuba #marchingband #drumline #band #percussion #brass #musicmemes '
                     '#pixelart #lowpoly #satisfying',
        'tiktok': '#fyp #ballfight #bassdrum #tuba #marchingband #bandtok #drumline #musictok #pixelart #satisfying',
        'shorts': ('Bass Drum vs Tuba: who owns the BOOM?', 'The bass drum rolls like a steamroller, the tuba swallows '
                   'and spits - every hit in key over a dark drumline. Who wins?',
                   '#shorts #ballfight #bassdrum #tuba #marchingband #music #pixelart')}
