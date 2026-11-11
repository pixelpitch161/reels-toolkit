//! What every interface test shares: the one-at-a-time drawing lock and the harness builders.
//!
//! The rule: every test that builds a harness or reads pixels takes `common::gpu()` first and keeps the guard to its
//! end. The builders and `pixels` refuse to run (they panic) when the calling test does not hold it.

use std::sync::{Mutex, MutexGuard, TryLockError};
use std::thread::{self, ThreadId};

use bf_app::app::BfApp;
use egui_kittest::Harness;

/// The reference window, in points (one pixel per point in tests).
pub const REF: (f32, f32) = (1440.0, 900.0);
/// The smallest window the app supports.
pub const MIN: (f32, f32) = (1280.0, 800.0);

static GPU: Mutex<()> = Mutex::new(());
/// The test thread that last took `GPU`, so a test is not fooled by another test holding the lock.
static HOLDER: Mutex<Option<ThreadId>> = Mutex::new(None);

/// Tests that draw with wgpu run one at a time: two software-adapter devices drawing at once crash the test process
/// on Windows. Every such test holds this guard for its whole length. A test that panicked while holding it does
/// not stop the others, so poisoning is ignored.
pub fn gpu() -> MutexGuard<'static, ()> {
    let guard = GPU.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    *HOLDER.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(thread::current().id());
    guard
}

/// Whether `lock` is held by someone right now. A poisoned lock that is held still counts as held (`try_lock` then
/// reports `WouldBlock`); a poisoned lock nobody holds is free.
fn is_held(lock: &Mutex<()>) -> bool {
    matches!(lock.try_lock(), Err(TryLockError::WouldBlock))
}

/// Panics unless the calling test holds `gpu()`.
fn require_gpu() {
    let mine = *HOLDER.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) == Some(thread::current().id());
    assert!(is_held(&GPU) && mine, "take common::gpu() first");
}

/// The wgpu setup of every test that draws: kittest's own (it picks the software adapter), with only the DirectX 12
/// backend started on Windows, where that adapter (Microsoft Basic Render Driver) is a DirectX 12 one anyway.
///
/// Why: the default set also starts Vulkan and OpenGL in every harness, and the Vulkan loader then pulls every
/// overlay and capture hook registered on the PC (Overwolf, OBS, Steam, Epic) and more driver DLLs into the test
/// process, though nothing draws through them. One of them is the suspect (not proven, Task 24d) for the rare
/// non-zero exit after every test had passed. This cannot hide a test failure: it only narrows which graphics
/// libraries are loaded, and the adapter the tests draw on is the same. `WGPU_BACKEND`, when set, still wins, and
/// other systems (lavapipe on Linux is Vulkan) keep the default set.
pub fn wgpu_setup() -> eframe::egui_wgpu::WgpuSetup {
    let mut setup = egui_kittest::wgpu::default_wgpu_setup();
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(new) = &mut setup
        && cfg!(windows)
        && eframe::egui_wgpu::wgpu::Backends::from_env().is_none()
    {
        new.instance_descriptor.backends = eframe::egui_wgpu::wgpu::Backends::DX12;
    }
    setup
}

/// The whole app in a window of `size`: edge to edge, one pixel per point, no cursor drawn, drawn with wgpu.
pub fn app_harness<'a>(
    size: (f32, f32),
    make: impl FnOnce(&mut eframe::CreationContext<'a>) -> BfApp,
) -> Harness<'a, BfApp> {
    require_gpu();
    Harness::builder()
        .with_size(egui::Vec2::new(size.0, size.1))
        .with_pixels_per_point(1.0)
        .with_render_cursor(false)
        .wgpu_setup(wgpu_setup())
        .build_eframe(make)
}

/// One piece of interface in a window of `size`, with the app's theme installed, drawn the same way.
pub fn widget_harness<'a>(size: (f32, f32), ui: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
    require_gpu();
    Harness::builder()
        .with_size(egui::Vec2::new(size.0, size.1))
        .with_pixels_per_point(1.0)
        .with_render_cursor(false)
        .wgpu_setup(wgpu_setup())
        .with_context_setup(bf_app::theme::install)
        .build_ui(ui)
}

/// Renders the harness once and reads the colour of each pixel at `at` (x, y), alpha dropped.
pub fn pixels<S>(harness: &mut Harness<'_, S>, at: &[(u32, u32)]) -> Vec<[u8; 3]> {
    require_gpu();
    let image = harness.render().expect("the harness renders");
    at.iter()
        .map(|&(x, y)| {
            let [r, g, b, _] = image.get_pixel(x, y).0;
            [r, g, b]
        })
        .collect()
}

#[test]
fn a_poisoned_lock_counts_as_held_only_while_held() {
    let lock = Mutex::new(());
    assert!(!is_held(&lock));
    let poisoner = std::panic::catch_unwind(|| {
        let _guard = lock.lock().unwrap();
        panic!("poison the lock");
    });
    assert!(poisoner.is_err() && lock.is_poisoned());
    assert!(!is_held(&lock), "a poisoned lock nobody holds is free");
    let _guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    assert!(is_held(&lock), "a poisoned lock that is held counts as held");
}
