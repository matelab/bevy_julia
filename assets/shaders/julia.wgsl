struct JuliaParams {
    c: vec2<f32>,
    w: f32,
    h: f32,
    view_center: vec2<f32>,
    view_scale: f32,
    view_aspect: f32,
    iters: u32,
};

@group(0) @binding(0)
var texture: texture_storage_2d<r32float, read_write>;

@group(0) @binding(1)
var<uniform> params: JuliaParams;

@compute @workgroup_size(8, 8, 1)
fn julia(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let uv = vec2<f32>(f32(invocation_id.x) / params.w, f32(invocation_id.y) / params.h);

    var i: i32;
    var z: vec2<f32> = vec2<f32>((params.view_aspect * (uv.x - 0.5)) / params.view_scale + params.view_center.x, (uv.y - 0.5) / params.view_scale + params.view_center.y);    
    var top: i32 = i32(params.iters) + 2;

    for (i = 2; i < top; i = i + 1) {
        var x: f32 = (z.x * z.x - z.y * z.y) + params.c.x;
        var y: f32 = (2.0 * z.x * z.y) + params.c.y;

        if (((x * x) + (y * y)) > 4.0) {
            break;
        }

        z.x = x;
        z.y = y;
    }
    var col: f32;
    if (i == top) {
        col = 1.0;
    } else {
        col = min(f32(i) / f32(top), 1.0);
    }


    let color = vec4<f32>(col, 0.0, 0.0, 0.0);
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    textureStore(texture, location, color);
}