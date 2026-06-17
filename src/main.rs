use std::{
    sync::Arc,
    io::IsTerminal,
    time::Instant,
    collections::HashSet,
};

use winit::{
    event_loop::{ControlFlow, ActiveEventLoop, EventLoop},
    application::ApplicationHandler,
    event::{WindowEvent},
    window::{Window, WindowId, WindowAttributes},
    keyboard::{KeyCode, PhysicalKey}
};

use tracing::{info, error};
use tracing_subscriber::EnvFilter;

pub mod renderer;
use renderer::Renderer;
use renderer::MeshID;
use renderer::Camera;

pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    square: MeshID,
    first_square_transform: glam::Mat4,
    camera: Camera,
    frames: u32,
    last_frame: Instant,
    fps_timer: Instant,
    dt: f32,
    pressed_keys: HashSet<KeyCode>,
}

impl Default for App {
    fn default() -> Self {
	return Self {
	    window: None,
	    renderer: None,
	    square: 0,
	    first_square_transform: glam::Mat4::IDENTITY,
	    frames: 0,
	    dt: 0.0,
	    last_frame: Instant::now(),
	    fps_timer: Instant::now(),
	    camera: Camera::default(),
	    pressed_keys: HashSet::new(),
	}
    }
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
	    .upload_mesh(&VERTICES_SQUARE, &INDICES_SQUARE);
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
		let now = Instant::now();
		
		self.dt = (now - self.last_frame).as_secs_f32();
		self.last_frame = now;
		
		let speed = self.dt * 5.0;
		
		for key in &self.pressed_keys {
		    match *key {
			KeyCode::KeyW => self.camera.move_with(0.0, 0.0, speed),
			KeyCode::KeyS => self.camera.move_with(0.0, 0.0, -speed),
			KeyCode::KeyA => self.camera.move_with(-speed, 0.0, 0.0),
			KeyCode::KeyD => self.camera.move_with(speed, 0.0, 0.0),
			KeyCode::KeyE => self.camera.move_with(0.0, speed, 0.0),
			KeyCode::KeyQ => self.camera.move_with(0.0, -speed, 0.0),
			KeyCode::ArrowUp => self.camera.pitch += speed * 25.0,
			KeyCode::ArrowDown => self.camera.pitch += -speed * 25.0,
			KeyCode::ArrowLeft => self.camera.yaw += -speed * 25.0,
			KeyCode::ArrowRight => self.camera.yaw += speed * 25.0,
			_ => {},
		    }
		}


		renderer.submit_mesh(
		    self.square,
		    self.first_square_transform
		);
		
		renderer.draw(self.camera);
		
		self.frames += 1;

		self.first_square_transform *= glam::Mat4::from_rotation_x((6.0_f32).to_radians() * self.dt);
		
		if self.fps_timer.elapsed().as_secs_f32() >= 1.0 {
		    let fps =
			self.frames as f32 /
			self.fps_timer.elapsed().as_secs_f32();
		    
		    println!("FPS: {:.2}", fps);
		    
		    self.frames = 0;
		    self.fps_timer = Instant::now();
		}
		
                window.request_redraw();
            },
	    WindowEvent::KeyboardInput {event, ..} => {
		if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
		    match event.state {
			winit::event::ElementState::Pressed => {
			    self.pressed_keys.insert(code);
			}
			winit::event::ElementState::Released => {
			    self.pressed_keys.remove(&code);
			}
		    }
		}
	    }
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
