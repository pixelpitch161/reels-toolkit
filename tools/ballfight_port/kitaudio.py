"""a kit episode's audio: python ../tools/ballfight_port/kitaudio.py <episode> <out.wav> [buses.npz]
(the master, and the pre-master buses for cmp.py buses)"""
import os, sys
import numpy as np
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'projects'))
from ballfight.kit import episode
ep = episode.load(sys.argv[1])
from ballfight.kit import music as M
mix = M.score()
m, st = M.master(mix)
M.A.write(sys.argv[2], m)
if len(sys.argv) > 3:
    np.savez(sys.argv[3], **{k: v for k, v in mix.buses.items()})
print('wrote', sys.argv[2], sorted(mix.buses))
