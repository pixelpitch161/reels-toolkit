"""pyhash: Python's hash(str) under PYTHONHASHSEED=0, for the Studio's port of it (bf_core::pyhash).

The kit's caption puff seeds itself with hash(line) % 997 (reelkit/ui.py), so frames depend on it.
studio/reference/pyhash/vectors.json: 500 ASCII strings with their hash: the empty string, every fighter name,
ability name and weapon name in fighters.py (the two retired fighters left out), the caption lines the renderer
draws (with and without their * marks) and the title card's words, and seeded random printable ASCII strings of
length 1 to 40. A few strings beyond ASCII follow under "non_ascii", because CPython hashes a string's internal
storage, not its UTF-8 bytes.

Layout: {"about", "format": 1, "numbers", "rows": [[string, hash, hash % 997], ...], "non_ascii": [the same]}.
The hash is a signed 64-bit integer as a decimal string; hash % 997 is Python's (0 to 996) as a JSON integer."""
import random

from . import common

OUT = common.ref('pyhash', 'vectors.json')
N = 500
SEED = 20261110
RETIRED = ('GUITAR', 'SAX')
# the caption lines of kit/picture.py, as written and as hashed (reelkit/ui.py strips the * marks first), then the
# words of the title card and the count-in
CAPTIONS = ['*K.O.*', 'K.O.', 'WINS!', 'COMMENT THE', '*NEXT MATCH-UP*', 'NEXT MATCH-UP', 'VS', '3', '2', '1',
            'FIGHT!', 'BALLFIGHT', 'BALLFIGHT #15']
NON_ASCII = ['é', 'naïve', 'FAÇADE', 'ÿ', 'Ā', 'aĀ', '→', 'K.O. →',
             '\U0001f447', 'COMMENT THE NEXT MATCH-UP \U0001f447', '\U0001fa88\U0001f3ba', 'a\U0001f447']


def roster_strings():
    """fighter, ability and weapon names from fighters.py, in its order, without the retired fighters"""
    out = []
    for name, cfg in common.roster().items():
        if name in RETIRED:
            continue
        out.append(name)
        out += [cfg['abilities'][slot]['name'] for slot in ('unlock1', 'unlock2', 'low')]
        if 'name' in cfg['weapon']:
            out.append(cfg['weapon']['name'])
    return out


def strings():
    out = ['']
    for s in roster_strings() + CAPTIONS:
        if s not in out:
            out.append(s)
    r = random.Random(SEED)                               # random() alone: the one stream Python guarantees
    while len(out) < N:
        n = 1 + int(r.random() * 40)
        s = ''.join(chr(0x20 + int(r.random() * 95)) for _ in range(n))
        if s not in out:
            out.append(s)
    assert len(out) == N and len(set(out)) == N
    assert all(s.isascii() for s in out)
    assert not any(s in RETIRED for s in out), 'a retired fighter must never be written'
    return out


def row(s):
    h = hash(s)
    assert -2 ** 63 <= h < 2 ** 63
    return [s, common.dec(h), h % 997]


def run():
    ss = strings()
    doc = dict(
        about='hash(s) of Python strings under PYTHONHASHSEED=0 (CPython 3.13: SipHash-1-3 with an all-zero key), '
              'for bf_core::pyhash. Made by tools/studio_export/pyhash_vectors.py.',
        format=1,
        numbers=dict(
            row='[string, hash, hash % 997]',
            hash='a signed 64-bit integer as a decimal string (it does not fit a JSON number safely)',
            mod997='Python\'s hash % 997, always 0 to 996 (the sign follows the divisor), as a JSON integer: what '
                   'reelkit/ui.py uses',
            storage='CPython hashes the bytes of the string\'s internal storage: one byte per character when every '
                    'character is below U+0100 (so ASCII hashes as its own bytes), else two bytes little-endian '
                    'when every character is below U+10000, else four bytes little-endian. The empty string '
                    'hashes to 0. "rows" are all ASCII; "non_ascii" has strings of each of the three widths'),
        rows=[row(s) for s in ss],
        non_ascii=[row(s) for s in NON_ASCII])
    common.write_json(OUT, doc)                           # written and listed in the manifest
    print(f"pyhash: {common.rel(OUT)}  {len(doc['rows'])} ASCII strings, {len(doc['non_ascii'])} beyond ASCII")


EXPORTS = {'pyhash': run}
