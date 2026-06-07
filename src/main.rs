use std::sync::Arc;

use winit::{
    event_loop::{ControlFlow, ActiveEventLoop, EventLoop},
    application::ApplicationHandler,
    event::{WindowEvent},
    window::{Window, WindowId, WindowAttributes},
};

use tracing::{info, error};
use tracing_subscriber::EnvFilter;

pub mod renderer;
use renderer::Renderer as Renderer;

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
	let window_attributes = WindowAttributes::default();

        self.window = match event_loop.create_window(window_attributes) {
            Ok(window) => Some(Arc::new(window)),
            Err(err) => {
                error!("Error while creating window! Reason: {err}");
                event_loop.exit();
                return;
            },
        };

	self.renderer = match Renderer::new(event_loop.owned_display_handle(), self.window.clone().unwrap().clone()) {
	    Ok(renderer) => Some(renderer),
	    Err(err) => {
		error!("Error while creating renderer! Reason: {err}");
		event_loop.exit();
		return;
	    },
	};
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
	let window = self.window.as_mut().unwrap();
	let renderer = self.renderer.as_mut().unwrap();
	
	match event {
            WindowEvent::CloseRequested => {
                info!("Close was requested; stopping");
                event_loop.exit();
            },
            WindowEvent::Resized(size) => {
		renderer.resize(size.width, size.height);
		
		window.request_redraw();
            },
            WindowEvent::RedrawRequested => {
		// TODO: Add Renderer stuff here.
                window.request_redraw();
            },
            _ => (),
        }
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
	.with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();
    
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}
