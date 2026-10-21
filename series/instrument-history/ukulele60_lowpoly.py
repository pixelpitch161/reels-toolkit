"""Pixel Pitch: history of the ukulele, 60-second cut.  python3 ukulele60_lowpoly.py [out.mp4] | --still t ..."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_guitar as G
import pp_shapes as S
import piano_lowpoly as P
import violin_lowpoly as V
import trumpet_lowpoly as T
import drums60_lowpoly as D
from pp_framework import mix, dim, lerp, clamp, ease, midi_hz, SR, RW, RH, CX, YY, XX, BAY, HOT, HOTSTR, CREAM, GOLD

OUT, TAG = "ukulele60_lowpoly.mp4", "uk"
BPM = 104
BEAT_S = 60/BPM; BAR_S = 4*BEAT_S
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 1
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, .875, -12, 20, .3
LIGHTING = True
HEAD_INTRO, HEAD, TODAY, START_YEAR = "THE UKULELE", "147 YEARS", "TODAY", 1870
INTRO_LINES = (("ITS NAME MEANS", "'JUMPING FLEA'"), ("BUT HOW DID WE", "GET HERE?"))
INTRO_STEPS = ((0, INTRO_LINES[0], (CREAM, CREAM)),               # hook
               (.42, ("21 INCHES LONG,", "13-INCH SCALE"), (CREAM, CREAM)),   # the modern soprano's own fact
               (.75, INTRO_LINES[1], (CREAM, GOLD)))
OUTRO_LINES = ("147 YEARS FROM", "A MADEIRAN MACHETE")
OUTRO_CREDIT = "INDIE FOLK STOMP"
OUTRO_NAME = "KAMAKA SOPRANO"        # the outro returns to the modern soprano

# ---------------------------------------------------------------- music
CH = {"C": (36, (55, 60, 64)), "Am": (45, (57, 60, 64)), "F": (41, (57, 60, 65)), "G": (43, (55, 59, 62)), "Em": (40, (55, 59, 64)),
      "D": (38, (57, 62, 66)), "A7": (45, (55, 61, 64)), "D7": (38, (54, 57, 60)), "G7": (43, (53, 59, 62)), "E7": (40, (56, 59, 62)),
      "C7": (36, (55, 58, 64)), "Bb": (46, (53, 58, 62)), "C6": (36, (57, 60, 64))}
BARS = ["C", "Am", "F", "G",                                         # modern showcase
        ("D", "D", "A7", "A7"), ("A7", "A7", "D", "D"),              # 1879 machete
        ("C", "C", "A7", "A7"), ("D7", "G7", "C", "C"),              # 1880s hula
        ("F", "F", "D7", "D7"), ("G7", "C7", "F", "F"),              # 1915 hapa haole
        ("G", "G", "E7", "E7"), ("A7", "D7", "G", "G"),              # 1917 vaudeville
        ("C6", "C6", "E7", "E7"), ("A7", "D7", "G7", "C6"),          # 1928 charleston
        ("F", "F", "Bb", "Bb"), ("C7", "C7", "F", "F"),              # 1949 singalong
        ("G", "G", "C", "C"), ("D", "D", "G", "G"),                  # 1960s hootenanny
        ("C", "C", "Em", "Em"), ("F", "F", "G", "G"),                # 1993 reggae
        ("Am", "Am", "F", "F"), ("C", "C", "G", "G"),                # 2006 rock ballad
        ("F", "F", "G", "G"),                                        # outro
        "C"]
# chord shapes in string order (re-entrant G C E A), machete D G B D, baritone D G B E
UC = {"C": [67, 60, 64, 72], "Am": [69, 60, 64, 69], "F": [69, 60, 65, 69], "G": [67, 62, 67, 71], "Em": [67, 64, 67, 71],
      "D7": [69, 62, 66, 72], "G7": [67, 62, 65, 71], "A7": [67, 61, 64, 69], "E7": [68, 62, 64, 71], "C7": [67, 60, 64, 70],
      "D": [69, 62, 66, 69], "Bb": [70, 62, 65, 70], "C6": [67, 60, 64, 69]}
MC = {"D": [62, 69, 74, 78], "A7": [64, 69, 73, 79]}
BC = {"G": [50, 55, 59, 67], "C": [52, 55, 60, 64], "D": [50, 57, 62, 66]}

def ch(b, name, d=.5, v=.8, flag="d", shapes=UC): return (b, shapes[name], d, v, flag)
def nt(b, m, d=.5, v=.88, flag=""): return (b, m, d, v, flag)
def folk(b0, name, v=1.):            # indie-folk strum: D . D U . U D U
    return [ch(b0, name, .5, .92*v), ch(b0 + 1, name, .5, .74*v), ch(b0 + 1.5, name, .5, .58*v, "u"),
            ch(b0 + 2.5, name, .5, .6*v, "u"), ch(b0 + 3, name, .5, .8*v), ch(b0 + 3.5, name, .5, .6*v, "u")]
def trem(b0, m, beats, v=.78):       # tremolo picking in sextuplets
    k = int(round(beats*6)); return [(b0 + j/6, m, 1/6, v if j % 2 == 0 else v*.8, "t") for j in range(k)]

SHOW = (folk(0, "C") + folk(4, "Am") +
        [ch(8, "F", .5, .9), nt(9, 72), nt(9.5, 74), nt(10, 77, 1), nt(11, 74), nt(11.5, 72)] +
        [ch(12, "G", .5, .9), nt(13, 71), nt(13.5, 72), nt(14, 74, .75), nt(14.75, 71, .25), ch(15, "G", .5, .8), ch(15.5, "G", .5, .6, "u")])
MACHETE = [ch(0, "D", .5, .9, shapes=MC), nt(.5, 74, .25), nt(.75, 76, .25), nt(1, 78), nt(1.5, 74),
           ch(2, "A7", .5, .9, shapes=MC), nt(2.5, 73), nt(3, 76), nt(3.5, 73),
           ch(4, "A7", .5, .9, shapes=MC), nt(4.5, 79, .25), nt(4.75, 78, .25), nt(5, 76), nt(5.5, 73),
           ch(6, "D", .5, .9, shapes=MC), nt(6.5, 78), ch(7, "D", 1, .95, shapes=MC)]
HULA = [nt(0, 76, 1.5, .9), nt(1.5, 74, .5, .85), nt(2, 73, 1, .88), nt(3, 69, 1, .85),
        nt(4, 72, .75, .88), nt(4.75, 69, .25, .8), nt(5, 71, 1, .88), nt(6, 72, 2, .9),
        ch(1, "C", .5, .5), ch(2, "A7", .5, .55), ch(3, "A7", .5, .48, "u"), ch(4, "D7", .5, .55), ch(5, "G7", .5, .55),
        ch(6, "C", .5, .6), ch(7, "C", 1, .52, "u")]
HAPA = [nt(0, 72), nt(.5, 74), nt(1, 77, 1, .9), nt(2, 78, .75, .9), nt(2.75, 76, .25), nt(3, 74, 1),
        nt(4, 74), nt(4.5, 71), nt(5, 72), nt(5.5, 70), nt(6, 69, 2, .9),
        ch(1, "F", .4, .55), ch(3, "D7", .4, .55), ch(5, "C7", .4, .55), ch(7, "F", .4, .55)]
VAUDE = [ch(0, "G", .25, .9), ch(.5, "G", .25, .5, "u"), nt(.75, 74, .25, .8), ch(1, "G", .25, .8), nt(1.5, 76, .25, .8), nt(1.75, 74, .25, .75),
         ch(2, "E7", .25, .9), ch(2.5, "E7", .25, .5, "u"), nt(2.75, 76, .25, .8), ch(3, "E7", .25, .8), nt(3.5, 80, .25, .85), nt(3.75, 76, .25, .75),
         ch(4, "A7", .25, .9), nt(4.5, 76, .25, .8), nt(4.75, 73, .25, .7), ch(5, "D7", .25, .9), nt(5.5, 78, .25, .8), nt(5.75, 72, .25, .7),
         ch(6, "G", .25, .95), nt(6.5, 71, .25, .8), ch(7, "G", .5, .95)]
CHARL = [ch(0, "C6", .5, .95), nt(.5, 79, .5, .85), nt(1, 76, .5, .8), ch(1.5, "C6", .5, .9),
         ch(2, "E7", .5, .95), nt(2.5, 80, .5, .85), nt(3, 76, .5, .8), ch(3.5, "E7", .5, .9),
         ch(4, "A7", .5, .95), nt(4.5, 76), nt(5, 72), ch(5.5, "D7", .5, .9),
         ch(6, "G7", .5, .95), nt(6.5, 77, .25, .85), nt(6.75, 76, .25, .8), ch(7, "C6", 1, .95)]
SING = [nt(0, 72, 1, .9), nt(1, 72, .5, .85), nt(1.5, 74, .5, .85), nt(2, 77, 1, .9), nt(3, 74, 1, .85),
        nt(4, 76, 1.5, .9), nt(5.5, 74, .5, .85), nt(6, 72, 2, .9)] + \
       [ch(k, nm, .5, .5 if k % 2 == 0 else .42) for k, nm in enumerate(["F", "F", "Bb", "Bb", "C7", "C7", "F", "F"])]
HOOT = [nt(0, 55, .5, .9), ch(1, "G", .5, .7, shapes=BC), ch(1.5, "G", .5, .5, "u", BC),
        nt(2, 52, .5, .88), ch(3, "C", .5, .7, shapes=BC), nt(3.5, 57, .25, .75), nt(3.75, 59, .25, .75),
        nt(4, 50, .5, .9), ch(5, "D", .5, .7, shapes=BC), ch(5.5, "D", .5, .5, "u", BC),
        nt(6, 55, .5, .9), nt(6.5, 57, .25, .78), nt(6.75, 59, .25, .78), ch(7, "G", 1, .9, shapes=BC)]
REGGAE = [nt(0, 72, .75, .85), nt(.75, 76, .25, .8), nt(1, 79, 1.5, .9), nt(2.5, 76, .5, .85), nt(3, 71, .5, .82), nt(3.5, 74, .5, .82),
          nt(4, 77, .75, .88), nt(4.75, 76, .25, .8), nt(5, 72, 1, .85), nt(6, 74, 1, .85), nt(7, 71, 1, .82)] + \
         [ch(k + .5, nm, .2, .55, "x") for k, nm in enumerate(["C", "C", "Em", "Em", "F", "F", "G", "G"])]
BALLAD = ([ch(0, "Am", .5, .95)] + trem(.5, 76, 1) + trem(1.5, 74, .5) + [ch(2, "F", .5, .9)] + trem(2.5, 72, 1) + trem(3.5, 69, .5) +
          [ch(4, "C", .5, .95)] + trem(4.5, 72, .5) + trem(5, 76, 1) + [ch(6, "G", .5, .9)] +
          [nt(6.5, 74, .25, .85), nt(6.75, 76, .25, .85), nt(7, 79, .25, .88), nt(7.25, 81, .25, .9), nt(7.5, 83, .25, .92), nt(7.75, 81, .25, .85)] +
          # outro (back to the modern soprano): indie-folk strum + final chord
          [ch(8, "F", .5, .92), ch(9, "F", .5, .74), ch(9.5, "F", .5, .58, "u"), ch(10, "G", .5, .85), ch(10.5, "G", .5, .6, "u"),
           ch(11, "G", .5, .8), ch(11.5, "G", .5, .62, "u"), ch(12, "C", 4.3, .95)])

SCENES = [
 dict(key="modern", theme="modern", name="KAMAKA SOPRANO", credit="INDIE FOLK STOMP", notes=SHOW),
 dict(key="machete", theme="machete", year=1879, ys="1879", name="MADEIRAN MACHETE", credit="MADEIRAN FOLK DANCE",
      fact=("SAILED TO HAWAII", "ON THE RAVENSCRAG"), notes=MACHETE),
 dict(key="honolulu", theme="royal", year=1885, ys="1880s", name="HONOLULU UKULELE", credit="HAWAIIAN COURT HULA",
      fact=("KING KALAKAUA", "CHAMPIONED IT"), notes=HULA),
 dict(key="kumalae", theme="expo", year=1915, ys="1915", name="KUMALAE UKULELE", credit="HAPA HAOLE SONG",
      fact=("A GOLD AWARD AT", "THE 1915 SF EXPO"), notes=HAPA),
 dict(key="banjolele", theme="vaude", year=1917, ys="1917", name="BANJOLELE", credit="VAUDEVILLE NOVELTY",
      fact=("A BANJO BODY FOR", "VAUDEVILLE VOLUME"), notes=VAUDE),
 dict(key="pineapple", theme="pineapple", year=1928, ys="1928", name="KAMAKA PINEAPPLE", credit="JAZZ AGE CHARLESTON",
      fact=("PINEAPPLE-SHAPED,", "PATENTED IN 1928"), notes=CHARL),
 dict(key="islander", theme="tv", year=1949, ys="1949", name="MACCAFERRI ISLANDER", credit="1950s TV SINGALONG",
      fact=("ABOUT 9 MILLION", "PLASTIC UKES MADE"), notes=SING),
 dict(key="baritone", theme="hoot", year=1962, ys="1960s", name="BARITONE UKULELE", credit="FOLK HOOTENANNY",
      fact=("ARTHUR GODFREY", "PUSHED IT ON TV"), notes=HOOT),
 dict(key="tenor93", theme="rainbow", year=1993, ys="1993", name="TENOR UKULELE", credit="ISLAND REGGAE",
      fact=("HAWAII'S FIRST", "PLATINUM ALBUM"), notes=REGGAE),
 dict(key="kamaka06", theme="night", year=2006, ys="2006", name="KAMAKA TENOR", credit="ROCK BALLAD",
      fact=("ONE OF YOUTUBE'S", "FIRST VIRAL HITS"), notes=BALLAD),
]
def build_sizes():
    for _s in SCENES: _s["size"] = UKES[_s["key"]]["size"]
for _s in SCENES:                    # onset (s) -> items, so synth/build can find chord shapes, string order and strum flags
    _s["_items"] = {}
    for _it in _s["notes"]: _s["_items"].setdefault(round(_it[0]*BEAT_S, 3), []).append(_it)

# ---------------------------------------------------------------- instruments
U, Y0 = .068, .08                    # model units per inch; body bottom height on the stand
def fig8(Wl, Ww, Wu, ul=.27, uw=.585, uu=.8, n=25):
    """figure-8 outline: half-widths (fractions of body length) of lower bout, waist, upper bout"""
    def w(u):
        if u <= ul: k = u/ul; return Wl*math.sqrt(max(0., 1 - (1 - k)**2))
        if u <= uw: k = (u - ul)/(uw - ul); return Wl + (Ww - Wl)*(1 - math.cos(math.pi*k))/2
        if u <= uu: k = (u - uw)/(uu - uw); return Ww + (Wu - Ww)*(1 - math.cos(math.pi*k))/2
        k = (u - uu)/(1 - uu); return Wu*math.sqrt(max(0., 1 - k**2))
    us = (1 - np.cos(np.pi*np.linspace(0, 1, n)))/2
    return [(float(u), max(w(u), .085 if u > .5 else .04)) for u in us]
PINE = [(0, .11), (.03, .22), (.08, .29), (.16, .335), (.26, .355), (.36, .36), (.46, .356), (.56, .345), (.66, .325), (.76, .295),
        (.85, .26), (.92, .225), (.97, .19), (1, .17)]
def grain(base, amp=.08, freq=9., ph=0.):
    lo, hi = dim(base, 1 - amp), dim(base, 1 + amp)
    return lambda u: mix(lo, hi, .5 + .5*math.sin(u*freq + ph))
def flame(a, b, n=11): return lambda u: a if int(u*n*2) % 2 == 0 else b
def burst(edge, center): return lambda u: mix(edge, center, clamp(1 - abs(u - .42)*2.4))
KOA, ROSE, EBONY, MAHOG, NYLON, GUT, STEEL, WOUND = (198, 118, 58), (54, 36, 30), (26, 22, 22), (160, 78, 50), (238, 234, 222), (230, 214, 170), (214, 218, 228), (190, 190, 202)
CHROME, PEARL, FRET = (200, 202, 212), (242, 238, 228), (206, 204, 210)
UKE, MACH, BARI = [67, 60, 64, 69], [62, 67, 71, 74], [50, 55, 59, 64]
HEADS = {   # outline (x, y) in inches from the nut, string-post positions (x, y)
 "crown":    ([(-.69, 0), (.69, 0), (.82, 2.75), (.92, 3.3), (.42, 3.08), (0, 3.4), (-.42, 3.08), (-.92, 3.3), (-.82, 2.75)], (.38, (1.0, 2.1))),
 "round":    ([(-.6, 0), (.6, 0), (.74, .9), (.84, 1.9), (.76, 2.7), (.48, 3.2), (0, 3.35), (-.48, 3.2), (-.76, 2.7), (-.84, 1.9), (-.74, .9)], (.4, (1.0, 2.15))),
 "shield":   ([(-.62, 0), (.62, 0), (.7, 2.35), (.9, 2.85), (.82, 3.25), (.4, 3.15), (0, 3.5), (-.4, 3.15), (-.82, 3.25), (-.9, 2.85), (-.7, 2.35)], (.38, (.95, 2.0))),
 "banjo":    ([(-.62, 0), (.62, 0), (.8, 2.6), (.64, 3.1), (.22, 3.0), (0, 3.25), (-.22, 3.0), (-.64, 3.1), (-.8, 2.6)], (.4, (1.0, 2.05))),
 "islander": ([(-.66, 0), (.66, 0), (.82, 2.6), (.8, 3.0), (.45, 3.2), (0, 3.06), (-.45, 3.2), (-.8, 3.0), (-.82, 2.6)], (.4, (.95, 2.0))),
 "vega":     ([(-.76, 0), (.76, 0), (.86, 4.5), (.62, 4.95), (0, 5.1), (-.62, 4.95), (-.86, 4.5)], (.3, (1.5, 3.3))),
 "tenor90":  ([(-.7, 0), (.7, 0), (.9, 3.5), (.78, 3.95), (0, 4.1), (-.78, 3.95), (-.9, 3.5)], (.3, (1.25, 2.75))),
}
UKES = {   # dimensions in inches
 "modern":    dict(prof=fig8(.335, .226, .271), Lb=9.4, d=2.4, scale=13.6, fj=12, nf=15, ext=.9, nut_w=1.38, joint_w=1.65,
                   top=KOA, side=(150, 82, 40), colfn=grain(KOA, .07, 11), neckcol=(170, 90, 48), fbcol=ROSE, dots=(5, 7, 10, 12),
                   head="crown", headcol=(188, 108, 52), tuners="rear", button=PEARL, post=(34, 30, 32),
                   hole=dict(r=1.0, dy=2.3, rings=[("ring", .22, (236, 226, 204)), ("ring", .13, (44, 32, 26))]),
                   bridge="tiebar", bcol=(46, 30, 24), sb=.82, sn=.55, strings=UKE, scol=[NYLON]*4, size=1.0),
 "machete":   dict(prof=fig8(.29, .17, .22, uw=.58), Lb=10.0, d=2.7, scale=13.0, fj=12, nf=16, flush=True, ext=1.6, nut_w=1.2, joint_w=1.45,
                   top=(226, 196, 138), side=(146, 88, 46), neckcol=(132, 78, 42), fbcol=(52, 36, 26), dots=(), fret_col=(214, 206, 180),
                   head="round", headcol=(110, 64, 34), tuners="pegs", peg=(66, 42, 26),
                   hole=dict(r=.85, dy=2.5, rings=[("rope", .34, [(44, 30, 22), (226, 206, 156)]), ("ring", .07, (44, 30, 22))]),
                   bind=("rope", [(44, 30, 22), (230, 210, 160)]), bridge="machete", bcol=(42, 28, 22), sb=.7, sn=.5,
                   strings=MACH, scol=[STEEL]*4, size=1.0),
 "honolulu":  dict(prof=fig8(.325, .215, .26), Lb=9.3, d=2.4, scale=13.0, fj=12, nf=16, flush=True, ext=1.35, nut_w=1.25, joint_w=1.5,
                   top=(172, 102, 52), side=(128, 70, 36), colfn=grain((172, 102, 52), .09, 7, 1.), neckcol=(140, 82, 42), fbcol=(156, 94, 50),
                   dots=(), fret_col=(212, 200, 170), head="round", headcol=(96, 54, 30), tuners="pegs", peg=(58, 36, 22),
                   hole=dict(r=.95, dy=2.3, rings=[("rope", .3, [(226, 204, 150), (92, 52, 28)]), ("ring", .06, (226, 204, 150))]),
                   bind=("rope", [(226, 204, 150), (92, 52, 28)]), bridge="tiebar", bcol=(92, 54, 30), sb=.78, sn=.52,
                   strings=UKE, scol=[GUT]*4, size=1.0),
 "kumalae":   dict(prof=fig8(.33, .25, .255, uw=.6), Lb=9.4, d=2.4, scale=13.0, fj=12, nf=12, flush=True, ext=0., nut_w=1.3, joint_w=1.55,
                   top=(204, 126, 58), side=(152, 84, 40), colfn=grain((204, 126, 58), .1, 14, .4), neckcol=(176, 102, 50), fbcol=(176, 102, 50),
                   dots=(), fret_col=(232, 196, 110), strip=(236, 214, 160), head="shield", headcol=(190, 112, 54), tuners="pegs", peg=(54, 34, 24),
                   hole=dict(r=.95, dy=2.2, rings=[("ring", .5, (236, 214, 160)), ("ring", .42, (204, 126, 58)), ("rope", .26, [(236, 214, 160), (130, 72, 34)]),
                                                     ("ring", .05, (236, 214, 160))]),
                   bind=("rope", [(236, 214, 160), (130, 72, 34)]), bridge="notch", bcol=(170, 98, 48), sb=.78, sn=.52,
                   strings=UKE, scol=[GUT]*4, size=1.0),
 "banjolele": dict(banjo=True, Lb=8.0, d=2.2, res=1.0, scale=13.0, fj=16, nf=16, ext=.3, nut_w=1.3, joint_w=1.5,
                   side=(116, 66, 36), res_col=(100, 56, 30), skin=(234, 222, 190), neckcol=(214, 186, 128), fbcol=(58, 40, 30), dots=(5, 7, 10, 12),
                   head="banjo", headcol=(214, 186, 128), tuners="rear", button=(236, 226, 198), post=CHROME,
                   bridge="banjo", bcol=(226, 200, 140), sb=.62, sn=.52, strings=UKE, scol=[GUT]*4, size=1.0),
 "pineapple": dict(prof=PINE, Lb=9.6, d=2.5, scale=13.6, fj=12, nf=15, ext=.9, nut_w=1.38, joint_w=1.65,
                   top=(214, 140, 66), side=(160, 90, 44), colfn=grain((214, 140, 66), .07, 10, 2.), neckcol=(170, 90, 48), fbcol=ROSE,
                   dots=(5, 7, 10, 12), head="crown", headcol=(196, 118, 56), tuners="rear", button=PEARL, post=(34, 30, 32),
                   hole=dict(r=1.0, dy=2.4, rings=[("ring", .2, (236, 226, 204)), ("ring", .12, (44, 32, 26))]),
                   bridge="tiebar", bcol=(46, 30, 24), sb=.82, sn=.55, strings=UKE, scol=[NYLON]*4, size=1.0),
 "islander":  dict(prof=fig8(.34, .22, .27), Lb=9.5, d=2.3, scale=13.25, fj=12, nf=15, ext=.7, nut_w=1.38, joint_w=1.65,
                   top=(240, 230, 204), side=(232, 220, 192), neckcol=(236, 226, 200), fbcol=(26, 24, 28), fret_col=(84, 82, 90),
                   dots=(5, 7, 10, 12), dot_col=(236, 232, 222), head="islander", headcol=(238, 228, 202), tuners="rear",
                   button=(240, 232, 210), post=(236, 228, 206),
                   hole=dict(r=1.0, dy=2.3, rings=[("ring", .12, (26, 24, 28))]), bridge="tiebar", bcol=(26, 24, 28), saddle=(40, 38, 44),
                   sb=.82, sn=.55, strings=UKE, scol=[NYLON]*4, size=1.0),
 "baritone":  dict(prof=fig8(.343, .214, .264, ul=.26, uw=.56, uu=.79), Lb=14.0, d=3.3, scale=19.0, fj=14, nf=19, ext=1.6,
                   nut_w=1.5, joint_w=1.9, top=(158, 82, 42), side=(104, 50, 26), colfn=burst((92, 40, 20), (176, 98, 50)),
                   neckcol=(124, 62, 32), fbcol=ROSE, dots=(5, 7, 10, 12, 15), head="vega", headcol=(64, 34, 20), tuners="side",
                   button=(236, 226, 200), post=CHROME,
                   hole=dict(r=1.4, dy=3.3, rings=[("ring", .34, (236, 226, 204)), ("ring", .26, (30, 24, 22)), ("ring", .18, (236, 226, 204)),
                                                     ("ring", .1, (30, 24, 22))]),
                   bind=("plain", [(236, 226, 204)]), bridge="tiebar", bcol=(42, 28, 22), sb=1.0, sn=.62,
                   strings=BARI, scol=[WOUND, WOUND, NYLON, NYLON], size=.8),
 "tenor93":   dict(prof=fig8(.339, .217, .27), Lb=11.5, d=2.8, scale=17.0, fj=14, nf=18, ext=1.2, nut_w=1.4, joint_w=1.75,
                   top=MAHOG, side=(124, 58, 38), colfn=grain(MAHOG, .05, 16), neckcol=(140, 68, 42), fbcol=(60, 40, 32),
                   dots=(5, 7, 10, 12, 15), head="tenor90", headcol=(124, 58, 38), tuners="side", button=CHROME, post=CHROME,
                   hole=dict(r=1.1, dy=2.7, rings=[("ring", .22, (236, 228, 210)), ("ring", .14, (30, 22, 20))]),
                   bind=("plain", [(236, 228, 210)]), bridge="tiebar", bcol=(58, 38, 30), sb=.9, sn=.56,
                   strings=UKE, scol=[NYLON]*4, size=.88),
 "kamaka06":  dict(prof=fig8(.339, .217, .27, n=33), Lb=11.6, d=2.9, scale=17.0, fj=14, nf=18, ext=1.0, nut_w=1.4, joint_w=1.75,
                   top=(224, 130, 70), side=(172, 92, 48), colfn=flame((228, 138, 76), (200, 108, 56), 13), neckcol=(176, 96, 50), fbcol=EBONY,
                   dots=(5, 7, 10, 12, 15), dot_col=(150, 230, 220), head="crown", headcol=(212, 122, 64), tuners="rear", button=PEARL,
                   post=(30, 28, 30), hole=dict(r=1.1, dy=2.7, rings=[("abalone", .2), ("ring", .07, (24, 20, 20))]),
                   bind=("abalone",), bridge="tiebar", bcol=EBONY, sb=.9, sn=.56, strings=UKE, scol=[NYLON]*4, size=.88),
}
build_sizes()
ABALONE = [(90, 210, 196), (60, 140, 210), (176, 236, 214), (120, 110, 210), (70, 190, 150)]

def slab_star(g, pts, z0, z1, col, spec=0.):
    """extruded outline whose caps are fanned from the centroid (keeps notches of crown-shaped heads)"""
    cx = sum(p[0] for p in pts)/len(pts); cy = sum(p[1] for p in pts)/len(pts); c = (cx, cy, (z0 + z1)/2)
    for z in (z0, z1):
        for k in range(len(pts)):
            a, b = pts[k], pts[(k + 1) % len(pts)]
            g.face([(cx, cy, z), (a[0], a[1], z), (b[0], b[1], z)], col, ref=(cx, cy, (z0 + z1)/2), spec=spec)
    for k in range(len(pts)):
        (xa, ya), (xb, yb) = pts[k], pts[(k + 1) % len(pts)]
        g.face([(xa, ya, z0), (xb, yb, z0), (xb, yb, z1), (xa, ya, z1)], col, ref=c, spec=spec)

def stand(g, y0, back, Lb):
    metal = (40, 38, 46)
    g.box(-.16, .16, 0, .03, -.14, .1, metal, spec=.5)
    g.box(-.1, .1, .03, y0, -.04, .04, metal, spec=.5)
    g.box(-.012, .012, .03, y0 + .55*Lb, back - .03, back, metal, spec=.5)

def ring_band(g, cx, cy, r0, r1, z, cols, n=24, shimmer=0):
    for k in range(n):
        a0, a1 = 2*math.pi*k/n, 2*math.pi*(k + 1)/n
        q = [(cx + r1*math.cos(a0), cy + r1*math.sin(a0)), (cx + r1*math.cos(a1), cy + r1*math.sin(a1)),
             (cx + r0*math.cos(a1), cy + r0*math.sin(a1)), (cx + r0*math.cos(a0), cy + r0*math.sin(a0))]
        V.decal(g, q, z, cols[(k + shimmer) % len(cols)])

def binding(g, prof, Lb, y0, z, cols, w, shimmer=0):
    ymid = y0 + Lb/2
    for side in (-1, 1):
        pts = [(side*p[1]*Lb, y0 + p[0]*Lb) for p in prof]; k = 0
        for a, b in zip(pts, pts[1:]):
            for j in range(2):
                pa = (lerp(a[0], b[0], j/2), lerp(a[1], b[1], j/2)); pb = (lerp(a[0], b[0], (j + 1)/2), lerp(a[1], b[1], (j + 1)/2))
                dx, dy = pb[0] - pa[0], pb[1] - pa[1]; ln = math.hypot(dx, dy)
                if ln < 1e-6: continue
                nx, ny = -dy/ln, dx/ln
                mx, my = (pa[0] + pb[0])/2, (pa[1] + pb[1])/2
                if nx*(0 - mx) + ny*(ymid - my) < 0: nx, ny = -nx, -ny
                V.decal(g, [pa, pb, (pb[0] + nx*w, pb[1] + ny*w), (pa[0] + nx*w, pa[1] + ny*w)], z, cols[(k + shimmer) % len(cols)]); k += 1

def build_pot(g, t, ev, c, y0):
    """banjo pot: wooden rim, resonator with a pierced flange, chrome tension hoop + hooks, skin head (flashes on hits)"""
    u = U; R = c["Lb"]*u/2; cy = y0 + R; d = c["d"]*u; z0, z1 = -d/2, d/2; rd = c["res"]*u
    T.tube(g, (0, cy, z0), (0, cy, z1), R, col=c["side"], n=20, spec=.2)
    T.tube(g, (0, cy, z0 - rd), (0, cy, z0), R*1.13, col=c["res_col"], n=20, spec=.35)
    g.face(S.ring((0, cy, z0 - rd), (0, 0, 1), R*1.13, 20), dim(c["res_col"], .85), ref=(0, cy, 0))
    g.face(S.ring((0, cy, z0 + .001), (0, 0, 1), R*1.13, 20), dim(c["res_col"], 1.15), ref=(0, cy, z0 - 1))
    for k in range(18):
        a = 2*math.pi*k/18
        V.decal(g, F.ngon_xy(1.075*R*math.cos(a), cy + 1.075*R*math.sin(a), .22*u, 6), z0 + .003, (24, 16, 12))
    T.tube(g, (0, cy, z1 - .15*u), (0, cy, z1 + .22*u), R*1.02, col=CHROME, n=20, spec=.9)
    hit = max([math.exp(-e[0]*14) for e in ev if e[0] < .4] + [0])
    hz = z1 + .1*u
    g.face(S.ring((0, cy, hz), (0, 0, 1), R*.985, 20), mix(c["skin"], (255, 250, 232), .45*hit), ref=(0, cy, hz - 1), emit=hit > .5)
    for k in range(16):
        a = 2*math.pi*k/16 + math.pi/16; ca, sa = math.cos(a), math.sin(a)
        p1 = (1.03*R*ca, cy + 1.03*R*sa, z1 + .12*u); p2 = (1.05*R*ca, cy + 1.05*R*sa, z0 + .35*u)
        T.tube(g, p1, p2, .05*u, col=CHROME, n=4, spec=.9)
        g.box(p2[0] - .1*u, p2[0] + .1*u, p2[1] - .1*u, p2[1] + .1*u, p2[2] - .12*u, p2[2] + .12*u, CHROME, spec=.8)
    return hz

def string_for(strings, m):
    best = None
    for i, o in enumerate(strings):
        f = m - o
        if 0 <= f <= 15 and (best is None or f < best[1]): best = (i, f)
    return best[0] if best else int(np.argmax(strings))

def build_uke(g, t, ev, fx, s, c):
    u = U; Lb, d = c["Lb"]*u, c["d"]*u; zf = d/2; y0 = Y0; banjo = c.get("banjo", False)
    stand(g, y0, -d/2 - (c.get("res", 0)*u if banjo else 0), Lb)
    if banjo: zf = build_pot(g, t, ev, c, y0)
    else:
        G.body_asym(g, c["prof"], Lb, y0, d, c["top"], c["side"], c.get("colfn"))
    yt = y0 + Lb; sc = c["scale"]*u
    yn = yt + sc*(1 - 2**(-c["fj"]/12)); yb = yn - sc
    nw, jw = c["nut_w"]*u/2, c["joint_w"]*u/2; ncol = c.get("neckcol", c["side"])
    flush = c.get("flush", False)
    if banjo:
        nz0, nz1, fbz = zf - .9*u, zf + .2*u, zf + .45*u
    else:
        nz0, nz1, fbz = zf - 1.0*u, zf, zf + (.04*u if flush else .2*u)
    V.slab_xy(g, [(-jw, yt - .45*u), (jw, yt - .45*u), (nw, yn), (-nw, yn)], nz0, nz1, ncol)
    g.box(-jw*.75, jw*.75, yt - .05*u, yt + .8*u, nz0 - .55*u, nz0 + .01, ncol)                     # heel
    yfb0 = yt - c.get("ext", 0.)*u
    def wfb(y): return jw if y <= yt else lerp(jw, nw, (y - yt)/(yn - yt))
    V.slab_xy(g, [(-wfb(yfb0), yfb0), (wfb(yfb0), yfb0), (nw, yn), (-nw, yn)], nz1, fbz, c["fbcol"], spec=.15)
    if c.get("strip"):
        V.thin(g, (0, yfb0 + .1*u), (0, yn - .1*u), .16*u, fbz + .0006, c["strip"])
    def fret_y(k): return yn - sc*(1 - 2**(-k/12))
    for k in range(1, c["nf"] + 1):
        y = fret_y(k)
        if y < yfb0 + .1*u: break
        w = wfb(y); V.thin(g, (-w, y), (w, y), .006, fbz + .0009, c.get("fret_col", FRET))
    for k in c.get("dots", ()):
        ym = (fret_y(k) + fret_y(k - 1))/2
        if ym > yfb0 + .05*u: V.decal(g, V.octo(0, ym, .1*u, .1*u, 6), fbz + .0012, c.get("dot_col", PEARL))
    g.box(-nw, nw, yn - .01, yn + .004, nz1, fbz + .009, c.get("nutcol", (238, 232, 216)))
    # ---- headstock + tuners
    outline_, (px, pys) = HEADS[c["head"]]
    hz0, hz1 = fbz - .62*u, fbz - .26*u
    slab_star(g, [(x*u, yn + y*u) for x, y in outline_], hz0, hz1, c["headcol"])
    posts = [(-px, pys[0]), (-px, pys[1]), (px, pys[1]), (px, pys[0])]
    ptop = []
    for x, y in posts:
        X, Y = x*u, yn + y*u; kind = c["tuners"]
        if kind == "pegs":
            V.decal(g, F.ngon_xy(X, Y, .11*u, 6), hz1 + .0008, dim(c["peg"], .8))
            g.box(X - .05*u, X + .05*u, Y - .05*u, Y + .05*u, hz0 - .25*u, hz0, c["peg"])
            g.box(X - .1*u, X + .1*u, Y - .32*u, Y + .32*u, hz0 - .85*u, hz0 - .22*u, c["peg"])
            ptop.append((X, Y, hz1 + .02*u))
        elif kind == "rear":
            g.box(X - .07*u, X + .07*u, Y - .07*u, Y + .07*u, hz1, hz1 + .32*u, c["post"], spec=.5)
            g.box(X - .1*u, X + .1*u, Y - .1*u, Y + .1*u, hz0 - .3*u, hz0, CHROME, spec=.7)
            g.box(X - .17*u, X + .17*u, Y - .22*u, Y + .22*u, hz0 - .82*u, hz0 - .3*u, c["button"], spec=.3)
            ptop.append((X, Y, hz1 + .3*u))
        else:   # side tuners: capstan post on the face, shaft and button out to the side
            sg = 1 if x > 0 else -1; zm = (hz0 + hz1)/2
            g.box(X - .07*u, X + .07*u, Y - .07*u, Y + .07*u, hz1, hz1 + .3*u, c["post"], spec=.7)
            x0, x1 = sg*.82*u, sg*1.12*u
            g.box(min(x0, x1), max(x0, x1), Y - .05*u, Y + .05*u, zm - .05*u, zm + .05*u, CHROME, spec=.8)
            b0, b1 = sg*1.1*u, sg*1.42*u
            g.box(min(b0, b1), max(b0, b1), Y - .2*u, Y + .2*u, zm - .1*u, zm + .1*u, c["button"], spec=.4)
            ptop.append((X, Y, hz1 + .28*u))
    # ---- soundhole, binding
    if not banjo:
        h = c["hole"]; yh = yt - h["dy"]*u; r = h["r"]*u; z = zf + .0012
        shim = int(t*7)
        for kind, *a in h["rings"]:
            if kind == "ring": V.decal(g, F.ngon_xy(0, yh, r + a[0]*u, 18), z, a[1])
            elif kind == "rope": ring_band(g, 0, yh, r + .08*u, r + a[0]*u, z, a[1])
            else: ring_band(g, 0, yh, r + .05*u, r + a[0]*u, z, ABALONE, shimmer=shim)
            z += .0006
        V.decal(g, F.ngon_xy(0, yh, r, 18), z, (16, 10, 8))
        bd = c.get("bind")
        if bd:
            cols = ABALONE if bd[0] == "abalone" else bd[1]
            binding(g, c["prof"], Lb, y0, zf + .0011, cols, .1*u if bd[0] == "plain" else .14*u, shim if bd[0] == "abalone" else 0)
        pluck_y = yh + r + .35*u
    else:
        pluck_y = yb + 1.6*u
    # ---- bridge
    sb, sn = c["sb"]*u, c["sn"]*u; br = c["bridge"]
    if br == "banjo":
        V.slab_xy(g, [(-.3*u, y0 + .15*u), (.3*u, y0 + .15*u), (.2*u, y0 + 1.3*u), (-.2*u, y0 + 1.3*u)], zf + .05*u, zf + .2*u, CHROME, spec=.8)
        g.box(-.75*u, .75*u, yb - .06*u, yb + .06*u, zf, zf + .5*u, c["bcol"])
        sz = zf + .5*u
        for i in range(4):
            g.line((lerp(-.15*u, .15*u, i/3), y0 + 1.3*u, zf + .2*u), (lerp(-sb, sb, i/3), yb, sz), dim(c["scol"][i], .75))
    elif br == "machete":
        g.box(-.95*u, .95*u, yb - .32*u, yb + .1*u, zf, zf + .16*u, c["bcol"])
        g.box(-sb - .08*u, sb + .08*u, yb - .02*u, yb + .05*u, zf + .16*u, zf + .3*u, (232, 222, 198))
        sz = zf + .3*u
    elif br == "notch":
        g.box(-1.15*u, 1.15*u, yb - .42*u, yb + .1*u, zf, zf + .3*u, c["bcol"])
        sz = zf + .3*u
    else:
        g.box(-1.2*u, 1.2*u, yb - .5*u, yb + .12*u, zf, zf + .22*u, c["bcol"])
        g.box(-sb - .1*u, sb + .1*u, yb - .03*u, yb + .05*u, zf + .22*u, zf + .36*u, c.get("saddle", (240, 234, 218)))
        sz = zf + .36*u
    B = [np.array([lerp(-sb, sb, i/3), yb, sz]) for i in range(4)]
    N = [np.array([lerp(-sn, sn, i/3), yn, fbz + .007]) for i in range(4)]
    for i in range(4): g.line(N[i], ptop[i], dim(c["scol"][i], .8))
    strings_layer(g, t, ev, fx, s, c, B, N, yb, yn, pluck_y)

def strings_layer(g, t, ev, fx, s, c, B, N, yb, yn, pluck_y):
    FL = V.FLIP[0]; st = c["strings"]; active = {}; frets = {}; strums = set()
    for age, m, v, dur, tp, cs in ev:
        if age > min(dur + .2, 1.2): continue
        on = round(t - age, 3); items = s["_items"].get(on, [])
        it = next((x for x in items if (m in x[1] if isinstance(x[1], list) else x[1] == m)), None)
        if it is not None and isinstance(it[1], list):
            for i, mm in enumerate(it[1]):
                if i < 4 and (i not in active or age < active[i]): active[i] = age; frets[i] = mm - st[i]
            strums.add((on, it[4] if len(it) > 4 else "d"))
        else:
            i = string_for(st, m)
            if i not in active or age < active[i]: active[i] = age; frets[i] = m - st[i]
    for i in range(4):
        b, n_ = B[i], N[i]
        if i in active:
            age = active[i]; amp = .006*math.exp(-age*5) + .0012
            col = HOTSTR if age < .3 else mix(HOTSTR, c["scol"][i], clamp((age - .3)/.5))
            pts = [b*(1 - k/8) + n_*(k/8) + np.array([amp*math.sin(math.pi*k/8)*FL, 0, 0]) for k in range(9)]
            for a, q in zip(pts, pts[1:]): g.line(a, q, col)
            kf = frets[i]
            if kf > 0:
                mid = ((1 - 2**(-kf/12)) + (1 - 2**(-(kf - 1)/12)))/2
                fp = n_ + (b - n_)*mid + [0, 0, .006]
                g.box(fp[0] - .012, fp[0] + .012, fp[1] - .01, fp[1] + .01, fp[2] - .003, fp[2] + .008, HOT, emit=True)
        else:
            g.line(b, n_, c["scol"][i])
    def at_pluck(i):
        k = (pluck_y - B[i][1])/(N[i][1] - B[i][1]); return B[i] + (N[i] - B[i])*k
    for on, flag in strums:
        age = t - on
        if age < .15:
            a, b = (at_pluck(0), at_pluck(3)) if flag != "u" else (at_pluck(3), at_pluck(0))
            fx.append(("strum", tuple(a + [0, 0, .03]), tuple(b + [0, 0, .03]), age))
    for age, m, v, dur, tp, cs in ev:
        if cs == 1 and age < .07: fx.append(("spark", tuple(at_pluck(string_for(st, m)) + [0, 0, .02]), age))
        if tp and v >= .84 and .0 <= age < .9: fx.append(("note", (at_pluck(3)[0] + .3, pluck_y + .45, .02), age))

def build(g, t, ev, fx, s):
    key, k_sc, rot = s["key"], 1., 0.
    if s is SCENES[-1]:                          # outro: spin the 2006 tenor out, the modern soprano back in
        L, q = s["len"], .35*BEAT_S
        if t >= L - 1e-9:
            key = "modern"; ev = [e for e in ev if e[0] <= t - L + 1e-6]
            p = (t - L)/q; k_sc = F.ease_out_back(min(p, 1.)) if p < 1 else 1.; rot = -100*(1 - ease(p))
            k_sc *= UKES["modern"]["size"]/UKES[s["key"]]["size"]
        else:
            p = (t - (L - q))/q
            if p > 0: k_sc = 1 - ease(p); rot = 100*p*p
    f0, l0, x0 = len(g.F), len(g.L), len(fx)
    build_uke(g, t, ev, fx, s, UKES[key])
    if k_sc != 1. or rot:
        a = math.radians(rot); R_ = np.array([[math.cos(a), 0, math.sin(a)], [0, 1, 0], [-math.sin(a), 0, math.cos(a)]])
        tr = lambda p: (np.asarray(p, float) @ R_.T)*k_sc
        for i in range(f0, len(g.F)): P_, n, *rest = g.F[i]; g.F[i] = (tr(P_), R_ @ n, *rest)
        for i in range(l0, len(g.L)): a_, b_, *rest = g.L[i]; g.L[i] = (tr(a_), tr(b_), *rest)
        for i in range(x0, len(fx)):
            f = fx[i]; fx[i] = (f[0], tuple(tr(f[1]))) + ((tuple(tr(f[2])),) + tuple(f[3:]) if f[0] == "strum" else tuple(f[2:]))

# ---------------------------------------------------------------- sound
def _t(n): return np.arange(n)/SR
def ks(f, dur, bright=.6, decay=.997, seed=0):
    """Karplus-Strong with a fractional-delay loop filter (in tune at high pitches)"""
    Pd = SR/f; N = int(Pd - .5); a = Pd - .5 - N
    n = int(dur*SR); r = np.random.default_rng(seed)
    buf = r.uniform(-1, 1, N + 2)
    for _ in range(int((1 - bright)*4)): buf = .5*(buf + np.roll(buf, 1))
    buf -= buf.mean()
    out = np.zeros(max(n, N + 3) + N + 3); out[:N + 2] = buf
    c0, c1, c2 = .5*(1 - a), .5, .5*a; i = N + 2
    while i < n:
        j = min(n, i + N)
        out[i:j] = decay*(c0*out[i - N:j - N] + c1*out[i - N - 1:j - N - 1] + c2*out[i - N - 2:j - N - 2]); i = j
    return out[:n]
TONES = {   # bright, loop decay, low-pass, body resonance (Hz, amount), extras
 "modern":    dict(bright=.75, decay=.9982, lp=6500, body=(380, .22)),
 "machete":   dict(bright=1.0, decay=.9991, lp=9000, body=(520, .12), metal=True),
 "honolulu":  dict(bright=.35, decay=.9976, lp=3000, body=(320, .32)),
 "kumalae":   dict(bright=.45, decay=.9978, lp=3800, body=(340, .28)),
 "banjolele": dict(bright=1.0, decay=.9962, lp=7500, banjo=True),
 "pineapple": dict(bright=.6, decay=.9981, lp=5000, body=(250, .4)),
 "islander":  dict(bright=.75, decay=.9950, lp=4800, hp=420, box=(950, .55)),
 "baritone":  dict(bright=.5, decay=.9985, lp=5200, body=(170, .32)),
 "tenor93":   dict(bright=.5, decay=.9982, lp=4400, body=(300, .3)),
 "kamaka06":  dict(bright=.85, decay=.9988, lp=7200, body=(350, .22)),
}
def lookup(s, t0, m):
    for it in s["_items"].get(round(t0, 3), []):
        if isinstance(it[1], list) and m in it[1]: return it, it[1].index(m)
        if not isinstance(it[1], list) and it[1] == m: return it, -1
    return None, -1
def synth(s, idx):
    t0, m, v, dur, tp, cs = s["ev"][idx]; key = s["key"]
    if s is SCENES[-1] and t0 >= s["len"] - 1e-6: key = "modern"
    tn = TONES[key]; it, si = lookup(s, t0, m); flag = (it[4] if it is not None and len(it) > 4 else "")
    f = midi_hz(m); final = dur > 2.0
    ring = 3.2 if final else (.32 if flag == "x" else (dur + .1 if flag == "t" else min(2.4, dur + .9)))
    x = ks(f, ring, min(1., tn["bright"] + (.1 if flag == "u" else 0)), .9989 if final else tn["decay"], idx)
    n = len(x); tt = _t(n)
    if tn.get("metal"): x = x + .35*ks(f*1.0012, ring, 1., tn["decay"], idx + 911)
    if tn.get("banjo"):
        x = x + D.bp(D.nz(n), 500, 2600)*np.exp(-tt*55)*.5 + np.sin(2*np.pi*235*tt)*np.exp(-tt*38)*.35
        x = x + .7*D.bp(x, 900, 3200)
    if flag == "x":
        x = x*np.exp(-tt*24) + D.bp(D.nz(n), 1500, 5000)*np.exp(-tt*90)*.35
    if flag == "t":
        x = x*np.where(tt < dur, 1, np.exp(-(tt - dur)/.03))
    if tn.get("hp"): x = D.hp(x, tn["hp"])
    if tn.get("body"): fc, a = tn["body"]; x = x + a*3*D.bp(x, fc*.7, fc*1.4)
    if tn.get("box"): fc, a = tn["box"]; x = x + a*3*D.bp(x, fc*.8, fc*1.25)
    x = D.lp(x, tn["lp"])
    x *= np.minimum(1, (ring - tt)/.06)
    if si >= 0:                                    # strum: string-by-string onset, down = G->A, up = A->G
        nstr = len(it[1]); k = si if flag != "u" else nstr - 1 - si
        x = np.concatenate([np.zeros(int(k*(.009 if flag == "u" else .012)*SR)), x])*(.62 if flag != "u" else .5)
    return x*v

# ---------------------------------------------------------------- backing: indie-folk stomp-clap (stomp, clap, tambourine, floor tom, whistled hook)
_R = np.random.default_rng(17)
def stomp(v):
    n = int(.5*SR); t = _t(n)
    x = D.kick_s(46, 92, 8, .05, .5, 1.25, .25) + D.bp(D.nz(n), 70, 420)*np.exp(-t*22)*.9 + np.sin(2*np.pi*118*t)*np.exp(-t*20)*.35
    return x*v
def clap(v):
    n = int(.4*SR); t = _t(n); x = np.zeros(n)
    for k in range(4):                     # a few people clapping, slightly apart
        o = int((k*.007 + _R.random()*.004)*SR)
        x[o:] += D.bp(D.nz(n - o), 800 + 300*k, 3600 + 400*k)*np.exp(-t[:n - o]*(70 - 8*k))*(.6 + .4*_R.random())
    x += D.bp(D.nz(n), 700, 3000)*np.exp(-t*14)*.18
    return x*v
def tamb(v):
    n = int(.3*SR); t = _t(n)
    j = sum(np.sin(2*np.pi*fq*t + k) for k, fq in enumerate((5200, 6100, 7300, 8150, 9400)))*.1
    return (j + D.hp(D.nz(n), 6000)*.5)*np.exp(-t*14)*v
def shake(v):
    n = int(.12*SR); t = _t(n)
    return D.hp(D.nz(n), 5500)*np.minimum(1, t/.02)*np.exp(-t*35)*.5*v
def ftom(v): return D.tom_s(80, 6.5, .3, .7)*v
def whistle(m, dur):
    def fn(v):
        f = midi_hz(m); n = int((dur + .18)*SR); t = _t(n)
        vib = 1 + .007*np.sin(2*np.pi*5.4*t)*np.clip((t - .12)/.2, 0, 1)
        fr = f*vib*(1 - .02*np.exp(-t*45))
        ph = 2*np.pi*np.cumsum(fr)/SR
        x = np.sin(ph) + .04*np.sin(2*ph) + D.bp(D.nz(n), f*.8, f*1.3)*.25
        env = np.minimum(1, t/.03)*np.where(t < dur, 1, np.exp(-(t - dur)/.06))
        return x*env*v*.9
    return fn
KIT = {"stomp": stomp, "clap": clap, "tamb": tamb, "shake": shake, "ftom": ftom}
KICK_KINDS, ACCENT_KINDS = ("stomp",), ("clap",)
WHISTLE = {1: [(.5, 76, .5), (1, 79, .5), (1.5, 81, 1), (2.5, 79, .5), (3, 76, .75), (3.75, 74, .25)],      # showcase answer (Am)
           22: [(0, 81, 1), (1, 79, .5), (1.5, 77, .5), (2, 79, 1.5), (3.5, 74, .5)],                    # outro (F, G)
           23: [(0, 76, 3)]}
def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT; nb = len(BARS)
    for b, phrase in WHISTLE.items():
        for o, m, d in phrase:
            k = f"w{m}_{d}"; KIT.setdefault(k, whistle(m, d*Q)); ev.append((b*B + o*Q, k, .85))
    for b in range(nb):
        T0 = b*B
        if b == nb - 1:
            ev += [(T0, "stomp", 1.), (T0, "clap", 1.), (T0, "ftom", 1.), (T0, "tamb", 1.)]
            ev += [(T0 + k*.125*Q, "shake", .6*(1 - k/24)) for k in range(24)]
            continue
        ev += [(T0, "stomp", 1. if b else .75), (T0 + 2*Q, "stomp", .9 if b else .7), (T0 + Q, "clap", 1. if b else .7), (T0 + 3*Q, "clap", 1. if b else .7)]
        if b >= 1:
            ev += [(T0 + k*.5*Q, "tamb" if k % 2 else "shake", .62 if k % 2 else .5) for k in range(8)]
        if b >= 2:
            ev += [(T0 + k*Q, "ftom", (.75, .45, .65, .45)[k]) for k in range(4)] + [(T0 + 3.5*Q, "ftom", .4)]
        if b >= 5 and b % 2 == 1 and b < nb - 2:
            ev += [(T0 + 3.5*Q, "ftom", .7), (T0 + 3.75*Q, "ftom", .8)]
    return ev
def _chord_at(b, beat):
    chd = BARS[min(b, len(BARS) - 1)]; return chd[int(beat)] if isinstance(chd, tuple) else chd
def _root(name):
    r = CH[name][0]
    while r > 45: r -= 12
    while r < 33: r += 12
    return r
def bass_events(c):
    out = []; nb = len(BARS)
    for b in range(1, nb):
        T0 = b*c.BAR
        if b == nb - 1: out.append((T0, _root(_chord_at(b, 0)), 2.5, 1.)); continue
        c0, c2 = _chord_at(b, 0), _chord_at(b, 2); r0 = _root(c0)
        second = r0 + 7 if c2 == c0 else _root(c2)
        out += [(T0, r0, 1.5*c.BEAT, .92), (T0 + 2*c.BEAT, second, 1.5*c.BEAT, .82)]
    return out
def bass_voice(m, dur, v):
    f = midi_hz(m); n = int((dur + .25)*SR); t = _t(n); ph = 2*np.pi*f*t
    x = np.sin(ph) + .35*np.sin(2*ph)*np.exp(-t*6) + .12*np.sin(3*ph)*np.exp(-t*9) + D.bp(D.nz(n), 60, 300)*np.exp(-t*60)*.4
    env = np.minimum(1, t/.006)*np.exp(-t*1.5)*np.where(t < dur, 1, np.exp(-(t - dur)/.06))
    return x*env*v
def pad_voice(notes, dur, v):
    n = int((dur + .5)*SR); t = _t(n); x = np.zeros(n)
    for m in notes:
        f = midi_hz(m)
        for det in (-1.2, 1.2):
            for k in (1, 2, 3, 4): x += np.sin(2*np.pi*(f + det)*k*t + k)/k**1.6
    env = np.minimum(1, t/.15)*np.where(t < dur, 1, np.exp(-(t - dur)/.3))
    return D.lp(x*env, 2200)*v*.2
MIX = dict(drums=-21, bass=-25, pad=-34, verb=-22, verb_len=1.6)

# ---------------------------------------------------------------- background: riding the barrel of a breaking wave
THEMES = {   # deep water, mid water, light water, sky top, sky bottom, sun, foam, accent
 "modern":    ((6, 50, 74), (14, 126, 148), (96, 214, 214), (110, 196, 238), (232, 246, 236), (255, 250, 222), (250, 252, 255), (180, 255, 248)),
 "machete":   ((6, 10, 38), (20, 42, 94), (74, 106, 162), (64, 14, 42), (198, 72, 84), (255, 198, 150), (236, 228, 238), (156, 30, 58)),
 "royal":     ((50, 6, 16), (134, 24, 38), (212, 92, 82), (92, 18, 30), (250, 202, 112), (255, 236, 150), (255, 236, 204), (244, 194, 82)),
 "expo":      ((96, 74, 62), (178, 152, 126), (238, 224, 198), (234, 142, 122), (252, 236, 214), (255, 246, 226), (255, 252, 244), (242, 120, 100)),
 "vaude":     ((42, 28, 16), (112, 80, 48), (188, 152, 106), (62, 44, 28), (222, 192, 142), (255, 244, 212), (250, 238, 212), (255, 236, 190)),
 "pineapple": ((64, 66, 12), (172, 152, 32), (246, 222, 92), (40, 112, 62), (222, 242, 152), (255, 255, 212), (255, 250, 212), (40, 142, 72)),
 "tv":        ((10, 60, 72), (40, 160, 172), (152, 230, 226), (62, 172, 192), (250, 240, 212), (255, 250, 230), (250, 244, 222), (250, 236, 196)),
 "hoot":      ((62, 36, 14), (172, 128, 38), (232, 192, 82), (122, 72, 30), (242, 202, 122), (255, 236, 172), (250, 236, 202), (112, 64, 30)),
 "rainbow":   ((72, 92, 142), (132, 172, 212), (202, 228, 242), (172, 202, 242), (250, 228, 238), (255, 250, 240), (255, 255, 255), (250, 192, 222)),
 "night":     ((16, 4, 34), (62, 26, 102), (124, 74, 172), (6, 2, 20), (62, 30, 102), (228, 240, 204), (200, 255, 122), (172, 255, 62)),
}
_STATE = {"ok": 0.}
def ped_cols(th):
    def pc(k): T_ = THEMES[k]; return F.ped_cols_from(mix(mix(T_[0], T_[1], .55), T_[7], .18))
    base = pc(th)
    if _STATE["ok"] > 0 and th == SCENES[-1]["theme"]:
        m_ = pc("modern"); return {k: mix(base[k], m_[k], _STATE["ok"]) for k in base}
    return base
EX, EY, ERX, ERY = CX + 6, 178, 74, 60
DXn, DYn = (XX - EX)/ERX, (YY - EY)/ERY
TH = np.arctan2(DYn, DXn)
RR = np.sqrt(DXn**2 + DYn**2)*(1 + .1*np.cos(TH - 2.5)) + 1e-6; LR = np.log(RR)   # the lip leans in from the lower left
DEPTH = np.clip((RR - 1.12)/2.6, 0, 1)
EYE = RR < 1.
TOPK = np.clip(-(.85*DYn + .5*DXn)/np.hypot(DXn, DYn).clip(1e-6), 0, 1)                                   # 1 at the top of the barrel (the throwing lip)
FACEK = (np.clip((DYn/RR - .25)/.45, 0, 1)*~EYE)[..., None]     # 1 on the wave face below us
HZN = EY + 22
SKYK = np.clip((YY - (EY - ERY))/(HZN - (EY - ERY)), 0, 1)[..., None]
SUNX, SUNY = EX + 26, HZN - 9
DSUN = np.hypot(XX - SUNX, YY - SUNY)
UPW = np.clip((EY + 70 - YY)/170, 0, 1)
TEXTB = ((YY >= 40) & (YY < 100)) | ((YY >= 330) & (YY < 380))
TEXTE = (((YY >= 36) & (YY < 40)) | ((YY >= 100) & (YY < 104)) | ((YY >= 326) & (YY < 330)) | ((YY >= 380) & (YY < 384))) & (BAY < .5)
_r = np.random.default_rng(23)
STARS = EYE & (YY < HZN - 4) & (_r.random((RH, RW)) < .012); STAR_PH = _r.random((RH, RW))*6.28
RAINR = np.hypot(XX - EX, (YY - (HZN + 30))*1.1)
RAINBOW = [(240, 90, 110), (250, 170, 90), (250, 236, 110), (120, 220, 130), (100, 170, 240), (170, 120, 230)]
SPOT = np.sin(np.arctan2(YY - SUNY, XX - SUNX)*14) > .55
FISH = ["...##.#", "#######", "...##.#"]
TURTLE = ["..###..", "#.###.#", ".#####.", "#.###.#", "..#.#.."]
CREATURES = [(FISH, .07, .0, .4), (FISH, .055, .33, 2.3), (TURTLE, .04, .6, 4.0), (FISH, .065, .8, 5.4), (TURTLE, .035, .15, 1.2),
             (FISH, .06, .5, 3.1)]

def _sprite(bg, spr, x, y, col, k, flip):
    h, w = len(spr), len(spr[0])
    for r_, row in enumerate(spr):
        for c_, px in enumerate(row[::-1] if flip else row):
            if px != "#": continue
            X0, Y0_ = int(x + (c_ - w/2)*k), int(y + (r_ - h/2)*k)
            if 0 <= X0 < RW - k and 0 <= Y0_ < RH - k: bg[Y0_:Y0_ + k, X0:X0 + k] = col
def theme_w(name, th0, th1, kb): return (1 - kb)*(th0 == name) + kb*(th1 == name)

def background(c, x):
    T_ = x["T"]; th0, th1, kb = x["th0"], x["th1"], x["kb"]
    if T_ >= c.OUTRO_T:                       # outro: back to today's turquoise water
        _STATE["ok"] = ease((T_ - c.OUTRO_T)/(.6*c.BEAT)); th0, th1, kb = SCENES[-1]["theme"], "modern", _STATE["ok"]
    else: _STATE["ok"] = 0.
    A_, B_ = THEMES[th0], THEMES[th1]
    deep, midw, light, skyt, skyb, sun, foam, acc = (np.array(mix(A_[i], B_[i], kb), float) for i in range(8))
    # water wall: depth shading; translucent sheets and dashed streaks running down the tube (they rush past us)
    fq = np.clip(np.floor(DEPTH*6 + BAY)/6, 0, 1)[..., None]
    bg = np.where(fq < .5, light*(1 - fq*2) + midw*fq*2, midw*(2 - fq*2) + deep*(fq*2 - 1))
    sheet = np.sin(8*TH + 1.2*LR - .9*T_)
    bg = np.where((sheet + BAY*.2 > 1.0)[..., None], bg*.86 + light*.14, bg)
    strk = (np.sin(17*TH + 2.4*LR - 1.1*T_) > 1 - .03/RR**2) & (np.sin(6*LR - 5.5*T_ + 3*TH) > -.1)
    wl = (1 - DEPTH)[..., None]*(1 - FACEK)*.6 + .15
    bg = np.where(strk[..., None], bg*(1 - wl) + light*wl, bg)
    # the wave face under us: lighter water, foam lines racing toward us
    facec = bg*.6 + (midw*.55 + light*.45)*.4
    rph = 9*LR - 6*T_ + 2.2*np.sin(5*TH + LR + .7*T_)
    rip = np.abs(np.sin(rph)) < .05 + .1*BAY + .04*RR
    facec = np.where(rip[..., None], facec*.5 + foam*.5, facec)
    bg = bg*(1 - FACEK) + facec*FACEK
    # sunlight ripples through the water
    n1 = np.sin(XX*.11 + 2*np.sin(YY*.07 + T_*1.2)); n2 = np.sin(YY*.13 - T_*1.5 + 2*np.sin(XX*.06 - T_*.8))
    cau = (np.abs(n1 + n2) < .1 + .08*x["loud"]) & ~EYE
    w_ = ((1 - DEPTH)**1.2*(.25 + .75*UPW))[..., None]*(1 - FACEK)*.6
    bg = np.where(cau[..., None], bg*(1 - w_) + (acc*.4 + sun*.6)*w_, bg)
    # creatures drifting through the wall
    sil = deep*.55
    for spr, sp, ph, th_ in CREATURES:
        fr = (T_*sp + ph) % 1.; R_ = 1.3*math.exp(1.25*fr); ang = th_ + 1.6*fr
        px_, py_ = EX + R_*ERX*math.cos(ang), EY + R_*ERY*math.sin(ang)
        k = 1 if R_ < 2.3 else 2
        if 104 < py_ < 326 or py_ > 384: _sprite(bg, spr, px_, py_, sil*min(1, (R_ - 1.3)/.3) + bg[int(np.clip(py_, 0, RH - 1)), int(np.clip(px_, 0, RW - 1))]*(1 - min(1, (R_ - 1.3)/.3)), k, math.sin(ang) > 0)
    # the eye of the barrel: sky, horizon, sun
    sky = skyt*(1 - SKYK) + skyb*SKYK
    far = midw*.55 + light*.45
    eyec = np.where((YY >= HZN)[..., None], far, sky)
    gl = ((YY >= HZN) & ((YY - HZN) % 3 == 0) & (np.sin(XX*.31 + T_*3 + YY) > .6))[..., None]
    eyec = np.where(gl, sun*.8 + far*.2, eyec)
    sr = 11 + 2.5*x["pulse"]
    glow = (DSUN < sr + 9) & (BAY < .5*(1 - (DSUN - sr)/9)) & (YY < HZN)
    eyec = np.where(glow[..., None], eyec*.5 + sun*.5, eyec)
    eyec = np.where(((DSUN < sr) & (YY < HZN))[..., None], sun, eyec)
    wr = theme_w("rainbow", th0, th1, kb)
    if wr > 0:
        for i, rc in enumerate(RAINBOW):
            band = (RAINR >= 86 - i*4) & (RAINR < 90 - i*4) & (YY < HZN)
            eyec = np.where(band[..., None], eyec*(1 - .55*wr) + np.array(rc, float)*.55*wr, eyec)
    wn = theme_w("night", th0, th1, kb)
    if wn > 0:
        tw = STARS & (np.sin(STAR_PH + T_*3) > -.2); eyec[tw] = eyec[tw]*(1 - wn) + 235*wn
    wv = theme_w("vaude", th0, th1, kb)
    if wv > 0: eyec = np.where((SPOT & (YY < HZN))[..., None], eyec*(1 - .22*wv) + sun*.22*wv, eyec)
    bg = np.where(EYE[..., None], eyec, bg)
    # foamy lip around the eye + foam bursts on every clap
    if not hasattr(c, "_claps"): c._claps = [t for t, k, v in c.DRUMS if k == "clap"]
    ca = [T_ - t for t in c._claps if 0 <= T_ - t < .7]
    flash = max([math.exp(-a*9) for a in ca] + [0])
    lump = .5 + .25*np.sin(9*TH + 2.1*T_) + .25*np.sin(17*TH - 3.3*T_ + 1.)
    edge = 1.025 + TOPK*(.05 + .12*lump) + .03*flash
    lip = ~EYE & (RR < edge + .02*BAY)
    spray = ~EYE & ~lip & (RR < edge + .1*TOPK) & (np.sin(53*TH + 9*T_)*np.sin(31*TH - 7*T_ + RR*20) > .8)
    bg[spray] = bg[spray]*.4 + foam*.6
    bg[lip] = bg[lip]*.15 + foam*.85
    for a in ca:
        base = int(round((T_ - a)*100))
        for k in range(20):
            hsh = ((base*31 + k*2654435761) % 1009)/1009.; ang = hsh*6.283
            rr = 1.08 + 1.5*(1 - math.exp(-a*4))*(.6 + .4*((hsh*7.1) % 1))
            px_, py_ = int(EX + rr*ERX*math.cos(ang)), int(EY + rr*ERY*math.sin(ang) + a*a*40)
            s_ = 2 if a < .3 else 1; al = 1 - a/.7
            if 0 <= px_ < RW - 2 and 0 <= py_ < RH - 2: bg[py_:py_ + s_, px_:px_ + s_] = bg[py_:py_ + s_, px_:px_ + s_]*(1 - al) + foam*al
    wt = theme_w("tv", th0, th1, kb)
    if wt > 0: bg[YY % 3 == 0] *= 1 - .12*wt
    bg[TEXTB] *= .55; bg[TEXTE] *= .75
    return np.clip(bg, 0, 255).astype(np.uint8)

if __name__ == "__main__":
    F.run(sys.modules[__name__])
