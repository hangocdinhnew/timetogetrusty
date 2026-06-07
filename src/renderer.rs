use std::sync::Arc;

use wgpu::{
    Instance, InstanceDescriptor,
    Adapter, RequestAdapterOptions, Device, DeviceDescriptor, Queue,
    Surface, SurfaceCapabilities, SurfaceConfiguration, PresentMode, TextureUsages
};

use winit::{
    window::Window,
    event_loop::OwnedDisplayHandle
};

pub struct Renderer {
    window: Arc<Window>,
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    surface_caps: SurfaceCapabilities,
}

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
	    present_mode: PresentMode::Fifo,
	    alpha_mode: caps.alpha_modes[0],
	    view_formats: vec![],
	    desired_maximum_frame_latency: 2,
	};

	surface.configure(
	    &device,
	    &surface_config,
	);

	Ok(Self {
	    window,
	    instance,
	    adapter,
	    device,
	    queue,
	    surface,
	    surface_config,
	    surface_caps: caps,
	})
    }

    pub fn resize(&mut self, width: u32, height: u32) {
	if width == 0 || height == 0 {
	    return;
	}

	self.surface_config.width = width;
	self.surface_config.height = height;

	self.surface.configure(
	    &self.device,
	    &self.surface_config,
	);
    }
}
