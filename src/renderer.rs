use std::sync::Arc;

use wgpu::{
    BufferUsages, BufferDescriptor,
};

use winit::{
    window::Window,
    event_loop::OwnedDisplayHandle
};

mod graphics_context;
use graphics_context::GraphicsContext;

struct Mesh {
    vertices_buf: wgpu::Buffer,
    indices_buf: wgpu::Buffer,
    index_count: u32,
}

pub enum RenderCommand {
    Mesh { id: MeshID, transform: glam::Mat4 },
}

pub struct Renderer {
    gfx: GraphicsContext,
    commands: Vec<RenderCommand>,
    meshes: Vec<Mesh>,
    last_transform: glam::Mat4,
}

pub type MeshID = usize;

impl Renderer {
    pub fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> anyhow::Result<Self> {
	let gfx = GraphicsContext::new(display, window)?;

	Ok(Self {
	    gfx,
	    commands: Vec::new(),
	    meshes: Vec::new(),
	    last_transform: glam::Mat4::from_translation(glam::Vec3::ZERO),
	})
    }

    pub fn create_mesh(&mut self, vertices: &[f32], indices: &[u32]) -> MeshID {
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

    pub fn submit_mesh(&mut self, id: MeshID, transform: glam::Mat4) {
	self.commands.push(RenderCommand::Mesh {
	    id,
	    transform,
	});
    }

    pub fn draw(&mut self) {
	let surface_texture = match self.gfx.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,

            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => return,

            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.gfx.reconfigure_surface();
                return;
            }

            wgpu::CurrentSurfaceTexture::Outdated => {
                self.gfx.reconfigure_surface();
                return;
            }

            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!()
            }

            wgpu::CurrentSurfaceTexture::Lost => {
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
		depth_stencil_attachment: None,
		timestamp_writes: None,
		occlusion_query_set: None,
		multiview_mask: None,
            });

	    renderpass.set_pipeline(&self.gfx.mesh_pipeline);

	    let mut last_id: Option<usize> = None;

	    for command in &self.commands {
		if let RenderCommand::Mesh {id, transform} = command {
		    let id = *id;
		    let mesh = &self.meshes[id];

		    if self.last_transform != *transform {
			self.gfx.queue.write_buffer(
			    &self.gfx.model_ubuf,
			    0,
			    bytemuck::bytes_of(transform),
			);
			
			self.last_transform = *transform;
		    }
		    
		    if last_id != Some(id) {
			renderpass.set_vertex_buffer(0, mesh.vertices_buf.slice(..));
			renderpass.set_index_buffer(mesh.indices_buf.slice(..), wgpu::IndexFormat::Uint32);
			renderpass.set_bind_group(0, &self.gfx.uniform_bg, &[]);

			last_id = Some(id);
		    }

		    renderpass.draw_indexed(0..mesh.index_count, 0, 0..1);
		}
	    }
	}

	self.gfx.queue.submit(Some(encoder.finish()));
	surface_texture.present();

	self.commands.clear();
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
	if width == 0 || height == 0 {
	    return;
	}

	self.gfx.surface_config.width = width;
	self.gfx.surface_config.height = height;

	self.gfx.surface.configure(
	    &self.gfx.device,
	    &self.gfx.surface_config,
	);
    }
}
