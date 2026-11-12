"""Shared helpers for the reference export: repo paths, exact number formats, deterministic JSON, zip and file
writers, and the one way an export loads a kit episode.

    write_json(path, obj, sort_keys=True, register=True)      a reference JSON file in the fixed layout
    write_npz(path, arrays, register=True)                    a reproducible .npz of plain numeric arrays

Both list the file in the manifest themselves (as recording.write_bfrec does); a file written any other way needs
manifest.add(path). Nothing here imports numpy at module level, so `verify` runs on a bare Python."""
import io
import json
import math
import numbers
import os
import re
import subprocess
import sys
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
REFERENCE = os.path.join(REPO, 'studio', 'reference')
CACHE = os.path.join(REFERENCE, '.cache')              # git-ignored: episode copies, the kit's own caches, the lock
KIT_DIRS = ('projects', 'reelkit', 'series')           # what the reference is made from; never edited
I64_MIN, I64_MAX = -2 ** 63, 2 ** 63 - 1
ZIP_DATE = (1980, 1, 1, 0, 0, 0)                       # every zip member's timestamp
ZIP_LEVEL = 9                                          # deflate level of every zip member
NPZ_DTYPES = ('|u1', '|b1', '<i4', '<i8', '<f4', '<f8')
NPZ_NAME = re.compile(r'[A-Za-z0-9_][A-Za-z0-9_.-]*\Z')


# ---------------------------------------------------------------- paths
def ref(*parts):
    """an absolute path under studio/reference/"""
    return os.path.join(REFERENCE, *parts)


def rel(path):
    """the repo-relative form of a path, with forward slashes (what the manifest and the logs print)"""
    r = os.path.relpath(os.path.abspath(path), REPO).replace(os.sep, '/')
    if r.startswith('..'):
        raise ValueError(f'{path} is outside the repo')
    return r


def git(*args):
    """stdout of `git <args>` run in the repo root"""
    out = subprocess.run(('git',) + args, cwd=REPO, capture_output=True, text=True, encoding='utf-8')
    if out.returncode:
        raise RuntimeError(f"git {' '.join(args)}: {out.stderr.strip()}")
    return out.stdout.strip()


def kit_commit():
    """the commit the kit is at: the last one that changed projects/, reelkit/ or series/, plus '-dirty' when those
    folders have uncommitted changes. Unlike HEAD it does not move when the Studio's own files are committed, so a
    reference file that names it stays byte-identical from one export to the next"""
    sha = git('log', '-1', '--format=%H', '--', *KIT_DIRS)
    return sha + ('-dirty' if kit_dirty() else '')


def kit_dirty():
    return bool(git('status', '--porcelain', '--', *KIT_DIRS))


# ---------------------------------------------------------------- exact numbers
def hexf(x):
    """a float as Python's float.hex() string, e.g. 0.1 -> '0x1.999999999999ap-4': every bit, no rounding"""
    if isinstance(x, numbers.Integral) or not isinstance(x, numbers.Real):
        raise TypeError(f'hexf wants a float, got {type(x).__name__} {x!r}')
    return float(x).hex()


def dec(n):
    """an integer as a decimal string: for values a JSON number cannot carry safely (above 2**53: hashes, PCG64
    state)"""
    if isinstance(n, bool) or not isinstance(n, numbers.Integral):
        raise TypeError(f'dec wants an int, got {type(n).__name__} {n!r}')
    return str(int(n))


def plain(x, where='value'):
    """a JSON-ready copy of a Python value: tuples and numpy arrays become lists, numpy scalars become Python
    numbers of the same value, dict keys must be strings. A float that is not finite, an int outside 64 bits and
    any other type are errors that name the place"""
    if x is None or isinstance(x, bool):
        return x
    if isinstance(x, str):
        return str(x)
    if hasattr(x, 'dtype') and hasattr(x, 'tolist'):    # a numpy scalar or array: tolist() gives exact Python values
        return plain(x.tolist(), where)
    if isinstance(x, int):
        if not I64_MIN <= x <= I64_MAX:
            raise ValueError(f'{where}: {x} does not fit 64 bits (store it as a decimal string)')
        return int(x)
    if isinstance(x, float):
        if not math.isfinite(x):
            raise ValueError(f'{where}: {x!r} is not a finite float')
        return float(x)
    if isinstance(x, (list, tuple)):
        return [plain(v, f'{where}[{i}]') for i, v in enumerate(x)]
    if isinstance(x, dict) and x and all(isinstance(k, numbers.Integral) and not isinstance(k, bool) for k in x):
        # a dict keyed by whole numbers (the rule roll_rows: {fighter index: rows}, ruling M3-R19) is the list of
        # [key, value] pairs in key order, as recording.write_bfrec writes it and the Rust side reads it
        return [[plain(k, f'{where} key'), plain(v, f'{where}[{k}]')] for k, v in sorted(x.items())]
    if isinstance(x, dict):
        for k in x:
            if not isinstance(k, str):
                raise TypeError(f'{where}: dict key {k!r} is not a string')
        return {str(k): plain(v, f'{where}.{k}') for k, v in x.items()}
    raise TypeError(f'{where}: {type(x).__name__} has no JSON form')


def same(a, b, where='value', ordered=False):
    """assert that b (read back from a file) is the JSON form of a, with the same types all the way down: 1 is not
    1.0, 0.0 is not -0.0, a tuple or list must come back as a list. `ordered` also wants dict keys in the same
    order"""
    if isinstance(a, (list, tuple)):
        assert isinstance(b, list) and len(a) == len(b), where
        for i, (x, y) in enumerate(zip(a, b)):
            same(x, y, f'{where}[{i}]', ordered)
    elif isinstance(a, dict):
        assert isinstance(b, dict) and (list(a) == list(b) if ordered else sorted(a) == sorted(b)), where
        for k in a:
            same(a[k], b[k], f'{where}.{k}', ordered)
    elif isinstance(a, float):
        assert type(b) is float and a.hex() == b.hex(), (where, a, b)
    else:
        assert type(a) is type(b) and a == b, (where, a, b)


# ---------------------------------------------------------------- deterministic text
def _scalar(x):
    return x is None or isinstance(x, (bool, int, float, str))


def _fmt(x, level, sort_keys):
    pad = ' ' * (level + 1)
    if isinstance(x, dict):
        if not x:
            return '{}'
        keys = sorted(x) if sort_keys else list(x)
        body = ',\n'.join(f'{pad}{json.dumps(k)}: {_fmt(x[k], level + 1, sort_keys)}' for k in keys)
        return '{\n' + body + '\n' + ' ' * level + '}'
    if isinstance(x, list):
        if all(_scalar(v) for v in x):
            return json.dumps(x, allow_nan=False)
        body = ',\n'.join(pad + _fmt(v, level + 1, sort_keys) for v in x)
        return '[\n' + body + '\n' + ' ' * level + ']'
    return json.dumps(x, allow_nan=False)


def dumps(obj, sort_keys=True):
    """JSON text in the export's one fixed layout: one-space indent, a dict has one key per line, a list of plain
    values sits on one line, any other list has one item per line; floats are Python's shortest round-trip repr,
    non-ASCII characters are escaped, the text ends with a newline. `sort_keys=False` keeps the dicts' own order
    (for data whose order means something)"""
    return _fmt(plain(obj), 0, sort_keys) + '\n'


def dumps_rows(rows):
    """a JSON list with one compact row per line (per-frame data): '[', a line per row, ']'"""
    rows = plain(rows)
    if not isinstance(rows, list):
        raise TypeError('dumps_rows wants a list')
    if not rows:
        return '[]\n'
    return '[\n' + ',\n'.join(json.dumps(r, allow_nan=False) for r in rows) + '\n]\n'


# ---------------------------------------------------------------- files
def write_bytes(path, data):
    """write a file in one step (a temp file next to it, then an atomic replace); returns the path"""
    path = os.path.abspath(path)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = f'{path}.{os.getpid()}.tmp'
    with open(tmp, 'wb') as f:
        f.write(data)
    os.replace(tmp, path)
    return path


def write_text(path, text):
    """UTF-8, no BOM, LF line endings, a final newline"""
    if '\r' in text:
        raise ValueError(f'{path}: carriage return in the text')
    if not text.endswith('\n'):
        raise ValueError(f'{path}: no final newline')
    return write_bytes(path, text.encode('utf-8'))


def _caller():
    """the module that called the write helper: the `tool` its file is listed under"""
    return sys._getframe(2).f_globals.get('__name__', '?')


def _register(path, tool):
    from . import manifest                               # here, not at the top: manifest imports this module
    manifest.add(path, tool=tool)
    return path


def write_json(path, obj, sort_keys=True, register=True):
    """write `obj` as a reference JSON file in the fixed layout (see dumps) and list it in the manifest under the
    calling module's name. `register=False` writes only (a scratch file, or a file its export checks before it
    calls manifest.add itself). Returns the path"""
    tool = _caller()
    path = write_text(path, dumps(obj, sort_keys))
    return _register(path, tool) if register else path


def zip_bytes(members):
    """{member name: bytes} as the bytes of a zip that depend on nothing but the names and the contents: members
    in ascending name order, deflate at ZIP_LEVEL, ZIP_DATE on every member, no platform fields"""
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=ZIP_LEVEL) as z:
        for name in sorted(members):
            zi = zipfile.ZipInfo(name, date_time=ZIP_DATE)
            zi.create_system = 0                          # the default differs between Windows and Unix
            zi.external_attr = 0o600 << 16                # what writestr would put there itself; said outright
            z.writestr(zi, members[name], compress_type=zipfile.ZIP_DEFLATED, compresslevel=ZIP_LEVEL)
    return buf.getvalue()


def npy_bytes(a):
    """a numpy array as the bytes of a .npy file: format version 1.0, C order, no pickle"""
    import numpy as np
    from numpy.lib import format as npy_format
    if not a.flags.c_contiguous:                         # (ascontiguousarray would also turn a 0-d array into 1-d)
        a = np.ascontiguousarray(a)
    buf = io.BytesIO()
    npy_format.write_array(buf, a, version=(1, 0), allow_pickle=False)
    return buf.getvalue()


def write_npz(path, arrays, register=True):
    """write {name: numpy array} as a reproducible .npz (np.savez stamps the time, so its bytes change on every
    run): a zip of <name>.npy members made by zip_bytes and npy_bytes, readable with
    np.load(path, allow_pickle=False). Only plain numeric arrays in the dtypes of NPZ_DTYPES (uint8, bool, int32,
    int64, float32, float64, little-endian) are taken, as they are: nothing is converted, so the dtype the kit
    produced is the dtype Rust reads. Lists, object arrays, strings and other dtypes are refused. The file is
    listed in the manifest under the calling module's name unless `register=False`. Returns the path"""
    import numpy as np
    tool = _caller()
    if not str(path).endswith('.npz'):
        raise ValueError(f'{path}: an .npz file must be named .npz')
    if not arrays:
        raise ValueError(f'{path}: no arrays')
    members = {}
    for name, a in arrays.items():
        if not isinstance(name, str) or not NPZ_NAME.match(name):
            raise ValueError(f'{path}: {name!r} cannot name a member (letters, digits, _ . - only)')
        if not isinstance(a, np.ndarray):
            raise TypeError(f'{path}: {name} is a {type(a).__name__}, not a numpy array (convert it yourself, so '
                            f'the dtype is a choice)')
        if a.dtype.str not in NPZ_DTYPES:
            raise TypeError(f"{path}: {name} has dtype {a.dtype.str!r} ({a.dtype}); allowed: {', '.join(NPZ_DTYPES)}")
        members[f'{name}.npy'] = npy_bytes(a)
    path = write_bytes(path, zip_bytes(members))
    return _register(path, tool) if register else path


# ---------------------------------------------------------------- the kit
def kit():
    """the kit's modules (episode, story), importable the way `python projects/ballfight <step>` makes them:
    projects/ first on sys.path, so `ballfight.kit` is the package under projects/ballfight"""
    projects = os.path.join(REPO, 'projects')
    if projects not in sys.path:
        sys.path.insert(0, projects)
    if REPO not in sys.path:                             # reelkit, when PYTHONPATH was not set
        sys.path.append(REPO)
    from ballfight.kit import episode as E
    from ballfight.kit import story
    return E, story


def roster():
    """fighters.FIGHTERS, the kit's roster table (read-only)"""
    E, _ = kit()
    E.activate_paths()
    import fighters
    return fighters.FIGHTERS


def episode(name, tag):
    """load the kit example `name` (a folder under projects/ballfight/examples/) and make it the current episode,
    as kit/cli.py does with kit.episode.load, but from a private copy of its episode.py at
    studio/reference/.cache/episodes/<tag>/<name>/, so every cache the kit writes (check/run_<seed>.pkl, sprites,
    audio.wav, stems) lands there and nothing is written under projects/. `tag` is the export's name: two exports
    running at once never share a folder"""
    src = os.path.join(REPO, 'projects', 'ballfight', 'examples', name, 'episode.py')
    with open(src, 'rb') as f:
        data = f.read()
    folder = os.path.join(CACHE, 'episodes', tag, name)
    dst = os.path.join(folder, 'episode.py')
    same = False
    if os.path.exists(dst):
        with open(dst, 'rb') as f:
            same = f.read() == data
    if not same:
        write_bytes(dst, data)
    E, _ = kit()
    return E.load(folder)
