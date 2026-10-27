"""BALLFIGHT VS title card on bar 0 (series standard from Oct 24; built for #30's intro test, shared here so every
episode uses one).

The frame splits diagonally into two panels in the fighters' colours (left fighter top-left, right fighter bottom-right),
each with its fighter's ball big and close, like a fighting-game versus screen, with a small "BALLFIGHT #N" tag at the
top. The words slam in one per beat: name A (beat 1 = frame 0), VS (beat 2: it cracks the diagonal divider, sparks),
name B (beat 3); each slam comes down from huge to size with an overshoot, motion streaks, a shake, a dust burst and a
local light in its colour. Beat 4: VS pulses, then the panels slide apart like doors along the cracked divider to reveal
the ring (VS splits with them). In the last T_CLOSE s of the file the doors slide shut again and name A falls in as a
ghost, so the slam on frame 0 is the loop's reset and the seam stays small.

The episode supplies (in its own timeline.py / render.py, found first on sys.path):
    timeline: TITLE = [(0.0, A), (BEAT, 'VS'), (2 * BEAT, B)], T_DOORS = (t_open, t_gone), T_CLOSE, SERIES, DUR, FPS,
              BEAT, BAR, beat_pulse
    render:   NAMES, COL, DARK, CFG (fighter colours, the instrument view for the big sprites)
Names use the biggest pixel-font scale (<= 7, VS is 12) that fits the safe zone (A: x 20-340 above the divider; B: left
of the button column); a fighter's panel pattern comes from PATTERNS (by name; a plain gradient if it has none).
Opt-in (#38): `timeline.TITLE_LINES = {name: (line 1, line 2)}` stacks a long name on two lines (each line at the
scale that fits the longest; e.g. BASS / CLARINET at 6 instead of BASS CLARINET at 4), and
`timeline.TITLE_BALLS = [(cx, cy, r), (cx, cy, r)]` moves the big balls to make room. #40: TITLE_LINES may also name
fighter B (two lines at the bottom, the lower one where a one-line name sits). Without them the card draws as before.

Everything is screen space at the internal resolution (W x H) and deterministic in t.

    import title as TT
    img = TT.draw(img, t)          # float or uint8 (H, W, 3) -> same dtype; no-op when the doors are open
    TT.door_offset(t) == 0.0       # the card is shut (hide the HUD)"""
import math
import os
import numpy as np
from reelkit.config import W, H, BAYER_FULL, SAFE
from reelkit.font import ADV, text_mask, dilate
from reelkit import ui
import timeline as T

EP = os.path.dirname(os.path.abspath(T.__file__))      # the episode's folder (its check/ holds the sprite cache)
INK = np.array([16, 10, 26], float)
GOLD = np.array([255, 226, 120], float)
WHITE = np.array([255, 250, 236], float)

# the divider: bottom-left -> top-right; N points to the top-left (the left fighter's side)
DA, DB = np.array([0.0, 470.0]), np.array([360.0, 170.0])
DD = (DB - DA) / np.linalg.norm(DB - DA)
N = np.array([DD[1], -DD[0]])
if (np.array([0.0, 0.0]) - DA) @ N < 0:
    N = -N
OPEN_PX = 560.0                                       # how far each door travels to clear the frame
YY, XX = np.mgrid[0:H, 0:W].astype(np.float64)
SIDE = (XX - DA[0]) * N[0] + (YY - DA[1]) * N[1]      # > 0: top-left (left fighter) half

# layout (closed doors): words inside the safe zone at the biggest pixel-font scale that fits
NAME_A, NAME_B = T.TITLE[0][1], T.TITLE[2][1]
MAX_NAME_SCALE = 7                                    # VS (12) stays the largest word


def _fit_scale(text, avail):
    """the biggest scale <= MAX_NAME_SCALE whose word + outline fits `avail` px"""
    for sc in range(MAX_NAME_SCALE, 1, -1):
        w = text_mask(text, sc).shape[1] + 2 * max(2, int(round(sc * 0.6)))
        if w <= avail:
            return sc
    return 2


def _layout():
    """name A centred at x 180 near the top (below the series tag), name B at the bottom, left of the button column
    (shifted left in 10 px steps if it doesn't fit centred); VS on the divider. #30: BAGPIPES 6 @ (180, 134),
    ACCORDION 5 @ (160, 468)"""
    out = {'VS': dict(scale=12, cx=180, cy=300, side=-1)}             # the largest letters, on the divider
    lines = getattr(T, 'TITLE_LINES', {}).get(NAME_A)
    if lines:                                                        # #38 (opt-in): a long name on two lines
        sa = min(_fit_scale(ln, 320) for ln in lines)
        ha, ra = 7 * sa, max(2, int(round(sa * 0.6)))
        gap = ha + 2 * ra - 2                                        # line pitch: outlines just touching
        cy0 = int(max(134, round(109 + ha / 2 + ra)))
        out[NAME_A] = dict(scale=sa, cx=180, cy=cy0 + gap / 2, side=0, lines=tuple(lines), dy=(-gap / 2, gap / 2),
                           h=ha + gap, w=max(text_mask(ln, sa).shape[1] for ln in lines))
    else:
        sa = _fit_scale(NAME_A, 320)
        ha, ra = 7 * sa, max(2, int(round(sa * 0.6)))
        out[NAME_A] = dict(scale=sa, cx=180, cy=int(max(134, round(109 + ha / 2 + ra))), side=0)
    lines_b = getattr(T, 'TITLE_LINES', {}).get(NAME_B)
    if lines_b:                                                      # #40 (opt-in): name B on two lines too; the lower
        sb = min(_fit_scale(ln, 280) for ln in lines_b)              # line sits where a one-line name would
        wb, hb, rb = max(text_mask(ln, sb).shape[1] for ln in lines_b), 7 * sb, max(2, int(round(sb * 0.6)))
        gap = hb + 2 * rb - 2
        cxb = 180 if 180 + wb / 2 + rb <= 300 else int((300 - wb / 2 - rb) // 10 * 10)
        cyb = int(min(468, 500 - hb / 2 - rb - 3)) - gap / 2
        out[NAME_B] = dict(scale=sb, cx=cxb, cy=cyb, side=1, lines=tuple(lines_b), dy=(-gap / 2, gap / 2),
                           h=hb + gap, w=wb)
        return out
    sb = _fit_scale(NAME_B, 280)
    wb, hb, rb = text_mask(NAME_B, sb).shape[1], 7 * sb, max(2, int(round(sb * 0.6)))
    cxb = 180 if 180 + wb / 2 + rb <= 300 else int((300 - wb / 2 - rb) // 10 * 10)
    out[NAME_B] = dict(scale=sb, cx=cxb, cy=int(min(468, 500 - hb / 2 - rb - 3)), side=1)
    return out


WORDS = _layout()
BALLS = [dict(cx=78, cy=222, r=56), dict(cx=282, cy=396, r=56)]   # left fighter, right fighter: big and close
if getattr(T, 'TITLE_BALLS', None):                                 # #38 (opt-in): room for a two-line name
    BALLS = [dict(cx=c[0], cy=c[1], r=c[2]) for c in T.TITLE_BALLS]


def _ease_in(u):
    u = min(1.0, max(0.0, u))
    return u * u * u


def _ease_out(u):
    u = min(1.0, max(0.0, u))
    return 1 - (1 - u) ** 3


def door_offset(t):
    """how far the doors are open (px along the divider normal): 0 closed .. OPEN_PX gone; None = no title card"""
    if t >= T.DUR - T.T_CLOSE:                                     # the run-up: the doors slide shut, closed on the last frame
        u = (t - (T.DUR - T.T_CLOSE)) / (T.T_CLOSE - 1.0 / T.FPS)
        return OPEN_PX * (1 - _ease_out(u))
    if t < T.T_DOORS[0]:
        return 0.0
    if t < T.T_DOORS[1]:
        return OPEN_PX * _ease_in((t - T.T_DOORS[0]) / (T.T_DOORS[1] - T.T_DOORS[0]))
    return None


def slam_k(t, t0):
    """the word's size factor: huge at t0, down to 1 in 0.16 s, a small overshoot, settled by 0.26 s"""
    u = (t - t0) / 0.16
    if u < 0:
        return None
    if u < 1:
        return 1 + 1.25 * (1 - u) ** 2
    if u < 1.6:
        return 1 - 0.08 * math.sin(math.pi * (u - 1) / 0.6)
    return 1.0


def word_state(t, j):
    """(size factor, alpha) of word j at t; name A also falls in as a ghost at the end of the file"""
    t0 = T.TITLE[j][0]
    if t >= T.DUR - T.T_CLOSE and (j > 0 or t < T.DUR - 0.12):      # the closing doors carry only the panels, then
        if j > 0 or t < T.DUR - 0.12:                               # name A's ghost falls in (the frame-0 slam)
            return None
    if j == 0 and t >= T.DUR - 0.12:                               # the ghost of the frame-0 slam
        u = (t - (T.DUR - 0.12)) / (0.12 - 1.0 / T.FPS)
        u = min(1.0, u)
        return (2.25 if u >= 1.0 - 1e-6 else 2.25 + 0.6 * (1 - u)), u      # on the last frame: exactly frame 0's word
    k = slam_k(t, t0)
    if k is None:
        return None
    pulse = 0.05 * T.beat_pulse(t, 0.12) if (j == 1 and 3 * T.BEAT - 0.07 <= t < T.T_DOORS[0] + 0.05) else 0.0   # VS
    # (only VS pulses in size on beat 4: the names already use the widest scale that fits the safe zone)
    return k + pulse, 1.0


def shake(t):
    """the title's own shake: a decaying jolt after each slam, zero on the slam frame itself (a clean seam on frame 0)"""
    dx = dy = 0.0
    for j, (t0, w) in enumerate(T.TITLE):
        u = t - t0 - 1.0 / T.FPS
        if 0 < u < 0.35:
            a = (7.0 if j == 1 else 5.0) * math.exp(-u / 0.09)
            dx += a * math.sin(2 * math.pi * 23 * u)
            dy += a * 0.7 * math.sin(2 * math.pi * 29 * u + 1.0)
    return dx, dy


# ---------------------------------------------------------------- panels
def _tartan(base):                                                   # BAGPIPES: a large tartan sett, low contrast
    u, v = (XX + YY) / 46.0, (XX - YY) / 46.0
    chk = ((np.floor(u) + np.floor(v)) % 2)
    line = (np.abs((u % 1) - 0.5) < 0.05) | (np.abs((v % 1) - 0.5) < 0.05)
    base = base * (0.92 + 0.12 * chk[..., None])
    return np.where(line[..., None], base * 0.7 + np.array([200, 50, 46]) * 0.3, base)


def _pleats(base):                                                   # ACCORDION: pleats parallel to the divider
    s = (SIDE / 14.0) % 2
    return base * np.where((s < 1)[..., None], 1.06, 0.84)


def _organ_pipes(base):                                              # ORGAN: a front of pipes, tall to short, with mouths
    col = np.floor(XX / 16.0)
    xc = (XX % 16.0) - 8.0
    top = 60.0 + 70.0 * np.abs(np.sin(col * 0.9)) + 2.0 * xc * xc / 8.0
    body = (np.abs(xc) < 6.0) & (YY > top)
    mouth = body & (np.abs(YY - (top + 70)) < 3) & (np.abs(xc) < 3.5)
    shade = 1.0 + 0.10 * np.where(xc < -2, 1.0, np.where(xc > 3, -1.0, 0.0))
    base = np.where(body[..., None], base * (0.98 * shade)[..., None] + 14, base * 0.80)
    return np.where(mouth[..., None], base * 0.45, base)


def _harp_strings(base):                                             # HARP: fine strings fanning across, pearl glints
    a = np.arctan2(YY - 900.0, XX - 520.0)
    s = (a * 80.0) % 1.0
    line = s < 0.12
    return np.where(line[..., None], base * 0.78 + np.array([255, 246, 222]) * 0.22, base * 0.94)


def _slide_tubes(base):                                              # TROMBONE: pairs of slide tubes along the divider
    q = (SIDE / 34.0) % 1
    tube = (np.abs(q - 0.40) < 0.045) | (np.abs(q - 0.60) < 0.045)
    gap = np.abs(q - 0.5) < 0.06
    return base * np.where(tube[..., None], 1.16, np.where(gap[..., None], 0.86, 1.0))


def _horn_coils(base):                                               # HORN: tubing coiled round the panel's big ball
    b = np.where(SIDE > 0, 0, 1)                                     # (works on either side of the divider)
    cx = np.where(b == 0, BALLS[0]['cx'], BALLS[1]['cx'])
    cy = np.where(b == 0, BALLS[0]['cy'], BALLS[1]['cy'])
    q = (np.hypot(XX - cx, YY - cy) / 16.0) % 1
    return base * np.where((q < 0.22)[..., None], 1.14, np.where((q > 0.88)[..., None], 0.86, 1.0))


def _surf(base):                                                     # UKULELE (#33): rolling wave crests, foam lines
    y = YY + 14.0 * np.sin(XX / 30.0) + 6.0 * np.sin(XX / 11.0 + YY / 40.0)
    s = (y / 24.0) % 1.0
    crest = s < 0.12
    trough = (s > 0.55) & (s < 0.75)
    base = np.where(trough[..., None], base * 0.88, base)
    return np.where(crest[..., None], base * 0.72 + np.array([250, 246, 252]) * 0.28, base)


def _woodgrain(base):                                                # ACOUSTIC (#33): spruce grain + six steel strings
    g = np.sin((XX * 0.30 + YY * 0.95) / 5.5 + 2.2 * np.sin(XX / 37.0 + YY / 90.0))
    base = base * (0.94 + 0.08 * (g > 0.55)[..., None] - 0.05 * (g < -0.7)[..., None])
    for j in range(6):                                               # strings parallel to the divider
        line = np.abs(SIDE + 70.0 + 11.0 * j) < 0.9
        base = np.where(line[..., None], base * 0.6 + np.array([236, 226, 206]) * 0.4, base)
    return base


def _clarinet_keys(base):                                            # CLARINET (#37): ebony grain + rows of ring keys
    g = np.sin((XX * 0.95 - YY * 0.30) / 4.5 + 1.6 * np.sin(XX / 41.0 + YY / 77.0))
    base = base * (0.93 + 0.06 * (g > 0.6)[..., None] - 0.06 * (g < -0.75)[..., None])
    u = SIDE + 30.0                                                  # key rows run parallel to the divider
    for j, off in enumerate((0.0, 64.0, 128.0)):
        along = (XX * DD[0] + YY * DD[1]) / 26.0 + 0.5 * j
        c = (np.abs(u - off) < 8.0)
        q = (along % 1.0 - 0.5) * 26.0
        r = np.hypot(q, u - off)
        ring = c & (np.abs(r - 6.0) < 1.3)
        cup = c & (r < 3.2)
        base = np.where(ring[..., None], base * 0.55 + np.array([236, 236, 248]) * 0.45, base)
        base = np.where(cup[..., None], base * 0.78, base)
        rod = np.abs(u - off - 9.5) < 0.8
        base = np.where(rod[..., None], base * 0.7 + np.array([226, 226, 240]) * 0.3, base)
    return base


def _oboe_reeds(base):                                               # OBOE (#37): tuning forks + reed blades + A waves
    along = (XX * DD[0] + YY * DD[1])
    u = -SIDE                                                        # (works below the divider: the right fighter)
    out = base * 1.0
    w = np.abs(u - 40.0 - 6.0 * np.sin(along / 9.0)) < 1.2           # the A: a sine running along the divider
    out = np.where(w[..., None], out * 0.6 + np.array([250, 240, 170]) * 0.4, out)
    for j in range(5):                                               # tuning forks, prongs toward the divider
        a0 = -390.0 + 120.0 * j
        q = along - a0
        stem = (np.abs(q) < 1.6) & (u > 96) & (u < 150)
        prong = ((np.abs(q - 5.0) < 1.6) | (np.abs(q + 5.0) < 1.6)) & (u > 64) & (u <= 98)
        bow = (np.abs(q) < 6.6) & (np.abs(u - 97.0) < 1.6)
        fork = stem | prong | bow
        out = np.where(fork[..., None], out * 0.58 + np.array([236, 236, 214]) * 0.42, out)
    for j in range(5):                                               # double-reed blades between the forks
        q = along - (-330.0 + 120.0 * j)
        hw = (1 - np.clip((u - 96.0) / 46.0, 0, 1)) * 6.5 + 2.0       # wide, flat tip; narrow at the thread
        blade = (np.abs(q) < hw) & (u > 96) & (u < 142)
        thread = (np.abs(q) < 2.6) & (u >= 142) & (u < 160)
        out = np.where(blade[..., None], out * 0.5 + np.array([232, 208, 150]) * 0.5, out)
        out = np.where(thread[..., None], out * 0.5 + np.array([60, 150, 86]) * 0.5, out)
    return out


def _bassclarinet(base):                                             # BASS CLARINET (#38): grenadilla + pad cups + the neck
    g = np.sin((XX * 0.95 - YY * 0.30) / 5.0 + 1.4 * np.sin(XX / 47.0 + YY / 83.0))
    out = base * (0.93 + 0.06 * (g > 0.6)[..., None] - 0.06 * (g < -0.75)[..., None])
    u = SIDE + 36.0                                                  # two rows of big pad cups along the divider
    for j, off in enumerate((0.0, 78.0)):
        along = (XX * DD[0] + YY * DD[1]) / 34.0 + 0.5 * j
        q = (along % 1.0 - 0.5) * 34.0
        r = np.hypot(q, u - off)
        ring = (np.abs(r - 9.5) < 1.6)
        cup = r < 7.0
        out = np.where(cup[..., None], out * 0.62 + np.array([214, 218, 232]) * 0.38, out)
        out = np.where(ring[..., None], out * 0.5 + np.array([236, 238, 248]) * 0.5, out)
        rod = np.abs(u - off - 13.0) < 0.9
        out = np.where(rod[..., None], out * 0.65 + np.array([226, 228, 240]) * 0.35, out)
    r = np.hypot(XX - 268.0, YY - 66.0)                              # the silver gooseneck looping over the top
    neck = (np.abs(r - 50.0) < 6.5) & ((YY < 66.0) | (XX > 268.0))
    hl = (np.abs(r - 47.0) < 1.5) & neck
    out = np.where(neck[..., None], out * 0.45 + np.array([212, 216, 228]) * 0.55, out)
    return np.where(hl[..., None], out * 0.3 + np.array([255, 255, 255]) * 0.7, out)


def _euphonium(base):                                                # EUPHONIUM (#38): a big upright bell + four valves
    out = base * 1.0
    cx, cy = 300.0, 560.0                                            # the bell's mouth, seen from just above (far corner)
    e = ((XX - cx) / 120.0) ** 2 + ((YY - cy) / 46.0) ** 2
    rim = np.abs(e - 1.0) < 0.10
    inner = e < 0.9
    out = np.where(inner[..., None], out * (0.80 + 0.12 * np.clip(1 - e, 0, 1))[..., None], out)
    out = np.where(rim[..., None], out * 0.45 + np.array([255, 222, 140]) * 0.55, out)
    flare = (np.abs(XX - cx) < 26.0 + 90.0 * np.clip((YY - 330.0) / 230.0, 0, 1) ** 3) & (YY > 330.0) & (YY < cy) & ~inner
    out = np.where(flare[..., None], out * 1.10 + 6, out)
    along = (XX * DD[0] + YY * DD[1])                                # four valve caps in a row along the divider
    u = -SIDE
    for j in range(4):
        a0 = -150.0 + 46.0 * j
        r = np.hypot(along - a0, u - 64.0)
        cap = r < 13.0
        ring = np.abs(r - 13.0) < 1.6
        btn = r < 6.0
        out = np.where(cap[..., None], out * 0.7 + np.array([255, 214, 120]) * 0.3, out)
        out = np.where(ring[..., None], out * 0.45 + np.array([255, 236, 170]) * 0.55, out)
        out = np.where(btn[..., None], out * 0.4 + np.array([250, 244, 226]) * 0.6, out)
        stem = (np.abs(along - a0) < 2.0) & (u > 77.0) & (u < 112.0)
        out = np.where(stem[..., None], out * 0.6 + np.array([255, 214, 120]) * 0.4, out)
    return out


def _bassoon(base):                                                  # BASSOON (#39): flamed maple, twin bores, the crook
    g = np.sin((XX * 0.25 + YY * 0.97) / 3.2 + 2.0 * np.sin(XX / 23.0 + YY / 61.0))   # maple flame, across the grain
    out = base * (0.92 + 0.10 * (g > 0.5)[..., None] - 0.07 * (g < -0.6)[..., None])
    u = SIDE                                                         # (works above the divider: the left fighter)
    along = (XX * DD[0] + YY * DD[1])
    for off in (54.0, 92.0):                                         # the twin bores (long joint + wing) along the divider
        edge = np.abs(np.abs(u - off) - 16.0) < 1.2
        out = np.where(edge[..., None], out * 0.72, out)
        hl = np.abs(u - off + 9.0) < 1.0
        out = np.where(hl[..., None], out * 0.75 + np.array([255, 236, 220]) * 0.25, out)
    for j in range(7):                                               # silver key rods + touches on the bores
        a0 = -330.0 + 95.0 * j
        rod = (np.abs(u - 73.0) < 0.9) & (np.abs(along - a0) < 36.0)
        dot = np.hypot(along - a0, u - 54.0) < 4.2
        out = np.where((rod | dot)[..., None], out * 0.5 + np.array([226, 228, 240]) * 0.5, out)
    cx, cy = 248.0, 96.0                                             # the bocal: a silver crook arcing over the top corner
    r = np.hypot(XX - cx, YY - cy)
    arc = (np.abs(r - 44.0) < 4.5) & (YY < cy + 8.0)
    tail = np.abs((YY - cy) - (XX - cx - 44.0) * 0.55) < 4.5
    tail &= (XX > cx + 40.0) & (XX < cx + 110.0) & (YY > cy - 2.0)
    stem = (np.abs(XX - (cx - 44.0)) < 4.5) & (YY > cy) & (YY < cy + 70.0)
    crook = arc | tail | stem
    out = np.where(crook[..., None], out * 0.4 + np.array([216, 220, 232]) * 0.6, out)
    reed = (np.abs((YY - cy) - (XX - cx - 44.0) * 0.55) < 6.0) & (XX >= cx + 110.0) & (XX < cx + 128.0)
    return np.where(reed[..., None], out * 0.4 + np.array([232, 208, 150]) * 0.6, out)


def _barisax(base):                                                  # BARI SAX (#39): pearl key touches + the pigtail loop
    out = base * 1.0
    along = (XX * DD[0] + YY * DD[1])
    u = -SIDE                                                        # (works below the divider: the right fighter)
    for j in range(6):                                               # the main stack: cups + pearl touches along the divider
        a0 = -250.0 + 62.0 * j
        r = np.hypot(along - a0, u - 58.0)
        cup = r < 15.0
        ring = np.abs(r - 15.0) < 1.6
        pearl = np.hypot(along - a0, u - 96.0) < 7.0
        out = np.where(cup[..., None], out * 0.72 + np.array([255, 214, 150]) * 0.28, out)
        out = np.where(ring[..., None], out * 0.45 + np.array([255, 226, 170]) * 0.55, out)
        out = np.where(pearl[..., None], out * 0.35 + np.array([250, 244, 232]) * 0.65, out)
        arm = (np.abs(along - a0) < 1.4) & (u > 73.0) & (u < 90.0)
        out = np.where(arm[..., None], out * 0.6 + np.array([255, 226, 170]) * 0.4, out)
    cx, cy = 92.0, 560.0                                             # the pigtail: the tube coiling once in the far corner
    r = np.hypot(XX - cx, YY - cy)
    loop = np.abs(r - 34.0) < 6.0
    neck = (np.abs(YY - (cy - 34.0) + (XX - cx) * 0.18) < 5.0) & (XX < cx) & (XX > cx - 92.0)
    tube = loop | neck
    out = np.where(tube[..., None], out * 0.45 + np.array([255, 206, 120]) * 0.55, out)
    hl = (np.abs(r - 31.0) < 1.2) & (YY < cy)
    return np.where(hl[..., None], out * 0.3 + np.array([255, 250, 230]) * 0.7, out)


PATTERNS = {'BAGPIPES': _tartan, 'ACCORDION': _pleats, 'ORGAN': _organ_pipes, 'HARP': _harp_strings,
            'TROMBONE': _slide_tubes, 'HORN': _horn_coils, 'UKULELE': _surf, 'ACOUSTIC': _woodgrain,
            'CLARINET': _clarinet_keys, 'OBOE': _oboe_reeds, 'BASS CLARINET': _bassclarinet, 'EUPHONIUM': _euphonium,
            'BASSOON': _bassoon, 'BARI SAX': _barisax}


def _panel_rgb(colour, dark, side, name=None):
    """a panel's fill: a gradient from the fighter's deep shade at the far corner to its colour at the divider, its
    fighter's pattern (PATTERNS), dithered bands"""
    d = np.abs(SIDE) / 400.0
    k = np.clip(1 - d, 0, 1) ** 1.3
    base = dark[None, None] * (1 - k[..., None]) + colour[None, None] * (0.40 + 0.25 * k[..., None]) * k[..., None] \
        + dark[None, None] * 0.6 * k[..., None]
    if name in PATTERNS:
        base = PATTERNS[name](base)
    q = np.floor((base / 255.0) * 10 + BAYER_FULL[..., None] * 0.9) / 10 * 255
    return np.clip(q, 0, 255)


_PANEL = {}


def panels():
    if not _PANEL:
        import render as RD
        for side, k in ((0, 0), (1, 1)):
            _PANEL[side] = _panel_rgb(RD.COL[k], RD.DARK[k], side, RD.NAMES[k])
    return _PANEL


# ---------------------------------------------------------------- big balls
_SPR = {}
SPR_PATH = os.path.join(EP, 'check', 'sprites_title.npz')
CLOSEUP = {}       # fighter name -> fn(radius_px) -> RGBA: its own close-up sprite (e.g. #32's flute: the model is a line)
GLASS = {}         # fighter name -> rgb: its glass tint at close-up (e.g. #32's silver flute: cool, darker glass)


def big_sprite(k):
    if k not in _SPR:
        import render as RD
        if RD.NAMES[k] in CLOSEUP:                                    # opt-in: the episode draws this one itself
            _SPR[k] = CLOSEUP[RD.NAMES[k]](int(BALLS[k]['r'] * 0.8))
            return _SPR[k]
        nm = RD.CFG[k]['pp']
        key = f'{nm}_title'
        if os.path.exists(SPR_PATH):
            z = np.load(SPR_PATH)
            _SPR.update({int(kk.split('_')[-1]): z[kk] for kk in z.files})
        if k not in _SPR:
            import sprites as SP
            yc, sw, roll = RD.CFG[k]['view']
            _SPR[k] = SP.render_sprite(nm, int(BALLS[k]['r'] * 0.8), float(yc), float(roll))
            np.savez_compressed(SPR_PATH, **{f'title_{kk}': v for kk, v in _SPR.items()})
    return _SPR[k]


def draw_big_ball(layer, alpha, k, cx, cy, rp, glow, col):
    """a glass ball (as in the ring, at close-up size) with the Pixel Pitch instrument inside"""
    ri = int(rp) + 3
    x0, y0 = int(cx) - ri, int(cy) - ri
    n = 2 * ri + 1
    yy, xx = np.mgrid[0:n, 0:n]
    dx = (xx + x0 + 0.5 - cx) / rp
    dy = (yy + y0 + 0.5 - cy) / rp
    d2 = dx * dx + dy * dy
    inside = d2 <= 1.0
    rim = inside & (d2 > (1 - 3.0 / rp) ** 2)
    edge = (d2 > 1.0) & (d2 <= (1 + 2.5 / rp) ** 2)
    nz = np.sqrt(np.clip(1 - d2, 0, 1))
    lam = np.clip(-0.25 * dx - 0.55 * dy + nz * 0.6, 0, 1)
    band = np.floor(lam * 4 + BAYER_FULL[:n, :n] * 0.9) / 4
    import render as RD
    glass = np.asarray(GLASS.get(RD.NAMES[k], (236, 232, 226)), float)   # opt-in tint (default: #30's cream glass)
    inner = (col * 0.30 + glass * 0.70)[None, None] * (0.70 + 0.36 * band[..., None])
    xa, ya, xb, yb = max(0, x0), max(0, y0), min(W, x0 + n), min(H, y0 + n)
    if xb <= xa or yb <= ya:
        return
    sl = (slice(ya - y0, yb - y0), slice(xa - x0, xb - x0))
    reg, ar = layer[ya:yb, xa:xb], alpha[ya:yb, xa:xb]
    m = inside[sl]
    reg[m] = inner[sl][m]
    spr = big_sprite(k)
    sn = spr.shape[0]
    sx0, sy0 = int(round(cx)) - sn // 2, int(round(cy)) - sn // 2
    ax0, ay0, ax1, ay1 = max(0, sx0), max(0, sy0), min(W, sx0 + sn), min(H, sy0 + sn)
    if ax1 > ax0 and ay1 > ay0:
        sub = spr[ay0 - sy0:ay1 - sy0, ax0 - sx0:ax1 - sx0]
        ms = sub[..., 3] > 0
        rg = layer[ay0:ay1, ax0:ax1]
        rg[ms] = np.clip(sub[..., :3].astype(float)[ms] * (1.06 + 0.3 * glow), 0, 255)
    reg[rim[sl]] = np.clip(col * (0.9 + 0.5 * glow) + 50 * glow, 0, 255)
    reg[edge[sl]] = INK
    spec = inside & ((dx + 0.38) ** 2 + (dy + 0.5) ** 2 < 0.012)
    reg[spec[sl]] = np.array([255, 252, 244])
    ar[(inside | edge)[sl]] = 1.0


# ---------------------------------------------------------------- words
def word_masks(text, scale, k):
    """(fill, outline, highlight) bool masks of a pixel-font word at integer scale, resized by k (nearest)"""
    base = text_mask(text, scale)
    if abs(k - 1) > 1e-3:
        h0, w0 = base.shape
        hh, ww = max(1, int(round(h0 * k))), max(1, int(round(w0 * k)))
        ys = np.minimum((np.arange(hh) * h0 / hh).astype(int), h0 - 1)
        xs = np.minimum((np.arange(ww) * w0 / ww).astype(int), w0 - 1)
        m = base[ys][:, xs]
    else:
        m = base
    r = max(2, int(round(scale * 0.6 * k)))
    o = dilate(m, r)
    gh = max(1, m.shape[0] // 7 * 2)                                 # the top two font rows: a highlight
    hi = np.zeros_like(m)
    hi[:gh] = m[:gh]
    return m, o, hi, r


def stamp_word(layer, alpha, text, scale, k, cx, cy, col, a=1.0, note=False):
    m, o, hi, r = word_masks(text, scale, k)
    hm, wm = m.shape
    x0, y0 = int(round(cx - wm / 2)), int(round(cy - hm / 2))
    for mask, c, ox, oy, rr in ((o, INK * 0.5, 3, 4, r), (o, INK, 0, 0, r), (m, col, 0, 0, 0),
                                (hi, np.clip(col * 0.55 + 255 * 0.45, 0, 255), 0, 0, 0)):
        mh, mw = mask.shape
        X0, Y0 = x0 - rr + ox, y0 - rr + oy
        xa, ya, xb, yb = max(0, X0), max(0, Y0), min(W, X0 + mw), min(H, Y0 + mh)
        if xb <= xa or yb <= ya:
            continue
        sub = mask[ya - Y0:yb - Y0, xa - X0:xb - X0]
        reg, ar = layer[ya:yb, xa:xb], alpha[ya:yb, xa:xb]
        reg[sub] = reg[sub] * (1 - a) + c * a
        ar[sub] = np.maximum(ar[sub], a)
    if note:                                                         # the settled word, for reelkit.safezone
        ui._note(x0 - r, y0 - r, x0 + wm + r, y0 + hm + r, 'caption', text)
    return x0, y0, wm, hm


# ---------------------------------------------------------------- effects (screen space, deterministic)
def _rng(j):
    return np.random.default_rng(3000 + j)


def streaks(img, t, j, cx, cy, w, h, col):
    """motion streaks either side of a slamming word, shooting outward and fading over 0.25 s"""
    u = (t - T.TITLE[j][0] - 1.0 / T.FPS) / 0.25                 # from the frame after the slam (a clean seam)
    if not 0 <= u < 1:
        return
    rg = _rng(j)
    for i in range(14):
        y = int(cy + rg.uniform(-0.6, 0.6) * h)
        side = 1 if i % 2 else -1
        x_in = cx + side * (w / 2 + 4 + 40 * u)
        L = rg.uniform(30, 80) * (1 - u)
        xa, xb = sorted((int(x_in), int(x_in + side * L)))
        xa, xb = max(0, xa), min(W, xb)
        if 0 <= y < H and xb > xa:
            img[y, xa:xb] = img[y, xa:xb] * (0.35 + 0.65 * u) + np.clip(col * 0.5 + 140, 0, 255) * (0.65 * (1 - u))


def dust(img, t, j, cx, cy, w, h):
    """a dust burst along the word's base: puffs rolling outward, growing, fading over 0.6 s"""
    u = (t - T.TITLE[j][0] - 1.0 / T.FPS) / 0.6
    if not 0 <= u < 1:
        return
    rg = _rng(j + 10)
    for i in range(12):
        x0 = cx + rg.uniform(-0.55, 0.55) * w
        vx = (x0 - cx) / (w / 2 + 1e-6) * rg.uniform(40, 90)
        x = x0 + vx * (1 - (1 - u) ** 2) * 0.8
        y = cy + h * 0.5 + rg.uniform(-4, 4) - 18 * u
        rad = 3 + 12 * u * rg.uniform(0.7, 1.2)
        a = 0.55 * (1 - u) ** 1.5
        x0i, x1i = int(max(0, x - rad)), int(min(W, x + rad + 1))
        y0i, y1i = int(max(0, y - rad)), int(min(H, y + rad + 1))
        if x1i <= x0i or y1i <= y0i:
            continue
        yy, xx = np.mgrid[y0i:y1i, x0i:x1i]
        dd = ((xx - x) ** 2 + (yy - y) ** 2) / (rad * rad)
        m = np.clip(1 - dd, 0, 1)
        m = np.floor(m * 3 + BAYER_FULL[y0i:y1i, x0i:x1i] * 0.9) / 3
        img[y0i:y1i, x0i:x1i] += (np.array([236, 226, 214]) - img[y0i:y1i, x0i:x1i]) * (m * a)[..., None]


def light(img, t, j, cx, cy, col):
    """a local light in the word's colour behind it (soft, small: flash-safe)"""
    u = (t - T.TITLE[j][0])
    if not 0 <= u < 0.5:
        return
    k = min(1.0, u / 0.06) * math.exp(-max(0.0, u - 0.06) / 0.14) * 0.55     # rises from 0 on the slam frame
    r = 110.0
    x0i, x1i = int(max(0, cx - r)), int(min(W, cx + r))
    y0i, y1i = int(max(0, cy - r * 0.6)), int(min(H, cy + r * 0.6))
    yy, xx = np.mgrid[y0i:y1i, x0i:x1i]
    dd = ((xx - cx) / r) ** 2 + ((yy - cy) / (r * 0.6)) ** 2
    img[y0i:y1i, x0i:x1i] += (np.clip(1 - dd, 0, 1) ** 2 * k)[..., None] * col[None, None] * 0.6


CRACK = None


def crack_pts():
    """the divider crack from VS outward to both ends: jagged polylines (+ branches), deterministic"""
    global CRACK
    if CRACK is None:
        rg = np.random.default_rng(77)
        c = np.array([180.0, 320.0])
        lines = []
        for sgn in (1, -1):
            p, pts = c.copy(), [c.copy()]
            while 0 <= p[0] <= W:
                p = p + DD * sgn * rg.uniform(10, 18) + N * rg.uniform(-4, 4)
                pts.append(p.copy())
                if rg.random() < 0.18:                               # a branch
                    q, br = p.copy(), [p.copy()]
                    for _ in range(3):
                        q = q + DD * sgn * rg.uniform(5, 9) + N * rg.choice([-1, 1]) * rg.uniform(5, 10)
                        br.append(q.copy())
                    lines.append(br)
            lines.append(pts)
        CRACK = lines
    return CRACK


def draw_crack(layer, alpha, t):
    """VS cracks the divider: a white-gold flash running out from the centre, then a dark crack line"""
    u = (t - T.TITLE[1][0])
    if u < 0:
        return
    from PIL import Image, ImageDraw
    grow = min(1.0, u / 0.12)
    im = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(im)
    for pts in crack_pts():
        n = max(2, int(len(pts) * grow))
        d.line([tuple(p) for p in pts[:n]], fill=255, width=2)
    m = np.asarray(im, np.float32) / 255.0 > 0.5
    hot = math.exp(-u / 0.12)
    c = INK * (1 - hot) + GOLD * hot
    layer[m] = c
    alpha[m] = 1.0


def sparks(img, t):
    u = t - T.TITLE[1][0]
    if not 0 <= u < 0.45:
        return
    rg = _rng(99)
    for i in range(20):
        a = rg.uniform(0, 2 * math.pi)
        v = rg.uniform(90, 220)
        p0 = np.array([180.0, 300.0])
        p = p0 + v * u * np.array([math.cos(a), math.sin(a)]) + np.array([0, 160 * u * u])
        q = p - v * 0.03 * np.array([math.cos(a), math.sin(a)])
        k = 1 - u / 0.45
        for s in np.linspace(0, 1, 5):
            x, y = (q + (p - q) * s).astype(int)
            if 0 <= x < W and 0 <= y < H:
                img[y, x] = img[y, x] * (1 - k) + GOLD * k


# ---------------------------------------------------------------- the card
def draw(img, t):
    off = door_offset(t)
    if off is None or off >= OPEN_PX - 1:
        return img
    import render as RD
    u8 = img.dtype == np.uint8
    out = img.astype(np.float64)
    out *= 0.55                                                      # the arena dims behind the card
    pn = panels()
    sx, sy = shake(t)
    for side in (0, 1):
        layer = pn[side].copy()
        alpha = (SIDE > 0) if side == 0 else (SIDE <= 0)
        alpha = alpha.astype(np.float64) * 0.94
        # the divider's bright edge on each panel
        edge = np.abs(SIDE) < 2.2
        layer[edge] = np.clip(RD.COL[side] * 0.5 + 150, 0, 255)
        alpha[edge] = 1.0
        b = BALLS[side]
        ts = T.TITLE[0 if side == 0 else 2][0]
        gl = 0.25 + 0.5 * (min(1.0, (t - ts) / 0.06) * math.exp(-max(0.0, t - ts - 0.06) / 0.2)
                           if ts <= t < T.DUR - 1 else 0.0)
        bob = 3.0 * math.sin(2 * math.pi * t / T.BAR + side * math.pi)
        draw_big_ball(layer, alpha, side, b['cx'], b['cy'] + bob, b['r'] * (1 + 0.03 * T.beat_pulse(t, 0.15)), gl,
                      RD.COL[side])
        if side == 0:
            ui_tag(layer, alpha)
        # words on this panel (VS on both, clipped to the half-plane so it splits with the doors)
        for j, (t0, text) in enumerate(T.TITLE):
            spec = WORDS[text]
            if spec['side'] not in (side, -1):
                continue
            st = word_state(t, j)
            if st is None:
                continue
            k, a = st
            col = GOLD if text == 'VS' else np.clip(RD.COL[0 if text == NAME_A else 1] * 0.85 + 50, 0, 255)
            wl = np.zeros_like(layer)
            wa = np.zeros_like(alpha)
            settled = abs(k - 1) < 0.06 and a >= 1.0 and off == 0.0
            if 'lines' in spec:                                      # #38: one stamp per line, scaled about the block
                for ln, dy in zip(spec['lines'], spec['dy']):
                    stamp_word(wl, wa, ln, spec['scale'], k, spec['cx'], spec['cy'] + dy * k, col, a,
                               note=settled and (spec['side'] == side or side == 0))
            else:
                x0, y0, wm, hm = stamp_word(wl, wa, text, spec['scale'], k, spec['cx'], spec['cy'], col, a,
                                            note=settled and (spec['side'] == side or side == 0))
            if spec['side'] == -1:                                   # VS: keep only this side's half
                wa = wa * ((SIDE > 0) if side == 0 else (SIDE <= 0))
            m = wa > 0
            layer[m] = layer[m] * (1 - wa[m, None]) + wl[m] * wa[m, None]
            alpha[m] = np.maximum(alpha[m], wa[m])
        if t < T.DUR - 1:
            draw_crack(layer, alpha, t)
        # slide the door: integer pixel shift along its normal (+ the shake)
        vec = N * off * (1 if side == 0 else -1) + np.array([sx, sy])
        dx, dy = int(round(vec[0])), int(round(vec[1]))
        xs0, xs1 = max(0, -dx), min(W, W - dx)
        ys0, ys1 = max(0, -dy), min(H, H - dy)
        if xs1 <= xs0 or ys1 <= ys0:
            continue
        src = layer[ys0:ys1, xs0:xs1]
        sa = alpha[ys0:ys1, xs0:xs1][..., None]
        dst = out[ys0 + dy:ys1 + dy, xs0 + dx:xs1 + dx]
        out[ys0 + dy:ys1 + dy, xs0 + dx:xs1 + dx] = dst * (1 - sa) + src * sa
    if off == 0.0 and t < T.DUR - 1:                                 # slam effects over the closed card
        for j, (t0, text) in enumerate(T.TITLE):
            spec = WORDS[text]
            st = word_state(t, j)
            if st is None:
                continue
            col = GOLD if text == 'VS' else RD.COL[0 if text == NAME_A else 1]
            w_px = spec.get('w', len(text) * ADV * spec['scale'])
            h_px = spec.get('h', 7 * spec['scale'])
            light(out, t, j, spec['cx'] + sx, spec['cy'] + sy, col)
            streaks(out, t, j, spec['cx'] + sx, spec['cy'] + sy, w_px, h_px, col)
            dust(out, t, j, spec['cx'] + sx, spec['cy'] + sy, w_px, h_px)
        sparks(out, t)
    out = np.clip(out, 0, 255)
    return out.astype(np.uint8) if u8 else out


def ui_tag(layer, alpha):
    """the small series tag at the top of the card"""
    txt = T.SERIES
    m = text_mask(txt, 1)
    o = dilate(m, 1)
    w_ = m.shape[1]
    x0, y0 = int(round(180 - w_ / 2)), 100
    for mask, c, rr in ((o, INK, 1), (m, np.array([255, 214, 96]), 0)):
        mh, mw = mask.shape
        sub = layer[y0 - rr:y0 - rr + mh, x0 - rr:x0 - rr + mw]
        sub[mask] = c
        alpha[y0 - rr:y0 - rr + mh, x0 - rr:x0 - rr + mw][mask] = 1.0
