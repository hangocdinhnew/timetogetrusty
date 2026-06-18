use glam::{Vec3, Mat4};

#[derive(PartialEq, Clone, Copy)]
pub struct Camera {
    pub position: Vec3,
    
    pub yaw: f32,
    pub pitch: f32,
    
    pub fov: f32,
    pub draw_distance: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 2.0),
            yaw: -90.0,
            pitch: 0.0,
            fov: 75.0,
            draw_distance: 100.0,
        }
    }
}

impl Camera {
    pub fn move_to(&mut self, x: f32, y: f32, z: f32) {
        self.position = Vec3::new(x, y, z);
    }
    
    pub fn move_with(&mut self, right: f32, up: f32, forward: f32) {
        let forward_vec = self.forward();
        
        let right_vec = self.right();
        
        self.position += right_vec * right;
        self.position += Vec3::Y * up;
        self.position += forward_vec * forward;
    }
    
    pub fn view(&self) -> Mat4 {
        Mat4::look_at_rh(
            self.position,
            self.position + self.forward(),
            Vec3::Y,
        )
    }
    
    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov;
    }
    
    pub fn set_draw_distance(&mut self, draw_distance: f32) {
        self.draw_distance = draw_distance;
    }
    
    fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }
    
    fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos(),
        ).normalize()
    }
}
