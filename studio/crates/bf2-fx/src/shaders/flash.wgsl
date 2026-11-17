// Full-screen flash (towards an HDR colour) and invert.

fn flash_invert(c: vec3<f32>) -> vec3<f32> {
    // Squared: the tail of an envelope adds light over a dark picture, where the sRGB curve is steepest, so a
    // linear mix would still show at the last frame of life.
    let fw = clamp(su.flash.w, 0.0, 1.0);
    var o = mix(c, su.flash.rgb, fw * fw);
    let inv = clamp(su.misc.x, 0.0, 1.0) * clamp(su.misc.x, 0.0, 1.0);
    if inv > 0.0 {
        // Inverted in a bounded (Reinhard) space, then back: bright turns dark and black turns a light grey
        // (0.82 HDR), never a burning white, so an invert never reads as a flash.
        let v = o / (1.0 + o);
        let iv = clamp(1.0 - v, vec3<f32>(0.0), vec3<f32>(0.45));
        o = mix(o, iv / (1.0 - iv), inv);
    }
    return o;
}
