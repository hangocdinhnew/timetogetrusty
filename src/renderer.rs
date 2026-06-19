use std::sync::Arc;

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
pub use pipeline::DrawMethod;

mod buffer;
use buffer::{BufferManager, BufferType, BgType};

mod pass;
use pass::{CurrentRenderFrame, RenderFrame};

mod state;
use state::StateManager;

struct Mesh {
    vertices_buf: wgpu::Buffer,
    indices_buf: wgpu::Buffer,
    index_count: u32,
}

pub enum RenderCommand {
    Object {
	id: MeshID,
	transform: glam::Mat4,
	draw_method: DrawMethod,
    },
}

pub struct Renderer {
    gfx: GraphicsContext,
    buffer: BufferManager,
    pipeline: PipelineManager,
    state: StateManager,
}

pub type MeshID = usize;

impl Renderer {
    pub fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> anyhow::Result<Self> {
        let gfx = GraphicsContext::new(display, window)?;
        let buffer = BufferManager::new(&gfx);
        let pipeline = PipelineManager::new(&gfx, &buffer);
	let state = StateManager::new();
        
        Ok(Self {
            gfx,
            buffer,
            pipeline,
	    state,
        })
    }
    
    pub fn upload_mesh(&mut self, vertices: &[f32], indices: &[u32]) -> MeshID {
        let vertices_buf = self.gfx.create_vertex_buffer(std::mem::size_of_val(vertices) as u64);
        let indices_buf = self.gfx.create_index_buffer(std::mem::size_of_val(indices) as u64);
        
        self.gfx.write_buf(&vertices_buf, bytemuck::cast_slice(vertices));
        self.gfx.write_buf(&indices_buf, bytemuck::cast_slice(indices));
        
        self.state.meshes.push(Mesh {
            vertices_buf,
            indices_buf,
            index_count: indices.len() as u32,
        });
        
        return self.state.meshes.len() - 1;
    }
    
    pub fn submit_object(&mut self, id: MeshID, transform: glam::Mat4, draw_method: DrawMethod) {
        self.state.commands.push(RenderCommand::Object {
            id,
	    transform,
	    draw_method,
        });
    }
    
    pub fn draw(&mut self, camera: Camera) {
        self.state.batches.clear();

	for command in &self.state.commands {
	    match command {
		RenderCommand::Object { id, transform, .. } => {
		    self.state.batches
			.entry(*id)
			.or_default()
			.push(MeshInstance {
			    model: *transform,
			});
		}
	    }
	}
	
        let mut frame = match RenderFrame::begin(&self.gfx) {
            CurrentRenderFrame::Success(pass) => pass,
            CurrentRenderFrame::Timeout | CurrentRenderFrame::Occluded => {
                self.state.commands.clear();
                
                return;
            }
            
            CurrentRenderFrame::Suboptimal | CurrentRenderFrame::Outdated => {
                self.gfx.reconfigure_surface();
                
                self.state.commands.clear();
                
                return;
            },
            
            CurrentRenderFrame::Lost => {
                self.gfx.recreate_surface();
                self.gfx.reconfigure_surface();
                
                self.state.commands.clear();
                
                return;
            },
            
            CurrentRenderFrame::Validation => unreachable!(),
        };
        
        {
            let mut pass = frame.begin_pass(&self.gfx);
            
	    let last_mesh_draw_method = DrawMethod::Triangles;
	    pass.set_pipeline(&self.pipeline, PipelineType::Mesh);
            
            for command in &self.state.commands {
                match command {
                    RenderCommand::Object { id, draw_method, .. } => {
                        let id = *id;
                        let mesh = &self.state.meshes[id];
                        let instances = self.state.batches
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
                        
                        if self.state.last_camera != camera {
                            let projection = glam::Mat4::perspective_rh(camera.fov.to_radians(),
                                                                        self.gfx.surface_config.width as f32 / self.gfx.surface_config.height as f32,
                                                                        0.1,
                                                                        camera.draw_distance);
                            
                            let view_projection = ViewProjection {
                                view: camera.view(),
                                projection,
                            };
                            
                            self.buffer.write_buf(&self.gfx, BufferType::Camera, bytemuck::bytes_of(&view_projection));
                            
                            self.state.last_camera = camera;
                        }
                        
			if *draw_method != last_mesh_draw_method {
			    match draw_method {
				DrawMethod::Triangles => pass.set_pipeline(&self.pipeline, PipelineType::Mesh),
				DrawMethod::Lines => pass.set_pipeline(&self.pipeline, PipelineType::Mesh),
			    }
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
        
        self.state.commands.clear();
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
