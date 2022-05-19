use glam::{Vec2, Vec4};

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
