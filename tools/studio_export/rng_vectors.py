"""rng: what numpy's default_rng gives, for the Studio's port of it (bf_core::rng).

studio/reference/rng/vectors.json: for each seed (ints, and the list seeds the engine makes mid-fight with
default_rng([seed, 44, round(t * 1000)])) a set of streams. EVERY stream starts from a fresh
np.random.default_rng(seed); only the stream named "interleaved" mixes methods on one generator. Plus one resume
case: the generator's state after 1,000 normals, and the normals that follow.

Layout: {"about", "format": 1, "numbers", "seeds": [{"seed", "init", "streams": {name: stream}}], "resume"}.
A stream: {"call" (the Python call, as text), "op", "args", "calls" (how many times it was called), "size" (only for
one array call), "values", "end" (the bit generator's state after the last call)}. Floats are float.hex() strings;
128-bit state and anything else that can pass 2**53 are decimal strings."""
import numpy as np

from . import common

OUT = common.ref('rng', 'vectors.json')
SEEDS = [0, 1, 2, 392, 2 ** 40 + 7, [392, 44, 3588], [1, 2, 3]]
BIG = 2 ** 53                 # an integer stream whose bound passes this is written as decimal strings


def state(rng):
    """the PCG64 state as numpy reports it: the 128-bit state and increment, and the buffered half of a 64-bit
    output that 32-bit draws leave behind (has_uint32, uinteger)"""
    s = rng.bit_generator.state
    assert s['bit_generator'] == 'PCG64', s['bit_generator']
    return dict(state=common.dec(s['state']['state']), inc=common.dec(s['state']['inc']),
                has_uint32=int(s['has_uint32']), uinteger=int(s['uinteger']))


def init(seed):
    """how the seed becomes a generator: the four 64-bit words SeedSequence hands PCG64, the state a fresh generator
    starts in, and its first raw 64-bit outputs"""
    words = np.random.SeedSequence(seed).generate_state(4, np.uint64)
    rng = np.random.default_rng(seed)
    start = state(rng)
    raw = rng.bit_generator.random_raw(8)
    return dict(seed_sequence_u64=[common.dec(w) for w in words], state=start['state'], inc=start['inc'],
                raw_u64=[common.dec(v) for v in raw])


def _floats(seed, call, op, args, calls, fn):
    rng = np.random.default_rng(seed)
    vals = []
    for _ in range(calls):
        v = fn(rng)
        assert type(v) is float, (call, type(v))
        vals.append(common.hexf(v))
    return dict(call=call, op=op, args=args, calls=calls, values=vals, end=state(rng))


def _float_array(seed, call, op, args, size, fn):
    rng = np.random.default_rng(seed)
    a = fn(rng)
    assert a.dtype == np.float64 and a.shape == (size,), (call, a.dtype, a.shape)
    return dict(call=call, op=op, args=args, calls=1, size=size, values=[common.hexf(v) for v in a.tolist()],
                end=state(rng))


def _integers(seed, high, calls=64):
    rng = np.random.default_rng(seed)
    vals = []
    for _ in range(calls):
        v = rng.integers(high)
        assert isinstance(v, np.int64) and 0 <= int(v) < high, (high, v)
        vals.append(int(v))
    big = high > BIG
    text = {2 ** 31 + 1: '2**31 + 1', 2 ** 32: '2**32', 2 ** 32 + 1: '2**32 + 1', 2 ** 40: '2**40',
            3 * 2 ** 61: '3 * 2**61'}.get(high, str(high))
    return dict(call=f'rng.integers({text})', op='integers', args=[common.dec(high) if big else high], calls=calls,
                values=[common.dec(v) for v in vals] if big else vals, end=state(rng))


def _permutation(seed, n):
    rng = np.random.default_rng(seed)
    p = rng.permutation(n)
    assert p.dtype == np.int64 and sorted(p.tolist()) == list(range(n)), (n, p)
    return dict(call=f'rng.permutation({n})', op='permutation', args=[n], calls=1, values=p.tolist(), end=state(rng))


def _choice(seed, items, calls=64):
    rng = np.random.default_rng(seed)
    vals = []
    for _ in range(calls):
        v = rng.choice(items)
        assert isinstance(v, np.int64) and int(v) in items, (items, v)
        vals.append(int(v))
    return dict(call=f'rng.choice({items})', op='choice', args=[items], calls=calls, values=vals,
                indices=[items.index(v) for v in vals], end=state(rng))


def _interleaved(seed, rounds=32):
    """one generator, five methods in turn: the 32-bit draws of integers and permutation leave a buffered half that
    the 64-bit draws between them must not disturb"""
    rng = np.random.default_rng(seed)
    rows = []
    for _ in range(rounds):
        u = rng.uniform(-1, 1)
        n = rng.normal(0, 1)
        r = rng.random()
        i = rng.integers(5)
        p = rng.permutation(4)
        assert type(u) is float and type(n) is float and type(r) is float and isinstance(i, np.int64)
        rows.append([common.hexf(u), common.hexf(n), common.hexf(r), int(i)] + p.tolist())
    return dict(call='each round, in this order: rng.uniform(-1, 1), rng.normal(0, 1), rng.random(), rng.integers(5), '
                     'rng.permutation(4)',
                op='interleaved', args=[], calls=rounds,
                row='[uniform, normal, random, integers, then the 4 values of the permutation]', values=rows,
                end=state(rng))


def streams(seed):
    s = {}
    s['random'] = _floats(seed, 'rng.random()', 'random', [], 64, lambda g: g.random())
    s['uniform_m1_1'] = _floats(seed, 'rng.uniform(-1, 1)', 'uniform', [-1, 1], 64, lambda g: g.uniform(-1, 1))
    s['uniform_5_9'] = _floats(seed, 'rng.uniform(5, 9)', 'uniform', [5, 9], 64, lambda g: g.uniform(5, 9))
    s['normal_0_1'] = _floats(seed, 'rng.normal(0, 1)', 'normal', [0, 1], 256, lambda g: g.normal(0, 1))
    s['normal_2_0p5'] = _floats(seed, 'rng.normal(2, 0.5)', 'normal', [2, 0.5], 64, lambda g: g.normal(2, 0.5))
    s['normal_0_1_size1000'] = _float_array(seed, 'rng.normal(0, 1, 1000)', 'normal', [0, 1], 1000,
                                            lambda g: g.normal(0, 1, 1000))
    s['uniform_0_1_size1000'] = _float_array(seed, 'rng.uniform(0, 1, 1000)', 'uniform', [0, 1], 1000,
                                             lambda g: g.uniform(0, 1, 1000))
    s['integers_7'] = _integers(seed, 7)
    s['integers_1000'] = _integers(seed, 1000)
    s['integers_2p40'] = _integers(seed, 2 ** 40)
    # the corners of numpy's bounded-integer code, beyond the three bounds above: a bound of 1 draws nothing; up to
    # 2**32 the draws are 32-bit (2**32 itself takes the raw 32 bits, 2**31 + 1 rejects about half its draws); above
    # that they are 64-bit (3 * 2**61 rejects about a quarter)
    s['integers_1'] = _integers(seed, 1)
    s['integers_2p31_plus_1'] = _integers(seed, 2 ** 31 + 1)
    s['integers_2p32'] = _integers(seed, 2 ** 32)
    s['integers_2p32_plus_1'] = _integers(seed, 2 ** 32 + 1)
    s['integers_3x2p61'] = _integers(seed, 3 * 2 ** 61)
    for n in (2, 5, 16, 100):
        s[f'permutation_{n}'] = _permutation(seed, n)
    s['choice_m1_1'] = _choice(seed, [-1, 1])
    s['choice_5'] = _choice(seed, [10, 20, 30, 40, 50])
    s['interleaved'] = _interleaved(seed)
    return s


def resume(seed=77, skip=1000, calls=64):
    rng = np.random.default_rng(seed)
    for _ in range(skip):
        rng.normal(0, 1)
    st = state(rng)
    assert st['has_uint32'] == 0, st                      # normals take whole 64-bit outputs: nothing is buffered
    vals = [common.hexf(rng.normal(0, 1)) for _ in range(calls)]
    return dict(seed=seed, skipped=dict(call='rng.normal(0, 1)', calls=skip), state=st['state'], inc=st['inc'],
                has_uint32=st['has_uint32'], uinteger=st['uinteger'],
                next=dict(call='rng.normal(0, 1)', op='normal', args=[0, 1], calls=calls, values=vals, end=state(rng)))


def run():
    doc = dict(
        about='numpy.random.default_rng reference streams for bf_core::rng. Every stream starts from a fresh '
              'np.random.default_rng(seed); only "interleaved" mixes methods on one generator. Made by '
              'tools/studio_export/rng_vectors.py.',
        format=1,
        numbers=dict(
            f64='a Python float.hex() string: [-]0x1.<13 hex digits>p<signed decimal exponent>; zero is 0x0.0p+0',
            int='a JSON integer where the bound is at most 2**53',
            decimal='a decimal string: the 128-bit "state" and "inc", the 64-bit words under "init", and the '
                    'bound and values of an integer stream whose bound is above 2**53',
            seed='a JSON integer is default_rng(seed); a list is the list seed default_rng([a, b, c])',
            end='the bit generator after the stream\'s last call: "state" and "inc" (PCG64, 128-bit each), and the '
                'half of a 64-bit output that an odd number of 32-bit draws leaves buffered ("has_uint32" 1 and '
                'the half in "uinteger")'),
        seeds=[dict(seed=seed, init=init(seed), streams=streams(seed)) for seed in SEEDS],
        resume=resume())
    common.write_json(OUT, doc)                           # written and listed in the manifest
    n = len(doc['seeds'][0]['streams'])
    print(f'rng: {common.rel(OUT)}  {len(SEEDS)} seeds x {n} streams, 1 resume case')


EXPORTS = {'rng': run}
