#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }
}

#[repr(C)]
pub struct Vertex {
    pub pos: Vec2,
    pub uv: Vec2,
}
