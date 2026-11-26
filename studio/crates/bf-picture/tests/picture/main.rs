//! Every integration test of bf-picture, in one test program (code review of 2026-11-23, I2): each file was its own
//! program before, linked on its own, which made a full test run slow. Run one group with a filter, for example
//! `cargo test --release -p bf-picture --test picture ring_stages`.

mod kit_fx_vectors;
mod kit_ui_vectors;
mod kit_vectors;
mod ndimage;
mod older_optins;
mod older_optins_viola;
mod pillow_as_used;
mod pillow_blur_resize;
mod pillow_vectors;
mod ring_stages;
mod sprite_pitched;
mod sprites;
mod title_frames;
mod title_optins;
