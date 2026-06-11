use std::sync::Arc;

use wgpu::{
    Instance, InstanceDescriptor,
    Adapter, RequestAdapterOptions, Device, DeviceDescriptor, Queue,
    Surface, SurfaceCapabilities, SurfaceConfiguration, PresentMode, TextureUsages,
    BufferUsages, BufferDescriptor,
    RenderPipeline
};

use winit::{
    window::Window,
    event_loop::OwnedDisplayHandle
};

struct GraphicsContext {
    window: Arc<Window>,
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    surface_caps: SurfaceCapabilities,
    mesh_pipeline: RenderPipeline,
}

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
}

pub type MeshID = usize;

pub const VERTEX_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: (size_of::<f32>() * 3) as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &[
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
    ],
};

impl Renderer {
    pub fn new(display: OwnedDisplayHandle, window: Arc<Window>) -> anyhow::Result<Self> {
	let instance_descriptor = InstanceDescriptor::new_with_display_handle(Box::new(display)).with_env();
	let instance = Instance::new(instance_descriptor);

	let adapter_descriptor = RequestAdapterOptions::default();
	let adapter = pollster::block_on(instance.request_adapter(&adapter_descriptor))?;

	let device_descriptor = DeviceDescriptor::default();
	let (device, queue) = pollster::block_on(adapter.request_device(&device_descriptor))?;

	let surface = instance.create_surface(window
					      .clone())?;

	let caps = surface.get_capabilities(&adapter);
	let format = caps.formats[0];

	let ref_window = window.clone();
	let window_size = ref_window.as_ref().inner_size();
	
	let surface_config = SurfaceConfiguration {
	    usage: TextureUsages::RENDER_ATTACHMENT,
	    format,
	    width: window_size.width,
	    height: window_size.height,
	    present_mode: PresentMode::AutoVsync,
	    alpha_mode: caps.alpha_modes[0],
	    view_formats: vec![],
	    desired_maximum_frame_latency: 2,
	};

	surface.configure(
	    &device,
	    &surface_config,
	);

	let mesh_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
	    label: Some("Mesh"),
	    source: wgpu::ShaderSource::Wgsl(include_str!("Mesh.wgsl").into()),
	});

	let mesh_playout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
	    label: Some("Mesh pipeline layout"),
	    bind_group_layouts: &[],
	    immediate_size: 0,
	});

	let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
	    label: Some("Mesh pipeline"),
	    layout: Some(&mesh_playout),
	    
	    vertex: wgpu::VertexState {
		module: &mesh_shader,
		entry_point: Some("vs_main"),
		compilation_options: Default::default(),
		buffers: &[VERTEX_LAYOUT],
	    },
	    
	    fragment: Some(wgpu::FragmentState {
		module: &mesh_shader,
		entry_point: Some("fs_main"),
		compilation_options: Default::default(),
		targets: &[Some(caps.formats[0].into())],
	    }),

	    primitive: wgpu::PrimitiveState::default(),
	    multisample: wgpu::MultisampleState::default(),
	    multiview_mask: None,
	    depth_stencil: None,
	    cache: None,
	});

	let gfx = GraphicsState {
	    window,
	    instance,
	    adapter,
	    device,
	    queue,
	    surface,
	    surface_config,
	    surface_caps: caps,
	    mesh_pipeline,
	};

	Ok(Self {
	    gfx,
	    commands: Vec::new(),
	    meshes: Vec::new(),
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

    pub fn submit_mesh(&mut self, id: MeshID) {
	self.commands.push(RenderCommand::Mesh {
	    id,
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
		if let RenderCommand::Mesh {id} = command {
		    let id = *id;
		    let mesh = &self.meshes[id];
		    
		    if last_id != Some(id) {
			renderpass.set_vertex_buffer(0, mesh.vertices_buf.slice(..));
			renderpass.set_index_buffer(mesh.indices_buf.slice(..), wgpu::IndexFormat::Uint32);

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

impl GraphicsState {
    pub fn reconfigure_surface(&mut self) {
	self.surface.configure(
	    &self.device,
	    &self.surface_config,
	);
    }
}
