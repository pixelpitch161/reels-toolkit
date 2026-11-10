"""stems: the example episode's soundtrack, bus by bus, before and after the master, and the mastered file.

studio/reference/sound/flute_vs_trombone/ (kit.music.main() for the episode, in its own interpreter):

    audio.wav         the mastered soundtrack exactly as the kit writes it: 16 bit, 48,000 Hz, stereo, 2,928,640
                      samples (2,860 AAC frames, 61.013 s)
    stems.npz         every bus after the master (kit.music.master()'s stems: the folded bus times its gain, the
                      compressor and the two limiter scales), as float32 windows of 2.0 s starting at 0.0, 3.4, 12.0,
                      30.0, 43.0, T_KO - 1.0, T_KO + 3.0 and 59.0 s, plus the last 0.25 s before the loop point
    premaster.npz     every bus as kit.music.score() leaves it (before bus gains, reverb, fold and master), the same
                      windows, plus the 0.25 s past the loop point that the master folds onto the start
    rms.json          the RMS of every 100 ms block, in dB: each stem and the master over the 61.013 s, each
                      pre-master bus over its whole 71 s, the reverb's wet part and the compressor's gain; the
                      section levels kit.music.main() prints
    master_case.npz   a small synthetic three-bus mix (its placed signals stored) run through kit.music.master():
                      windows of the master and the stems, for a test of the master on its own

The buses' samples depend on the Pixel Pitch voices' shared noise generator, so the export checks that it starts at
the state in pp_noise_start.json, logs the first-time notes again and compares them with note_calls.json (both
written by `voices`), then proves the replay: with the note cache emptied and the generator put back at the recorded
start state, kit.music.score() gives every bus again bit for bit; without putting it back, the FLUTE's bus differs.

The numbers inside the master (the two scales, the compressor's gain, the reverb's part) are read from a copy of
kit.music.master()'s lines kept here; the export stops unless that copy's result is bit for bit the kit's."""
import contextlib
import hashlib
import io
import json
import os

from . import common
from .dsp_vectors import Cases, check_case_file, fresh, jarg
from . import voices as V

EPISODE = V.EPISODE
TAG = 'stems'
DIR = os.path.join(V.SOUND, EPISODE)
WAV = os.path.join(DIR, 'audio.wav')
STEMS = os.path.join(DIR, 'stems.npz')
PRE = os.path.join(DIR, 'premaster.npz')
RMS = os.path.join(DIR, 'rms.json')
CASE = os.path.join(DIR, 'master_case.npz')
SR = 48000
NA = 2928640                                               # the soundtrack's length in samples (global constraint)
WINDOW = 2 * SR                                            # 2.0 s
SHORT = SR // 4                                            # 0.25 s: the seam and tail windows
BLOCK = SR // 10                                           # 100 ms
BRIEF_STARTS = ('0.0', '3.4', '12.0', '30.0', 'T_KO - 1.0', 'T_KO + 3.0', '59.0')
EXTRA_START = '43.0'                                       # a window in the back half: the only one with the glock
FLOOR = 1e-12                                              # added inside the log: silence is -240 dB, not -inf
CASE_BUSES = ('drums', 'flute', 'fx')                      # the synthetic mix: a bed bus, a fighter's, a shared one


def windows(t_ko):
    """[(label, start time in s)] in time order: the brief's seven and the extra one"""
    table = {'0.0': 0.0, '3.4': 3.4, '12.0': 12.0, '30.0': 30.0, '43.0': 43.0, 'T_KO - 1.0': t_ko - 1.0,
             'T_KO + 3.0': t_ko + 3.0, '59.0': 59.0}
    return sorted(((k, table[k]) for k in BRIEF_STARTS + (EXTRA_START,)), key=lambda kv: kv[1])


def block_db(x):
    """the RMS of each 100 ms block of a (n, 2) buffer in dB, over both channels; the last block may be shorter"""
    import numpy as np
    return [float(20 * np.log10(np.sqrt((x[i:i + BLOCK] ** 2).mean()) + FLOOR)) for i in range(0, len(x), BLOCK)]


def sha(x):
    import numpy as np
    return hashlib.sha256(np.ascontiguousarray(x, '<f8').tobytes()).hexdigest()


# ---------------------------------------------------------------- the master, opened up
def master_parts(M, mix, ceiling=0.89):
    """the same lines as kit/music.py master(), kept to read the values inside it. The caller compares `m` and
    `stems` with the kit's own result bit for bit before it trusts anything here"""
    import numpy as np
    from scipy.signal import lfilter
    A, T, n_a = M.A, M.T, M.NA
    levels = {}
    for s in M.SOUNDS:
        levels[s.bus] = (s.bus_gain, s.send)
    levels.update(M.BUS)
    levels.update(M.bed.buses)
    levels.update({k: tuple(v) for k, v in T.MIX.items()})
    buses = {k: v * levels.get(k, (1.0, 0.0))[0] for k, v in mix.buses.items()}
    send = sum(buses.get(k, 0) * levels[k][1] for k in levels)
    rdur, rdec, rwet = M.bed.reverb
    wet = A.reverb(send, dur=rdur, decay=rdec) * rwet
    full = sum(buses.values()) + wet

    def fold(x):
        y = x[:n_a].copy()
        tail = x[n_a:]
        y[:len(tail)] += tail[:n_a]
        return y
    m = fold(full)
    thr, ex = M.bed.comp
    env = np.sqrt(lfilter([0.002], [1, -0.998], (np.concatenate([m[-A.SR:], m]) ** 2).mean(1)))[A.SR:]
    gain = np.where(env > thr, (thr / np.maximum(env, 1e-9)) ** ex, 1.0)[:, None]
    m = m * gain
    p9997 = np.percentile(np.abs(m), 99.97)
    s1 = ceiling / (p9997 + 1e-9)
    m = np.tanh(m * s1 * 1.1) / np.tanh(1.1)
    peak = np.abs(m).max()
    s2 = ceiling / (peak + 1e-9)
    m *= s2
    stems = {k: fold(buses[k]) * gain * s1 * s2 for k in buses}
    return dict(m=m, stems=stems, levels=levels, wet=fold(wet), gain=gain[:, 0], s1=float(s1), s2=float(s2),
                p9997=float(p9997), peak=float(peak), ceiling=ceiling, comp=(thr, ex), reverb=(rdur, rdec, rwet),
                bus_order=list(mix.buses), send_order=[k for k in levels if k in buses])


def master_doc(parts):
    """the master's settings and inner numbers for a sidecar (floats as float.hex())"""
    hx = common.hexf
    thr, ex = parts['comp']
    rdur, rdec, rwet = parts['reverb']
    return dict(
        ceiling=hx(parts['ceiling']), comp=dict(thr=hx(thr), ex=hx(ex)),
        reverb=dict(dur=hx(rdur), decay=hx(rdec), wet=hx(rwet), seed=3),
        levels={k: dict(gain=hx(float(g)), send=hx(float(s))) for k, (g, s) in parts['levels'].items()},
        bus_order=parts['bus_order'], send_order=parts['send_order'],
        p9997=hx(parts['p9997']), s1=hx(parts['s1']), peak=hx(parts['peak']), s2=hx(parts['s2']),
        notes=dict(
            levels='bus: its gain and its reverb send (the fighters\' Sound.bus_gain / send, kit.music.BUS, the '
                   'bed\'s buses, then the episode\'s MIX overrides)',
            bus_order='the order the buses are added up in (the order kit.music.score() first touched them)',
            send_order='the order the reverb send adds the buses in (the order of `levels`)',
            p9997='np.percentile(|folded mix * compressor gain|, 99.97); s1 = ceiling / (p9997 + 1e-9)',
            peak='max |tanh(mix * s1 * 1.1) / tanh(1.1)|; s2 = ceiling / (peak + 1e-9)',
            stems='stem = fold(bus * gain) * compressor gain * s1 * s2 (no reverb: the wet part is only in the '
                  'master)'))


# ---------------------------------------------------------------- the files
def window_file(path, about, buffers, wins, short, note_bus, **top):
    """`buffers` {bus: (n, 2) float64} cut into the windows `wins` [(label, t)] and one short window `short`
    (label, first sample), stored as float32"""
    import numpy as np
    c = Cases(path, tool='stems', module=__name__, about=about, sample_rate=SR, buses=list(buffers),
              dtype='float32, converted from the kit\'s float64 (the comparison bar is -120 dBFS; float32 keeps '
                    '24 bits); every window is (samples, 2): left, right',
              windows='a case is one window: its `arrays` map each bus to the samples [i0, i0 + n) of that bus, '
                      'with i0 = int(round(t * 48000)); `window` names the start time as the brief writes it. '
                      '`silent` lists the [case, bus] pairs that are all zero',
              **top)
    silent = []

    def add(label, t, i0, n, what):
        arrays = {}
        for bus, x in buffers.items():
            assert x.dtype == np.float64 and x.ndim == 2 and x.shape[1] == 2 and i0 + n <= len(x), (bus, x.shape, i0)
            arrays[bus] = x[i0:i0 + n].astype(np.float32)
            assert arrays[bus].shape == (n, 2)
        cid = c.add(f'{note_bus}[i0:i0 + n]', dict(t=jarg(float(t)), i0=i0, n=n), arrays, window=label, what=what)
        silent.extend([cid, bus] for bus, a in arrays.items() if not a.any())
    for label, t in wins:
        add(label, t, int(round(t * SR)), WINDOW, f'2.0 s from {label} s')
    add(short[0], short[1] / SR, short[1], SHORT, short[2])
    c.top['silent'] = silent
    c.top['full_sha256'] = {bus: sha(x) for bus, x in buffers.items()}
    return c.write()


def rms_doc(M, parts, mix, sections):
    return dict(
        about=f'{EPISODE}: the RMS of every 100 ms block in dB, for each stem and the master (after the master, '
              f'{NA} samples) and each bus before the master (the whole mix buffer)',
        tool='stems', episode=EPISODE, sample_rate=SR, block=BLOCK,
        formula='20 * log10(sqrt(mean(x ** 2)) + 1e-12) over both channels of samples [b * 4800, (b + 1) * 4800); '
                'the last block is what is left (640 samples for the stems). Silence is -240',
        stems=dict(length=NA, blocks=-(-NA // BLOCK), last_block=NA % BLOCK,
                   db=dict({k: block_db(v) for k, v in parts['stems'].items()}, master=block_db(parts['m']))),
        premaster=dict(length=len(next(iter(mix.buses.values()))),
                       blocks=-(-len(next(iter(mix.buses.values()))) // BLOCK),
                       db={k: block_db(v) for k, v in mix.buses.items()}),
        master_chain=dict(
            notes=dict(wet_db='the reverb\'s part of the mix, after the fold and before the compressor (it is in '
                              'the master only, in no stem)',
                       gain_first='the compressor\'s gain at the first sample of each block (1.0 = not compressing)',
                       gain_min='the compressor\'s smallest gain inside each block'),
            wet_db=block_db(parts['wet']),
            gain_first=[float(parts['gain'][i]) for i in range(0, NA, BLOCK)],
            gain_min=[float(parts['gain'][i:i + BLOCK].min()) for i in range(0, NA, BLOCK)]),
        sections=dict(
            about='reelkit.audio.section_levels(audio.wav, spans): what kit.music.main() prints (dBFS of the 16-bit '
                  'file, both channels)',
            rows=[dict(a=float(a), b=float(b), rms=float(r), peak=float(p)) for a, b, r, p in sections]))


def master_case_file(M):
    """a sparse three-bus mix placed with Mix.place, mastered by the kit: something at the very start, in the
    middle and across the loop point, loud enough to work the compressor"""
    import numpy as np
    A, T = M.A, M.T
    r = np.random.default_rng(61)
    t06 = A.tt(int(0.6 * SR))
    tone = (np.sin(2 * np.pi * 880.0 * t06) + 0.4 * np.sin(2 * np.pi * 1320.0 * t06)) * np.exp(-t06 / 0.2)
    burst = r.normal(0, 0.5, int(0.1 * SR)) * np.exp(-A.tt(int(0.1 * SR)) / 0.03)
    signals = dict(kick=A.kick(), tone=tone, burst=burst, crash=A.crash(1.0))
    places = [('drums', 'kick', 0.004, 0.7, 0.0), ('flute', 'tone', 0.05, 0.4, -0.2), ('fx', 'burst', 0.12, 0.5, 0.3),
              ('drums', 'kick', 30.0, 0.7, 0.0), ('flute', 'tone', 29.95, 0.4, -0.2), ('fx', 'burst', 30.1, 0.5, 0.3),
              ('fx', 'crash', T.DUR - 0.6, 0.6, 0.0), ('drums', 'kick', T.DUR - 0.2, 0.7, 0.0),
              ('flute', 'tone', T.DUR - 0.1, 0.4, 0.3)]
    mix = A.Mix(T.DUR + 4.0)
    for bus, name, t, gain, pan in places:
        mix.place(bus, signals[name], t, gain, pan)
    assert tuple(sorted(mix.buses)) == CASE_BUSES
    m, st = M.master(mix)
    parts = master_parts(M, mix)
    assert np.array_equal(parts['m'], m) and all(np.array_equal(parts['stems'][k], st[k]) for k in st)
    assert m.shape == (NA, 2) and parts['gain'].min() < 0.9, 'the synthetic mix does not work the compressor'
    assert parts['p9997'] > 0.05, 'the synthetic mix is too sparse: its 99.97th percentile is next to nothing'
    c = Cases(CASE, tool='stems', module=__name__,
              about='a synthetic three-bus mix run through kit.music.master() with the flute_vs_trombone episode '
                    'loaded (its bus levels, the big_top bed\'s reverb and compressor): the placed signals, and '
                    'float64 windows of the master and of each stem',
              episode=EPISODE, sample_rate=SR, na=NA, mix_dur=jarg(float(T.DUR + 4.0)),
              mix='mix = reelkit.audio.Mix(mix_dur); then mix.place(bus, signal, t, gain, pan) for each row of '
                  '`placements`, in order',
              placements=[dict(bus=bus, signal=name, t=jarg(float(t)), gain=jarg(gain), pan=jarg(pan),
                               i0=int(round(t * SR))) for bus, name, t, gain, pan in places],
              signals={k: f'{len(v)} samples, mono float64 (npz member "{k}")' for k, v in signals.items()},
              master=master_doc(parts),
              db=dict(about='the RMS of every 100 ms block (rms.json\'s formula)',
                      master=block_db(m), **{k: block_db(v) for k, v in st.items()}))
    for k, v in signals.items():
        c.shared(k, v)
    for label, i0, what in (('start', 0, 'the first 0.25 s: what folds back from past the loop point lands here'),
                            ('middle', int(round(29.95 * SR)), '0.25 s from 29.95 s: the compressor closing'),
                            ('seam', NA - SHORT, 'the last 0.25 s before the loop point')):
        arrays = dict(master=m[i0:i0 + SHORT], **{k: st[k][i0:i0 + SHORT] for k in CASE_BUSES})
        c.add('kit.music.master(mix): (m, stems)[i0:i0 + n]', dict(i0=i0, n=SHORT), arrays, window=label, what=what)
    # the signals are members no window case names: give them a case of their own
    c.add('the placed signals', dict(), {k: k for k in signals}, window='inputs',
          what='the mono signals `placements` puts into the mix')
    return c.write()


# ---------------------------------------------------------------- the checks the files must pass
def _json(path):
    assert os.path.isfile(path), f'{common.rel(path)}: missing'
    with open(path, encoding='utf-8') as f:
        return json.load(f)


def check():
    """what the stems export must leave on disk, from the files alone: audio.wav is 16 bit, 48,000 Hz, stereo and
    exactly 2,928,640 samples; every window has its stated length, is float32 and stereo, and the brief's seven
    start times are all there; every bus is heard in at least one window; rms.json has a value for every block and
    agrees with the windows; the master case reads back"""
    import numpy as np
    from scipy.io import wavfile
    assert os.path.isfile(WAV), f'{common.rel(WAV)}: missing'
    sr, w = wavfile.read(WAV)
    assert sr == SR and w.dtype == np.int16 and w.shape == (NA, 2), (sr, w.dtype, w.shape)
    assert os.path.getsize(WAV) == 44 + NA * 4, os.path.getsize(WAV)
    rms = _json(RMS)
    assert rms['block'] == BLOCK and rms['stems']['length'] == NA and rms['stems']['blocks'] == 611
    out = {}
    for path, kind in ((STEMS, 'stems'), (PRE, 'premaster')):
        doc, arr = check_case_file(path)
        buses = doc['buses']
        labels = [c['window'] for c in doc['cases']]
        assert all(lab in labels for lab in BRIEF_STARTS), f'{common.rel(path)}: a start time of the brief is missing'
        heard = set()
        for c in doc['cases']:
            a = c['args']
            n = SHORT if c['window'] not in BRIEF_STARTS + (EXTRA_START,) else WINDOW
            assert a['n'] == n and a['i0'] == int(round(float.fromhex(a['t']) * SR)), c['id']
            assert sorted(c['arrays']) == sorted(buses), c['id']
            for bus, member in c['arrays'].items():
                x = arr[member]
                assert x.dtype.str == '<f4' and x.shape == (n, 2), f"{kind} {c['id']} {bus}: {x.dtype} {x.shape}"
                if x.any():
                    heard.add(bus)
                assert ([c['id'], bus] in doc['silent']) == (not x.any()), (c['id'], bus)
                db = rms[kind]['db'][bus]
                if a['i0'] % BLOCK == 0:                      # the window's blocks against rms.json
                    for j in range(n // BLOCK):
                        ref = db[a['i0'] // BLOCK + j]
                        blk = x[j * BLOCK:(j + 1) * BLOCK].astype(np.float64)
                        got = 20 * np.log10(np.sqrt((blk ** 2).mean()) + FLOOR)
                        assert ref < -90 or abs(got - ref) < 1e-3, (kind, c['id'], bus, j, got, ref)
        assert heard == set(buses), f'{kind}: no window in which {sorted(set(buses) - heard)} is heard'
        length = rms[kind]['length']
        for bus in buses:
            assert len(rms[kind]['db'][bus]) == -(-length // BLOCK), (kind, bus)
        out[kind] = (len(doc['cases']), len(buses))
    assert len(rms['stems']['db']['master']) == 611
    assert all(len(rms['master_chain'][k]) == 611 for k in ('wet_db', 'gain_first', 'gain_min'))
    # the 16-bit file against rms.json's master profile (16 bits are good to about -90 dBFS)
    wf = w.astype(np.float64) / 32767
    for b in range(0, 611, 7):
        blk = wf[b * BLOCK:(b + 1) * BLOCK]
        got = 20 * np.log10(np.sqrt((blk ** 2).mean()) + FLOOR)
        assert abs(got - rms['stems']['db']['master'][b]) < 0.05, (b, got, rms['stems']['db']['master'][b])
    doc, arr = check_case_file(CASE)
    for c in doc['cases']:
        if c['window'] != 'inputs':
            for member in c['arrays'].values():
                assert arr[member].dtype.str == '<f8' and arr[member].shape == (SHORT, 2), c['id']
    assert sorted(doc['master']['levels']) and len(doc['placements']) >= 6
    out['master_case'] = len(doc['cases'])
    return out


# ---------------------------------------------------------------- the export
def build():
    """kit.music.main() in this interpreter with the notes logged: (M, B, D, the mix, the master and stems the kit
    made, the note log, the generator state before and after the score). audio.wav and the kit's stem wavs land in
    the episode's cache folder"""
    M, B, D = V.load(TAG)
    start = V.noise_state()
    kept = {}
    score0, master0 = M.score, M.master

    def score():
        kept['mix'], kept['log'] = V.record_score(M, B, D, score=score0)
        kept['end'] = V.noise_state()
        return kept['mix']

    def master(mix, *a, **k):
        kept['m'], kept['st'] = master0(mix, *a, **k)
        return kept['m'], kept['st']
    M.score, M.master = score, master
    try:
        with contextlib.redirect_stdout(io.StringIO()):      # (main prints the section levels: rms.json has them)
            wav = M.main()
    finally:
        M.score, M.master = score0, master0
    return M, B, D, kept, start, wav


def child():
    import numpy as np
    from scipy.io import wavfile
    from . import manifest
    for p in (V.START, V.CALLS):
        if not os.path.isfile(p):
            raise SystemExit(f'stems: {common.rel(p)} is missing: run the voices export first')
    M, B, D, kept, start, wav = build()
    mix, m, st, log = kept['mix'], kept['m'], kept['st'], kept['log']
    assert M.NA == NA and M.SR == SR and m.shape == (NA, 2) and m.dtype == np.float64
    # the same notes as the voices export logged, from the same start state
    want_start, want = V._json(V.START), V._json(V.CALLS)
    assert start == dict(state=want_start['state'], inc=want_start['inc']), 'the noise generator did not start fresh'
    common.same(common.plain(log.rows()), want['calls'], 'note_calls.json')
    assert kept['end']['state'] == want['end']
    # the master opened up: its copy here must be the kit's master bit for bit
    parts = master_parts(M, mix)
    assert np.array_equal(parts['m'], m) and list(parts['stems']) == list(st)
    assert all(np.array_equal(parts['stems'][k], st[k]) for k in st)
    # the replay: the recorded start state and an empty note cache give the same buses again; any other state does not
    first = {k: sha(v) for k, v in mix.buses.items()}
    V.reset(M, start)
    again = {k: sha(v) for k, v in M.score().buses.items()}
    assert again == first, sorted(k for k in first if first[k] != again.get(k))
    M._VC.clear()
    M._GAIN.clear()                                          # (the generator stays where that score left it)
    other = {k: sha(v) for k, v in M.score().buses.items()}
    differ = sorted(k for k in first if first[k] != other[k])
    assert 'flute' in differ, 'the flute\'s bus does not depend on the noise state: the replay proves nothing'
    # the mastered file, as the kit wrote it
    sr, w = wavfile.read(wav)
    assert sr == SR and np.array_equal(w, (np.clip(m, -1, 1) * 32767).astype(np.int16))
    with open(wav, 'rb') as f:
        common.write_bytes(WAV, f.read())
    wins = windows(M.T_KO)
    span = [(0, M.T.BAR), (M.T.BAR, M.T.T_FIGHT), (M.T.T_FIGHT, 10), (10, 20), (20, 32), (32, 44), (44, M.T_KO),
            (M.T_KO, M.P.T_SOLO), (M.P.T_SOLO, M.T.T_REASSEMBLE), (M.T.T_REASSEMBLE, 61)]   # kit.music.main()'s
    files = [
        window_file(STEMS, f'{EPISODE}: every bus after the master (kit.music.master(kit.music.score())[1]), in '
                           f'windows', st, wins, ('seam', NA - SHORT, 'the last 0.25 s before the loop point: what '
                                                                      'runs into sample 0'),
                    'kit.music.master(mix)[1][bus]', episode=EPISODE, length=NA, t_ko=common.hexf(M.T_KO),
                    master=master_doc(parts)),
        window_file(PRE, f'{EPISODE}: every bus as kit.music.score() leaves it (mix.buses, before the bus gains, the '
                         f'reverb, the fold and the master), in the same windows', dict(mix.buses), wins,
                    ('tail', NA, 'the 0.25 s past the loop point: what the master folds onto the start (after '
                                 'it the buses are silent)'),
                    'kit.music.score().buses[bus]', episode=EPISODE, length=len(mix.buses['fx']),
                    t_ko=common.hexf(M.T_KO), fold_at=NA,
                    last_sound={k: int(np.nonzero(v.any(1))[0].max()) for k, v in mix.buses.items()}),
    ]
    common.write_json(RMS, rms_doc(M, parts, mix, M.A.section_levels(wav, span)), register=False)
    files.append(master_case_file(M))
    got = check()
    for p in (WAV, RMS):                                     # listed only now that the checks passed
        manifest.add(p)
        print(f'stems: {common.rel(p)}  {os.path.getsize(p)} bytes')
    for f in files:
        f.register()
    print(f"stems: {len(first)} buses ({', '.join(first)}); the note log equals note_calls.json ({len(log.calls)} "
          f'first-time notes); a second score() from the recorded start state gives every bus again bit for bit; '
          f"from the state the first one ended in, these differ: {', '.join(differ)}")
    print(f"stems: master s1 {parts['s1']!r}, s2 {parts['s2']!r}, smallest compressor gain {parts['gain'].min():.4f}; "
          f"audio.wav {w.shape[0]} samples = {w.shape[0] / SR:.6f} s; checked: {got}")
    return 0


def run():
    fresh('stems')


EXPORTS = {'stems': run}
ORDER = {'stems': 55}                                        # after voices (50): it reads the files voices writes
