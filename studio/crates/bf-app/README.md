# bf-app

The BALLFIGHT Studio desktop app: the window, the interface, the live preview with sound and the render queue. It is
drawn with egui through wgpu (eframe). The interface is a state machine that talks to the engine only through plain
commands and events, so it never waits on a fight, a picture, a soundtrack or a file.

`bf-app` is not an engine crate. Every number that reaches a fight, a frame, a soundtrack or a fight file comes from
the engine crates.

## Run the app

From `studio/`:

```sh
cargo run --release -p bf-app -- --fixtures
```

The program is `target/release/ballfight-studio.exe`. Until the real engine is behind the seam (Task 35), the whole
interface runs on sample data:

- `--fixtures` runs the app on the sample engine (`seam::fake`, answering at a real engine's pace) with real sound
  out. Its settings, queue and cache go in a folder of its own under the system temp folder, removed when the window
  closes, and the status bar's right side reads "Sample data, not a real fight". Nothing it shows is a real fight
  and no video is saved.
- Without `--fixtures` the window says "The engine is not connected in this build. Start it with --fixtures to see
  the interface."
- `--data-dir <folder>` keeps the settings, the queue and the cache in `<folder>` (`config` and `cache` under it)
  instead of the temp folder or the PC's own app folders. The folder is created if missing; a folder that cannot be
  used stops the app with one sentence on stderr and exit code 2.
- `--smoke` opens the window, paints 30 frames, closes it and exits with code 0, with no sound and its files in a
  temp folder of its own. `ballfight-studio.exe --fixtures --smoke` is the quick check that the interface starts on
  this PC.

At start the app writes one line to stderr naming the graphics adapter its window draws with, for example
`BALLFIGHT Studio draws with AMD Radeon ... (Dx12, DiscreteGpu)`. A release build has no console, so redirect stderr
to a file to read it.

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

Tests that build the whole app (`tests/ui/flows.rs`) keep its files in a temp folder of their own and use a
`NoAudio` output, so they never touch the real `%APPDATA%`, `%LOCALAPPDATA%` or Videos folders or open a sound
device. They wait for the engine on a condition, bounded in time, and never assert anything through timing.

### Blessing pictures

Reference pictures live in `tests/snapshots/` and are blessed on Windows. To bless, run the module whose pictures
changed with `UPDATE_SNAPSHOTS` set:

```sh
UPDATE_SNAPSHOTS=true cargo test -p bf-app --test ui small_window::
```

This rewrites only the pictures that differ. Without it, a failed comparison leaves `<name>.new.png` and
`<name>.diff.png` beside the reference, and a bless leaves the previous reference behind as `<name>.old.png`; git
ignores all three and none is committed. A picture is blessed only after it has been looked at next to its section
of `docs/studio/interface-reference.md`, and the differences from the mock-up are written in the task's report.
Each test builds one harness per picture, so a bless rewrites each picture on its own. The shell's picture,
`shell_1440x900.png`, shows the Match-up screen and is re-blessed only by Match-up work (widget rule 18).

On Linux the sound crate needs `libasound2-dev` and `pkg-config` to build.
