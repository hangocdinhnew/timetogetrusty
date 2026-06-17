struct Instance {
 model: mat4x4<f32>,
};

struct Camera {
 view: mat4x4<f32>,
 projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<storage, read> instances: array<Instance>;
@group(0) @binding(1)
var<uniform> camera: Camera;

@vertex
fn vs_main(@location(0) pos: vec3<f32>, @builtin(instance_index) instance_idx: u32) -> @builtin(position) vec4<f32> {
  let instance = instances[instance_idx];
  
  return  camera.projection * camera.view * instance.model * vec4<f32>(pos, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
  return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
