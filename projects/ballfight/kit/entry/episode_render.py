"""reelkit's video / loop tools load a render.py by path and call render(f); this is that file for kit episodes. It
renders the episode named in the BALLFIGHT_EPISODE environment variable (the CLI sets it). Kept alone in its folder so
reelkit putting this folder on sys.path shadows nothing."""
import os
import sys

_PROJECTS = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..'))
if _PROJECTS not in sys.path:
    sys.path.insert(0, _PROJECTS)
from ballfight.kit import episode  # noqa: E402

EP = episode.current()
from ballfight.kit import picture  # noqa: E402

render = picture.render
NF = EP.T.NF
