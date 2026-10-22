"""Who sits where. 199 metronomes on 5 risers (4 rows x 10, one empty seat on the bass riser = the soloist's)
plus the golden soloist on a podium in front. Shared by sim, motion, scene, render and audio.

World units: metronome spacing 1.0 across, 0.95 front to back (the #1 metronome model, 1.13 tall).
Tier k (0 = bass, front and lowest ... 4 = high bells, back and highest): step top YT[k], centre z ZT[k]."""
import numpy as np
import timeline as T

DX, DZ = 1.0, 0.95
PLAT_T = 0.4                                   # shared platform thickness (its top is y = 0.4 at rest)
ZT = np.array([0.0, -4.4, -8.8, -13.2, -17.6])  # tier centres
YT = np.array([1.0, 2.5, 4.0, 5.5, 7.0])        # step tops at rest
STEP_HZ = 2.2                                  # step half depth
STEP_HX = 9.6                                  # step half width (the risers sit in the middle)
BOARD_HX, BOARD_HZ, BOARD_T = 5.3, 2.0, 0.3
CAN_R = 0.42                                   # riser soda cans (lying along z, as in #1)
CAN_X = 3.6
GCAN_R, GCAN_L = 1.15, 3.4                     # giant cans under the platform (real can proportions)
GCAN_X = 6.2
GCAN_Z = [4.2, 0.0, -4.4, -8.8, -13.2, -17.6]
PLAT_HX, PLAT_Z0, PLAT_Z1 = 10.0, -20.4, 6.6
POD = np.array([0.0, 4.5])                     # podium (x, z)
POD_R, POD_H = 0.85, 0.9
SOLO_S = 1.35                                  # soloist scale

SEC, ROW, COL, LX, LZ = [], [], [], [], []
for k in range(T.NSEC):
    for r in range(T.ROWS):                    # r = 0 front row
        for c in range(T.COLS):
            if (k, r, c) == T.EMPTY:
                continue
            SEC.append(k); ROW.append(r); COL.append(c)
            LX.append((c - 4.5) * DX)
            LZ.append((1.5 - r) * DZ)
SEC, ROW, COL = np.array(SEC), np.array(ROW), np.array(COL)
LX, LZ = np.array(LX), np.array(LZ)
NR = len(SEC)                                  # 199 riser metronomes
SOLO = NR                                      # index of the soloist
N = NR + 1                                     # 200
MEMBERS = [np.nonzero(SEC == k)[0] for k in range(T.NSEC)]
SEC_ALL = np.r_[SEC, -1]                       # section per metronome, -1 = soloist
