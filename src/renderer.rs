use std::sync::Arc;
use std::collections::HashMap;

use wgpu::{
    BufferUsages, BufferDescriptor,
};

use winit::{
    window::Window,
    event_loop::OwnedDisplayHandle
};

mod graphics_context;
pub use graphics_context::MeshInstance;
use graphics_context::GraphicsContext;
use graphics_context::ViewProjection;

pub mod camera;
pub use camera::Camera;

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
    commands: Vec<RenderCommand>,
    meshes: Vec<Mesh>,
    batches: HashMap<MeshID, Vec<MeshInstance>>,
    last_camera: Camera,
}

pub type MeshID = usize;

impl Renderer {
    pub fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> anyhow::Result<Self> {
	let gfx = GraphicsContext::new(display, window)?;

	Ok(Self {
	    gfx,
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
	let vertices_buf = self.gfx.device.create_buffer(&BufferDescriptor {
	    label: None,
	    size: std::mem::size_of_val(vertices) as u64,
	    usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
	    mapped_at_creation: false,
	});

	let indices_buf = self.gfx.device.create_buffer(&BufferDescriptor {
	    label: None,
	    size: std::mem::size_of_val(indices) as u64,
	    usage: BufferUsages::COPY_DST | BufferUsages::INDEX,
	    mapped_at_creation: false,
	});

	self.gfx.queue.write_buffer(&vertices_buf, 0, bytemuck::cast_slice(vertices));
	self.gfx.queue.write_buffer(&indices_buf, 0, bytemuck::cast_slice(indices));

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
	let surface_texture = match self.gfx.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,

            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => {
		self.batches.clear();
		self.commands.clear();

		return;
	    },

            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
		self.batches.clear();
		self.commands.clear();
		
                drop(texture);
                self.gfx.reconfigure_surface();
		return;
            }

            wgpu::CurrentSurfaceTexture::Outdated => {
		self.batches.clear();
		self.commands.clear();
		
                self.gfx.reconfigure_surface();
		return;
            }

            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!()
            }

            wgpu::CurrentSurfaceTexture::Lost => {
		self.batches.clear();
		self.commands.clear();
		
                self.gfx.surface = self.gfx.instance.create_surface(self.gfx.window.clone()).unwrap();
                self.gfx.reconfigure_surface();
		return;
            }
        };

	let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.gfx.surface_caps.formats[0].add_srgb_suffix()),
                ..Default::default()
            });

	let mut encoder = self.gfx.device.create_command_encoder(&Default::default());
	{
            let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
		label: None,
		color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
			load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
			store: wgpu::StoreOp::Store,
                    },
		})],
		depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
		    view: &self.gfx.depth_view,
		    depth_ops: Some(wgpu::Operations {
			load: wgpu::LoadOp::Clear(1.0),
			store: wgpu::StoreOp::Store,
		    }),
		    stencil_ops: None,
		}),
		timestamp_writes: None,
		occlusion_query_set: None,
		multiview_mask: None,
            });

	    renderpass.set_pipeline(&self.gfx.mesh_pipeline);

	    for command in &self.commands {
	    	match command {
	    	    RenderCommand::Mesh { id } => {
			let id = *id;
			let mesh = &self.meshes[id];
	    		let instances = self.batches
	    		    .get(&id)
	    		    .expect("MeshID is invalid, this is a bug!");

			let required_size = instances.len() * size_of::<MeshInstance>();
			
			if required_size <= self.gfx.model_sbuf_size {
			    self.gfx.queue.write_buffer(
				&self.gfx.model_sbuf,
				0,
				bytemuck::cast_slice(instances),
			    );
			} else {
			    self.gfx.model_sbuf_size = required_size.next_power_of_two();
			    self.gfx.recreate_model_sbuf();
			    
			    self.gfx.queue.write_buffer(
				&self.gfx.model_sbuf,
				0,
				bytemuck::cast_slice(instances),
			    );
			    
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
			    
			    self.gfx.queue.write_buffer(&self.gfx.camera_ubuf, 0, bytemuck::bytes_of(&view_projection));

			    self.last_camera = camera;
			}
			
			renderpass.set_vertex_buffer(0, mesh.vertices_buf.slice(..));
			renderpass.set_index_buffer(mesh.indices_buf.slice(..), wgpu::IndexFormat::Uint32);
			renderpass.set_bind_group(0, &self.gfx.bind_group, &[]);
			
			renderpass.draw_indexed(0..mesh.index_count, 0, 0..(instances.len() as u32));
	    	    }
	    	}
	    }

	}

	self.gfx.queue.submit(Some(encoder.finish()));
	surface_texture.present();

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
