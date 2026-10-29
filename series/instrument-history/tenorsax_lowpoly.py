"""Pixel Pitch: the modern tenor saxophone (B-flat) - low-poly model. No history video yet: built for BALLFIGHT #51
(Alto Sax vs Tenor Sax), on the rebuilt alto's construction (altosax_lowpoly.sax(), used read-only as a library) and
used through projects/which_instrument_wins/pp_bridge.py ('TENOR SAX').

    python3 tenorsax_lowpoly.py              # face count + extent per scene

Reference photos (Claude in Chrome, 2026-10-29, Wikimedia Commons, CC BY-SA 4.0): "Yamaha Saxophone YTS-62
(recrop).png" (a YTS-62 tenor, three-quarter front) and "4 Saxophone Amati white background.jpg" (alto and tenor side by
side at one scale). What makes it a tenor and not a big alto, measured there (heights from the bottom of the bow to the
top of the neck):
  - ~1.22 x the alto's height (here ~0.80 m to the top of the neck), the tube wider all the way up;
  - the neck: it rises straight out of the body, then arches over in the tenor's hump (a crook that turns past the
    horizontal, so the mouthpiece points a little down and sits ~3-4 % of the height below the top of the arch);
    the mouthpiece ends ~22 % of the height out from the body;
  - a deeper bow and a longer straight run of the bell tube, then a bigger bell (rim ~18 % of the height across, its
    centre at ~45 % of the height and further out from the body than the alto's), its mouth turned up and forward;
  - bigger cups, guards and pearls (the alto's key layout, scaled), a bigger mouthpiece.
Lacquer: a deeper amber gold than the alto's bright lacquer, so the two read apart in a fight.
Local model frame (as the alto): y up, +x toward the bell, +z toward the camera (the pearl side).
"""
import math
import sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))  # sibling engine files
import altosax_lowpoly as AX                         # read-only: sax(), build_model(), the alto's helpers

OUT, TAG = "tenorsax_lowpoly.mp4", "ts"
BPM = 118
SHOW_BARS = 2
HEAD_INTRO = "THE TENOR SAX"

_NOTES = [(0, 46, .5, .85), (.5, 49, .5, .85), (1, 53, 1, .85), (2, 51, .5, .85), (2.5, 49, .5, .85), (3, 46, 1, .85)]
SCENES = [
    dict(key="modern", theme="intro", name="TENOR SAXOPHONE", credit="B-FLAT", notes=_NOTES),
    # the same tenor for small sprites (BALLFIGHT balls): true heights, every tube's radius x `chunk`, the keys grown
    dict(key="ball", theme="intro", name="TENOR SAXOPHONE", credit="B-FLAT", notes=_NOTES, chunk=1.4),
]
KICK_KINDS, ACCENT_KINDS = (), ()


def drum_events(c):
    return []


METAL = (218, 156, 52)             # deep amber lacquer
KEYC = (250, 196, 92)              # the bright keys
ROD = (150, 100, 32)

TENOR = dict(
    YB=.130, YT=.700, BOWR=.058, RB=.033, RT=.0150, RBELL0=.038,
    y_run=.270, r_run=.046, ctrl=(.156, .330, .012), rim=(.168, .372, .036), rim_r=.076, flare_pow=2.3, rim_tilt=None,
    # the hump: up out of the socket, over the top of the arch, then down a little to the mouthpiece
    neck=[(0, 0, 0), (0, .034, 0), (.004, .060, 0), (.001, .082, 0), (-.012, .097, 0), (-.032, .104, 0),
          (-.054, .101, 0), (-.074, .092, 0), (-.092, .085, 0)],
    neck_r=(.0148, .0102), mp=(.098, .0132), key=1.22,
    metal=METAL, keyc=KEYC, rod=ROD,
)
POSE = AX.rot_z(math.radians(-6)) @ AX.rot_y(math.radians(16))


def build(g, t, ev, fx, s):
    tip, rim = AX.build_model(g, TENOR, s, pose=POSE, lift=.48)
    for age, mm, v, dur, tp, cs in ev:
        if tp and age < .9:
            fx.append(("note", tuple(rim + [.07, .12, 0]), age))
        if tp and age < .6:
            fx.append(("wave", tuple(rim + [.02, .02, 0]), age, 0))


# ---------------------------------------------------------------- sound: the tenor's husky honk
# Between the alto's singing tone and the bari's window-rattler: harmonics rolled off by frequency (as the bari's, so the
# low notes keep their body) through the tenor's own colour - a broad low formant near 620 Hz (the husk), a second bump
# near 1.5 kHz (the bark) - over a breathy, airy noise band that never quite goes away (the husky part: the air around
# the reed, louder on soft notes). `honk` (0..1): the rock 'n' roll honk - a scoop up into the note, a growl, the reed's
# slap and more bark. `sub` (0..1): subtone - the Ben Webster hush: fewer harmonics, the breath up, a slower attack.
SR = AX.SR
FORMANTS = ((620., 340., 1.6), (1500., 560., .85), (2800., 850., .25))          # (Hz, width, gain)


def _midi_hz(m):
    return 440.0 * 2 ** ((m - 69) / 12.0)


def tenor_tone(f, dur, v, honk=0.0, sub=0.0, bright=.42):
    from scipy.signal import butter, sosfilt
    n = int((dur + .18) * SR)
    t = np.arange(n) / SR
    att = .014 + .05 * sub
    rel = .04 if dur < .15 else .07
    env = np.minimum(1, t / att) ** 1.3 * np.where(t < dur, 1, np.exp(-(t - dur) / rel)) * (1 + (.18 + .2 * honk) *
                                                                                          np.exp(-t / .05))
    fr = f * (1 - (.03 + .06 * honk) * np.exp(-t / (.05 + .03 * honk)))                 # the scoop up into the note
    fr = fr * (1 + .0045 * np.sin(2 * np.pi * 5.1 * t) * np.clip((t - .22) / .3, 0, 1))  # a lazy vibrato on long notes
    ph = 2 * np.pi * np.cumsum(fr) / SR
    x = np.zeros(n)
    tot = 0.0
    br = bright + .3 * honk - .35 * sub
    for k in range(1, 70):
        fk = f * k
        if fk > 9000:
            break
        a = k ** -.75 * (.3 + sum(g * math.exp(-((fk - c) / w) ** 2) for c, w, g in FORMANTS))
        a *= math.exp(-fk / max(500.0, 1100 + 3600 * br * v))
        x += a * np.sin(k * ph + .37 * k)
        tot += a
    x /= tot + 1e-9
    gr = .04 + .26 * honk
    x *= 1 + gr * np.sin(2 * np.pi * 31 * t + 2.6 * np.sin(2 * np.pi * 6.5 * t))       # growl
    rng = np.random.default_rng(int(f * 10 + dur * 1000) % 100003)
    nz = sosfilt(butter(2, [380, 2600], 'band', fs=SR, output='sos'), rng.standard_normal(n))
    air = sosfilt(butter(2, [2200, 7000], 'band', fs=SR, output='sos'), rng.standard_normal(n))
    husk = .05 + .06 * sub + .03 * (1 - v)
    x += nz * husk * (.65 + 1.4 * np.exp(-t * 16)) + air * (.012 + .012 * sub)
    if honk:                                                                         # the reed's slap on a hard attack
        k = int(.028 * SR)
        tt = t[:k]
        x[:k] += .45 * honk * (np.sin(2 * np.pi * 190 * tt) * np.exp(-tt * 110) + nz[:k] * np.exp(-tt * 240) * 2)
    return x * env * v


def synth(s, idx):
    nt, m, v, dur, tp, cs = s["ev"][idx]
    return tenor_tone(_midi_hz(m), dur, v, honk=.35)


if __name__ == "__main__":
    import piano_lowpoly as P
    for sc in SCENES:
        g = P.Geo()
        build(g, 0.0, [], [], sc)
        V = np.concatenate([f[0] for f in g.F])
        print(sc["key"], len(g.F), "faces", len(g.L), "lines, extent (m):", np.round(V.max(0) - V.min(0), 3))
