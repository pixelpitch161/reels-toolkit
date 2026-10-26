"""Loop-clean mux for #34: reuses projects/maze_race/mux.py (keyframe on the last frame, x264 mbtree off, the
soundtrack muxed untrimmed as whole AAC frames) on this project's kept raw frames. No re-render.

    python mux.py            # frames_raw.rgb + audio.wav -> ballfight_trombone_vs_horn_v5.mp4"""
import importlib.util
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location('maze_mux', os.path.join(HERE, '..', 'maze_race', 'mux.py'))
M = importlib.util.module_from_spec(spec)
spec.loader.exec_module(M)

if __name__ == '__main__':
    os.chdir(HERE)
    M.main(raw='frames_raw.rgb', audio='audio.wav', out=sys.argv[1] if len(sys.argv) > 1 else 'ballfight_trombone_vs_horn_v5.mp4')
