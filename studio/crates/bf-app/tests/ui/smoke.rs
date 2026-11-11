//! The harness itself: a software adapter, and the blank app filling the reference window.

use bf_app::app::BfApp;

use crate::common::{REF, app_harness, gpu, pixels};

#[test]
fn the_test_adapter_is_software() {
    let _gpu = gpu();
    let state = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
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
