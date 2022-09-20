struct VertexIn {
    @location(0) pos: vec2<f32>,
   @builtin(vertex_index) index: u32,
}

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
}

@vertex
fn vertex(in: VertexIn) -> VertexOut {
    var out = VertexOut();
    // works:
    let x = f32(i32(in.index) - 1);
    let y = f32(i32(in.index & 1u) * 2 - 1);
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    // doesn't work
    out.pos = vec4(in.pos, 0.0, 0.0);
    return out;
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 0.0, 1.0);
}
