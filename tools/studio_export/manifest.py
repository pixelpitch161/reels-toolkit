"""studio/reference/manifest.json: one entry per reference file, with its SHA-256 and what made it.

    manifest.add(path)        record (or replace) the entry of a file an export just wrote
    manifest.adopt(path)      record a reference file that a Rust test blessed instead (tool "rust-bless")
    manifest.verify()         re-hash every entry; the problems found (missing or changed files)

An entry: path (repo-relative, forward slashes), sha256, bytes, tool (the module that wrote the file), commit
(`git rev-parse HEAD` when it was exported), dirty (projects/, reelkit/ or series/ had uncommitted changes then),
python, numpy, scipy, pillow (versions) and platform (for example Windows-11-AMD64).

Safe for exports running at the same time: every update happens under a lock file, re-reads the manifest, and
replaces it in one step. Entries are sorted by path, one per line, so two branches that add different files merge
cleanly. An export that reproduces a file bit for bit leaves its entry as it is (only `commit` would differ), so
re-running the export on an unchanged kit changes nothing in the working tree."""
import hashlib
import importlib.metadata
import json
import os
import platform
import sys
import time

from . import common

MANIFEST = common.ref('manifest.json')
LOCK = os.path.join(common.CACHE, 'manifest.lock')
FORMAT = 1
KEYS = ('path', 'sha256', 'bytes', 'tool', 'commit', 'dirty', 'python', 'numpy', 'scipy', 'pillow', 'platform')
ADOPTED = 'rust-bless'        # the `tool` of a file a Rust test wrote and `adopt` recorded
LOCK_WAIT = 120.0             # s to wait for another export's update before giving up
LOCK_STALE = 60.0             # s after which a lock file is taken for a crashed run's (an update takes milliseconds)


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
        os.makedirs(os.path.dirname(LOCK), exist_ok=True)
        t0 = time.monotonic()
        while True:
            try:
                fd = os.open(LOCK, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
                os.write(fd, str(os.getpid()).encode())
                os.close(fd)
                return self
            except (FileExistsError, PermissionError):
                try:
                    if time.time() - os.path.getmtime(LOCK) > LOCK_STALE:
                        os.remove(LOCK)
                        continue
                except OSError:
                    pass                                 # the other run just released it
                if time.monotonic() - t0 > LOCK_WAIT:
                    raise RuntimeError(f'{common.rel(LOCK)}: still locked after {LOCK_WAIT:.0f} s') from None
                time.sleep(0.05)

    def __exit__(self, *exc):
        try:
            os.remove(LOCK)
        except OSError:
            pass


def load():
    """the entries, sorted by path (an empty list when there is no manifest yet)"""
    if not os.path.exists(MANIFEST):
        return []
    for attempt in range(40):                            # Windows: the file may be mid-replace by another export
        try:
            with open(MANIFEST, 'r', encoding='utf-8') as f:
                doc = json.load(f)
            break
        except PermissionError:
            if attempt == 39:
                raise
            time.sleep(0.05)
    if doc.get('format') != FORMAT:
        raise RuntimeError(f"{common.rel(MANIFEST)}: format {doc.get('format')!r}, this tool writes {FORMAT}")
    return sorted(doc['entries'], key=lambda e: e['path'])


def _save(entries):
    entries = sorted(entries, key=lambda e: e['path'])
    rows = ['  ' + json.dumps({k: e[k] for k in KEYS}) for e in entries]
    text = '{\n "format": %d,\n "entries": [\n%s\n ]\n}\n' % (FORMAT, ',\n'.join(rows))
    for attempt in range(40):
        try:
            common.write_text(MANIFEST, text)
            return
        except PermissionError:                          # Windows: a reader has it open; it lets go in a moment
            if attempt == 39:
                raise
            time.sleep(0.05)


def add(path, tool=None):
    """record the file at `path` (absolute, or relative to the repo root) in the manifest; returns its entry.
    `tool` defaults to the name of the module that calls this"""
    if tool is None:
        tool = sys._getframe(1).f_globals.get('__name__', '?')
    path = path if os.path.isabs(path) else os.path.join(common.REPO, path)
    if common.rel(path) == common.rel(MANIFEST) or common.rel(path).startswith(common.rel(common.CACHE) + '/'):
        raise ValueError(f'{common.rel(path)} is not a reference file (write a scratch file with register=False)')
    entry = dict(path=common.rel(path), sha256=sha256(path), bytes=os.path.getsize(path), tool=tool,
                 commit=common.git('rev-parse', 'HEAD'), dirty=common.kit_dirty(), **environment())
    with _Lock():
        entries = load()
        old = next((e for e in entries if e['path'] == entry['path']), None)
        if old is not None and {**old, 'commit': entry['commit']} == entry:
            return old                                   # the same bytes from the same tools: nothing to record
        _save([e for e in entries if e['path'] != entry['path']] + [entry])
    return entry


def adopt(path):
    """record a reference file that no export writes: one a Rust test blessed (for example
    studio/reference/dmath/bits.json). The entry has the same fields as any other, with `tool` = "rust-bless"; the
    version fields describe the environment at adoption. Adopt it again after the file is blessed again. A file an
    export owns is refused: its export records it"""
    p = path if os.path.isabs(path) else os.path.join(common.REPO, path)
    if not os.path.isfile(p):
        raise SystemExit(f'adopt: {path}: no such file (give a path relative to the repo root)')
    r = common.rel(p)
    if r == common.rel(MANIFEST) or r.startswith(common.rel(common.CACHE) + '/'):
        raise SystemExit(f'adopt: {r} is not a reference file')
    old = next((e for e in load() if e['path'] == r), None)
    if old is not None and old['tool'] != ADOPTED:
        raise SystemExit(f"adopt: {r} is written by {old['tool']}; run that export instead")
    return add(p, tool=ADOPTED)


def verify():
    """(entries checked, problems, unlisted): every entry's file re-hashed. A problem is a line naming a missing or
    changed file. `unlisted` are files under studio/reference/ that no entry names (the cache folder aside): no
    export wrote them, so they are reported for someone to `adopt` (a file a Rust test blessed) or remove"""
    entries = load()
    problems = []
    for e in entries:
        p = os.path.join(common.REPO, e['path'])
        if not os.path.isfile(p):
            problems.append(f"missing  {e['path']}")
        elif os.path.getsize(p) != e['bytes'] or sha256(p) != e['sha256']:
            problems.append(f"changed  {e['path']}")
    listed = {e['path'] for e in entries}
    unlisted = []
    for folder, dirs, files in os.walk(common.REFERENCE):
        dirs[:] = sorted(d for d in dirs if os.path.join(folder, d) != common.CACHE)
        for name in sorted(files):
            r = common.rel(os.path.join(folder, name))
            if r != common.rel(MANIFEST) and r not in listed and not name.endswith('.tmp'):
                unlisted.append(r)
    return len(entries), problems, unlisted
