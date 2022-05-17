use glam::Mat4;

// TODO: Set where the world origin is - might want center of screen, not bottom left.
// TODO: Set Pixels-Per-Unit and scale things accordingly.
#[allow(dead_code)]
pub struct Camera {
    width: u32,
    height: u32,
    view: Mat4,
    projection: Mat4,
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Self {
        let projection =
            glam::Mat4::orthographic_lh(0.0, width as f32, 0.0, height as f32, -1.0, 1.0);

        Self {
            width,
            height,
            view: Mat4::IDENTITY,
            projection,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let projection =
            glam::Mat4::orthographic_lh(0.0, width as f32, 0.0, height as f32, -1.0, 1.0);

        self.width = width;
        self.height = height;
        self.projection = projection;
    }

    pub fn get_view(&self) -> Mat4 {
        // Just use some jankey values for look at for now.
        let view = glam::Mat4::look_at_lh(
            glam::Vec3::new(-200.0, -200.0, -1.0),
            glam::Vec3::new(-200.0, -200.0, 0.0),
            glam::Vec3::Y,
        );

        view
    }

    pub fn get_projection(&self) -> Mat4 {
        self.projection
    }
}
