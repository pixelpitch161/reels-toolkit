//! The harness itself: a software adapter, and the blank app filling the reference window.

use bf_app::app::BfApp;

use crate::common::{REF, app_harness, gpu, pixels, widget_harness};

#[test]
fn the_test_adapter_is_software() {
    let _gpu = gpu();
    let state = egui_kittest::wgpu::create_render_state(
        crate::common::wgpu_setup(),
        eframe::egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let info = state.adapter.get_info();
    println!(
        "test adapter: {} ({:?}, {:?})",
        info.name, info.device_type, info.backend
    );
    assert_eq!(format!("{:?}", info.device_type), "Cpu");
}

#[test]
fn the_app_fills_a_1440x900_window() {
    let _gpu = gpu();
    let mut harness = app_harness(REF, |cc| BfApp::blank(cc));
    harness.run();
    let image = harness.render().expect("the harness renders");
    assert_eq!((image.width(), image.height()), (1440, 900));
    let ground = [0x1e, 0x1e, 0x1e];
    assert_eq!(
        pixels(&mut harness, &[(0, 0), (720, 450), (1439, 899)]),
        vec![ground; 3]
    );
}

/// The panic message of `f`, which must panic.
fn panic_message(f: impl FnOnce() + std::panic::UnwindSafe) -> String {
    let payload = std::panic::catch_unwind(f).expect_err("it refuses to run");
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_default()
}

#[test]
fn the_harnesses_refuse_to_draw_without_the_lock() {
    // No `gpu()` here on purpose: each builder panics before it creates a device, even while another test holds
    // the lock.
    let app = panic_message(|| drop(app_harness(REF, |cc| BfApp::blank(cc))));
    assert_eq!(app, "take common::gpu() first");
    let widget = panic_message(|| drop(widget_harness((100.0, 100.0), |_ui| {})));
    assert_eq!(widget, "take common::gpu() first");
}

#[test]
fn a_lock_taken_after_a_panic_still_lets_a_test_draw() {
    let poisoner = std::thread::spawn(|| {
        let _gpu = gpu();
        panic!("a drawing test that failed");
    });
    assert!(poisoner.join().is_err());
    let _gpu = gpu();
    let mut harness = app_harness((64.0, 64.0), |cc| BfApp::blank(cc));
    assert_eq!(pixels(&mut harness, &[(0, 0)]), vec![[0x1e, 0x1e, 0x1e]]);
}
