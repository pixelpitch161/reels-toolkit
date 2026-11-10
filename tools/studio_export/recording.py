"""The recording container (.bfrec), Python side: write a kit run, read one back, and its content hash.

    write_bfrec(run, rules, path)     run = kit story.run_seed(seed); rules = the Rules it was fought under;
                                      the file is listed in the manifest (register=False: written only)
    read_bfrec(path)                  -> dict(format, source, names, seed, winner, t_ko, f0, fps, corner, rules,
                                              events, arrays)
    rec_hash(rec)                     the 16 hex digits Recording::hash() gives in the Studio

studio/docs/recording-format.md is the contract; this module and the Rust reader both follow it. A .bfrec is a zip:
meta.json (every scalar field, the events, and which arrays there are), one <KEY>.npy per rectangular numeric
per-frame array and one <KEY>.json per ragged one. The same run always gives the same bytes: members in name order,
a fixed timestamp, one compression setting, no paths or times inside."""
import hashlib
import io
import json
import struct
import sys
import zipfile

import numpy as np
from numpy.lib import format as npy_format

from . import common, manifest

FORMAT = 1
HASH_TAG = b'BFREC-HASH-1\n'
META_KEYS = ('seed', 'winner', 't_ko', 'events', 'names', 'corner', 'f0')     # the run's keys that are not per-frame


# ---------------------------------------------------------------- write
def _array(key, v, frames, rect_keys):
    """one per-frame key as ('npy', bytes) or ('json', bytes). A key the kit itself turns into an array
    (story.REC_KEYS) is stored as .npy when it is rectangular and numeric; every other key (WAVES, SLAMS, COB, ...)
    is always JSON, so a key's kind does not depend on what happened in the fight"""
    if not isinstance(v, (list, np.ndarray)) or len(v) != frames:
        n = len(v) if hasattr(v, '__len__') else type(v).__name__
        raise ValueError(f'{key}: not a per-frame list of {frames} frames ({n}); format {FORMAT} has no place for it')
    if key in rect_keys:
        try:
            a = np.array(v)
        except ValueError:                                # ragged for this pair: the kit keeps the list, so do we
            a = None
        if a is not None and a.dtype in (np.float64, np.int64):
            return 'npy', common.npy_bytes(a.astype('<f8' if a.dtype == np.float64 else '<i8'))
    return 'json', common.dumps_rows(common.plain(list(v), key)).encode('ascii')


def write_bfrec(run, rules, path, source=None, rect_keys=None, register=True):
    """write the kit run `run` (the dict story.run_seed returns, recorded) and the rules it was fought under (a
    fight.Rules object or a dict of its attributes) to `path`, and list the file in the manifest under the calling
    module's name (`register=False`: write only, for a caller that checks the file before it calls manifest.add).
    `source` defaults to this Python kit at its current commit; `rect_keys` to the kit's own list of array keys
    (story.REC_KEYS). Returns the path"""
    tool = sys._getframe(1).f_globals.get('__name__', '?')
    if rect_keys is None:
        _, story = common.kit()
        rect_keys = story.REC_KEYS
    rules = dict(rules) if isinstance(rules, dict) else dict(vars(rules))
    if 'HP' not in run:
        raise ValueError('the run was made with record=False: there is nothing per-frame to store')
    frames = len(run['HP'])
    kinds, members = {}, {}
    for key in run:
        if key in META_KEYS:
            continue
        if not (key.isascii() and key.replace('_', '').isalnum()) or key == 'meta':
            raise ValueError(f'{key!r} cannot name a zip member')
        kinds[key], data = _array(key, run[key], frames, rect_keys)
        members[f'{key}.{kinds[key]}'] = data
    meta = dict(
        format=FORMAT,
        source=source or dict(kind='python', commit=common.kit_commit()),
        names=run['names'], seed=run['seed'], winner=run['winner'], t_ko=run['t_ko'], f0=run['f0'],
        fps=rules['FPS'], corner=run['corner'], rules=rules,
        events=[list(e) for e in run['events']],
        arrays=kinds)
    _check_meta(common.plain(meta, 'meta'))
    members['meta.json'] = common.dumps(meta).encode('ascii')
    path = common.write_bytes(path, common.zip_bytes(members))
    if register:
        manifest.add(path, tool=tool)
    return path


def _check_meta(m):
    """the types the format promises (studio/docs/recording-format.md), checked on what is about to be written"""
    def is_int(x):
        return isinstance(x, int) and not isinstance(x, bool)
    assert is_int(m['seed']) and 0 <= m['seed'] < 2 ** 63, m['seed']
    assert m['winner'] is None or (is_int(m['winner']) and 0 <= m['winner'] < len(m['names'])), m['winner']
    assert m['t_ko'] is None or isinstance(m['t_ko'], float), m['t_ko']
    assert (m['winner'] is None) == (m['t_ko'] is None)
    assert is_int(m['f0']) and m['f0'] >= 0 and is_int(m['fps']) and m['fps'] > 0
    assert all(isinstance(n, str) for n in m['names']) and len(m['names']) == len(m['corner'])
    assert all(len(c) == 2 and all(isinstance(x, float) for x in c) for c in m['corner']), m['corner']
    assert all(isinstance(k, str) for k in m['rules'])
    for e in m['events']:
        assert len(e) >= 3 and isinstance(e[0], float) and isinstance(e[1], str) and is_int(e[2]), e
        assert all(x is None or isinstance(x, (bool, int, float, str, list)) for x in e), e
    assert all(a[0] <= b[0] for a, b in zip(m['events'], m['events'][1:])), 'events are not in time order'
    assert set(m['arrays'].values()) <= {'npy', 'json'} and 'HP' in m['arrays']


# ---------------------------------------------------------------- read
class BadRecording(ValueError):
    pass


def read_bfrec(path):
    """read a .bfrec as the format document says a reader must: meta.json, then every array its "arrays" names.
    Arrays come back as numpy arrays ('<f8' or '<i8') or, for JSON keys, as nested lists"""
    where = common.rel(path)
    try:
        z = zipfile.ZipFile(path)
    except (zipfile.BadZipFile, OSError) as ex:
        raise BadRecording(f'{where}: not a readable zip ({ex})') from None
    with z:
        names = z.namelist()
        if 'meta.json' not in names:
            raise BadRecording(f'{where}: no meta.json')
        meta = json.loads(z.read('meta.json').decode('utf-8'))
        if meta.get('format') != FORMAT:
            raise BadRecording(f"{where}: format {meta.get('format')}, this tool reads {FORMAT}")
        want = sorted(f'{k}.{kind}' for k, kind in meta['arrays'].items()) + ['meta.json']
        if sorted(names) != sorted(want):
            raise BadRecording(f'{where}: members {sorted(names)} do not match meta.json "arrays" {sorted(want)}')
        arrays = {}
        for key, kind in meta['arrays'].items():
            data = z.read(f'{key}.{kind}')
            if kind == 'npy':
                a = npy_format.read_array(io.BytesIO(data), allow_pickle=False)
                if a.dtype.str not in ('<f8', '<i8'):
                    raise BadRecording(f'{where}: {key}.npy has dtype {a.dtype.str}')
                arrays[key] = a
            else:
                arrays[key] = json.loads(data.decode('utf-8'))
    frames = len(arrays['HP'])
    for key, a in arrays.items():
        if len(a) != frames:
            raise BadRecording(f'{where}: {key} has {len(a)} frames, HP has {frames}')
    rec = {k: meta[k] for k in ('format', 'source', 'names', 'seed', 'winner', 't_ko', 'f0', 'fps', 'corner', 'rules',
                                'events')}
    rec['arrays'] = arrays
    return rec


# ---------------------------------------------------------------- the content hash
def _u64(n):
    return struct.pack('<Q', n)


def _value(x):
    """the canonical bytes of a JSON-like value: a tag byte, then the payload"""
    if x is None:
        return b'N'
    if x is True:
        return b'T'
    if x is False:
        return b'F'
    if isinstance(x, int):
        return b'I' + struct.pack('<q', x)
    if isinstance(x, float):
        return b'D' + struct.pack('<d', x)
    if isinstance(x, str):
        raw = x.encode('utf-8')
        return b'S' + _u64(len(raw)) + raw
    if isinstance(x, (list, tuple)):
        return b'L' + _u64(len(x)) + b''.join(_value(v) for v in x)
    raise TypeError(f'{type(x).__name__} has no canonical form')


def canonical_bytes(rec):
    """the byte string the hash is taken over (recording-format.md, "The content hash"): what the fight was, not
    who wrote the file (`format` and `source` are left out) nor how the zip was packed"""
    out = [HASH_TAG, _value(rec['names']), b'I' + _u64(rec['seed']), _value(rec['winner']), _value(rec['t_ko']),
           _value(rec['f0']), _value(rec['fps']), _value(rec['corner']),
           _value([[k, rec['rules'][k]] for k in sorted(rec['rules'])]), _value(rec['events']),
           _u64(len(rec['arrays']))]
    for key in sorted(rec['arrays']):
        a = rec['arrays'][key]
        out.append(_value(key))
        if isinstance(a, np.ndarray):
            out.append({'<f8': b'f', '<i8': b'i'}[a.dtype.str] + _u64(a.ndim) + b''.join(_u64(n) for n in a.shape))
            out.append(np.ascontiguousarray(a).tobytes())
        else:
            out.append(b'j' + _value(a))
    return b''.join(out)


def rec_hash(rec):
    return hashlib.sha256(canonical_bytes(rec)).hexdigest()[:16]
