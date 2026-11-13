"""The reference manifest: one entry per reference file, with its SHA-256 and what made it, sharded by folder.

    manifest.add(path)        record (or replace) the entry of a file an export just wrote
    manifest.adopt(path)      record a reference file that a Rust test blessed instead (tool "rust-bless")
    manifest.remove(paths)    drop the entries of files an export deleted
    manifest.verify()         re-hash every entry of every shard; the problems found (missing, changed, misplaced)

The shards: every `studio/reference/fighters/<slug>/`, `themes/<name>/` and `episodes/<pair>/` folder (milestone 3
on) has its own `manifest.json`, holding the entries of the files under that folder, so two lanes that export two
fighters never write the same manifest. Every other file (milestones 0 to 2, and files an export writes outside
`studio/reference/`) is listed in the top-level `studio/reference/manifest.json`, with one exception: a fighter's
model, `studio/crates/bf-content/data/models/<NAME>.json`, belongs to its fighter's shard
(`fighters/<slug>/manifest.json`, slug = NAME.lower().replace(' ', '_')), so a port's model never touches a shared
manifest. `shard_of(path)` says which shard a file belongs to; `verify` walks them all and reports an entry that
sits in the wrong one.

An entry: path (repo-relative, forward slashes), sha256, bytes, tool (the module that wrote the file), commit
(`git rev-parse HEAD` when it was exported), dirty (projects/, reelkit/ or series/ had uncommitted changes then),
python, numpy, scipy, pillow (versions) and platform (for example Windows-11-AMD64).

Safe for exports running at the same time: every update happens under one lock file, re-reads the shard, and
replaces it in one step. Entries are sorted by path, one per line, so two branches that add different files merge
cleanly. An export that reproduces a file bit for bit leaves its entry as it is (only `commit` would differ), so
re-running the export on an unchanged kit changes nothing in the working tree.

Paths are worked out from `common.REPO`, `common.REFERENCE` and `common.CACHE` when a function runs (not when the
module is imported), so `unit_checks/m3refs_checks.py` can point the whole tool at a scratch folder."""
import hashlib
import importlib.metadata
import json
import os
import platform
import sys
import time

from . import common

NAME = 'manifest.json'
MANIFEST = common.ref(NAME)   # the top-level manifest of the real reference set (what `top()` gives unredirected)
AREAS = ('fighters', 'themes', 'episodes')        # the sharded areas: one manifest per folder below each
MODELS = 'studio/crates/bf-content/data/models/'   # <NAME>.json here is listed in fighters/<slug>/manifest.json
FORMAT = 1
KEYS = ('path', 'sha256', 'bytes', 'tool', 'commit', 'dirty', 'python', 'numpy', 'scipy', 'pillow', 'platform')
ADOPTED = 'rust-bless'        # the `tool` of a file a Rust test wrote and `adopt` recorded
LOCK_WAIT = 120.0             # s to wait for another export's update before giving up
LOCK_STALE = 60.0             # s after which a lock file is taken for a crashed run's (an update takes milliseconds)


def top():
    """the top-level manifest: studio/reference/manifest.json"""
    return os.path.join(common.REFERENCE, NAME)


def lock_path():
    return os.path.join(common.CACHE, 'manifest.lock')


def area_folder(rel_path):
    """'<area>/<name>' when the repo-relative path lies inside a sharded folder (studio/reference/<area>/<name>/...,
    with something below <name>), else None"""
    base = common.rel(common.REFERENCE) + '/'
    if not rel_path.startswith(base):
        return None
    parts = rel_path[len(base):].split('/')
    if len(parts) >= 3 and parts[0] in AREAS and parts[1] and parts[1] != '.cache':
        return f'{parts[0]}/{parts[1]}'
    return None


def model_folder(rel_path):
    """'fighters/<slug>' for a fighter's model (studio/crates/bf-content/data/models/<NAME>.json), else None"""
    if not rel_path.startswith(MODELS):
        return None
    name = rel_path[len(MODELS):]
    if '/' in name or not name.endswith('.json') or len(name) <= 5:
        return None
    return 'fighters/' + name[:-5].lower().replace(' ', '_')


def shard_of(path):
    """the absolute path of the manifest that lists the file at `path` (absolute or repo-relative): its folder's
    own `manifest.json` inside fighters/<slug>/, themes/<name>/ or episodes/<pair>/, else the top-level one"""
    r = common.rel(path if os.path.isabs(path) else os.path.join(common.REPO, path))
    folder = area_folder(r) or model_folder(r)
    return os.path.join(common.REFERENCE, *folder.split('/'), NAME) if folder else top()


def shards():
    """every manifest that exists: the top-level one first, then the folders' shards in path order"""
    out = [top()] if os.path.exists(top()) else []
    for area in AREAS:
        d = os.path.join(common.REFERENCE, area)
        if not os.path.isdir(d):
            continue
        for name in sorted(os.listdir(d)):
            p = os.path.join(d, name, NAME)
            if os.path.isfile(p):
                out.append(p)
    return out


def is_manifest(rel_path):
    """the repo-relative path is a manifest (the top-level one or a shard): never an entry itself"""
    if rel_path == common.rel(top()):
        return True
    folder = area_folder(rel_path)
    return folder is not None and rel_path == f'{common.rel(common.REFERENCE)}/{folder}/{NAME}'


def sha256(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def environment():
    """what the files depend on besides the kit: interpreter, the three pinned libraries, the platform"""
    def version(name):
        try:
            return importlib.metadata.version(name)
        except importlib.metadata.PackageNotFoundError:
            return None
    return dict(python=platform.python_version(), numpy=version('numpy'), scipy=version('scipy'),
                pillow=version('pillow'), platform=f'{platform.system()}-{platform.release()}-{platform.machine()}')


class _Lock:
    def __enter__(self):
        lock = lock_path()
        os.makedirs(os.path.dirname(lock), exist_ok=True)
        t0 = time.monotonic()
        while True:
            try:
                fd = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
                os.write(fd, str(os.getpid()).encode())
                os.close(fd)
                return self
            except (FileExistsError, PermissionError):
                try:
                    if time.time() - os.path.getmtime(lock) > LOCK_STALE:
                        os.remove(lock)
                        continue
                except OSError:
                    pass                                 # the other run just released it
                if time.monotonic() - t0 > LOCK_WAIT:
                    raise RuntimeError(f'{common.rel(lock)}: still locked after {LOCK_WAIT:.0f} s') from None
                time.sleep(0.05)

    def __exit__(self, *exc):
        try:
            os.remove(lock_path())
        except OSError:
            pass


def load(shard=None):
    """the entries of one manifest (the top-level one unless `shard` names another), sorted by path; an empty list
    when it does not exist yet"""
    shard = shard or top()
    if not os.path.exists(shard):
        return []
    for attempt in range(40):                            # Windows: the file may be mid-replace by another export
        try:
            with open(shard, 'r', encoding='utf-8') as f:
                doc = json.load(f)
            break
        except PermissionError:
            if attempt == 39:
                raise
            time.sleep(0.05)
    if doc.get('format') != FORMAT:
        raise RuntimeError(f"{common.rel(shard)}: format {doc.get('format')!r}, this tool writes {FORMAT}")
    return sorted(doc['entries'], key=lambda e: e['path'])


def load_all():
    """{shard path: its entries} for every manifest"""
    return {s: load(s) for s in shards()}


def _save(entries, shard=None):
    """write one manifest; a shard left with no entries is removed (the top-level one is always written)"""
    shard = shard or top()
    entries = sorted(entries, key=lambda e: e['path'])
    if not entries and shard != top():
        if os.path.exists(shard):
            os.remove(shard)
        return
    rows = ['  ' + json.dumps({k: e[k] for k in KEYS}) for e in entries]
    text = '{\n "format": %d,\n "entries": [\n%s\n ]\n}\n' % (FORMAT, ',\n'.join(rows))
    for attempt in range(40):
        try:
            common.write_text(shard, text)
            return
        except PermissionError:                          # Windows: a reader has it open; it lets go in a moment
            if attempt == 39:
                raise
            time.sleep(0.05)


def _refuse(r):
    if is_manifest(r) or r.startswith(common.rel(common.CACHE) + '/'):
        raise ValueError(f'{r} is not a reference file (write a scratch file with register=False)')


def add(path, tool=None):
    """record the file at `path` (absolute, or relative to the repo root) in its shard; returns its entry.
    `tool` defaults to the name of the module that calls this"""
    if tool is None:
        tool = sys._getframe(1).f_globals.get('__name__', '?')
    path = path if os.path.isabs(path) else os.path.join(common.REPO, path)
    _refuse(common.rel(path))
    entry = dict(path=common.rel(path), sha256=sha256(path), bytes=os.path.getsize(path), tool=tool,
                 commit=common.git('rev-parse', 'HEAD'), dirty=common.kit_dirty(), **environment())
    shard = shard_of(path)
    with _Lock():
        entries = load(shard)
        old = next((e for e in entries if e['path'] == entry['path']), None)
        if old is not None and {**old, 'commit': entry['commit']} == entry:
            return old                                   # the same bytes from the same tools: nothing to record
        _save([e for e in entries if e['path'] != entry['path']] + [entry], shard)
    return entry


def remove(paths):
    """drop the entries of these files (absolute or repo-relative paths) from their shards; returns how many went"""
    gone = {common.rel(p if os.path.isabs(p) else os.path.join(common.REPO, p)) for p in paths}
    by_shard = {}
    for r in gone:
        by_shard.setdefault(shard_of(r), set()).add(r)
    n = 0
    with _Lock():
        for shard, names in sorted(by_shard.items()):
            entries = load(shard)
            keep = [e for e in entries if e['path'] not in names]
            if len(keep) != len(entries):
                n += len(entries) - len(keep)
                _save(keep, shard)
    return n


def find(rel_path):
    """the entry of a repo-relative path in its shard, or None"""
    return next((e for e in load(shard_of(rel_path)) if e['path'] == rel_path), None)


def adopt(path):
    """record a reference file that no export writes: one a Rust test blessed (for example
    studio/reference/dmath/bits.json). The entry has the same fields as any other, with `tool` = "rust-bless"; the
    version fields describe the environment at adoption. Adopt it again after the file is blessed again. A file an
    export owns is refused: its export records it"""
    p = path if os.path.isabs(path) else os.path.join(common.REPO, path)
    if not os.path.isfile(p):
        raise SystemExit(f'adopt: {path}: no such file (give a path relative to the repo root)')
    r = common.rel(p)
    if is_manifest(r) or r.startswith(common.rel(common.CACHE) + '/'):
        raise SystemExit(f'adopt: {r} is not a reference file')
    old = find(r)
    if old is not None and old['tool'] != ADOPTED:
        raise SystemExit(f"adopt: {r} is written by {old['tool']}; run that export instead")
    return add(p, tool=ADOPTED)


def verify():
    """(entries checked, problems, unlisted): every entry of every shard re-hashed. A problem is a line naming a
    missing or changed file, an entry listed in a shard it does not belong to, or a path listed twice. `unlisted`
    are files under studio/reference/ that no entry names (the cache folder and the manifests aside): no export
    wrote them, so they are reported for someone to `adopt` (a file a Rust test blessed) or remove"""
    problems, listed, n = [], {}, 0
    for shard, entries in load_all().items():
        sr = common.rel(shard)
        for e in entries:
            n += 1
            if e['path'] in listed:
                problems.append(f"twice    {e['path']}  ({listed[e['path']]} and {sr})")
                continue
            listed[e['path']] = sr
            if shard_of(e['path']) != shard:
                problems.append(f"misplaced {e['path']}  (listed in {sr}, belongs in {common.rel(shard_of(e['path']))})")
            p = os.path.join(common.REPO, e['path'])
            if not os.path.isfile(p):
                problems.append(f"missing  {e['path']}")
            elif os.path.getsize(p) != e['bytes'] or sha256(p) != e['sha256']:
                problems.append(f"changed  {e['path']}")
    unlisted = []
    for folder, dirs, files in os.walk(common.REFERENCE):
        dirs[:] = sorted(d for d in dirs if os.path.join(folder, d) != common.CACHE)
        for name in sorted(files):
            r = common.rel(os.path.join(folder, name))
            if not is_manifest(r) and r not in listed and not name.endswith('.tmp'):
                unlisted.append(r)
    return n, problems, unlisted
