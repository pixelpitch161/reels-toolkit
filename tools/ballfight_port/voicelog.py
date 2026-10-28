"""log pp_bridge.voice calls (cache misses) in order: python voicelog.py ep|kit <folder-or-example> <out.txt>"""
import os, sys, importlib.util
mode, name, out = sys.argv[1], sys.argv[2], sys.argv[3]
LOG = []
def patch(B):
    _v = B.voice
    def voice(nm, midi, vel, dur):
        LOG.append((nm, midi, vel, dur))
        return _v(nm, midi, vel, dur)
    B.voice = voice
from reelkit import audio as A
if mode == 'ep':
    folder = os.path.abspath(name)
    sys.path.insert(0, folder); os.chdir(folder)
    A.write = lambda p, x: None
    A.section_levels = lambda p, spans: []
    spec = importlib.util.spec_from_file_location('audio_ep', os.path.join(folder, 'audio.py'))
    AU = importlib.util.module_from_spec(spec); spec.loader.exec_module(AU)
    patch(AU.B)
    AU.main()
else:
    sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'projects'))
    from ballfight.kit import episode
    episode.load(name)
    from ballfight.kit import picture
    import pp_bridge as B
    patch(B)
    from ballfight.kit import music as M
    M.score()
with open(out, 'w') as f:
    for r in LOG:
        f.write(' '.join(map(repr, r)) + '\n')
print(len(LOG))
