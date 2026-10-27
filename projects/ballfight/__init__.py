"""BALLFIGHT: the shared engine (fight.py, fighters.py, scene.py, title.py, #27's own episode files) and the episode kit
(kit/), run as one command per step:

    cd projects && python -m ballfight <step> <episode>        (or, from anywhere: python projects/ballfight ...)

See README.md, "Making an episode". Episodes from #27 to #42 keep importing this folder's modules by name (it stays on
their sys.path), so this file only marks the folder as a package for `python -m ballfight`."""
