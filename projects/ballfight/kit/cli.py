"""python -m ballfight <step> <episode> [args]       (from projects/; or python projects/ballfight <step> ... anywhere)

<episode> is an episode folder holding episode.py: a path, a folder under projects/ (ballfight_harp_vs_tuba), the
part after ballfight_ (harp_vs_tuba), or a kit example (flute_vs_trombone). One step at a time, in this order:

    new      <a>_vs_<b> A B [--mood big_top] [--bpm 120]   scaffold projects/ballfight_<a>_vs_<b>/episode.py
    preview  FIGHTER [--vs B] [--seconds 7]       a new fighter's signature + desperation clip, for the user's OK
    tune     <ep> N [A=h1,h2 B=h] [scale=..] [A.weapon.field=..]   balance grid over N seeds (then freeze in the config)
    search   <ep> [A B] [--stop 5]                early-stopping seed search -> check/seeds.txt (then set SEED)
    sim      <ep> [seed]                          the run's story
    sheet    <ep> [t ...]                         contact sheet -> test/sheet.png (default: the run's key moments)
    audio    <ep>                                 audio.wav + check/stem_*.wav
    check    <ep> [--fast]                        safe zone, focus, levels, band, loudness, flash, loop (before render)
    render   <ep> [--frames A:B] [--only-mux]     the ONE full render + loop-clean encode -> <ep>/<name>.mp4
    verify   <ep>                                 ffprobe, loop + flash on the mp4, an mp4 contact sheet -> test/
    post     <ep>                                 the post caption and tags
    hash     <ep> [seeds]                         replay-guard hash lines (tools/ballfight_replay_hash.py format)
Every step pins PYTHONHASHSEED=0 (reelkit.ui's caption puff uses Python's salted str hash: without it two renders of
the same frame differ by a few pixels, so --frames patches would not match the full render)."""
import os
import sys
import time

from . import episode as E

NEW = '''"""{title}: {a} vs {b} (episode kit config: projects/ballfight/README.md, "Making an episode").
Idea + hook: (one or two lines).
Music: mood {mood}; who plays what (ROLES), the lines, the solos."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ({qa}, {qb})       # first named = left corner (screen left)
SEED = None                      # python -m ballfight search {slug} -> the best fit
DMG_SCALE = 1.0
DMG_SIDE = None                  # a crossover's only lever: (left, right) damage handicap from `tune`, frozen here
RULES = dict()                   # opt-in engine rules (e.g. swat_cobras=True)

# ---------------------------------------------------------------- tempo + music
BPM = {bpm}                        # loop_bpm makes it a whole number of bars in 61 s
MOOD = '{mood}'
KEY, SCALE = 'E', 'harmonic minor'
LOOP = ['Em', 'Am', 'B7', 'Em', 'C', 'Am', 'B7', 'B7']
CHORDS = {chords}
ROLES = {{{qa}: 'melody', {qb}: 'bass'}}
LINES = {{}}                       # {{fighter: 8 notes a bar over the LOOP}}; a fighter without one gets a sketch line
SOLO = {{}}                        # {{fighter: [(beat, midi, beats)]}}; default: a 2-bar sketch on the tonic
TITLE_HITS = {{}}                  # {{fighter: notes it plays as its name slams in}}

# ---------------------------------------------------------------- the post
CAPTION = 'Comment the next match-up 👇'
TAGS = {{}}
'''


def _load(name):
    ep = E.load(name)
    return ep


def _times(P):
    """the run's key moments: the card, the count, the first exchanges, each caption, the desperation moves, the K.O.,
    the solo, the reassembly, the seam"""
    T = P.T
    ts = [0.0, 1.2, T.BAR + 0.6, T.T_FIGHT + 4.0]
    ts += [t + 0.1 for t, w, n in P.STORY['caps']]
    lows = [P.CFG[k]['abilities']['low']['name'] for k in (0, 1)]
    ts += [t + 0.15 for t, w, n in P.STORY['firsts'] if n in lows]
    ts += [P.T_KO + 0.08, P.T_SOLO + 0.5, T.T_REASSEMBLE + 0.6, T.DUR - 0.2]
    return sorted({round(t, 2) for t in ts})


def main(argv):
    if not argv or argv[0] in ('-h', '--help', 'help'):
        print(__doc__)
        return 0
    step, args = argv[0], argv[1:]
    t0 = time.time()
    if step == 'new':
        import argparse
        ap = argparse.ArgumentParser(prog='ballfight new')
        ap.add_argument('slug')
        ap.add_argument('a')
        ap.add_argument('b')
        ap.add_argument('--mood', default='big_top')
        ap.add_argument('--bpm', type=int, default=120)
        a = ap.parse_args(args)
        from reelkit.config import loop_bpm
        bars = int(round(61.0 / (240.0 / loop_bpm(a.bpm, 61.0))))
        n_loop = (bars - 2) // 8
        tail = bars - 2 - 8 * n_loop
        chords = "['Em', 'B7'] + LOOP * %d" % n_loop + (" + LOOP[:%d]" % tail if tail else '')
        folder = os.path.join(E.PROJECTS, 'ballfight_' + a.slug)
        os.makedirs(folder, exist_ok=True)
        path = os.path.join(folder, 'episode.py')
        if os.path.exists(path):
            raise SystemExit(f'{path} exists')
        with open(path, 'w') as f:
            f.write(NEW.format(title='BALLFIGHT', a=a.a.upper(), b=a.b.upper(), qa=repr(a.a.upper()),
                               qb=repr(a.b.upper()), slug=a.slug, bpm=a.bpm, mood=a.mood, chords=chords))
        print(f'wrote {path} ({bars} bars at loop_bpm({a.bpm}))')
        return 0
    if step == 'preview':
        import argparse
        ap = argparse.ArgumentParser(prog='ballfight preview')
        ap.add_argument('fighter')
        ap.add_argument('--vs', default=None)
        ap.add_argument('--seconds', type=float, default=7.0)
        ap.add_argument('--no-sound', action='store_true')
        a = ap.parse_args(args)
        from . import preview
        preview.main(a.fighter, a.vs, a.seconds, not a.no_sound)
        return 0
    if not args:
        raise SystemExit(f'ballfight {step}: name the episode (see python -m ballfight help)')
    ep = _load(args[0])
    rest = args[1:]
    T = ep.T
    code = 0
    if step == 'tune':
        from . import tuning
        tuning.main(int(rest[0]) if rest else 60, rest[1:])
    elif step == 'search':
        import argparse
        from . import search
        ap = argparse.ArgumentParser(prog='ballfight search')
        ap.add_argument('a', type=int, nargs='?', default=1)
        ap.add_argument('b', type=int, nargs='?', default=4000)
        ap.add_argument('--stop', type=int, default=5)
        a = ap.parse_args(rest)
        search.main(a.a, a.b, a.stop)
    elif step == 'sim':
        from . import story
        run = story.load(int(rest[0])) if rest else story.load()
        print(story.summary(run))
    elif step == 'hash':
        from . import story
        for s in ([int(x) for x in rest] or [ep.seed, 1, 2, 3, 500]):
            print(ep.dir, story.replay_hash(s))
    elif step == 'sheet':
        from . import picture as P
        from reelkit.sheet import sheet
        if not os.path.exists(P.SPR_PATH):
            P.build_sprites()
        times = [float(x) for x in rest] or _times(P)
        out = ep.path('test', 'sheet.png')
        os.makedirs(os.path.dirname(out), exist_ok=True)
        sheet(P, times, out)
        print('sheet:', out, ' times', ' '.join(f'{t:g}' for t in times))
    elif step == 'audio':
        from . import music
        music.main()
    elif step == 'check':
        from . import picture as P
        from . import checks
        if not os.path.exists(P.SPR_PATH):
            P.build_sprites()
        if not os.path.exists(ep.path('audio.wav')):
            from . import music
            music.main()
        res = {}
        res['safezone'] = checks.safezone(step=0.5 if '--fast' in rest else 0.25)
        res['focus'] = checks.focus()
        res['levels'] = checks.levels()
        res['band'] = checks.band()
        checks.loudness()
        res['flash'] = checks.flash()
        res['loop'] = checks.loop()
        print('checks:', '  '.join(f'{k} {"PASS" if v else "FAIL"}' for k, v in res.items()))
        code = 0 if all(res.values()) else 1
    elif step == 'render':
        from . import picture as P
        from . import encode
        if not os.path.exists(P.SPR_PATH):
            P.build_sprites()
        if not os.path.exists(ep.path('audio.wav')):
            from . import music
            music.main()
        frames = None
        if '--frames' in rest:
            from reelkit.video import _parse_frames
            frames = _parse_frames(rest[rest.index('--frames') + 1])
        encode.render(frames=frames, only_mux='--only-mux' in rest)
    elif step == 'verify':
        from . import checks
        from . import picture as P
        from reelkit.video import frames_sheet
        ok = checks.verify(ep.out)
        times = _times(P)
        out = ep.path('test', 'mp4_sheet.png')
        os.makedirs(os.path.dirname(out), exist_ok=True)
        frames_sheet(ep.out, times, out)
        print('mp4 sheet:', out)
        code = 0 if ok else 1
    elif step == 'post':
        print(T.CAPTION)
        for k, v in T.TAGS.items():
            print(f'{k}: {v if isinstance(v, str) else chr(10).join(v)}')
    else:
        raise SystemExit(f'unknown step {step!r} (see python -m ballfight help)')
    print(f'[{step} {ep.name}: {time.time() - t0:.0f} s]')
    return code
