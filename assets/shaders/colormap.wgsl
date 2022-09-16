[[group(0), binding(0)]]
var input: texture_storage_2d<r32float, read>;

[[group(0), binding(1)]]
var output: texture_storage_2d<rgba8unorm, write>;

[[group(0), binding(2)]]
var mapping: texture_storage_1d<rgba8unorm, read>;

[[stage(compute), workgroup_size(8, 8, 1)]]
fn colormap([[builtin(global_invocation_id)]] invocation_id: vec3<u32>, [[builtin(num_workgroups)]] num_workgroups: vec3<u32>) {
    let mapping_size = f32(textureDimensions(mapping));
    let pos = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let val = textureLoad(input, pos).r;
    let val_mapping = val * mapping_size / 1.05;
    //let val_mapping = clamp(val_mapping, 0.0, f32(mapping_size));
    let col = textureLoad(mapping, i32(val_mapping));
    textureStore(output, pos, col);
}