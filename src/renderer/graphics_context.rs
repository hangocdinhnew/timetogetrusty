use winit::{
    window::Window,
    event_loop::OwnedDisplayHandle
};

use wgpu::{
    Instance, InstanceDescriptor,
    Adapter, RequestAdapterOptions, Device, DeviceDescriptor, Queue,
    Surface, SurfaceCapabilities, SurfaceConfiguration, PresentMode, TextureUsages,
    BufferUsages, BufferDescriptor,
    RenderPipeline,
};

use std::sync::Arc;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewProjection {
    pub view: glam::Mat4,
    pub projection: glam::Mat4,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshInstance {
    pub model: glam::Mat4,
}

pub struct GraphicsContext {
    pub window: Arc<Window>,
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub surface_config: SurfaceConfiguration,
    pub surface_caps: SurfaceCapabilities,
    pub mesh_pipeline: RenderPipeline,
    pub model_sbuf: wgpu::Buffer,
    pub bg_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub depth_view: wgpu::TextureView,
    pub model_sbuf_size: usize,
    pub camera_ubuf: wgpu::Buffer,
}

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

impl GraphicsContext {
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
	    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/Mesh.wgsl").into()),
	});

	let model_sbuf = device.create_buffer(&BufferDescriptor {
	    label: None,
	    size: (size_of::<MeshInstance>() as u64) * 128,
	    usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
	    mapped_at_creation: false,
	});

	let mesh_instance = MeshInstance {
	    model: glam::Mat4::IDENTITY,
	};

	queue.write_buffer(&model_sbuf, 0, bytemuck::bytes_of(&mesh_instance));

	let camera_ubuf = device.create_buffer(&BufferDescriptor {
	    label: None,
	    size: size_of::<ViewProjection>() as u64,
	    usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
	    mapped_at_creation: false,
	});

	let view_projection = ViewProjection {
	    view: glam::Mat4::IDENTITY,
	    projection: glam::Mat4::IDENTITY,
	};
	
	queue.write_buffer(&camera_ubuf, 0, bytemuck::bytes_of(&view_projection));

	let bg_layout =
	    device.create_bind_group_layout(
		&wgpu::BindGroupLayoutDescriptor {
		    label: Some("Bind Group Layout"),
		    entries: &[
			wgpu::BindGroupLayoutEntry {
			    binding: 0,
			    visibility: wgpu::ShaderStages::VERTEX,
			    ty: wgpu::BindingType::Buffer {
				ty: wgpu::BufferBindingType::Storage {
				    read_only: true,
				},
				has_dynamic_offset: false,
				min_binding_size: None,
			    },
			    count: None,
			},
			wgpu::BindGroupLayoutEntry {
			    binding: 1,
			    visibility: wgpu::ShaderStages::VERTEX,
			    ty: wgpu::BindingType::Buffer {
				ty: wgpu::BufferBindingType::Uniform,
				has_dynamic_offset: false,
				min_binding_size: None,
			    },
			    count: None,
			},
		    ],
		}
	    );

	let bind_group =
	    device.create_bind_group(
		&wgpu::BindGroupDescriptor {
		    label: Some("Bind Group"),
		    layout: &bg_layout,
		    entries: &[
			wgpu::BindGroupEntry {
			    binding: 0,
			    resource: model_sbuf.as_entire_binding(),
			},
			wgpu::BindGroupEntry {
			    binding: 1,
			    resource: camera_ubuf.as_entire_binding(),
			},
		    ],
		}
	    );

	let mesh_playout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
	    label: Some("Mesh pipeline layout"),
	    bind_group_layouts: &[Some(&bg_layout)],
	    immediate_size: 0,
	});
	
	let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
	    label: Some("Depth Texture"),
	    size: wgpu::Extent3d {
		width: window_size.width,
		height: window_size.height,
		depth_or_array_layers: 1,
	    },
	    mip_level_count: 1,
	    sample_count: 1,
	    dimension: wgpu::TextureDimension::D2,
	    format: wgpu::TextureFormat::Depth32Float,
	    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
	    view_formats: &[],
	});
	let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

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
	    depth_stencil: Some(wgpu::DepthStencilState {
		format: wgpu::TextureFormat::Depth32Float,
		depth_write_enabled: Some(true),
		depth_compare: Some(wgpu::CompareFunction::Less),
		stencil: wgpu::StencilState::default(),
		bias: wgpu::DepthBiasState::default(),
	    }),
	    cache: None,
	});

	Ok(Self {
	    window,
	    instance,
	    adapter,
	    device,
	    queue,
	    surface,
	    surface_config,
	    surface_caps: caps,
	    mesh_pipeline,
	    model_sbuf,
	    bg_layout,
	    bind_group,
	    depth_view,
	    model_sbuf_size: size_of::<MeshInstance>() * 128,
	    camera_ubuf,
	})
    }
    
    pub fn reconfigure_surface(&mut self) {
	self.surface.configure(
	    &self.device,
	    &self.surface_config,
	);
    }

    pub fn recreate_depth(&mut self, width: u32, height: u32) {
	let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
	    label: Some("Depth Texture"),
	    size: wgpu::Extent3d {
		width: width,
		height: height,
		depth_or_array_layers: 1,
	    },
	    mip_level_count: 1,
	    sample_count: 1,
	    dimension: wgpu::TextureDimension::D2,
	    format: wgpu::TextureFormat::Depth32Float,
	    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
	    view_formats: &[],
	});

	self.depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
    }
    
    pub fn recreate_model_sbuf(&mut self) {
	self.model_sbuf = self.device.create_buffer(&BufferDescriptor {
	    label: None,
	    size: self.model_sbuf_size as u64,
	    usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
	    mapped_at_creation: false,
	});

	self.bind_group = self.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
		label: Some("Bind Group"),
		layout: &self.bg_layout,
		entries: &[
                    wgpu::BindGroupEntry {
			binding: 0,
			resource: self.model_sbuf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
			binding: 1,
			resource: self.camera_ubuf.as_entire_binding(),
                    },
		],
            },
	);
    }
}
