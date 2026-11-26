# What it writes: the kit's `roll_rows` (kit/story.py 41 to 75, the rule `roll_rows` of a PIANO fight) and
# `roll_start8` for six configs, each the kit config ballfight_piano_vs_organ with its music changed at run time:
#   eight_a_bar     the config as it is: 8 notes a bar with rests (None), A minor, an 8-chord loop
#   four_a_bar      a line of beats (4 a bar): an 8th's rest follows each, a beat's rest among them
#   sketch          no line for PIANO: the kit's sketch line (default_line)
#   four_chords     a 4-chord loop with a 32-note line
#   outside         notes above and below the roll's range (they are folded in by octaves)
#   mostly_rests    a line that is nearly all rests, with a whole bar of them
# and what Python says to a line of 6 notes a bar (the `assert` of roll_rows).
# The expected rows in studio/crates/bf-sim/src/kinds/roll/roll_rows.json are this output; the test
# `kinds::roll::tests::roll_rows_are_the_kits_for_six_configs` reads them.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe tools/studio_export/unit_checks/roll_unit_vectors.py . \
#       studio/crates/bf-sim/src/kinds/roll/roll_rows.json
# The first argument is the repo root: the kit is imported from its projects/ballfight. The kit keeps an episode's
# caches in a folder: studio/reference/.cache/unit_checks/roll/<case>/ (git-ignored), never under projects/.
# This is a check script, not an export: the tool's export discovery does not look in this folder.
"""Python vectors for the roll's rows: six configs, the same call, what kit/story.py gave.
Usage: roll_unit_vectors.py <repo root> <output json>"""
import importlib.util
import json
import os
import sys

clone = os.path.abspath(sys.argv[1])
sys.path.insert(0, os.path.join(clone, 'projects'))
sys.path.append(clone)
from ballfight.kit import episode as E     # noqa: E402
from ballfight.kit import story            # noqa: E402

SOURCE = os.path.join(clone, 'projects', 'ballfight_piano_vs_organ', 'episode.py')
_ = None


def config(**changes):
    """the kit config as a fresh module, with `changes` to its attributes (LINES_PIANO: the PIANO's line, or
    'none' for no line)"""
    spec = importlib.util.spec_from_file_location('roll_vectors_config', SOURCE)
    cfg = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cfg)
    for k, v in changes.items():
        if k == 'LINES_PIANO':
            cfg.LINES = dict(cfg.LINES)
            if isinstance(v, str):
                cfg.LINES.pop('PIANO')
            else:
                cfg.LINES['PIANO'] = list(v)
        else:
            setattr(cfg, k, v)
    return cfg


def case(name, **changes):
    cfg = config(**changes)
    folder = os.path.join(clone, 'studio', 'reference', '.cache', 'unit_checks', 'roll', name)
    os.makedirs(folder, exist_ok=True)
    ep = E.load(folder, cfg=cfg)
    T = ep.T
    line = list(T.LINES['PIANO']) if 'PIANO' in T.LINES else None
    doc = dict(name=name, fighters=list(T.FIGHTERS), tempo_bpm=cfg.BPM, key=cfg.KEY, scale=cfg.SCALE,
               chords_intro=list(T.CHORDS[:T.LOOP_START]), chord_loop=list(T.LOOP), line=line)
    assert T.LOOP_START == 2 and list(T.CHORDS[:2]) == ['Am', 'E7']
    # the Studio's chords are intro + loop repeated: the config's must be that
    rest = list(T.CHORDS[2:])
    assert all(c == T.LOOP[i % len(T.LOOP)] for i, c in enumerate(rest)), name
    try:
        rules = story.rules()
    except AssertionError as ex:
        doc['error'] = str(ex)
        return doc
    rows = rules.roll_rows
    assert sorted(rows) == [0], rows
    doc['rows'] = [[int(m) for m in row] for row in rows[0]]
    doc['roll_start8'] = int(rules.roll_start8)
    return doc


LOOP4 = ['Am', 'F', 'G', 'E7']
out = [
    case('eight_a_bar'),
    case('four_a_bar', LINES_PIANO=[76, 72, 81, 76,  77, _, 81, 77,  79, 74, 83, 79,  80, 76, 83, 80,
                                    84, 81, 76, 72,  84, 81, 77, 72,  81, 77, 74, 69,  83, 80, 76, _]),
    case('sketch', LINES_PIANO='none'),
    case('four_chords', LOOP=LOOP4, CHORDS=['Am', 'E7'] + LOOP4 * 8,
         LINES_PIANO=[76, _, 72, 81, _, 76, 72, 74,  77, _, 72, 81, _, 77, 76, _,
                      79, _, 74, 83, _, 79, 77, 76,  80, _, 76, 83, _, 80, 81, 83]),
    case('outside', LINES_PIANO=[96, 100, 30, 36, 47, 85, 84, 48,  108, 24, 12, 127, 0, 60, 59, 61,
                                 79, 91, 103, 43, 31, 55, 67, 86,  88, 40, 44, 92, 45, 46, 87, 89,
                                 84, 96, 72, 60, 48, 36, 24, 120,  65, 53, 41, 29, 77, 89, 101, 113,
                                 81, 93, 105, 33, 21, 57, 69, 45,  83, 95, 35, 23, 71, 59, 47, 107]),
    case('mostly_rests', LINES_PIANO=[76, _, _, _, _, _, _, _,  _, _, _, _, _, _, _, _,
                                      _, _, _, 83, _, _, _, _,  80, _, _, _, _, _, _, 83,
                                      _, _, _, _, 60, _, _, _,  _, 48, _, _, _, _, _, _,
                                      _, _, _, _, _, _, 84, _,  _, _, _, _, _, _, _, 71]),
    case('six_a_bar', LINES_PIANO=[76, 72, 81, 76, 72, 74] * 8),
]
with open(sys.argv[2], 'w', encoding='utf-8', newline='\n') as f:
    for i, doc in enumerate(out):
        f.write(('[' if i == 0 else ' ') + json.dumps(doc) + (',\n' if i + 1 < len(out) else ']\n'))
for doc in out:
    print(doc['name'], doc.get('error') or f"{len(doc['rows'])} rows, start8 {doc['roll_start8']}")
