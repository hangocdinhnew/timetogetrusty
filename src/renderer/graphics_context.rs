use winit::{
    window::Window,
    event_loop::OwnedDisplayHandle
};

use wgpu::{
    Instance, InstanceDescriptor,
    Adapter, RequestAdapterOptions, Device, DeviceDescriptor, Queue,
    Surface, SurfaceCapabilities, SurfaceConfiguration, PresentMode, TextureUsages,
    BufferUsages, BufferDescriptor,
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
    pub depth_view: wgpu::TextureView,
}

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

	Ok(Self {
	    window,
	    instance,
	    adapter,
	    device,
	    queue,
	    surface,
	    surface_config,
	    surface_caps: caps,
	    depth_view,
	})
    }

    pub fn recreate_surface(&mut self) {
	self.surface = self.instance.create_surface(self.window.clone()).unwrap();
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

    pub fn create_vertex_buffer(&self, vertices_len: u64) -> wgpu::Buffer {
	let vertices_buf = self.device.create_buffer(&BufferDescriptor {
	    label: None,
	    size: vertices_len,
	    usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
	    mapped_at_creation: false,
	});

	return vertices_buf;
    }

    pub fn create_index_buffer(&self, indices_len: u64) -> wgpu::Buffer {
	let indices_buf = self.device.create_buffer(&BufferDescriptor {
	    label: None,
	    size: indices_len,
	    usage: BufferUsages::COPY_DST | BufferUsages::INDEX,
	    mapped_at_creation: false,
	});

	return indices_buf;
    }

    pub fn write_buf(&self, buffer: &wgpu::Buffer, data: &[u8]) {
	self.queue.write_buffer(buffer, 0, data);
    }
}
