"""python -m tools.studio_export <name>... | all | verify | list | adopt <path>...   (repo root, PYTHONHASHSEED=0)

    list      the exports the package's modules register, in the order `all` runs them
    <name>... run those exports, in the order given
    all       run every export: by each one's ORDER number (50 unless its module says otherwise), then by name
    verify    re-hash every file in studio/reference/manifest.json; exit 1 if one is missing or changed
    adopt     record files that a Rust test blessed (repo-relative paths) in the manifest, as tool "rust-bless"

A module registers exports with a module-level `EXPORTS = {"name": function}` (and, if one must run before or after
others under `all`, `ORDER = {"name": number}`). An export named in a module's `TAKES_ARGS` is not a reference export:
it is run alone as `<name> <argument>...`, gets the arguments as a list, and `all` leaves it out (`stages <frame>...`).
Everything but `verify` refuses to run unless the interpreter is
CPython 3.13.18 without -O, PYTHONHASHSEED is 0 and really in effect (not switched off by -E, -I or -R), and numpy,
scipy and pillow are exactly the versions in requirements.txt; `verify` only reads files and runs anywhere."""
import importlib
import importlib.metadata
import os
import pkgutil
import sys
import time

RESERVED = ('all', 'verify', 'list', 'adopt')
DEFAULT_ORDER = 50
PYTHON = (3, 13, 18)                                   # the CPython the reference numbers come from (README.md)
HASH_PROBE = ('FLUTE', -3109979385477651667)           # hash('FLUTE') under seed 0: a row of pyhash/vectors.json


def pins():
    """{package: version} from requirements.txt"""
    out = {}
    with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'requirements.txt'), encoding='utf-8') as f:
        for line in f:
            line = line.split('#')[0].strip()
            if line:
                name, _, version = line.partition('==')
                out[name.strip()] = version.strip()
    return out


def check_environment():
    """refuse unless this interpreter is the reference one in the state the reference numbers need. The hash seed is
    judged by what the interpreter really does, not by the variable alone: under -E, -I or -R the variable can say 0
    while string hashes are random"""
    problems = []
    if os.environ.get('PYTHONHASHSEED') != '0':
        problems.append(f"PYTHONHASHSEED is {os.environ.get('PYTHONHASHSEED')!r}; it must be '0' before Python "
                        f"starts")
    elif sys.flags.hash_randomization:
        problems.append('PYTHONHASHSEED=0 is set but not in effect: string hashes are random in this interpreter '
                        '(start Python without -E, -I or -R)')
    elif hash(HASH_PROBE[0]) != HASH_PROBE[1]:
        problems.append(f'hash({HASH_PROBE[0]!r}) is {hash(HASH_PROBE[0])}, not {HASH_PROBE[1]}: this Python does '
                        f'not hash strings as the reference one does')
    if sys.version_info[:3] != PYTHON:
        problems.append(f"Python {'.'.join(map(str, sys.version_info[:3]))}; the reference numbers come from "
                        f"CPython {'.'.join(map(str, PYTHON))}")
    if sys.flags.optimize:
        problems.append('Python runs with -O (or PYTHONOPTIMIZE): the exports check their files with assert '
                        'statements, which -O skips')
    for name, want in pins().items():
        try:
            have = importlib.metadata.version(name)
        except importlib.metadata.PackageNotFoundError:
            have = None
        if have != want:
            problems.append(f"{name} {have or 'is not installed'}; requirements.txt pins {want}")
    if problems:
        raise SystemExit('tools.studio_export refuses to run:\n  ' + '\n  '.join(problems))


def exports():
    """{name: (order, function, module name)} from every module of the package that has EXPORTS"""
    pkg = importlib.import_module(__package__)
    found = {}
    for info in sorted(pkgutil.iter_modules(pkg.__path__), key=lambda i: i.name):
        if info.name.startswith('_'):
            continue
        mod = importlib.import_module(f'{__package__}.{info.name}')
        order = getattr(mod, 'ORDER', {})
        for name, fn in getattr(mod, 'EXPORTS', {}).items():
            if name in RESERVED:
                raise SystemExit(f'{mod.__name__}: {name!r} is a command of the entry point, not an export name')
            if name in found:
                raise SystemExit(f'{mod.__name__}: export {name!r} is already registered by {found[name][2]}')
            found[name] = (order.get(name, DEFAULT_ORDER), fn, mod.__name__)
    return found


def in_order(found):
    return sorted(found, key=lambda n: (found[n][0], n))


def main(argv):
    if not argv or argv[0] in ('-h', '--help', 'help'):
        print(__doc__)
        return 0
    if argv[0] != 'adopt' and len(argv) > 1 and any(a in RESERVED for a in argv):
        raise SystemExit('all, verify and list stand alone; adopt comes first, then its paths')
    if argv == ['verify']:
        from . import manifest
        n, problems, unlisted = manifest.verify()
        for line in problems:
            print(line)
        for path in unlisted:
            print(f'note: not in the manifest: {path}')
        print(f'verify: {n} files, {len(problems)} missing or changed')
        return 1 if problems else 0
    check_environment()
    if argv[0] == 'adopt':
        from . import manifest
        if len(argv) < 2:
            raise SystemExit('adopt: name at least one file, by its path from the repo root')
        for path in argv[1:]:
            e = manifest.adopt(path)
            print(f"adopted {e['path']}  {e['bytes']} bytes  sha256 {e['sha256']}")
        return 0
    found = exports()
    if argv == ['list']:
        for name in in_order(found):
            print(f'{name:12s} order {found[name][0]:3d}  {found[name][2]}')
        return 0
    takes = {n for n in found if n in getattr(sys.modules[found[n][2]], 'TAKES_ARGS', ())}
    if argv[0] in takes:              # an export with arguments of its own, e.g. `stages 150 600 1500`
        t0 = time.time()
        found[argv[0]][1](argv[1:])
        print(f'[{argv[0]}: {time.time() - t0:.1f} s]')
        return 0
    names = [n for n in in_order(found) if n not in takes] if argv == ['all'] else argv
    unknown = [n for n in names if n not in found]
    if unknown:
        raise SystemExit(f"no export named {', '.join(unknown)} (there are: {', '.join(in_order(found))}; "
                         f"or all, verify, list, adopt <path>)")
    for name in names:
        t0 = time.time()
        found[name][1]()
        print(f'[{name}: {time.time() - t0:.1f} s]')
    return 0


if __name__ == '__main__':            # also what lets an export's spawn pool re-import this module in its workers
    sys.exit(main(sys.argv[1:]))
