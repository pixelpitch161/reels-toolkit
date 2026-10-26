"""#36 weapon styles: none new. Crossover rule: both fighters look exactly as in their own episodes, so their draw
functions are imported from those episodes' folders (series rule: styles live there so crossovers can import them),
not copied:

    SB  projects/ballfight_trombone_vs_horn/styles.py (#34): the slide lance (`slide_geom`, `draw_slide`,
        `slide_trail_polys`) and PEDAL TONE's giant bell (`bell_height`, `draw_giant_bell`), mint-lit
    SU  projects/ballfight_tuba_vs_flute/styles.py (#32 v2): the tuba's bell (`bell_rings`, `draw_tuba_bell`), the
        suction cone (`inhale_light`), BRASS WALL's dome (`draw_dome`) and SUBSONIC's floor ripple (`ripple_light`)"""
import importlib.util
import os

HERE = os.path.dirname(os.path.abspath(__file__))


def _load(name, folder):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, '..', folder, 'styles.py'))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


SB = _load('styles34', 'ballfight_trombone_vs_horn')      # TROMBONE (+ PEDAL TONE)
SU = _load('styles32', 'ballfight_tuba_vs_flute')         # TUBA (+ BRASS WALL, SUBSONIC)
BRASS, BRASS_HI, BRASS_DK = SB.BRASS, SB.BRASS_HI, SB.BRASS_DK
