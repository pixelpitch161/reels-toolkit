// The spot's shadow map: depth only, from the spot's view, opaque instances. (Prepended with common.wgsl, whose
// group 0 binding 0 is the only frame binding this pipeline's layout has.)

@vertex
fn vs_shadow(v: VIn, @builtin(instance_index) inst: u32) -> @builtin(position) vec4<f32> {
    return frame.spot_vp * (instances[inst].model * vec4<f32>(v.pos, 1.0));
}
