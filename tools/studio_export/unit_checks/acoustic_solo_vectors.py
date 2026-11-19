# What it prints: the kit's ACOUSTIC solo (kit/fighters/acoustic.py `Sound.solo`) on an empty mix, with chords in it
# (a tuple of notes is a strum, up on the off-beats; a number a picked note): the 'solo' bus's length and the SHA-256
# of its float64 samples (little-endian, C order), for
#   config   the config's own SOLO (projects/ballfight_electric_vs_acoustic/episode.py: EM, C, D on the beat) from
#            T_SOLO (the reassembly cuts it after 15 entries), and from 4 bars earlier (all 24)
#   ukulele  the ACOUSTIC's SOLO of projects/ballfight/examples/ukulele_vs_acoustic/episode.py (strums on the
#            off-beats, so up-strokes), put into T.SOLO at run time, from T_SOLO
# each case from a fresh note cache and the Pixel Pitch noise generator at its soundtrack start
# (studio/reference/sound/pp_noise_start.json), as a Rust build starts.
# The expected values in studio/crates/bf-harness/src/sound.rs (`the_acoustic_strums_the_chords_of_its_solo`) are
# this output.
#
# Run it from the repo root with the pinned interpreter (tools/studio_export/README.md), in Git Bash:
#   export PYTHONHASHSEED=0 PYTHONPATH="$PWD"
#   .studio-tools/py313/Scripts/python.exe -m tools.studio_export.unit_checks.acoustic_solo_vectors
# The config runs from a private copy under studio/reference/.cache/episodes/ (the kit's caches land there, never
# under projects/). This is a check script, not an export: the tool's export discovery does not look in this folder.
"""Python vectors for the ACOUSTIC's chord solo. Usage: python -m tools.studio_export.unit_checks.acoustic_solo_vectors"""
import json
import os
import runpy

from tools.studio_export import common, m3refs, voices as V

CONFIG = 'ballfight_electric_vs_acoustic'


def main():
    with open(m3refs.config_source(CONFIG), encoding='utf-8', newline='') as fh:
        folder = m3refs.config_folder('acoustic_solo_vectors', m3refs.config_slug(CONFIG), fh.read())
    E, _ = common.kit()
    E.load(folder)
    from ballfight.kit import music as M              # noqa: N812
    import pp_bridge as B
    for k in (0, 1):
        B.scene(M.CFG[k]['pp'])
    with open(V.START, encoding='utf-8') as fh:
        start = json.load(fh)
    T, P = M.T, M.P
    k = list(M.NAMES).index('ACOUSTIC')
    S = M.SOUNDS[k]
    ukulele = runpy.run_path(m3refs.config_source('examples/ukulele_vs_acoustic'))['SOLO']['ACOUSTIC']
    config = list(T.SOLO['ACOUSTIC'])
    print(f'// tempo_bpm {T.BPM!r} beat {T.BEAT.hex()} t_reassemble {T.T_REASSEMBLE.hex()} t_solo {P.T_SOLO.hex()}')
    early = P.T_SOLO - 4 * T.BAR
    for what, solo, t0 in (('config', config, P.T_SOLO), ('config_early', config, early),
                           ('ukulele', ukulele, P.T_SOLO)):
        T.SOLO['ACOUSTIC'] = solo
        V.reset(M, start)
        mix = M.A.Mix(T.DUR + 4.0)
        S.solo(mix, t0)
        x = mix.buses['solo']
        played = sum(1 for bt, m, d in solo if t0 + bt * T.BEAT <= T.T_REASSEMBLE - 0.05)
        print(f'("{what}", "{float(t0).hex()}", {played}, {x.shape[0]}, "{V.sha(x)}"),')
    T.SOLO['ACOUSTIC'] = config
    print('// ukulele solo:', [(float(b), list(m) if isinstance(m, tuple) else m, float(d)) for b, m, d in ukulele])
    return 0


if __name__ == '__main__':
    os.environ.setdefault('PYTHONHASHSEED', '0')
    raise SystemExit(main())
