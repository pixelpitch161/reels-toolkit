"""Pixel Pitch: history of the electric guitar, 60-second cut.  python3 electric60_lowpoly.py [out.mp4] | --still t ..."""
import math, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import pp_framework as F
import pp_guitar as G
import piano_lowpoly as P
import electric_lowpoly as E
from pp_framework import mix, dim, lerp, clamp, midi_hz, seq, SR

OUT, TAG = "electric60_lowpoly.mp4", "e6"
BPM = 116
SHOW_BARS, SCENE_BARS, OUTRO_BARS = 4, 2, 2
DIST, ISCALE, YAW, SWAY, FOOT = 3.0, .72, -12, 20, .34
HEAD_INTRO, HEAD, TODAY, START_YEAR = "ELECTRIC GUITAR", "90 YEARS", "TODAY", 1900
INTRO_LINES = E.INTRO_LINES
OUTRO_LINES = ("90 YEARS FROM", "A FRYING PAN")
OUTRO_CREDIT = "MODERN ROCK RIFF"

# ---------------------------------------------------------------- music
CH = dict(E.CH); CH.update({"D5": (38, (50, 57, 62)), "Bb5": (46, (53, 58, 62)), "C5": (36, (55, 60, 64)), "F5": (41, (53, 57, 60)),
                            "G5": (43, (55, 59, 62)), "A7": (45, (55, 57, 61)), "D7": (38, (54, 57, 60))})
PC = {"D5": [38, 45, 50], "Bb5": [46, 53, 58], "C5": [48, 55, 60], "F5": [41, 48, 53], "G5": [43, 50, 55]}
def chug(b0, chords, accents=(0, 3, 4, 7)):
    out = []
    for k in range(8):
        ch = chords[0] if k < 4 else chords[-1]
        out.append((b0 + k*.5, PC[ch], .45, .9 if k in accents else .55))
    return out
BARS = ["D5", ("Bb5", "Bb5", "C5", "C5"), "D5", ("F5", "F5", "G5", "G5"),
        "A", "E", "F", "C7", "A7", "D7", "G", "D", "C", "Am", "A", "D", "E", "E", "Am", "D7", "E", "A", "D", "G",
        "Am", ("F", "F", "G", "G"),
        "D5", ("Bb5", "Bb5", "C5", "C5"), "D5"]
SH = E.SCENES
SCENES = [
 dict(key="modern", theme="intro", name="MODERN ELECTRIC", credit="MODERN ROCK RIFF",
      notes=chug(0, ["D5"], (0, 3, 6)) + chug(4, ["Bb5", "C5"]) + chug(8, ["D5"], (0, 3))[:4] +
            [(10, 62, .5, .85), (10.5, 65, .5, .85), (11, 67, .25, .85), (11.25, 69, .25, .85), (11.5, 72, .5, .9)] + chug(12, ["F5", "G5"])),
 dict(SH[1], theme="pan"),
 dict(SH[2], theme="es150", credit="SWING JAZZ, CHARLIE CHRISTIAN STYLE"),
 dict(key="log", theme="log", year=1941, ys="1941", name="THE LOG", credit="1940S BOOGIE-WOOGIE",
      fact=("A PINE PLANK WITH", "PICKUPS: NO FEEDBACK"),
      notes=seq(0, [(m, .5) for m in [45, 49, 52, 54, 55, 54, 52, 49]], .8) + seq(4, [(m, .5) for m in [50, 54, 57, 59, 60, 59, 57, 54]], .8)),
 dict(SH[3], theme="tele", credit="HONKY-TONK COUNTRY TWANG"),
 dict(SH[4], theme="lespaul", credit="LES PAUL'S MULTI-TRACK POP"),
 dict(SH[5], theme="strat", credit="1950S ROCK AND ROLL"),
 dict(key="jazzmaster", theme="jazzmaster", year=1958, ys="1958", name="JAZZMASTER", credit="SURF ROCK TREMOLO PICKING",
      fact=("SURF BANDS LOVED ITS", "OFFSET BODY"),
      notes=sum([[(k + j*.25, m, .25, .8 if j == 0 else .65) for j in range(4)] for k, m in enumerate([64, 65, 68, 69, 71, 72, 69, 68])], [])),
 dict(key="flyingv", theme="flyingv", year=1958, ys="1958", name="FLYING V", credit="IN THE STYLE OF ALBERT KING",
      fact=("ALBERT KING'S V:", "BIG BLUES BENDS"),
      notes=seq(0, [(72, .5), (74, 1.5), (72, .5), (69, .5), (67, .5), (69, .5)], .85) + seq(4, [(74, .5), (76, 1.5), (74, .5), (72, .5), (69, 1)], .85)),
 dict(key="sg", theme="sg", year=1961, ys="1961", name="GIBSON SG", credit="1960S BRITISH BLUES-ROCK",
      fact=("THIN, LIGHT, AND", "BUILT TO BE LOUD"),
      notes=seq(0, [(52, .5), (55, .5), (57, .5), (58, .25), (57, .25), (55, .5), (52, .5), (50, .5), (52, .5)], .85) +
            seq(4, [(57, .5), (60, .5), (62, .5), (63, .25), (62, .25), (60, .5), (57, .5), (55, .5), (57, .5)], .85)),
 dict(key="rick12", theme="rick12", year=1963, ys="1963", name="RICKENBACKER 12", credit="1960S JANGLE POP",
      fact=("12 STRINGS: THE SOUND", "OF 60S JANGLE"),
      notes=seq(0, [(m, .5) for m in [50, 57, 62, 66, 69, 66, 62, 57]], .75) + seq(4, [(m, .5) for m in [43, 50, 55, 59, 62, 59, 55, 50]], .75)),
 dict(key="superstrat", theme="super", year=1985, ys="1980s", name="SUPERSTRAT", credit="1980S TAPPING SHRED",
      fact=("FAST NECKS AND FLOYD", "ROSE: THE SHRED ERA"),
      notes=seq(0, [(m, .25) for m in [57, 60, 64, 69, 72, 69, 64, 60]*2], .8) +
            seq(4, [(m, .25) for m in [53, 57, 60, 65, 69, 65, 60, 57, 55, 59, 62, 67, 71, 67, 62, 59]], .8) +
            chug(8, ["D5"], (0, 3, 6)) + chug(12, ["Bb5", "C5"]) + [(16, [38, 45, 50, 57], 4.3, 1.0)]),
]

# ---------------------------------------------------------------- instruments
ROSE, MAPLE, SC6, SIX = E.ROSE, E.MAPLE, E.SC6, E.SIX
GC = dict(E.GC); GC["modern"] = E.GC["superstrat"]
GC["log"] = dict(L=.8, d=.1, prof=[(0, .07), (1, .07)], top=(214, 174, 112), side=(180, 140, 86),
                 horns=[[(-.07, .06), (-.3, .14), (-.38, .32), (-.33, .55), (-.24, .72), (-.07, .78)],
                        [(.07, .06), (.07, .78), (.24, .72), (.33, .55), (.38, .32), (.3, .14)]], horn_col=(150, 80, 34),
                 neck=.7, fb0=.82, fbcol=ROSE, frets=20, head="gibson", headcol=(40, 30, 24), strings=SIX, scol=SC6, sb=.07, sn=.034,
                 pickups=[(.6, "single", (30, 28, 30)), (.32, "single", (30, 28, 30))], bridge="trapeze", bridge_u=.22, pluck_u=.42)
GC["jazzmaster"] = dict(L=.8, d=.09, prof=[(0, .15, .13), (.06, .33, .3), (.18, .38, .34), (.32, .36, .33), (.45, .31, .3), (.55, .29, .29),
                        (.65, .31, .33), (.74, .29, .31), (.8, .25, .27)], top=(142, 198, 170), side=(112, 160, 136),
                        horns=[[(-.25, .79), (-.08, .79), (-.09, .98), (-.17, 1.1), (-.25, 1.0)],
                               [(.08, .79), (.27, .79), (.26, .9), (.16, .98), (.11, .92)]], horn_col=(142, 198, 170), neck_u=.82,
                        neck=.7, fb0=.84, fbcol=ROSE, inlay_col=(240, 240, 236), frets=21, head="fender", headcol=MAPLE, strings=SIX,
                        scol=SC6, sb=.075, sn=.034, guards=[([(-.22, .76), (.06, .8), (.12, .6), (.26, .32), (.2, .16), (-.06, .2), (-.24, .44)], (96, 42, 24))],
                        pickups=[(.62, "single", (236, 230, 214)), (.32, "single", (236, 230, 214))], bridge="trem", bridge_u=.2,
                        pluck_u=.4, knobs=[(.2, .2), (.24, .12)])
GC["flyingv"] = dict(L=.85, d=.09, prof=[(.7, .05), (1, .1)], top=(218, 180, 108), side=(182, 142, 82),
                     horns=[[(-.1, 1.0), (-.42, .12), (-.26, 0.0), (0, .3), (.04, .86)],
                            [(.1, 1.0), (-.04, .86), (0, .3), (.26, 0.0), (.42, .12)]], horn_col=(218, 180, 108),
                     neck=.68, fb0=.92, fbcol=ROSE, frets=22, head="gibson", headcol=(218, 180, 108), strings=SIX, scol=SC6,
                     sb=.072, sn=.034, guards=[([(-.12, .92), (.12, .92), (.2, .58), (0, .76), (-.2, .58)], (236, 232, 222))],
                     pickups=[(.66, "hum", (30, 30, 34)), (.48, "hum", (30, 30, 34))], bridge="stop", bridge_u=.38, pluck_u=.56)
GC["sg"] = dict(L=.78, d=.08, prof=[(0, .12, .12), (.06, .3, .3), (.18, .35, .35), (.32, .34, .34), (.45, .3, .3), (.55, .28, .28),
                (.65, .3, .3), (.72, .28, .28), (.76, .24, .24)], top=(156, 30, 26), side=(112, 20, 18),
                horns=[[(-.24, .75), (-.08, .75), (-.08, .86), (-.15, 1.0), (-.22, .9)], [(.08, .75), (.24, .75), (.22, .9), (.15, 1.0), (.08, .86)]],
                horn_col=(156, 30, 26), neck_u=.78, neck=.7, fb0=.86, fbcol=ROSE, frets=22, head="gibson", headcol=(24, 20, 20),
                strings=SIX, scol=SC6, sb=.072, sn=.034, guards=[([(-.2, .62), (-.06, .72), (.08, .72), (.22, .6), (.12, .3), (-.12, .3)], (24, 22, 26))],
                pickups=[(.6, "hum", (214, 214, 222)), (.32, "hum", (214, 214, 222))], bridge="stop", bridge_u=.22, pluck_u=.42,
                knobs=[(.2, .12), (.27, .18)])
GC["rick12"] = dict(L=.8, d=.12, prof=E.STRAT, top=(200, 70, 30), colfn=E.burst((110, 18, 10), (236, 120, 40)), side=(110, 18, 10),
                    horns=[[(-.27, .79), (-.08, .79), (-.08, .95), (-.2, 1.06), (-.28, .92)], [(.08, .79), (.3, .79), (.31, .9), (.22, .96)]],
                    horn_col=(150, 36, 16), neck_u=.82, neck=.7, fb0=.84, fbcol=ROSE, inlay_col=(240, 240, 236), frets=21,
                    head="gibson", headcol=(240, 236, 226), strings=SIX, scol=SC6, courses=True, sb=.075, sn=.036, hole=dict(type="f"),
                    guards=[([(.04, .75), (.22, .72), (.26, .5), (.14, .3), (.04, .4)], (242, 240, 232))],
                    pickups=[(.62, "single", (220, 220, 228)), (.36, "single", (220, 220, 228))], bridge="trapeze", bridge_u=.24, pluck_u=.44)
def build(g, t, ev, fx, s): G.build_guitar(g, t, ev, fx, s, GC[s["key"]])

# ---------------------------------------------------------------- sound
def _t(n): return np.arange(n)/SR
def glide_pluck(f, fprev, dur, glide=.12, decay=1.0):
    n = int((dur + .4)*SR); t = _t(n)
    fr = f*((fprev/f)**(1 - np.clip(t/glide, 0, 1)))*(1 + .007*np.sin(2*np.pi*5.5*t)*np.clip((t - .25)/.2, 0, 1))
    ph = 2*np.pi*np.cumsum(fr)/SR; x = np.zeros(n)
    for k in range(1, 13):
        if f*k > 8000: break
        x += np.sin(k*ph)*np.exp(-t*decay*(1 + .25*k))/k
    return x*np.minimum(1, t/.003)*np.where(t < dur + .05, 1, np.exp(-(t - dur - .05)/.06))*.6
def synth(s, idx):
    key = s["key"]; nt, m, v, dur, tp, cs = s["ev"][idx]; f = midi_hz(m)
    if key in ("pan", "es150", "tele", "lespaul", "strat"): return E.synth(s, idx)
    if key == "log": x = E.damp(E.amp(P.ks_pluck(f, 1.2, .7, .997, idx)*1.6, 1.5, .45), dur)
    elif key == "jazzmaster": x = E.damp(E.amp(P.ks_pluck(f, 1.0, .9, .998, idx)*1.3, 1.3, .28), dur, .04)
    elif key == "flyingv":
        bend = dur >= .5*BEAT
        x = E.amp(glide_pluck(f, midi_hz(m - 2) if bend else f, dur)*2.5, 3.0, .4)
    elif key == "sg": x = E.damp(E.amp(P.ks_pluck(f, 1.2, .9, .998, idx)*4, 4.0, .45), dur)
    elif key == "rick12":
        x = P.ks_pluck(f, 1.4, .9, .998, idx) + .5*P.ks_pluck(f*1.003, 1.4, .9, .998, idx + 7) + .4*P.ks_pluck(2*f, 1.4, .9, .997, idx + 13)
        x = E.damp(E.amp(x*1.2, 1.3, .3), dur, .15)
    else:   # modern / superstrat: high gain, palm-muted chugs, ringing final chord
        x = P.ks_pluck(f, max(dur + .3, .5), 1.0, .999, idx)
        mute = cs >= 3 and v < .8; final = cs >= 3 and nt >= 16*BEAT
        x = E.damp(E.amp(x*7, 7.0, .5), 2.4 if final else (.1 if mute else dur), .04 if mute else .06)
    if cs >= 3:
        same = sorted(e[1] for e in s["ev"] if abs(e[0] - nt) < 1e-6)
        x = np.concatenate([np.zeros(int(.006*same.index(m)*SR)), x])
    return x*v

KIT, KICK_KINDS, ACCENT_KINDS = E.KIT, E.KICK_KINDS, E.ACCENT_KINDS
def drum_events(c):
    ev = []; B, Q = c.BAR, c.BEAT; nb = len(BARS)
    for b in range(nb):
        T0 = b*B
        if b == nb - 1: ev += [(T0, "kick", 1.), (T0, "crash", 1.), (T0, "snare", 1.)]; continue
        if b == 0: ev += [(T0 + k*.5*Q, "hat", .45) for k in range(8)] + [(T0, "kick", .9), (T0 + 2*Q, "kick", .9)]; continue
        ev += [(T0, "kick", 1.), (T0 + 2*Q, "kick", .95), (T0 + 2.5*Q, "kick", .8), (T0 + Q, "snare", 1.), (T0 + 3*Q, "snare", 1.)]
        ev += [(T0 + k*.5*Q, "hat", .6 if k % 2 == 0 else .4) for k in range(7)] + [(T0 + 3.5*Q, "ohat", .5)]
        if b >= 4 and b % 2 == 0: ev.append((T0, "crash", .75))
        if b in (3, nb - 4, nb - 2): ev += [(T0 + (3 + k*.25)*Q, f"tom{k}", .8) for k in range(4)]
    return ev
def bass_events(c):
    out = []; nb = len(BARS)
    for b in range(1, nb):
        T0, ch = b*c.BAR, BARS[b]
        seqc = list(ch) if isinstance(ch, tuple) else [ch]*4
        if b == nb - 1: out.append((T0, 38 - 12 + 12, 2.6, 1.)); continue
        for k in range(8):
            r = CH[seqc[k//2]][0]; r = r if r < 41 else r - 12
            out.append((T0 + k*.5*c.BEAT, r, .45*c.BEAT, .9 if k % 2 == 0 else .75))
    return out
bass_voice, pad_voice, MIX = E.bass_voice, None, E.MIX

# ---------------------------------------------------------------- background (neon synthwave) + new themes
E.THEMES.update({
 "super":      E.THEMES["intro"],
 "log":        ((10, 16, 22), (60, 92, 100), (160, 224, 220), (240, 240, 220), (120, 180, 180)),
 "jazzmaster": ((4, 24, 40), (20, 120, 160), (255, 140, 120), (255, 230, 180), (255, 120, 100)),
 "flyingv":    ((6, 8, 30), (30, 40, 120), (100, 140, 255), (200, 220, 255), (90, 110, 230)),
 "sg":         ((12, 2, 24), (60, 10, 110), (190, 80, 255), (255, 200, 255), (160, 60, 240)),
 "rick12":     ((30, 60, 120), (150, 200, 240), (255, 255, 255), (255, 250, 210), (255, 210, 140)),
})
for k in ("super", "log", "jazzmaster", "flyingv", "sg", "rick12"):
    E.SKY[k] = F.make_grad(E.THEMES[k][0], E.THEMES[k][1], y0=0, y1=E.HY)
background, ped_cols = E.background, E.ped_cols

if __name__ == "__main__":
    F.run(sys.modules[__name__])
