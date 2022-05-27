use glam::{Mat4, Vec2, Vec3, Vec4};

#[derive(Default)]
pub struct Script {
    pub wasm: String,
}

#[derive(Debug)]
pub struct Shape {
    pub color: Vec4,
}

#[derive(Debug)]
pub struct Tag(pub String);

#[derive(Debug)]
pub struct Transform {
    pub position: Vec2,
    pub size: Vec2,
    pub rotation: f32,
}

pub fn compute_transformation_matrix(t: &Transform) -> Mat4 {
    let mut transform = Mat4::from_translation(Vec3::new(t.position.x, t.position.y, 0.0));
    transform *= Mat4::from_rotation_z(-t.rotation.to_radians());
    transform *= Mat4::from_scale(Vec3::new(t.size.x, t.size.y, 0.0));
    transform
}

pub fn compute_inverse_transformation_matrix(t: &Transform) -> Mat4 {
    let mut transform = Mat4::from_scale(Vec3::new(1.0 / t.size.x, 1.0 / t.size.y, 0.0));
    transform *= Mat4::from_rotation_z(t.rotation.to_radians());
    transform *= Mat4::from_translation(Vec3::new(-t.position.x, -t.position.y, 0.0));
    transform
}
