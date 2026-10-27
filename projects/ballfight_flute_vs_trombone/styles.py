"""#41 weapon styles: none new. Crossover rule: both fighters look exactly as in their own episodes, so their draw
functions are imported from those episodes' folders (series rule: styles live there so crossovers can import them),
not copied:

    SB  projects/ballfight_trombone_vs_horn/styles.py (#34): the slide lance (`slide_geom`, `draw_slide`,
        `slide_trail_polys`) and PEDAL TONE's giant bell (`bell_height`, `draw_giant_bell`), mint-lit
    SU  projects/ballfight_tuba_vs_flute/styles.py (#32 v2): the FLUTE's pixel-art sprite (`flute_sprite`), SNAKE
        CHARMER's baskets and silver cobras (`BASKET_R`, `BASKET_H`, `draw_basket`, `cobra_pose`, `draw_cobra`) and
        PICCOLO's shivering ring (`piccolo_rd`); the silver tones (`SILVER`, `SILVER_HI`, `SILVER_DK`)

Both modules import `scene` (the shared ring) at load, as here. The VS title card's panel patterns need no mirroring
here: the trombone's slide tubes (title.py) and the flute's key cups (#32) are both drawn parallel to the divider, so
they read the same on either side."""
import importlib.util
import os

HERE = os.path.dirname(os.path.abspath(__file__))


def _load(name, folder):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, '..', folder, 'styles.py'))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


SB = _load('styles34', 'ballfight_trombone_vs_horn')      # TROMBONE (+ PEDAL TONE)
SU = _load('styles32', 'ballfight_tuba_vs_flute')         # FLUTE (SNAKE CHARMER, PICCOLO)
BRASS, BRASS_HI, BRASS_DK = SB.BRASS, SB.BRASS_HI, SB.BRASS_DK
SILVER, SILVER_HI, SILVER_DK = SU.SILVER, SU.SILVER_HI, SU.SILVER_DK
