use std::sync::Arc;
use std::collections::HashMap;

use winit::{
    window::Window,
    event_loop::OwnedDisplayHandle
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshInstance {
    pub model: glam::Mat4,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewProjection {
    pub view: glam::Mat4,
    pub projection: glam::Mat4,
}

mod graphics_context;
use graphics_context::GraphicsContext;

pub mod camera;
pub use camera::Camera;

mod pipeline;
use pipeline::{PipelineManager, PipelineType};

mod buffer;
use buffer::{BufferManager, BufferType, BgType};

mod pass;
use pass::{CurrentRenderFrame, RenderFrame};

struct Mesh {
    vertices_buf: wgpu::Buffer,
    indices_buf: wgpu::Buffer,
    index_count: u32,
}

pub enum RenderCommand {
    Mesh { id: MeshID },
}

pub struct Renderer {
    gfx: GraphicsContext,
    buffer: BufferManager,
    pipeline: PipelineManager,
    commands: Vec<RenderCommand>,
    meshes: Vec<Mesh>,
    batches: HashMap<MeshID, Vec<MeshInstance>>,
    last_camera: Camera,
}

pub type MeshID = usize;

impl Renderer {
    pub fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> anyhow::Result<Self> {
	let gfx = GraphicsContext::new(display, window)?;
	let buffer = BufferManager::new(&gfx);
	let pipeline = PipelineManager::new(&gfx, &buffer);
        
	Ok(Self {
	    gfx,
	    buffer,
	    pipeline,
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
	})
    }
    
    pub fn upload_mesh(&mut self, vertices: &[f32], indices: &[u32]) -> MeshID {
	let vertices_buf = self.gfx.create_vertex_buffer(std::mem::size_of_val(vertices) as u64);
	let indices_buf = self.gfx.create_index_buffer(std::mem::size_of_val(indices) as u64);
        
	self.gfx.write_buf(&vertices_buf, bytemuck::cast_slice(vertices));
	self.gfx.write_buf(&indices_buf, bytemuck::cast_slice(indices));
        
	self.meshes.push(Mesh {
	    vertices_buf,
	    indices_buf,
	    index_count: indices.len() as u32,
	});
        
	return self.meshes.len() - 1;
    }
    
    pub fn submit_mesh(&mut self, id: MeshID) {
	self.commands.push(RenderCommand::Mesh {
	    id,
	});
    }
    
    pub fn add_mesh_instances(&mut self, id: MeshID, transform: glam::Mat4) {
	let mesh_instance = MeshInstance {
	    model: transform,
	};
	
	self.batches
	    .entry(id)
	    .or_default()
	    .push(mesh_instance);
    }
    
    pub fn draw(&mut self, camera: Camera) {
	let mut frame = match RenderFrame::begin(&self.gfx) {
	    CurrentRenderFrame::Success(pass) => pass,
	    CurrentRenderFrame::Timeout | CurrentRenderFrame::Occluded => {
		self.batches.clear();
		self.commands.clear();
                
		return;
	    }
            
	    CurrentRenderFrame::Suboptimal | CurrentRenderFrame::Outdated => {
		self.gfx.reconfigure_surface();
		
		self.batches.clear();
		self.commands.clear();
                
		return;
	    },
            
	    CurrentRenderFrame::Lost => {
		self.gfx.recreate_surface();
		self.gfx.reconfigure_surface();
                
		self.batches.clear();
		self.commands.clear();
		
		return;
	    },
            
	    CurrentRenderFrame::Validation => unreachable!(),
	};
	
	{
	    let mut pass = frame.begin_pass(&self.gfx);
	    
	    pass.set_pipeline(&self.pipeline, PipelineType::Mesh);
	    
	    for command in &self.commands {
		match command {
	    	    RenderCommand::Mesh { id } => {
			let id = *id;
			let mesh = &self.meshes[id];
	    		let instances = self.batches
	    		    .get(&id)
	    		    .expect("MeshID is invalid, this is a bug!");
			
			let required_size = instances.len() * size_of::<MeshInstance>();
			
			if required_size <= self.buffer.model_sbuf_size {
			    self.buffer.write_buf(&self.gfx, BufferType::Model, bytemuck::cast_slice(instances));
			} else {
			    self.buffer.model_sbuf_size = required_size.next_power_of_two();
			    self.buffer.recreate_model_sbuf(&self.gfx);
			    
			    self.buffer.write_buf(&self.gfx, BufferType::Model, bytemuck::cast_slice(instances));
			    
			    tracing::debug!("Triggered: recreate buffer with size: {}", required_size.next_power_of_two());
			}
			
			if self.last_camera != camera {
			    let projection = glam::Mat4::perspective_rh(camera.fov.to_radians(),
									self.gfx.surface_config.width as f32 / self.gfx.surface_config.height as f32,
									0.1,
									camera.draw_distance);
			    
			    let view_projection = ViewProjection {
				view: camera.view(),
				projection,
			    };
			    
			    self.buffer.write_buf(&self.gfx, BufferType::Camera, bytemuck::bytes_of(&view_projection));
			    
			    self.last_camera = camera;
			}
			
			pass.set_vertex_buffer(&mesh.vertices_buf);
			pass.set_index_buffer(&mesh.indices_buf);
			pass.set_bind_group(&self.buffer, BgType::Mesh);
			
			pass.draw_indexed(0..mesh.index_count, 0, 0..(instances.len() as u32));
	    	    }
		}
	    }
	}
        
	frame.end(&self.gfx);
        
	self.batches.clear();
	self.commands.clear();
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
	if width == 0 || height == 0 {
	    return;
	}
        
	self.gfx.surface_config.width = width;
	self.gfx.surface_config.height = height;
        
	self.gfx.reconfigure_surface();
        
	self.gfx.recreate_depth(width, height);
    }
    
    pub fn set_vsync(&mut self, is_vsync: bool) {
	if is_vsync {
	    self.gfx.surface_config.present_mode = wgpu::PresentMode::AutoVsync;
	} else {
	    self.gfx.surface_config.present_mode = wgpu::PresentMode::AutoNoVsync;
	}
        
	self.gfx.reconfigure_surface();
    }
}
