"""Shared engine for the Pixel Pitch instrument-history videos (the cello_lowpoly.py formula).

A video script defines its scenes, chords, instrument builder, synth, beat and background,
then calls run(sys.modules[__name__]).  Timeline: scene 0 = showcase of the modern instrument
(2 bars), scenes 1-6 = history (2 bars each), last scene continues into a 1-bar outro and a
final chord.
"""
import math, subprocess, sys, wave
import numpy as np
from PIL import Image, ImageDraw
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import piano_lowpoly as P
import violin_lowpoly as V          # also registers the & ( ) glyphs
from piano_lowpoly import (Geo, render_geo, outline, Cam, text_w, draw_text, center_text, mix, ease, ease_out_back,
                           clamp, dim, lerp, midi_hz, sprite, NOTE_SPR, SPARK, build_pedestal, B4, RW, RH, UP,
                           FPS, SR, CREAM, GOLD, GOLD_D, INK, HOT, HOTSTR, CX)
P._G.setdefault("+", "".join([".....", "..#..", "..#..", "#####", "..#..", "..#..", "....."]))
YY, XX = np.mgrid[0:RH, 0:RW]
BAY = B4[YY % 4, XX % 4]
SHADOW = (8, 4, 16)

def make_grad(top, bot, K=8, y0=0, y1=RH):
    g = np.clip((YY - y0)/(y1 - y0), 0, 1)
    pal = np.array([mix(top, bot, i/(K-1)) for i in range(K)], float)
    return pal[np.clip(np.floor(g*(K-1) + BAY), 0, K-1).astype(int)]

def seq(start, items, vel=.8):
    out, b = [], start
    for m, d in items: out.append((b, m, d, vel)); b += d
    return out

from scipy.signal import lfilter as _lf, butter as _bt
_HP60, _HP1700 = _bt(2, 60/(SR/2), "high"), _bt(1, 1700/(SR/2), "high")
def kweight(x):
    """rough K-weighting (ear-like loudness): cut deep lows, lift the presence range"""
    y = _lf(*_HP60, x); return y + .58*_lf(*_HP1700, y)

def ped_cols_from(b):
    return dict(low_side=dim(b, .55), low_top=dim(b, .85), side=dim(b, .8), top=mix(b, (255, 255, 255), .14), shadow=dim(b, .45))

def decal_star(g, pts, z, col, center=None):
    cx = sum(p[0] for p in pts)/len(pts) if center is None else center[0]
    cy = sum(p[1] for p in pts)/len(pts) if center is None else center[1]
    for k in range(len(pts)):
        a, b = pts[k], pts[(k+1) % len(pts)]
        g.face([(cx, cy, z), (a[0], a[1], z), (b[0], b[1], z)], col, ref=(cx, cy, z - 1), emit=True)

def ngon_xy(cx, cy, r, n=12, ry=None):
    ry = r if ry is None else ry
    return [(cx + r*math.cos(2*math.pi*k/n), cy + ry*math.sin(2*math.pi*k/n)) for k in range(n)]

# ---------------------------------------------------------------- setup
def setup(c):
    c.BEAT = 60/c.BPM; c.BAR = 4*c.BEAT
    c.SHOW_BARS = getattr(c, "SHOW_BARS", 2); c.SCENE_BARS = getattr(c, "SCENE_BARS", 2); c.OUTRO_BARS = getattr(c, "OUTRO_BARS", 1)
    tt = 0.0
    for si, s in enumerate(c.SCENES):
        s["t0"] = tt; s["len"] = (c.SHOW_BARS if si == 0 else c.SCENE_BARS)*c.BAR; tt += s["len"]; s["ev"] = []
        for item in s["notes"]:
            beat, ms, dur, v = item[:4]
            ml = list(ms) if isinstance(ms, (list, tuple)) else [ms]
            for m in ml: s["ev"].append((beat*c.BEAT, m, v, dur*c.BEAT, m == max(ml), len(ml)))
        s["ev"].sort()
    on = sorted({round(s["t0"] + e[0], 4) for s in c.SCENES for e in s["ev"] if e[4]})
    c.ONSETS = []
    for o in on:
        if not c.ONSETS or o - c.ONSETS[-1] >= .2: c.ONSETS.append(o)
    c.OUTRO_T = c.SCENES[-1]["t0"] + c.SCENES[-1]["len"]
    c.FINAL_T = c.OUTRO_T + c.OUTRO_BARS*c.BAR
    c.TOTAL = c.FINAL_T + 2.6
    c.DRUMS = sorted(c.drum_events(c))
    c.KICKS = [t for t, k, v in c.DRUMS if k in c.KICK_KINDS]
    c.ACCENTS = [t for t, k, v in c.DRUMS if k in c.ACCENT_KINDS]

# ---------------------------------------------------------------- audio
def render_audio(c, path):
    n = int((c.TOTAL + 3)*SR)
    drums, bassb, padb, inst = (np.zeros(n) for _ in range(4))
    def add(buf, T0, x):
        a = int(T0*SR); e = min(n, a + len(x))
        if e > a: buf[a:e] += x[:e-a]
    for T0, k, v in c.DRUMS: add(drums, T0, c.KIT[k](v))
    for T0, m, dur, v in c.bass_events(c): add(bassb, T0, c.bass_voice(m, dur, v))
    if getattr(c, "pad_voice", None):
        for b, ch in enumerate(c.BARS):
            if isinstance(ch, tuple):
                for k, cc in enumerate(ch): add(padb, b*c.BAR + k*c.BEAT, c.pad_voice(c.CH[cc][1], c.BEAT + .05, .5))
            else:
                add(padb, b*c.BAR, c.pad_voice(c.CH[ch][1], 2.6 if b == len(c.BARS) - 1 else c.BAR + .05, .5 if b else .35))
    def rms(x):
        x = x[np.abs(x) > 1e-6]; return np.sqrt((x**2).mean()) if len(x) else 1.
    def at(x, db): return x*(10**(db/20))/rms(x) if np.abs(x).max() > 0 else x
    segs = []
    for si, s in enumerate(c.SCENES):
        last = si == len(c.SCENES) - 1
        main = np.zeros(n); tail = np.zeros(n)
        for idx, e in enumerate(s["ev"]):
            add(tail if (last and e[0] >= s["len"] - 1e-6) else main, s["t0"] + e[0], c.synth(s, idx))
        segs.append((main, s["t0"], s["t0"] + s["len"], s.get("trim", 0.)))
        if last and np.abs(tail).max() > 0: segs.append((tail, c.OUTRO_T, c.TOTAL, s.get("trim", 0.)))
    c.SEG_LEVELS = []
    for buf, a, b, trim in segs:          # trim: optional per-scene dB nudge after leveling
        ia, ib = int(a*SR), int(b*SR)
        lv = np.sqrt((kweight(buf)[ia:ib]**2).mean()) + 1e-9
        inst += buf*(10**((trim - 15)/20))/lv
    mx = c.MIX
    drums, bassb, padb = at(drums, mx["drums"]), at(bassb, mx["bass"]), at(padb, mx.get("pad", -32))
    wet_in = inst + padb*.6
    r = np.random.default_rng(1); irn = int(mx.get("verb_len", 1.5)*SR)
    ir = r.normal(0, 1, irn)*np.exp(-np.arange(irn)/SR*(5.4/mx.get("verb_len", 1.5))); ir[0] = 0; ir /= np.sqrt((ir**2).sum())
    Lf = 1 << (n + irn).bit_length()
    wet = np.fft.irfft(np.fft.rfft(wet_in, Lf)*np.fft.rfft(ir, Lf), Lf)[:n]
    out = drums + bassb + padb + inst + at(wet, mx.get("verb", -23))
    out = out[:int(c.TOTAL*SR)]
    fade = int(.8*SR); out[-fade:] *= np.linspace(1, 0, fade)
    base = out/np.abs(out).max(); tgt = getattr(c, "TARGET_LOUD", -11.0)
    for k in np.linspace(1, 9, 161):      # master every video to the same perceived loudness
        out = np.tanh(k*base)/np.tanh(k)*.94
        if 10*np.log10((kweight(out)**2).mean()) >= tgt: break
    with wave.open(path, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(SR); w.writeframes((out*32767).astype(np.int16).tobytes())
    return out

def analyze(c, x):
    nfr = int(c.TOTAL*FPS) + 2; W = 4096; win = np.hanning(W)
    edges = np.geomspace(45, 5000, 15); fr = np.fft.rfftfreq(W, 1/SR)
    masks = [(fr >= edges[b]) & (fr < edges[b+1]) for b in range(14)]
    E = np.zeros((nfr, 14)); Lo = np.zeros(nfr)
    for f in range(nfr):
        cc = int(f/FPS*SR); seg = x[max(0, cc - W//2): cc + W//2]
        if len(seg) < W: seg = np.pad(seg, (0, W - len(seg)))
        sp = np.abs(np.fft.rfft(seg*win))**2
        E[f] = [sp[m].mean() for m in masks]; Lo[f] = np.sqrt((seg**2).mean())
    E = 10*np.log10(E + 1e-12)
    lo, hi = np.percentile(E, 20, axis=0), np.percentile(E, 98, axis=0)
    E = np.clip((E - lo)/(hi - lo + 1e-9), 0, 1)
    Lo = np.clip(Lo/np.percentile(Lo, 97), 0, 1)
    for f in range(1, nfr):
        E[f] = np.maximum(E[f], E[f-1]*.82); Lo[f] = max(Lo[f], Lo[f-1]*.9)
    c.EQ, c.LOUD, c.CUM = E, Lo, np.cumsum(Lo)/FPS

# ---------------------------------------------------------------- frame
def year_str(y): return f"{-y} BC" if y < 0 else (f"{y} AD" if y < 1000 else str(y))

def frame(c, T, fi=0):
    V.FLIP[0] = 1 if fi % 2 == 0 else -1
    si = max(i for i, sc in enumerate(c.SCENES) if sc["t0"] <= T); s = c.SCENES[si]; nlast = len(c.SCENES) - 1
    th1 = s["theme"]; th0 = c.SCENES[si-1]["theme"] if si > 0 else th1
    t = T - s["t0"]
    kb = ease(t/(.5*c.BEAT)) if si > 0 else 1.0
    lk = max([k for k in c.KICKS if k <= T], default=-9); la = max([k for k in c.ACCENTS if k <= T], default=-9)
    ctx = dict(T=T, fi=fi, th0=th0, th1=th1, kb=kb, pulse=math.exp(-(T - lk)*7), accent=math.exp(-(T - la)*6),
               eq=c.EQ[fi], loud=c.LOUD[fi], cum=c.CUM[fi], ages=[T - o for o in c.ONSETS if 0 <= T - o < 1.6],
               beat_i=int(T/c.BEAT), beat_ph=(T % c.BEAT)/c.BEAT, si=si)
    pin = t/(.35*c.BEAT); scale = ease_out_back(pin); spin = -100*(1 - ease(pin))
    if si < nlast:
        pout = (t - (s["len"] - .35*c.BEAT))/(.35*c.BEAT)
        if pout > 0: scale *= 1 - ease(pout); spin += 100*pout*pout
    fx = []
    if getattr(c, "render3d", None) is not None:          # optional: the video renders its own lit 3D scene
        img, cam = c.render3d(c, ctx, s, t, scale, spin, fx)
    elif getattr(c, "LIGHTING", False):                    # shared lighting + shadows (pp_light.py)
        import pp_light
        img, cam = pp_light.render_lit(c, ctx, s, t, scale, spin, fx)
    else:
        img = c.background(c, ctx)
        pc0, pc1 = c.ped_cols(th0), c.ped_cols(th1); pc = {k: mix(pc0[k], pc1[k], kb) for k in pc1}
        rend = np.zeros((RH, RW, 3), np.uint8); zb = np.zeros((RH, RW)); gb = np.zeros((RH, RW), np.int8)
        yaw = c.YAW + c.SWAY*math.sin(2*math.pi*T/8) + spin
        cam = Cam(c.DIST, yaw, scale*c.ISCALE*s.get("size", 1.0))
        gp = Geo(); build_pedestal(gp, c.DIST, ctx["pulse"], c.FOOT*scale, pc)
        render_geo(gp, cam, rend, zb, gb, scale=1.0)
        if scale > .01:
            ev = [(t - e[0],) + tuple(e[1:]) for e in s["ev"] if t >= e[0]]
            gi = Geo(); c.build(gi, t, ev, fx, s); render_geo(gi, cam, rend, zb, gb)
        outline(rend, zb, gb)
        m = gb > 0; img[m] = rend[m]
    im = Image.fromarray(img); d = ImageDraw.Draw(im)
    hl = c.HEAD_INTRO if si == 0 else c.HEAD
    sc = 3 if text_w(hl, 3) <= 250 else 2
    hw = text_w(hl, sc); hh = 7*sc
    downbeat = math.exp(-((T % c.BAR))*6)
    d.rectangle([(RW - hw)//2 - 8, 9, (RW + hw)//2 + 7, 15 + hh + 6], fill=INK)
    d.rectangle([(RW - hw)//2 - 8, 9, (RW + hw)//2 + 7, 15 + hh + 6], outline=mix(GOLD_D, (255, 230, 150), downbeat))
    center_text(d, 13 + (21 - hh)//2, hl, CREAM, sc)
    outro = T >= c.OUTRO_T
    if si == 0 or outro: ys = c.TODAY
    else:
        prev = c.SCENES[si-1]["year"] if si > 1 else c.START_YEAR
        k = ease(t/c.BEAT)
        ys = year_str(int(lerp(prev, s["year"], k))) if k < 1 else s["ys"]
    center_text(d, 45, ys, GOLD, 3)
    name = getattr(c, "OUTRO_NAME", s["name"]) if outro else s["name"]   # optional: a different name for the outro instrument
    center_text(d, 74, name, CREAM, 2 if text_w(name, 2) <= 262 else 1)
    credit = c.OUTRO_CREDIT if outro else s["credit"]
    center_text(d, 92, credit, mix(SHADOW, (226, 196, 130), ease((t - .3)/.3)), 1)
    if si == 0:
        steps = getattr(c, "INTRO_STEPS", None)   # optional: ((start fraction, lines, colours), ...) replaces the 2-step intro
        if steps:
            k = max(i for i, st in enumerate(steps) if t >= st[0]*s["len"]); f0, lines, cols = steps[k]
            fk = ease((t - .2)/.3) if k == 0 else ease((t - f0*s["len"])/.25)
        else:
            first = t < .75*s["len"]
            lines = c.INTRO_LINES[0] if first else c.INTRO_LINES[1]
            cols = (CREAM, CREAM) if first else (CREAM, GOLD)
            fk = ease((t - .2)/.3) if first else ease((t - .75*s["len"])/.25)
        for li, line in enumerate(lines): center_text(d, 336 + li*19, line, mix(SHADOW, cols[li], fk), 2)
    elif not outro:
        fk = ease((t - .2)/.3)
        for li, line in enumerate(s["fact"]): center_text(d, 336 + li*19, line, mix(SHADOW, CREAM, fk), 2)
    else:
        fk = ease((T - c.OUTRO_T)/.3)
        center_text(d, 336, c.OUTRO_LINES[0], mix(SHADOW, CREAM, fk), 2)
        center_text(d, 355, c.OUTRO_LINES[1], mix(SHADOW, GOLD, fk), 2)
    rr = np.random.default_rng(fi)
    for f in fx:
        kind = f[0]
        if kind == "note":
            S = cam.screen(f[1]); a = f[2]/.9
            sprite(d, NOTE_SPR, int(S[0]) - 5 + int(4*math.sin(f[2]*8)), int(S[1]) - int(f[2]*40) - 14,
                   mix((120, 96, 140), (255, 246, 214), (1 - a)**.6), 2)
        elif kind == "spark":
            S = cam.screen(f[1]); sprite(d, SPARK, int(S[0]) - 2, int(S[1]) - 2, (255, 246, 190), 1, False)
        elif kind == "dust":
            S = cam.screen(f[1])
            for _ in range(3):
                d.point((int(S[0] + rr.normal(0, 6)), int(S[1] + rr.normal(0, 3) - rr.random()*6)), fill=(255, 250, 230))
        elif kind == "strum":
            A = cam.screen(f[1]); B = cam.screen(f[2]); k = f[3]/.15
            d.line([(int(A[0]), int(A[1])), (int(lerp(A[0], B[0], min(1, k*2))), int(lerp(A[1], B[1], min(1, k*2))))],
                   fill=(255, 250, 220), width=2)
        elif kind == "wavel":
            S = cam.screen(f[1]); age = f[2]
            for j in range(3):
                rad = 6 + (age*40 + j*7)
                if age*40 + j*7 > 46: continue
                col = mix((255, 250, 220), (120, 100, 150), clamp((age*40 + j*7)/46))
                d.arc([S[0] - rad*1.4, S[1] - rad, S[0] + rad*.6, S[1] + rad], 125, 235, fill=col, width=1)
        elif kind == "wave":
            S = cam.screen(f[1]); age = f[2]
            for j in range(3):
                rad = 6 + (age*40 + j*7)
                if age*40 + j*7 > 46: continue
                col = mix((255, 250, 220), (120, 100, 150), clamp((age*40 + j*7)/46))
                d.arc([S[0] - rad*.6 + f[3]*rad*.4, S[1] - rad, S[0] + rad*1.4, S[1] + rad], -55, 55, fill=col, width=1)
    if T >= c.FINAL_T:
        tb = T - c.FINAL_T; r2 = np.random.default_rng(11)
        for k in range(26):
            ang = r2.random()*6.28; sp = 40 + r2.random()*80
            x = CX + math.cos(ang)*sp*min(1, tb*2.2); y = 190 + math.sin(ang)*sp*.8*min(1, tb*2.2) + tb*tb*20
            if tb < 2.2: sprite(d, SPARK, int(x), int(y), mix((255, 240, 180), (120, 96, 140), clamp(tb/2.2)), 1, False)
    if si > 0:
        n = nlast; xs = [36 + i*(RW - 72)//(n - 1) for i in range(n)]; cur = si - 1; off = (110, 100, 124)
        d.line([(xs[0], 392), (xs[-1], 392)], fill=off)
        d.line([(xs[0], 392), (xs[-1] if outro else xs[cur], 392)], fill=GOLD_D)
        bob = int(2*math.exp(-((T % c.BEAT))*14))
        for i, x in enumerate(xs):
            if i < cur or outro: d.rectangle([x-2, 390, x+2, 394], fill=GOLD_D)
            elif i == cur:
                d.rectangle([x-3, 389-bob, x+3, 395-bob], fill=GOLD); d.rectangle([x-4, 388-bob, x+4, 396-bob], outline=(255, 230, 150))
            else: d.rectangle([x-2, 390, x+2, 394], fill=off)
    return im

def run(c):
    setup(c)
    out = c.OUT if len(sys.argv) < 2 or sys.argv[1].startswith("--") else sys.argv[1]
    wav = out.replace(".mp4", ".wav")
    mixed = render_audio(c, wav)
    analyze(c, mixed)
    if "--still" in sys.argv:
        for ts in sys.argv[sys.argv.index("--still") + 1:]:
            T = float(ts); frame(c, T, int(T*FPS)).resize((RW*2, RH*2), Image.NEAREST).save(f"{c.TAG}_{ts}.png")
        return
    nframes = int(c.TOTAL*FPS)
    p = subprocess.Popen(["ffmpeg", "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24",
        "-s", f"{RW}x{RH}", "-r", str(FPS), "-i", "-", "-i", wav,
        "-vf", f"scale={RW*UP}:{RH*UP}:flags=neighbor", "-c:v", "libx264", "-pix_fmt", "yuv420p",
        "-crf", "16", "-preset", "medium", "-c:a", "aac", "-b:a", "192k", "-shortest", out], stdin=subprocess.PIPE)
    for fi in range(nframes):
        p.stdin.write(frame(c, fi/FPS, fi).tobytes())
    p.stdin.close(); p.wait()
    print("wrote", out, f"{c.TOTAL:.1f}s")
