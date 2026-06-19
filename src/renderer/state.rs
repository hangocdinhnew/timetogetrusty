use std::collections::HashMap;
use crate::renderer::{Camera, RenderCommand, Mesh, MeshID, MeshInstance};

pub struct StateManager {
    pub commands: Vec<RenderCommand>,
    pub meshes: Vec<Mesh>,
    pub batches: HashMap<MeshID, Vec<MeshInstance>>,
    pub last_camera: Camera,
}

impl StateManager {
    pub fn new() -> Self {
	Self {
            commands: Vec::new(),
            meshes: Vec::new(),
            batches: HashMap::new(),
            last_camera: Camera {
                position: glam::Vec3::ZERO,
                yaw: 0.0,
                pitch: 0.0,
                fov: 0.0,
                draw_distance: 0.0,
            },
	}
    }
}
