use winit::{
    window::Window,
    event_loop::OwnedDisplayHandle
};

use wgpu::{
    Instance, InstanceDescriptor,
    Adapter, RequestAdapterOptions, Device, DeviceDescriptor, Queue,
    Surface, SurfaceCapabilities, SurfaceConfiguration, PresentMode, TextureUsages,
    BufferUsages, BufferDescriptor,
    RenderPipeline
};

use std::sync::Arc;

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
    pub model_ubuf: wgpu::Buffer,
    pub uniform_bg: wgpu::BindGroup,
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

	let model_ubuf = device.create_buffer(&BufferDescriptor {
	    label: None,
	    size: size_of::<glam::Mat4>() as u64,
	    usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
	    mapped_at_creation: false,
	});

	queue.write_buffer(&model_ubuf, 0, bytemuck::bytes_of(&glam::Mat4::from_translation(glam::Vec3::ZERO)));

	let uniform_bglayout =
	    device.create_bind_group_layout(
		&wgpu::BindGroupLayoutDescriptor {
		    label: Some("Uniform Layout"),
		    entries: &[
			wgpu::BindGroupLayoutEntry {
			    binding: 0,
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

	let uniform_bg =
	    device.create_bind_group(
		&wgpu::BindGroupDescriptor {
		    label: Some("Model Uniform Bind Group"),
		    layout: &uniform_bglayout,
		    entries: &[
			wgpu::BindGroupEntry {
			    binding: 0,
			    resource: model_ubuf.as_entire_binding(),
			},
		    ],
		}
	    );

	let mesh_playout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
	    label: Some("Mesh pipeline layout"),
	    bind_group_layouts: &[Some(&uniform_bglayout)],
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
	    model_ubuf,
	    uniform_bg,
	})
    }
    
    pub fn reconfigure_surface(&mut self) {
	self.surface.configure(
	    &self.device,
	    &self.surface_config,
	);
    }
}
