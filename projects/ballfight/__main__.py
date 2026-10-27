"""python -m ballfight <step> <episode> ... (from projects/), or python projects/ballfight <step> ... (anywhere).
See kit/cli.py for the steps."""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
if os.environ.get('PYTHONHASHSEED') != '0':                  # deterministic frames (see kit/cli.py)
    os.environ['PYTHONHASHSEED'] = '0'
    if __package__:
        os.execv(sys.executable, [sys.executable, '-m', __package__] + sys.argv[1:])
    os.execv(sys.executable, [sys.executable, HERE] + sys.argv[1:])
if sys.path and os.path.abspath(sys.path[0]) == HERE:        # run as a folder: the kit imports the shared modules itself
    sys.path.pop(0)
sys.path.insert(0, os.path.dirname(HERE))
from ballfight.kit.cli import main  # noqa: E402

sys.exit(main(sys.argv[1:]))
