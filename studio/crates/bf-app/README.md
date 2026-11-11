# bf-app

The BALLFIGHT Studio desktop app: the window, the interface, the live preview with sound and the render queue. It is
drawn with egui through wgpu (eframe). The interface is a state machine that talks to the engine only through plain
commands and events, so it never waits on a fight, a picture, a soundtrack or a file.

`bf-app` is not an engine crate. Every number that reaches a fight, a frame, a soundtrack or a fight file comes from
the engine crates.

## Run the app

From `studio/`:

```sh
cargo run --release -p bf-app
```

The program is `target/release/ballfight-studio.exe`. `--smoke` opens the window, paints 30 frames, closes it and
exits with code 0: a quick check that the app starts on this PC.

## Run its tests

```sh
cargo test -p bf-app --lib          # pure logic, tested inside each file
cargo test -p bf-app --test ui      # everything that needs egui, in one test program
cargo test -p bf-app --test ui smoke::   # one module of it
```

A note on a rare failure (Task 24d). About once in ten runs on the build PC, the UI test program printed that every
test passed and then exited with a non-zero code. **The cause was not proven**: 26 full runs under load, 230 shorter
runs and 30 `--smoke` runs all exited 0, and Windows logged no crash for it. The suspect is a third-party DLL
crashing while the process closes: wgpu's default backends started Vulkan and OpenGL in every harness, and the
Vulkan loader pulled the PC's overlay and capture hooks (Overwolf, OBS, Steam, Epic) and AMD's Vulkan and OpenGL
drivers into the test process. Ruled out by reading the code: no harness is dropped outside `gpu()`, no device or
app is leaked or kept in a static, and the scheduler and writer threads are joined. Changed: the harnesses
(`common::wgpu_setup`) start only DirectX 12 on Windows, and the app prefers DirectX 12 (`app::prefer_dx12`), so
those hooks are no longer loaded; `WGPU_BACKEND` set in the environment still overrides both. If it happens again,
capture it rather than rerun: build once with `cargo test -p bf-app --test ui --no-run`, then from `crates/bf-app`
run the test exe it names in a PowerShell loop that keeps each run's whole output and its exit code in hex, for
example

```powershell
$exe = (Get-ChildItem ..\..\target\debug\deps\ui-*.exe | Sort-Object LastWriteTime | Select-Object -Last 1).FullName
for ($i = 1; $i -le 60; $i++) {
  $p = Start-Process $exe -NoNewWindow -Wait -PassThru -RedirectStandardOutput "run$i.out.txt" -RedirectStandardError "run$i.err.txt"
  "$i 0x{0:X8}" -f $p.ExitCode | Add-Content exits.txt
}
```

and look up a failing run's time in the Windows Application event log (Application Error, event 1000: the faulting
module). 0xC0000005 is an access violation, 0xC0000374 heap corruption, 0xC0000409 a fast-fail or abort.

The interface tests draw on a software graphics adapter (Microsoft Basic Render Driver on Windows, lavapipe on Linux:
`mesa-vulkan-drivers`). Two such devices drawing at once crash the test process on Windows, so every test that draws
takes the lock `common::gpu()` before anything else. They never open a window, a sound device or a dialog.

The rule for every new test: a test that builds a harness or reads pixels takes `common::gpu()` first and keeps the
guard to its end; `app_harness`, `widget_harness` and `pixels` refuse to run (they panic) without it.

Reference pictures live in `tests/snapshots/` and are blessed on Windows. To bless a changed picture, run the test
with `UPDATE_SNAPSHOTS=1` and look at the new picture before you commit it. A failing comparison leaves
`*.new.png` and `*.diff.png` beside the reference, and a bless leaves the previous reference behind as `*.old.png`;
git ignores all three.

On Linux the sound crate needs `libasound2-dev` and `pkg-config` to build.
