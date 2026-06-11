use std::{
    sync::Arc,
    io::IsTerminal,
};

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
use renderer::MeshID as MeshID;

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    square: MeshID,
}

const VERTICES_SQUARE: [f32; 3*4] = [
    -0.5, 0.0, 0.0, // A
     0.5, 0.0, 0.0, // B
     0.5, 0.5, 0.0, // C
    -0.5, 0.5, 0.0, // D
];

const INDICES_SQUARE: [u32; 3*2] = [
    0, 1, 3, // ABD
    3, 2, 1 // DCB
];

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

	self.square = self.renderer
	    .as_mut()
	    .unwrap()
	    .create_mesh(&VERTICES_SQUARE, &INDICES_SQUARE);
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
		renderer.submit_mesh(self.square);
		renderer.draw();
		
                window.request_redraw();
            },
            _ => (),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let is_dumb = if !std::io::stdout().is_terminal() || std::env::var("TERM").unwrap_or_default() == "dumb" {
	true
    } else {
	false
    };
    
    tracing_subscriber::fmt()
	.with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
	.with_ansi(!is_dumb)
        .init();
    
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}
