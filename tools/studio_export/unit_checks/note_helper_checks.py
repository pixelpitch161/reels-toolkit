"""note_helper_checks: voices.NoteLog with a part's own note helper (plan m4 Task 10: `Sound.note_helper`, an older
episode's `_fit` with its own fade and its own cache and gain table), and without one (every kit fighter: today's
behaviour, a voice call outside kit.music.note is `loose`). About 10 s.

    python -m tools.studio_export.unit_checks.note_helper_checks"""
import sys
import types

from .. import __main__ as entry
from .. import common, voices as V


def _parts():
    """pp_bridge and the series' drums (the shared noise), and a stand-in kit.music with no note of its own"""
    E, _ = common.kit()
    E.activate_paths()
    import pp_bridge as B
    import drums60_lowpoly as D
    M = types.SimpleNamespace(_VC={}, _GAIN={}, note=lambda *a, **k: None, inst_gain=lambda n: 1.0)
    return M, B, D


class _Own:
    """a part with its own helper, as the SYNTH's (#31 audio.py note / _fit / inst_gain)"""
    note_helper = dict(note='_note', gain='_inst_gain', fade=0.03)

    def __init__(self, B):
        self.B, self._vc, self._gain = B, {}, {}

    def _inst_gain(self, name):
        if name not in self._gain:
            self._gain[name] = 0.5 + 0.0 * float(self.B.voice(name, 69, 0.8, 0.2)[0])
        return self._gain[name]

    def _note(self, name, midi, dur=0.18, cap=0.5, vel=0.85):
        key = (name, int(midi), round(dur, 3), cap, round(vel, 2))
        if key not in self._vc:
            self._vc[key] = self.B.voice(name, int(midi), vel, dur)[:int(cap * 48000)] * self._inst_gain(name)
        return self._vc[key]


class _Plain:
    """a part that calls the voice itself and declares no helper (not allowed: its calls are loose)"""

    def __init__(self, B):
        self.B = B

    def play(self):
        return self.B.voice('SYNTH', 64, 0.8, 0.1)


def a_parts_own_helper_is_logged_with_its_fade():
    M, B, D = _parts()
    s = _Own(B)
    M.SOUNDS = [s, types.SimpleNamespace()]

    def score():
        s._note('SYNTH', 64, 0.12, 0.34, 0.8)
        s._note('SYNTH', 64.4, 0.1204, 0.34, 0.804)             # the same key: cached, no voice call
        s._note('SYNTH', 67, 0.12, 0.34, 0.8)
        return 'mix'
    mix, log = V.record_score(M, B, D, score=score)
    assert mix == 'mix' and not log.loose and not log.stray
    assert [(c['midi'], c['fade'], c['side'], len(c['voice_calls'])) for c in log.calls] == \
        [(64, 0.03, 0, 2), (67, 0.03, 0, 1)], [(c['midi'], c.get('fade'), len(c['voice_calls'])) for c in log.calls]
    assert [v['why'] for v in log.calls[0]['voice_calls']] == ['note', 'inst_gain']
    assert log.total == 3 and log.uses[log.calls[0]['key']] == 2 and log.calls[0]['gain'] == 0.5
    assert '_note' not in vars(s) and '_inst_gain' not in vars(s)     # the helper is its own method again
    return f'{len(log.calls)} first-time notes, their fade and side, no loose voice call'


def a_part_without_one_is_as_before():
    M, B, D = _parts()
    p = _Plain(B)
    M.SOUNDS = [p, types.SimpleNamespace()]
    try:
        V.record_score(M, B, D, score=p.play)
    except AssertionError as ex:
        assert 'SYNTH' in str(ex), ex
        return 'its voice call is loose and the score is refused'
    raise AssertionError('a voice call outside a note passed')


def a_wrong_declaration_is_refused():
    M, B, D = _parts()

    class Bad(_Own):
        note_helper = dict(note='_note', fade=0.03)
    M.SOUNDS = [Bad(B)]
    try:
        V.record_score(M, B, D, score=lambda: None)
    except SystemExit as ex:
        assert 'note_helper' in str(ex), ex
        return str(ex)[:80]
    raise AssertionError('a helper without its gain was taken')


CHECKS = (a_parts_own_helper_is_logged_with_its_fade, a_part_without_one_is_as_before, a_wrong_declaration_is_refused)


def main():
    entry.check_environment()
    for check in CHECKS:
        try:
            print(f'ok    {check.__name__}: {check()}', flush=True)
        except (AssertionError, SystemExit, Exception) as ex:            # noqa: B014 (say which check, then stop)
            print(f'FAIL  {check.__name__}: {type(ex).__name__}: {ex}')
            return 1
    print(f'note_helper_checks: {len(CHECKS)} checks passed')
    return 0


if __name__ == '__main__':
    sys.exit(main())
