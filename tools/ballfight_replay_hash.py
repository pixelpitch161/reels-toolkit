"""BALLFIGHT parallel-production guard: hash honest runs of every finished episode (events + every recorded array) so an
engine change can be checked for bit-identical replays before it is pushed.

    python tools/ballfight_replay_hash.py                  # print the hashes
    python tools/ballfight_replay_hash.py > before.txt     # ... change fight.py / fighters.py ...
    python tools/ballfight_replay_hash.py | diff before.txt -

Each episode runs in its own interpreter (the episodes' modules share names: timeline, sim). Add an episode to
EPISODES once it is delivered: (folder, [its chosen seed + a few others]); episode-kit episodes go in KIT_EPISODES."""
import os
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
EPISODES = [
    ('projects/ballfight', [88, 1, 2, 3, 500]),                    # #27, seed 88
    ('projects/ballfight_drums_vs_violin', [568, 1, 2, 3, 500]),   # #28, seed 568
    ('projects/ballfight_tuba_vs_flute', [514, 1, 2, 3, 500]),     # #32 v2, seed 514
    ('projects/ballfight_trumpet_vs_sax', [191, 1, 2, 3, 500]),    # #29 v3, seed 191
    ('projects/ballfight_bagpipes_vs_accordion', [1442, 1, 2, 3, 500]),   # #30 v3 (intro test: fight at 2 bars), seed 1442
    ('projects/ballfight_trombone_vs_horn', [140, 1, 2, 3, 500]),   # #34 v5 (HORN v3 ECHO CALL, fight at 2 bars), seed 140
    ('projects/ballfight_turntables_vs_synth', [1339, 1, 2, 3, 500]),   # #31 v3 (VS title card, fight at 2 bars), seed 1339
    ('projects/ballfight_organ_vs_harp', [237, 1, 2, 3, 500]),    # #35 v3, seed 237 (fight at 2 bars)
    ('projects/ballfight_ukulele_vs_acoustic', [1130, 1, 2, 3, 500]),   # #33 v3 (fight at 2 bars), seed 1130
    ('projects/ballfight_trombone_vs_tuba', [358, 1, 2, 3, 500]),   # #36 (first crossover, dmg_side handicap), seed 358
    ('projects/ballfight_clarinet_vs_oboe', [654, 1, 2, 3, 500]),   # #37 (jump / tune + detune / stun / snip), seed 654
    ('projects/ballfight_bassclarinet_vs_euphonium', [1269, 1, 2, 3, 500]),   # #38 (anchor / mortar / barrage / twin / copy / lights_out), seed 1269
    ('projects/ballfight_bassoon_vs_barisax/v1', [190, 1, 2, 3, 500]),   # #39 v1 snapshot (tiptoe / hook / mountain, rattle / offbeat / quake), seed 190
    ('projects/ballfight_bassoon_vs_barisax/v2', [27, 1, 2, 3, 500]),   # #39 v2 snapshot (wrecking / snap, offbeat swings; no swing over the ropes), seed 27
    ('projects/ballfight_bassoon_vs_barisax/v3', [95, 1, 2, 3, 500]),   # #39 v3 snapshot (the swing out over the ropes, no 'steer'; the bari's hits x0.8), seed 95
    ('projects/ballfight_bassoon_vs_barisax', [77, 1, 2, 3, 500]),   # #39 v4 (a full swing near the corners too: 'steer'), seed 77
    ('projects/ballfight_barisax_vs_bassclarinet', [128, 1, 2, 3, 500]),   # #40 v2 (crossover, sides swapped; the bari swings out over the ropes), seed 128 (v1: 617, commit 8115d67)
    ('projects/ballfight_flute_vs_trombone', [392, 1, 2, 3, 500]),   # #41 (crossover, sides swapped; Rules(swat_cobras=True): the lance swats cobras), seed 392
    ('projects/ballfight_viola_vs_violin', [8219, 1, 2, 3, 500]),   # #42 (rests / double_stop / melody / joke; VIOLA vs the #28 VIOLIN), seed 8219
]
# Episode-kit configs (python -m ballfight hash): a config that rebuilds a delivered episode prints that episode's lines.
# Add every new kit episode once it is delivered: (its folder, [its SEED + a few others]).
KIT_EPISODES = [
    ('projects/ballfight/examples/flute_vs_trombone', [392, 1, 2, 3, 500]),   # #41 rebuilt as a kit config
    ('projects/ballfight/examples/clarinet_vs_oboe', [654, 1, 2, 3, 500]),    # #37 rebuilt as a kit config
    ('projects/ballfight/examples/ukulele_vs_acoustic', [1130, 1, 2, 3, 500]),   # #33 v3 (the ACOUSTIC port's check)
]
CODE = r'''
import hashlib, sys
import numpy as np
import sim
for s in map(int, sys.argv[1:]):
    r = sim.run_seed(s)
    h = hashlib.sha256(repr(r['events']).encode())
    for k in sorted(r):
        v = r[k]
        if isinstance(v, list) and v and isinstance(v[0], (list, np.ndarray)) and k not in ('events', 'names', 'corner'):
            try:
                h.update(np.ascontiguousarray(np.array(v, float)).tobytes())
            except (ValueError, TypeError):
                h.update(repr(v).encode())
        else:
            h.update(repr(v).encode())
    print(s, r['winner'], r['t_ko'], len(r['events']), h.hexdigest()[:16])
'''

if __name__ == '__main__':
    for folder, seeds in EPISODES:
        d = os.path.join(ROOT, folder)
        out = subprocess.run([sys.executable, '-c', CODE] + [str(s) for s in seeds], cwd=d, capture_output=True,
                             text=True)
        if out.returncode:
            print(folder, 'FAILED\n', out.stderr)
            continue
        for line in out.stdout.strip().split('\n'):
            print(folder, line)
    for folder, seeds in KIT_EPISODES:
        d = os.path.join(ROOT, folder)
        out = subprocess.run([sys.executable, '-m', 'ballfight', 'hash', d] + [str(s) for s in seeds],
                             cwd=os.path.join(ROOT, 'projects'), capture_output=True, text=True)
        if out.returncode:
            print(folder, 'FAILED\n', out.stderr)
            continue
        for line in out.stdout.strip().split('\n'):
            if line.startswith(d + ' '):
                print(folder, line[len(d) + 1:])
