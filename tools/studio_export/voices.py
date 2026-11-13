"""voices: the Pixel Pitch voices, the kit.music helpers, and the noise stream they share.

Every Pixel Pitch voice takes its noise from ONE generator, `drums60_lowpoly._rng = np.random.default_rng(77)`,
advanced by every `nz(n)` call (the FLUTE's breath noise is `lp(nz(n), 2500)`). A note's samples therefore depend on
every noisy note synthesised before it in the same interpreter, so this export runs in its own interpreter and
records the generator's state with everything it stores:

    studio/reference/sound/pp_noise_start.json
        the generator's state after all imports and before the first note
    studio/reference/sound/flute_vs_trombone/note_calls.json
        every first-time kit.music.note call while kit.music.score() builds the episode, in order, with its
        arguments, the generator state before and after, the pp_bridge.voice calls inside it (the first note of an
        instrument also synthesises the level-matching note of kit.music.inst_gain, AFTER its own), and a SHA-256 of
        its samples
    studio/reference/sound/flute_vs_trombone/first_notes.npz
        the first three of those notes as arrays: the note, the voice output behind it, the inst_gain note
    studio/reference/sound/voices/FLUTE.npz, TROMBONE.npz
        pp_bridge.voice(name, midi, vel, dur) at 4 pitches x 2 velocities x 3 durations (stored as float32), each
        with the generator state just before the call and a SHA-256 of its float64 samples
    studio/reference/sound/voices/helpers.npz
        kit.music's helpers: inst_gain, note, fit, gliss, bend, vibrato, wah, colour, clank, hiss, roar, thunder,
        riser
    studio/reference/sound/voices/scalars.json
        kit.music's number helpers: h01, Thinner, h01_37, thin_37, q16

The export proves what it records: it restores the start state from the files it wrote, synthesises every logged
note again in order and compares each with its SHA-256 (and the first three with their stored arrays), and checks
that the same call made from any other state gives different samples."""
import hashlib
import json
import os

from . import common
from .dsp_vectors import Cases, check_case_file, fresh, jarg, call_text

EPISODE = 'flute_vs_trombone'
TAG = 'voices'
SOUND = common.ref('sound')
START = os.path.join(SOUND, 'pp_noise_start.json')
CALLS = os.path.join(SOUND, EPISODE, 'note_calls.json')
FIRST = os.path.join(SOUND, EPISODE, 'first_notes.npz')
VOICE_DIR = os.path.join(SOUND, 'voices')
HELPERS = os.path.join(VOICE_DIR, 'helpers.npz')
SCALARS = os.path.join(VOICE_DIR, 'scalars.json')
NOISE_SEED = 77
N_FIRST = 3                                                # first-time notes kept as arrays
VOICES = {'FLUTE': (72, 76, 84, 93), 'TROMBONE': (40, 47, 52, 64)}
VELS = (0.6, 0.9)
DURS = (0.12, 0.25, 1.0)
HELPER_NAMES = ('inst_gain', 'note', 'fit', 'gliss', 'bend', 'vibrato', 'wah', 'colour', 'clank', 'hiss', 'roar',
                'thunder', 'riser')


# ---------------------------------------------------------------- the shared noise generator
def noise():
    """the generator itself (drums60_lowpoly is importable once pp_bridge is)"""
    import drums60_lowpoly as D
    return D._rng


def noise_state():
    """{'state', 'inc'} of the shared generator as decimal strings. PCG64 has two more fields, the buffered half
    of a 32-bit draw; normal() never leaves one, and this refuses to hide it if something did"""
    s = noise().bit_generator.state
    assert s['bit_generator'] == 'PCG64' and s['has_uint32'] == 0, s
    return dict(state=common.dec(s['state']['state']), inc=common.dec(s['state']['inc']))


def set_noise_state(d):
    noise().bit_generator.state = dict(bit_generator='PCG64', has_uint32=0, uinteger=0,
                                       state=dict(state=int(d['state']), inc=int(d['inc'])))


def sha(x):
    """SHA-256 of an array's float64 samples (little-endian, C order)"""
    import numpy as np
    return hashlib.sha256(np.ascontiguousarray(x, '<f8').tobytes()).hexdigest()


def load(tag):
    """the episode under its own cache tag, kit.music, pp_bridge and the series' drum module, with the two voices'
    Pixel Pitch modules loaded (pp_bridge imports them on first use; nothing they do on import draws noise, which
    the caller checks against the recorded start state)"""
    common.episode(EPISODE, tag)
    from ballfight.kit import music as M
    import pp_bridge as B
    for k in (0, 1):
        B.scene(M.CFG[k]['pp'])
    import drums60_lowpoly as D
    assert tuple(M.NAMES) == ('FLUTE', 'TROMBONE') and M.EP.seed == 392, (M.NAMES, M.EP.seed)
    return M, B, D


def reset(M, state):
    """forget every cached note and gain, and put the generator at `state`"""
    M._VC.clear()
    M._GAIN.clear()
    set_noise_state(state)


class NoteLog:
    """wraps kit.music.note, kit.music.inst_gain, pp_bridge.voice and the series' nz() while a score is built, and
    remembers every first-time note: its arguments, the generator state around it, the voice calls it made and
    how many normals each drew. `keep` first notes also keep their arrays. Nothing it wraps behaves differently"""

    def __init__(self, M, B, D, keep=0):
        self.M, self.B, self.D, self.keep = M, B, D, keep
        self.calls, self.uses, self.total = [], {}, 0
        self.loose, self.stray = [], []                     # voice calls outside a note; draws outside a voice call
        self._note = self._voice = None
        self._in_gain = 0
        self._undo = []

    def install(self):
        M, B, D = self.M, self.B, self.D
        note0, gain0, voice0, nz0 = M.note, M.inst_gain, B.voice, D.nz

        def nz(n):
            if self._voice is None:
                self.stray.append(int(n))
            else:
                self._voice['draws'].append(int(n))
            return nz0(n)

        def voice(name, midi, vel, dur):
            rec = dict(why='inst_gain' if self._in_gain else 'note', name=name, midi=int(midi), vel=float(vel),
                       dur=float(dur), draws=[], before=noise_state()['state'])
            outer, self._voice = self._voice, rec
            try:
                x = voice0(name, midi, vel, dur)
            finally:
                self._voice = outer
            rec.update(after=noise_state()['state'], samples=len(x), sha256=sha(x))
            if self._note is None:
                self.loose.append(rec)
            else:
                self._note['voice_calls'].append(rec)
                if len(self.calls) < self.keep:
                    rec['x'] = x
            return x

        def inst_gain(name):
            self._in_gain += 1
            try:
                return gain0(name)
            finally:
                self._in_gain -= 1

        def note(name, midi, dur=0.18, cap=0.6, vel=0.85):
            key = (name, int(midi), round(dur, 3), cap, round(vel, 2))      # kit.music.note's own cache key
            self.total += 1
            if key in M._VC:
                self.uses[key] += 1
                return note0(name, midi, dur, cap, vel)
            rec = dict(name=name, midi=int(midi), dur=float(dur), cap=float(cap), vel=float(vel), key=key,
                       voice_calls=[], before=noise_state()['state'])
            self._note = rec
            try:
                out = note0(name, midi, dur, cap, vel)
            finally:
                self._note = None
            rec.update(after=noise_state()['state'], samples=len(out), sha256=sha(out), gain=float(M._GAIN[name]))
            if len(self.calls) < self.keep:
                rec['out'] = out
            self.uses[key] = 1
            self.calls.append(rec)
            return out
        for obj, name, new in ((M, 'note', note), (M, 'inst_gain', inst_gain), (B, 'voice', voice), (D, 'nz', nz)):
            self._undo.append((obj, name, getattr(obj, name)))
            setattr(obj, name, new)
        for side, s in enumerate(getattr(M, 'SOUNDS', ())):
            self._own_helper(side, s)
        return self

    def _own_helper(self, side, s):
        """a part's own note helper (plan m4 Task 10: an older episode's `_fit` with its own fade), declared on its
        Sound as `note_helper = dict(note='<method>', gain='<method>', fade=<s>)`: `note(name, midi, dur, cap, vel)`
        caches by kit.music.note's key in a cache of the part's own and calls pp_bridge.voice, then `gain(name)`
        (the part's own level matching). Each first-time note is logged as kit.music.note's are, with `fade` and
        `side`; a Sound without one (every kit fighter) is left alone"""
        h = getattr(type(s), 'note_helper', None)
        if h is None:
            return
        if not (isinstance(h, dict) and set(h) == {'note', 'gain', 'fade'} and isinstance(h['fade'], float)):
            raise SystemExit(f'{type(s).__module__}.Sound.note_helper: dict(note=<method>, gain=<method>, fade=<float>)')
        note0, gain0, fade = getattr(s, h['note']), getattr(s, h['gain']), h['fade']
        seen = set()

        def gain(name):
            self._in_gain += 1
            try:
                return gain0(name)
            finally:
                self._in_gain -= 1

        def note(name, midi, dur=0.18, cap=0.6, vel=0.85):
            key = ('own', side, fade, name, int(midi), round(dur, 3), cap, round(vel, 2))
            self.total += 1
            if key in seen:
                self.uses[key] += 1
                return note0(name, midi, dur, cap, vel)
            rec = dict(name=name, midi=int(midi), dur=float(dur), cap=float(cap), vel=float(vel), key=key,
                       voice_calls=[], before=noise_state()['state'], fade=float(fade), side=side)
            self._note = rec
            try:
                out = note0(name, midi, dur, cap, vel)
            finally:
                self._note = None
            rec.update(after=noise_state()['state'], samples=len(out), sha256=sha(out), gain=float(gain0(name)))
            if len(self.calls) < self.keep:
                rec['out'] = out
            seen.add(key)
            self.uses[key] = 1
            self.calls.append(rec)
            return out
        for name, new in ((h['note'], note), (h['gain'], gain)):
            self._undo.append((s, name, None))
            setattr(s, name, new)

    def remove(self):
        for obj, name, old in reversed(self._undo):
            if old is None:
                delattr(obj, name)                          # a part's own helper: its method again
            else:
                setattr(obj, name, old)
        self._undo = []

    def rows(self):
        """the calls as the rows of note_calls.json"""
        out = []
        for i, c in enumerate(self.calls):
            vc = [dict(why=v['why'], name=v['name'], midi=v['midi'], vel=v['vel'], dur=v['dur'],
                       hex=dict(vel=common.hexf(v['vel']), dur=common.hexf(v['dur'])), draws=v['draws'],
                       noise_before=v['before'], samples=v['samples'], sha256=v['sha256']) for v in c['voice_calls']]
            out.append(dict(i=i, name=c['name'], midi=c['midi'], dur=c['dur'], cap=c['cap'], vel=c['vel'],
                            hex=dict(dur=common.hexf(c['dur']), cap=common.hexf(c['cap']), vel=common.hexf(c['vel'])),
                            uses=self.uses[c['key']], noise_before=c['before'], noise_after=c['after'],
                            normals=sum(sum(v['draws']) for v in c['voice_calls']), voice_calls=vc,
                            gain=common.hexf(c['gain']), samples=c['samples'], sha256=c['sha256']))
        return out


def record_score(M, B, D, keep=0, score=None):
    """kit.music.score() (or `score`, when the caller has wrapped the kit's) with a NoteLog in place: (the mix, the
    log)"""
    log = NoteLog(M, B, D, keep).install()
    try:
        mix = (score or M.score)()
    finally:
        log.remove()
    assert not log.stray and not log.loose, (log.stray, log.loose)   # every draw belongs to a note of the score
    return mix, log


def calls_doc(M, log, start, end):
    rows = log.rows()
    return dict(
        about=f'{EPISODE}: every first-time kit.music.note(name, midi, dur, cap, vel) call while kit.music.score() '
              f'builds the soundtrack, in order. A note is synthesised once and cached by (name, int(midi), '
              f'round(dur, 3), cap, round(vel, 2)); later calls with the same key reuse it and draw nothing',
        tool='voices', episode=EPISODE, seed=M.EP.seed, names=list(M.NAMES),
        generator=dict(bit_generator='PCG64', seed=NOISE_SEED, inc=start['inc'],
                       where='series/instrument-history/drums60_lowpoly.py: _rng = np.random.default_rng(77); '
                             'nz(n) = _rng.normal(0, 1, n)'),
        start=start['state'], end=end['state'],
        note_calls=log.total, first_time=len(rows), normals=sum(r['normals'] for r in rows),
        notes=dict(
            order='the order is the order kit.music.score() first asks for each note: the bed and the fighters in '
                  'the order score() calls them. The Studio must synthesise first-time notes in this order',
            args='dur, cap and vel are the values passed (floats; `hex` has their float.hex()); the voice is called '
                 'as pp_bridge.voice(name, int(midi), vel, dur), then cut or padded to int(cap * 48000) samples with '
                 'a fade (kit.music.fit) and multiplied by the instrument\'s gain',
            voice_calls='the pp_bridge.voice calls made inside the note, in order. "why": "note" is the note '
                        'itself; "inst_gain" is the level-matching note kit.music.inst_gain synthesises the first '
                        'time an instrument plays: voice(name, (lo + hi) // 2, 0.8, 0.2). It comes AFTER the '
                        'note\'s own voice call, because Python evaluates fit(voice(...)) before inst_gain(name)',
            draws='the n of each nz(n) call inside the voice call (the FLUTE draws one block per note; the '
                  'TROMBONE draws none)',
            noise='noise_before / noise_after: the generator\'s `state` as a decimal string (its `inc` never '
                  'changes: generator.inc). noise_after of one row is noise_before of the next',
            gain='kit.music.inst_gain(name) as float.hex()',
            uses='how many times score() asked for this note in all',
            sha256='SHA-256 of the note\'s float64 samples (little-endian); voice_calls[].sha256 of the voice '
                   'output at 48 kHz before fit and gain'),
        calls=rows)


# ---------------------------------------------------------------- the files
def start_doc(start):
    return dict(
        about='the Pixel Pitch voices\' shared noise generator (drums60_lowpoly._rng, numpy PCG64) after every '
              'import and before the first note: where a soundtrack build starts',
        tool='voices', episode=EPISODE,
        generator='np.random.default_rng(77) in series/instrument-history/drums60_lowpoly.py; every nz(n) = '
                  '_rng.normal(0, 1, n) advances it',
        seed=NOISE_SEED, state=start['state'], inc=start['inc'], has_uint32=0, uinteger=0,
        draws_before_start=0,
        note='this is the state of a fresh np.random.default_rng(77): importing kit.music, pp_bridge and the '
             'FLUTE\'s and TROMBONE\'s Pixel Pitch modules draws nothing from it')


def first_notes_file(log, start):
    import numpy as np
    c = Cases(FIRST, tool='voices', module=__name__,
              about=f'{EPISODE}: the first {N_FIRST} notes kit.music.score() synthesises, as arrays (float64, '
                    f'48,000 Hz), for a replay from the recorded start state',
              start=start,
              notes=dict(
                  out='kit.music.note(name, midi, dur, cap, vel): fit(voice, cap) * inst_gain(name)',
                  voice='pp_bridge.voice(name, midi, vel, dur): the note before fit and gain',
                  gain_voice='(first case only) pp_bridge.voice(name, (lo + hi) // 2, 0.8, 0.2): the note '
                             'kit.music.inst_gain synthesises, after `voice`, to measure the instrument\'s level',
                  noise='the generator\'s state and inc (decimal strings) just before the call; noise_after is '
                        'the state the call left'))
    for rec in log.calls[:N_FIRST]:
        args = dict(name=rec['name'], midi=rec['midi'], dur=rec['dur'], cap=rec['cap'], vel=rec['vel'])
        arrays = dict(out=np.asarray(rec['out']))
        for v in rec['voice_calls']:
            arrays['voice' if v['why'] == 'note' else 'gain_voice'] = np.asarray(v['x'])
        c.add(call_text('kit.music.note', args), {k: jarg(v) for k, v in args.items()}, arrays,
              noise=dict(state=rec['before'], inc=start['inc']), noise_after=rec['after'],
              gain=common.hexf(rec['gain']),
              voice_calls=[dict(why=v['why'], call=call_text('pp_bridge.voice', dict(
                  name=v['name'], midi=v['midi'], vel=v['vel'], dur=v['dur'])), draws=v['draws'],
                  noise=dict(state=v['before'], inc=start['inc'])) for v in rec['voice_calls']])
    return c.write()


def voice_file(M, B, D, name, start):
    """pp_bridge.voice for one instrument, each case from the state the one before left (the first from `start`)"""
    import numpy as np
    _, modname, key, register, *_ = B.entrant(name)
    scene = B.scene(name)
    c = Cases(os.path.join(VOICE_DIR, f'{name}.npz'), tool='voices', module=__name__,
              about=f'pp_bridge.voice({name!r}, midi, vel, dur): one note in the {name}\'s Pixel Pitch voice '
                    f'({modname}.synth, scene {scene["key"]!r}), made at 44,100 Hz and resampled to 48,000 Hz with '
                    f'scipy.signal.resample_poly(x, 160, 147)',
              voice=name, synth=f'{modname}.synth', scene=scene['key'], register=list(register), sample_rate=48000,
              voice_sample_rate=B.PP_SR,
              notes=dict(
                  out='48,000 Hz, float32: converted from the float64 the kit makes, to keep the reference set '
                      'inside its size budget (the comparison bar is -120 dBFS; float32 rounds near -156 dBFS). '
                      '`sha256` is of the float64 samples (little-endian). first_notes.npz and helpers.npz hold '
                      'float64 notes of both voices',
                  raw44k='(two cases) the note at 44,100 Hz as the synth made it, before resampling',
                  noise='the shared generator\'s `state` and `inc` (decimal strings) just before the call: restore '
                        'them, call, compare. noise_after is the `state` the call left (the same when the voice '
                        'has no noise)',
                  draws='the n of each nz(n) the call made (none for a voice without noise)'))
    set_noise_state(start)
    for midi in VOICES[name]:
        for vel in VELS:
            for dur in DURS:
                want_raw = midi == VOICES[name][0] and vel == VELS[1] and dur in DURS[:2]
                log = NoteLog(M, B, D).install()
                raw = {}
                resample0 = B._resample
                if want_raw:
                    def keep_raw(x, raw=raw, resample0=resample0):
                        raw['x'] = np.array(x, copy=True)
                        return resample0(x)
                    B._resample = keep_raw
                try:
                    x = B.voice(name, midi, vel, dur)
                finally:
                    B._resample = resample0
                    log.remove()
                rec, = log.loose
                assert not log.stray and x.dtype == np.float64 and x.ndim == 1
                arrays = dict(out=x.astype(np.float32))
                if want_raw:
                    arrays['raw44k'] = raw['x']
                    assert len(x) == -(-len(raw['x']) * 160 // 147)
                args = dict(name=name, midi=midi, vel=vel, dur=dur)
                c.add(call_text('pp_bridge.voice', args), {k: jarg(v) for k, v in args.items()}, arrays,
                      noise=dict(state=rec['before'], inc=start['inc']), noise_after=rec['after'],
                      draws=rec['draws'], samples=len(x), sha256=rec['sha256'])
    return c.write()


def helpers_file(M, B, start):
    """kit.music's helpers. The note cache and the gains are emptied and the generator put at `start` first, so the
    file depends on nothing but the kit; the cases run in the order they are stored"""
    import numpy as np
    T, A = M.T, M.A
    c = Cases(HELPERS, tool='voices', module=__name__,
              about='kit.music helpers (float64, 48,000 Hz), in the order they were called, from the recorded '
                    'start state of the noise generator with an empty note cache',
              episode=EPISODE, start=start,
              notes=dict(
                  order='the cases ran in this order in one interpreter; a case with `noise` drew from the shared '
                        'generator: `noise` is its state and inc before the call, noise_after the state after',
                  x='a shaping helper\'s input: the `out` of an earlier note case (the same npz member)',
                  result='scalar results as float.hex(); they are also stored as small float64 arrays',
                  colour='kit.music.colour reads the fight: `hp` holds P.hp_at(k, t) and P.hp_at(1 - k, t) of the '
                         'episode\'s recording (seed 392), so the case can be redone without it. `gl` is (gain, '
                         'lead)',
                  tempo='T.BEAT, T.BAR, T.S16 of the episode as float.hex() in `tempo`'),
              tempo=dict(BEAT=common.hexf(T.BEAT), BAR=common.hexf(T.BAR), S16=common.hexf(T.S16)))
    reset(M, start)

    def drew(fn):
        """(fn's result, the noise fields to store with the case: empty when the generator did not move)"""
        b = noise_state()['state']
        out = fn()
        a = noise_state()['state']
        return out, (dict(noise=dict(state=b, inc=start['inc']), noise_after=a) if a != b else {})
    # ---- inst_gain (first: afterwards no note case synthesises a level-matching note)
    for name in M.NAMES:
        g, nz = drew(lambda: M.inst_gain(name))
        lo, hi = B.entrant(name)[3]
        c.add(f'kit.music.inst_gain({name!r})', dict(name=name), dict(out=np.array([float(g)])),
              helper='inst_gain', result=common.hexf(float(g)),
              voice=call_text('pp_bridge.voice', dict(name=name, midi=(lo + hi) // 2, vel=0.8, dur=0.2)), **nz)
    # ---- note
    notes = {}

    def note(name, midi, dur=0.18, cap=0.6, vel=0.85):
        key = (name, midi, dur, cap, vel)
        if key not in notes:
            x, nz = drew(lambda: M.note(name, midi, dur, cap, vel))
            assert x.dtype == np.float64 and x.shape == (int(cap * M.SR),)
            args = dict(name=name, midi=midi, dur=dur, cap=cap, vel=vel)
            cid = c.add(call_text('kit.music.note', args), {k: jarg(v) for k, v in args.items()}, dict(out=x),
                        helper='note', gain=common.hexf(float(M._GAIN[name])), **nz)
            notes[key] = (f'{cid}__out', x)
        return notes[key]
    for name in M.NAMES:
        for midi in VOICES[name]:
            note(name, midi)                               # the defaults: dur 0.18, cap 0.6, vel 0.85
    note('FLUTE', 76, 0.2, 0.38, 0.8)                      # as flute.py's Sound.flute(76, 0.2, 0.8) asks
    note('FLUTE', 84, 0.06, 0.24, 1.0)
    note('TROMBONE', 40, 0.3, 0.6, 0.85)                   # as trombone.py's Sound.bone(40, 0.3, 0.85) asks
    note('TROMBONE', 52, 0.7, 1.0, 1.0)
    # ---- fit
    r = np.random.default_rng(480)
    for n, cap, fade in ((1000, 0.05, 0.04), (5000, 0.05, 0.04), (5000, 0.01, 0.04), (5000, 0.05, 0.005)):
        x = r.normal(0, 0.3, n)
        args = dict(cap=cap, fade=fade)
        c.add(call_text('kit.music.fit', dict(x=x, **args)), {k: jarg(v) for k, v in args.items()},
              dict(x=x, out=M.fit(x, cap, fade)), helper='fit')

    def shape(helper, x, **args):
        member, arr = x
        y = getattr(M, helper)(arr, **args)
        assert y.dtype == np.float64 and y.shape == arr.shape
        return c.add(call_text(f'kit.music.{helper}', dict(x=arr, **args)), {k: jarg(v) for k, v in args.items()},
                     dict(x=member, out=y), helper=helper)
    t52, t47, t64, f76 = note('TROMBONE', 52), note('TROMBONE', 47), note('TROMBONE', 64), note('FLUTE', 76)
    # ---- gliss, bend, vibrato, wah
    shape('gliss', t52, semis=2.0, dur=0.035)              # Sound.bone's default slide
    shape('gliss', t47, semis=3.0, dur=0.05)
    shape('gliss', t52, semis=12.0, dur=0.26)              # the title blast's smear
    shape('gliss', t64, semis=24.0, dur=0.32)              # POWER SLIDE
    shape('gliss', t52, semis=-10, dur=0.9)                # the fall when it shatters
    shape('gliss', f76, semis=1.0, dur=0.05)
    shape('bend', f76, semis=-2.0, t0=0.05, t1=0.3)
    shape('bend', t47, semis=3.0, t0=0.0, t1=0.2)
    shape('vibrato', f76, depth=0.35, rate=5.5, delay=0.15)
    shape('vibrato', t52, depth=0.5, rate=5.0, delay=0.2)  # SAD TROMBONE's last note
    d = 0.5 * T.BEAT
    shape('wah', t52, opens=[(0.0, min(0.18, d * 0.6))])   # SAD TROMBONE's wah
    shape('wah', t47, opens=[(0.0, 0.1), (0.25, 0.4)])
    # ---- colour
    flute_note, bone_note = note('FLUTE', 76, 0.2, 0.38, 0.8), note('TROMBONE', 40, 0.3, 0.6, 0.85)
    for k, x, t, drive_k, kw in ((0, flute_note, 12.0, 0.9, {}), (0, flute_note, 30.0, 0.9, {}),
                                 (0, flute_note, M.T_KO - 1.0, 0.9, {}), (1, bone_note, 12.0, 0.8, dict(low=True)),
                                 (1, bone_note, M.T_KO - 1.0, 0.8, dict(low=True)), (1, bone_note, 30.0, 1.0, {})):
        args = dict(k=k, t=t, drive_k=drive_k, cut0=90, cut_span=520, drive=4.5, low=False, low_cut=(30, 70))
        args.update(kw)
        y, g, lead = M.colour(x[1], **args)
        assert y.dtype == np.float64 and y.shape == x[1].shape
        c.add(call_text('kit.music.colour', dict(x=x[1], **args)), {a: jarg(v) for a, v in args.items()},
              dict(x=x[0], out=y, gl=np.array([float(g), float(lead)])), helper='colour',
              result=dict(gain=common.hexf(float(g)), lead=common.hexf(float(lead))),
              hp=dict(own=common.hexf(float(M.hp_at(k, t))), other=common.hexf(float(M.hp_at(1 - k, t)))))

    def made(helper, **args):
        y = getattr(M, helper)(**args)
        assert y.dtype == np.float64 and y.ndim == 1
        return c.add(call_text(f'kit.music.{helper}', args), {k: jarg(v) for k, v in args.items()}, dict(out=y),
                     helper=helper)
    # ---- the shared sound effects (each seeds its own generator)
    made('clank', m=84, seed=0, dur=0.4, decays=((1.0, 0.12), (2.76, 0.06), (5.4, 0.03)))
    made('clank', m=79, seed=5, dur=0.4, decays=((1.0, 0.12), (2.76, 0.06), (5.4, 0.03)))
    made('clank', m=64, seed=9, dur=0.4, decays=((1.0, 0.12), (2.76, 0.06), (5.4, 0.03)))
    made('hiss', dur=0.09, seed=0, lo=3500, hi=10000, dec=0.03)
    made('hiss', dur=0.16, seed=17, lo=3000, hi=9000, dec=0.05)
    made('hiss', dur=0.12, seed=70, lo=4000, hi=11000, dec=0.05)
    made('roar', dur=2.0, seed=3)
    made('thunder', dur=1.6, seed=0, crack=True)
    made('thunder', dur=0.5, seed=3, crack=False)
    t_last = (T.N_BARS - 1) * T.BAR
    made('riser', dur=T.BAR, over=0.0, seed=12)            # big_top: into the back half
    made('riser', dur=T.DUR - t_last, over=0.012, seed=11)  # big_top: the last bar
    return c.write()


def scalars_doc(M):
    """kit.music's number helpers on fixed inputs; every float as float.hex()"""
    T = M.T
    hx = common.hexf
    cases = []

    def add(call, args, result, **more):
        cases.append(dict(id=f'c{len(cases):03d}', call=call, args=args, result=result, **more))
    for seed in (5, 7):                                    # the FLUTE's and the TROMBONE's Thinner seeds
        slots = list(range(0, 96))
        add('[kit.music.h01(slot, seed) for slot in slots]', dict(slots=slots, seed=seed),
            [hx(M.h01(s, seed)) for s in slots])
    xs = [0.0, 0.25, 1.0, 3.5875, 17.123456, 50.021568627450975]
    add('[kit.music.h01(x) for x in xs]', dict(xs=[hx(x) for x in xs]), [hx(M.h01(x)) for x in xs],
        note='floats go in as int(round(x * 1000))')
    for seed in (5, 7):
        th = M.Thinner(seed)
        slots = list(range(40, 240))
        hp = [max(0.0, 1.0 - (s - 40) / 180.0) for s in slots]
        add('th = kit.music.Thinner(seed); [th.keep(slot, hp) for slot, hp in zip(slots, hps)]',
            dict(seed=seed, slots=slots, hps=[hx(h) for h in hp]), [bool(th.keep(s, h)) for s, h in zip(slots, hp)],
            note='one Thinner, called in order: it never drops two notes in a row')
    slots = list(range(0, 64))
    add('[kit.music.h01_37(slot, key) for slot in slots]', dict(slots=slots, key=3),
        [hx(M.h01_37(s, 3)) for s in slots])
    prev, out, hps = False, [], []
    for s in slots:
        h = max(0.0, 1.0 - s / 60.0)
        prev = bool(M.thin_37(s, 3, h, prev))
        out.append(prev)
        hps.append(hx(h))
    add('prev = False; for slot, hp: prev = kit.music.thin_37(slot, key, hp, prev)',
        dict(slots=slots, key=3, hps=hps), out, note='True means the note is dropped; each call gets the one before')
    ts = [0.0, 0.05, 0.056, 0.1121, 1.0, 3.5875, 12.34567, 50.021568627450975, 51.321568627450975, 60.99]
    add('[kit.music.q16(t) for t in ts]', dict(ts=[hx(t) for t in ts], S16=hx(T.S16)), [hx(M.q16(t)) for t in ts],
        note='round(t / T.S16) * T.S16 with Python\'s round (half to even)')
    return dict(about='kit.music number helpers on fixed inputs: h01, Thinner, h01_37, thin_37, q16 (floats as '
                      'float.hex(); the tempo is the flute_vs_trombone episode\'s)',
                tool='voices', episode=EPISODE, cases=cases)


# ---------------------------------------------------------------- the checks the files must pass
def _json(path):
    assert os.path.isfile(path), f'{common.rel(path)}: missing'
    with open(path, encoding='utf-8') as f:
        return json.load(f)


def replay(M, start, calls, upto=None):
    """restore the recorded start state, forget every cached note, and synthesise the logged notes again in order:
    the arrays, after checking each against its recorded state and SHA-256"""
    reset(M, start)
    out = []
    for c in calls[:upto]:
        assert noise_state()['state'] == c['noise_before'], f"note {c['i']}: the generator is not where the log says"
        x = M.note(c['name'], c['midi'], *(float.fromhex(c['hex'][k]) for k in ('dur', 'cap', 'vel')))
        assert len(x) == c['samples'] and sha(x) == c['sha256'], f"note {c['i']} ({c['name']} {c['midi']}) differs"
        assert noise_state()['state'] == c['noise_after'], f"note {c['i']}: drew a different number of normals"
        out.append(x)
    return out


def check(M=None, B=None):
    """what the voices export must leave on disk, proved from the files alone (the export runs this last, in its
    own interpreter; it can also be run by hand): the start state is a fresh default_rng(77); the note log is one
    unbroken chain of states; replaying it from the start state reproduces every note (SHA-256) and the first
    three bit for bit; a note made from any other state differs; every voice vector is reproduced from the state
    stored with it; every helper in the brief has cases; the scalar helpers give the stored values"""
    import numpy as np
    if M is None:
        M, B, _ = load(TAG)
    start = _json(START)
    fresh_state = np.random.default_rng(NOISE_SEED).bit_generator.state
    assert (int(start['state']), int(start['inc'])) == (fresh_state['state']['state'], fresh_state['state']['inc'])
    assert start['has_uint32'] == 0 and start['seed'] == NOISE_SEED
    state0 = dict(state=start['state'], inc=start['inc'])
    doc = _json(CALLS)
    calls = doc['calls']
    assert doc['start'] == start['state'] and doc['generator']['inc'] == start['inc'] and calls
    assert [c['i'] for c in calls] == list(range(len(calls))) and doc['first_time'] == len(calls)
    assert calls[0]['noise_before'] == doc['start'] and calls[-1]['noise_after'] == doc['end']
    for a, b in zip(calls, calls[1:]):
        assert a['noise_after'] == b['noise_before'], f"notes {a['i']}, {b['i']}: something drew noise in between"
    for c in calls:
        assert c['normals'] == sum(sum(v['draws']) for v in c['voice_calls'])
        assert (c['noise_before'] == c['noise_after']) == (c['normals'] == 0), c['i']
        assert [v['why'] for v in c['voice_calls']] in (['note'], ['note', 'inst_gain']), c['i']
    assert sum(len(c['voice_calls']) == 2 for c in calls) == 2            # one level-matching note per instrument
    assert sum(c['uses'] for c in calls) == doc['note_calls']
    # the replay: every note of the log, from the files alone
    notes = replay(M, state0, calls)
    assert noise_state()['state'] == doc['end']
    first, arr = check_case_file(FIRST)
    assert len(first['cases']) == N_FIRST and first['start'] == state0
    for case, c, x in zip(first['cases'], calls, notes):
        assert case['noise'] == dict(state=c['noise_before'], inc=start['inc'])
        assert arr[case['arrays']['out']].dtype.str == '<f8'
        assert np.array_equal(arr[case['arrays']['out']], x), f"first_notes {case['id']}: the replayed note differs"
    reset(M, state0)                                         # the voice outputs behind them, called directly
    for case, c in zip(first['cases'], calls):
        for v in c['voice_calls']:
            role = 'voice' if v['why'] == 'note' else 'gain_voice'
            x = B.voice(v['name'], v['midi'], float.fromhex(v['hex']['vel']), float.fromhex(v['hex']['dur']))
            assert np.array_equal(arr[case['arrays'][role]], x) and sha(x) == v['sha256'], (case['id'], role)
    # the control: the first noisy note again, from the state the score ended in, is a different note
    noisy = next(c for c in calls if c['normals'])
    reset(M, dict(state=doc['end'], inc=start['inc']))
    other = M.note(noisy['name'], noisy['midi'], *(float.fromhex(noisy['hex'][k]) for k in ('dur', 'cap', 'vel')))
    assert sha(other) != noisy['sha256'], 'a note does not depend on the generator state: the replay proves nothing'
    # the voice vectors, each from its own stored state
    n_cases = {}
    for name in VOICES:
        vdoc, varr = check_case_file(os.path.join(VOICE_DIR, f'{name}.npz'))
        assert len(vdoc['cases']) == len(VOICES[name]) * len(VELS) * len(DURS)
        seen = set()
        for case in vdoc['cases']:
            a = case['args']
            vel, dur = float.fromhex(a['vel']), float.fromhex(a['dur'])
            seen.add((a['midi'], vel, dur))
            assert case['noise']['inc'] == start['inc'], case['id']
            set_noise_state(case['noise'])
            x = B.voice(a['name'], a['midi'], vel, dur)
            out = varr[case['arrays']['out']]
            assert out.dtype.str == '<f4' and len(out) == case['samples'], case['id']
            again = sha(x) == case['sha256'] and np.array_equal(out, x.astype(np.float32))
            assert again, f"{name} {case['id']}: not reproduced from its stored state"
            assert noise_state()['state'] == case['noise_after'], case['id']
            if 'raw44k' in case['arrays']:
                assert len(out) == -(-len(varr[case['arrays']['raw44k']]) * 160 // 147)
        assert seen == {(m, v, d) for m in VOICES[name] for v in VELS for d in DURS}, name
        n_cases[name] = len(vdoc['cases'])
    hdoc, harr = check_case_file(HELPERS)
    have = {c['helper'] for c in hdoc['cases']}
    assert have == set(HELPER_NAMES), sorted(set(HELPER_NAMES) ^ have)
    for case in hdoc['cases']:
        if 'x' in case['arrays'] and case['helper'] != 'fit':
            assert harr[case['arrays']['x']].shape == harr[case['arrays']['out']].shape, case['id']
    n_cases['helpers'] = len(hdoc['cases'])
    sdoc = _json(SCALARS)
    again = scalars_doc(M)
    common.same(common.plain(again), sdoc, 'scalars.json')
    n_cases['scalars'] = len(sdoc['cases'])
    return dict(notes=len(calls), normals=doc['normals'], **n_cases)


# ---------------------------------------------------------------- the export
def child():
    """the whole export, in its own interpreter: nothing has touched the noise generator before the score"""
    from . import manifest
    M, B, D = load(TAG)
    start = noise_state()
    mix, log = record_score(M, B, D, keep=N_FIRST)
    end = noise_state()
    del mix
    common.write_json(START, start_doc(start), register=False)
    common.write_json(CALLS, calls_doc(M, log, start, end), sort_keys=False, register=False)
    files = [first_notes_file(log, start)]
    files += [voice_file(M, B, D, name, start) for name in VOICES]
    files.append(helpers_file(M, B, start))
    common.write_json(SCALARS, scalars_doc(M), sort_keys=False, register=False)
    got = check(M, B)
    for p in (START, CALLS, SCALARS):                        # listed only now that the checks passed
        manifest.add(p)
        print(f'voices: {common.rel(p)}  {os.path.getsize(p)} bytes')
    for f in files:
        f.register()
    print(f"voices: the score asked for a note {log.total} times, {got['notes']} of them first-time; they drew "
          f"{got['normals']} normals. Replayed from the recorded start state: every note matches its SHA-256, the "
          f'first {N_FIRST} bit for bit; from another state the first noisy note differs')
    print('voices: ' + ', '.join(f'{k} {v}' for k, v in got.items()))
    return 0


def run():
    fresh('voices')


EXPORTS = {'voices': run}
