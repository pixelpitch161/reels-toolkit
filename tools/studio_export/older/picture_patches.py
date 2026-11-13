"""Run-time patches of the kit's renderer (projects/ballfight/kit/picture.py) for older layers no Look hook can express
(plan m4 Task 4: one named opt-in per need, in Python here and in Rust as a new `Look` method whose default draws
nothing). Empty until Task 4 registers what the older looks need.

    PATCHES      [(name, fn)] in registration order
    register(name)   decorator: fn() is called once by older.install(), before the kit is imported; it must leave every
                     milestone 1 and 3 frame unchanged (off unless a look asks for it) and install its own import hook
                     when it patches something kit.picture builds at import
    apply()      call every registered patch once (older.install() does)"""
PATCHES = []
_APPLIED = set()


def register(name):
    def deco(fn):
        if any(n == name for n, _ in PATCHES):
            raise SystemExit(f'picture_patches: {name!r} is registered twice')
        PATCHES.append((name, fn))
        return fn
    return deco


def apply():
    for name, fn in PATCHES:
        if name not in _APPLIED:
            fn()
            _APPLIED.add(name)
    return [n for n, _ in PATCHES]
