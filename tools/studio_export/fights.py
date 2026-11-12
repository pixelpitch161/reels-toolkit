"""fights: the reference fights of the example episode, recorded by the Python kit.

studio/reference/fights/flute_vs_trombone/{ep392,seed1,seed2,seed3,seed500}.bfrec: story.run_seed(seed) for the
episode's own seed (392) and four others, each with the episode's rules. Every file is read back and compared with
the run it came from, value for value and type for type, and the kit's replay-guard hash is computed a second time
from the file alone: it must equal the hash of the live run, so the file holds everything the guard covers."""
import hashlib
import os

import numpy as np

from . import common, manifest, recording

EPISODE = 'flute_vs_trombone'
OTHER_SEEDS = (1, 2, 3, 500)
DOC = os.path.join(common.REPO, 'studio', 'docs', 'recording-format.md')


def replay_hash(r):
    """the replay guard's hash of a run dict: the same lines as kit/story.py replay_hash and
    tools/ballfight_replay_hash.py"""
    h = hashlib.sha256(repr(r['events']).encode())
    for k in sorted(r):
        v = r[k]
        per_frame = isinstance(v, list) and v and isinstance(v[0], (list, np.ndarray))
        if per_frame and k not in ('events', 'names', 'corner'):
            try:
                h.update(np.ascontiguousarray(np.array(v, float)).tobytes())
            except (ValueError, TypeError):
                h.update(repr(v).encode())
        else:
            h.update(repr(v).encode())
    return h.hexdigest()[:16]


def plain_scalars(x):
    """a copy of a run value with every numpy scalar replaced by the Python number of the same value: a numpy
    float by the float of its exact value (float64: the same bits; float32: widened exactly, as the recording stores
    it), a numpy integer by an int, a numpy bool by a bool. Tuples stay tuples, lists lists, dicts dicts, arrays
    arrays. Anything else numpy (a complex, a string) stops with its place"""
    if isinstance(x, np.ndarray):
        return x
    if isinstance(x, np.generic):
        if isinstance(x, np.bool_):
            return bool(x)
        if isinstance(x, np.integer):
            return int(x)
        if isinstance(x, np.floating):
            y = float(x)
            assert type(x)(y).tobytes() == x.tobytes(), (x, y)    # the value, to the bit
            return y
        raise TypeError(f'a numpy {type(x).__name__} in a run: a recording has no such value')
    if isinstance(x, tuple):
        return tuple(plain_scalars(v) for v in x)
    if isinstance(x, list):
        return [plain_scalars(v) for v in x]
    if isinstance(x, dict):
        return {k: plain_scalars(v) for k, v in x.items()}
    return x


def replay_hash_of_values(r):
    """replay_hash over the run's values rather than their Python types: replay_hash(plain_scalars(r)). It equals
    replay_hash(r) for every run without a numpy scalar in it (FLUTE, TROMBONE and the finished episodes); a kit run
    that leaves a numpy float in an event (ORGAN's `line` event, whose rope length `fight.py`'s `_to_ropes` returns
    as a numpy float when a rope cuts the row short) reprs it as `np.float64(...)`, a word no file of one float type
    can carry, so the guard that a recording holds everything compares this hash, of the same bits, with the file's.
    The kit itself is never patched for it"""
    return replay_hash(plain_scalars(r))


# how deep the kit's per-entry tuples sit under each ragged record key (a list a frame of tuples: 2; HOLD is PIANO's)
RAGGED_DEPTH = dict(WAVES=2, SLAMS=2, COB=3, HOLD=2, SPOT=2)


def as_kit_run(rec):
    """a run dict shaped like the kit's, rebuilt from a recording that was read back: events and the entries of the
    ragged keys are tuples again, arrays are lists of frames"""
    def tuples(x, depth):
        return tuple(x) if depth == 0 else [tuples(v, depth - 1) for v in x]

    def event(e):                                         # the kit's events are tuples, and so is any sequence in
        return tuple(event(v) if isinstance(v, list) else v for v in e)   # one (PIANO's pitches in roll, runoff)
    r = dict(seed=rec['seed'], winner=rec['winner'], t_ko=rec['t_ko'], names=rec['names'], corner=rec['corner'],
             f0=rec['f0'], events=[event(e) for e in rec['events']])
    for k, a in rec['arrays'].items():
        if isinstance(a, np.ndarray):
            r[k] = list(a)
        elif k in RAGGED_DEPTH:
            r[k] = tuples(a, RAGGED_DEPTH[k])
        else:
            raise SystemExit(f"fights.as_kit_run: the ragged record key {k!r} has no entry in fights.RAGGED_DEPTH (how "
                             f"deep the kit's per-entry tuples sit under it): add it there")
    return r


def check(run, rules, rec, rect_keys):
    """the recording read back from disk says exactly what the run said"""
    same = common.same
    for k in ('seed', 'winner', 't_ko', 'f0', 'names', 'corner'):
        same(run[k], rec[k], k)
    same([list(e) for e in run['events']], rec['events'], 'events')
    same(common.plain(dict(vars(rules))), rec['rules'], 'rules')
    assert rec['fps'] == rules.FPS and rec['format'] == recording.FORMAT
    keys = [k for k in run if k not in recording.META_KEYS]
    assert sorted(keys) == sorted(rec['arrays']), (sorted(keys), sorted(rec['arrays']))
    for k in keys:
        a = rec['arrays'][k]
        if isinstance(a, np.ndarray):
            assert k in rect_keys
            src = np.array(run[k])
            assert src.dtype.str == a.dtype.str and src.shape == a.shape and src.tobytes() == a.tobytes(), k
        else:
            same(common.plain(run[k]), a, k)


def run():
    ep = common.episode(EPISODE, 'fights')
    _, story = common.kit()
    T = ep.T
    assert ep.seed == 392 and tuple(T.FIGHTERS) == ('FLUTE', 'TROMBONE'), (ep.seed, T.FIGHTERS)
    rules = story.rules()
    hashes = {}
    for seed in (ep.seed,) + OTHER_SEEDS:
        name = f'ep{seed}' if seed == ep.seed else f'seed{seed}'
        out = common.ref('fights', EPISODE, f'{name}.bfrec')
        r = story.run_seed(seed)
        recording.write_bfrec(r, rules, out, rect_keys=story.REC_KEYS, register=False)   # listed once it checks out
        rec = recording.read_bfrec(out)
        check(r, rules, rec, story.REC_KEYS)
        guard, guard_file = replay_hash(r), replay_hash(as_kit_run(rec))
        line = story.replay_hash(seed)                    # the kit's own line, from a run of its own
        assert line == f"{seed} {r['winner']} {r['t_ko']} {len(r['events'])} {guard}", (line, guard)
        assert guard_file == guard, (seed, guard, guard_file)
        manifest.add(out)
        hashes[name] = recording.rec_hash(rec)
        frames = len(rec['arrays']['HP'])
        print(f"fights: {common.rel(out)}  {os.path.getsize(out)} bytes  frames {frames}  hash {hashes[name]}")
        print(f"        replay guard: {line}  (the same from the file alone: {guard_file == guard})")
    if ep.seed == 392:                                    # what the brief and the replay guard say about this fight
        r = recording.read_bfrec(common.ref('fights', EPISODE, 'ep392.bfrec'))
        assert (r['winner'], r['t_ko'], len(r['events'])) == (1, 50.021568627450975, 522), 'seed 392 changed'
    # the format document states these hashes: say so if it has fallen behind
    text = open(DOC, encoding='utf-8').read() if os.path.exists(DOC) else ''
    stale = [f'{n} {h}' for n, h in hashes.items() if h not in text]
    if stale:
        raise SystemExit(f"fights: {common.rel(DOC)} does not state these hashes, update it: {', '.join(stale)}")


EXPORTS = {'fights': run}
