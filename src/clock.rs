use std::time::Instant;

pub struct DeltaClock {
    last_frame: Instant,
    dt: f32,
}

impl Default for DeltaClock {
    fn default() -> Self {
	Self {
	    last_frame: Instant::now(),
	    dt: 0.0,
	}
    }
}

impl DeltaClock {
    pub fn clock(&mut self) {
	let now = Instant::now();
	self.dt = (now - self.last_frame).as_secs_f32();
	self.last_frame = now;
    }

    pub fn get_dt(&self) -> f32 {
	return self.dt;
    }
}
