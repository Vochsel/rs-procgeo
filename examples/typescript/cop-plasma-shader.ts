// Plasma shader — custom WGSL interference pattern
import type { ProcGeo } from "procgeo";
declare const pg: ProcGeo;

const size = 256;
const source = `
@group(0) @binding(0) var output: texture_storage_2d<rgba32float, write>;

fn psin(x: f32) -> f32 {
    return sin(fract(x / 6.2831853) * 6.2831853);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = textureDimensions(output);
    if gid.x >= dims.x || gid.y >= dims.y { return; }
    let uv = vec2f(f32(gid.x), f32(gid.y)) / vec2f(f32(dims.x), f32(dims.y));
    let p = uv * 8.0;

    var v = 0.0;
    v += psin(p.x + psin(p.y * 1.3 + 1.7));
    v += psin(p.y * 0.9 + psin(p.x * 1.1 + 2.3));
    v += psin(length(p - vec2f(4.0, 4.0)) * 1.5);
    v += psin(length(p - vec2f(2.0, 6.0)) * 2.0 + 0.5);
    v = v * 0.25 + 0.5;

    let r = psin(v * 6.2831853) * 0.5 + 0.5;
    let g = psin(v * 6.2831853 + 2.094) * 0.5 + 0.5;
    let b = psin(v * 6.2831853 + 4.189) * 0.5 + 0.5;
    textureStore(output, vec2i(gid.xy), vec4f(r, g, b, 1.0));
}
`;
return pg.copCustomShader(null, null, {
  source,
  language: "wgsl",
  width: size,
  height: size,
});
