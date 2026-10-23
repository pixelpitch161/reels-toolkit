"""Re-check score.py against the public-domain source it was transcribed from.

Source: Mutopia "Mutopia-2013/12/07-1888" (Grieg's own piano arrangement, typeset by Coyau from The University
Society, New York, 1918; CC0 / public domain). Its LilyPond file lives in the Mutopia project's GitHub repo.
This script parses the engraving's \\relative note entry (only the subset that file uses) and diffs the melody's
pitch classes and rhythms bar by bar against our phrases.

    python verify_score.py                 # downloads the .ly from GitHub
    python verify_score.py path/to/file.ly # or reads a local copy"""
import re
import sys
import urllib.request
from fractions import Fraction as Fr
import score as SC

URL = ('https://raw.githubusercontent.com/MutopiaProject/MutopiaProject/master/ftp/GriegE/O46/'
       'Dans_l_antre_du_roi_de_la_montagne/Dans_l_antre_du_roi_de_la_montagne.ly')
STEP = {'c': 0, 'd': 1, 'e': 2, 'f': 3, 'g': 4, 'a': 5, 'b': 6}
SEMI = {'c': 0, 'd': 2, 'e': 4, 'f': 5, 'g': 7, 'a': 9, 'b': 11}


def tokenise(body):
    body = re.sub(r'%.*', '', body)
    body = re.sub(r'\\[<>!]', ' ', body)                                  # hairpins
    body = re.sub(r'[-_^][.>!^]', ' ', body)                              # articulations
    body = re.sub(r'_\\markup', '\\\\markup', body)
    body = re.sub(r'\\markup\s*\{[^{}]*(\{[^{}]*\}[^{}]*)*\}', ' ', body)
    body = re.sub(r'\\appoggiatura\s*\{[^}]*\}', ' ', body)
    body = re.sub(r'\\tempo\s*"[^"]*"\s*\d+=\d+', ' ', body)
    body = re.sub(r'\\set [^\n]*', ' ', body)
    body = re.sub(r'\\ottava\s*#-?\d|\\bar\s*"[^"]*"|\\clef\s*\w+|\\key\s*\w+\s*\\\w+|\\time\s*\d+/\d+', ' ', body)
    return re.findall(r'\\times\s*2/3|\\\w+|<|>\d*\.?|\{|\}|[a-g](?:is|es)*[\',]*(?:\d+\.?)?|r\d*\.?|s\d*\.?(?:\*\d+)?', body)


def parse(body, ref):
    def place(tok, ref):
        L, acc, marks = re.match(r'([a-g])((?:is|es)*)([\',]*)', tok).groups()
        rl, ro = ref
        o = min((o for o in (ro - 1, ro, ro + 1) if abs(o * 7 + STEP[L] - ro * 7 - STEP[rl]) <= 3),
                key=lambda o: abs(o * 7 + STEP[L] - ro * 7 - STEP[rl]))
        o += marks.count("'") - marks.count(',')
        return (L, o), 12 * (o + 1) + SEMI[L] + acc.count('is') - acc.count('es')

    def dur_of(tok, dur):
        m = re.search(r'(\d+)(\.?)', tok)
        if not m:
            return dur
        return Fr(1, int(m.group(1))) * (Fr(3, 2) if m.group(2) else 1)

    out, stack, t, dur, scale, chord, pend = [], [], Fr(0), Fr(1, 4), Fr(1), None, False
    for tk in tokenise(body):
        if tk.startswith('\\times'):
            scale, pend = Fr(2, 3), True
        elif tk == '{':
            stack.append('times' if pend else 'brace')
            pend = False
        elif tk == '}':
            if stack and stack.pop() == 'times':
                scale = Fr(1)
        elif tk == '<':
            chord, first, cref = [], None, ref
        elif tk.startswith('>'):
            dur = dur_of(tk, dur)
            out.append((t, dur * scale, chord))
            t += dur * scale
            ref, chord = first, None
        elif tk.startswith('\\'):
            continue
        elif tk[0] in 'rs':
            m = re.match(r'[rs](\d+)?(\.)?(?:\*(\d+))?', tk)
            if m.group(1):
                dur = Fr(1, int(m.group(1))) * (Fr(3, 2) if m.group(2) else 1)
            n = int(m.group(3)) if m.group(3) else 1
            out.append((t, dur * n, [] if tk[0] == 'r' else None))
            t += dur * n
        elif chord is not None:
            cref, p = place(tk, cref)
            first = first or cref
            chord.append(p)
        else:
            ref, p = place(tk, ref)
            dur = dur_of(tk, dur)
            out.append((t, dur * scale, [p]))
            t += dur * scale
    return out


def voice(src, name):
    m = re.search(name + r'\s*=\s*\\relative\s+([a-g])([\',]*)\s*\{', src)
    depth, j = 1, m.end()
    while depth:
        depth += {'{': 1, '}': -1}.get(src[j], 0)
        j += 1
    return parse(src[m.end():j - 1], (m.group(1), 3 + m.group(2).count("'") - m.group(2).count(',')))


def bars(ev, b0, b1):
    return [(max(ns) % 12, int(d * 8)) for t, d, ns in ev if b0 <= t < b1 and ns]


def ours(name):
    return [(p % 12, d) for p, d in SC.PHRASES[name] if p is not None]


if __name__ == '__main__':
    src = open(sys.argv[1]).read() if len(sys.argv) > 1 else urllib.request.urlopen(URL, timeout=60).read().decode()
    rh, lh = voice(src, 'rightOne'), voice(src, 'left')
    ok = True
    for name, (b0, b1) in [('A', (1, 5)), ('A', (5, 9)), ('B', (9, 13)), ('B', (13, 17)), ('A', (49, 53))]:
        same = bars(rh, b0, b1) == ours(name)
        ok &= same
        print(f'phrase {name:3s} vs bars {b0:2d}-{b1 - 1:2d}: {"match" if same else "DIFF"}')
    same = bars(rh, 57, 59) == ours('B')[:13]
    ok &= same
    print(f'phrase B   vs bars 57-58 (stretto): {"match" if same else "DIFF"}  (bar 59 reads G natural: engraving slip)')
    # B#: bar 62 has D# in both hands; bars 63-64 of the right hand repeat bars 59-60 (G natural, D natural)
    # while the left hand has D# in bar 64 -> we follow the left hand (see score.py)
    b62 = bars(rh, 62, 63)
    lh64 = [max(ns) % 12 for t, d, ns in lh if 64 <= t < 65 and ns]
    print(f'B# bar 2 (bar 62) right hand: {"match" if b62 == ours("B#")[7:13] else "DIFF"};'
          f' bar 64 left hand starts on D#: {lh64[0] == 3}')
    run = bars(rh, 75, 77)
    print(f'coda run (bars 75-76): {"match" if run == ours("coda")[2:] else "DIFF"}')
    print('ALL OK' if ok else 'CHECK THE DIFFS')
