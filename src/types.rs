use std::ops;

/// A Vec2 type for uploading to opengl, and also basic vector operations.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }
}

impl ops::Add<Vec2> for Vec2 {
    type Output = Vec2;

    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl ops::Sub<Vec2> for Vec2 {
    type Output = Vec2;

    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2 { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}

impl ops::Mul<f32> for Vec2 {
    type Output = Vec2;

    fn mul(self, rhs: f32) -> Vec2 {
        Vec2 { x: self.x * rhs, y: self.y * rhs }
    }
}

impl ops::Div<f32> for Vec2 {
    type Output = Vec2;

    fn div(self, rhs: f32) -> Vec2 {
        Vec2 { x: self.x / rhs, y: self.y / rhs }
    }
}

/// A Vec2d (double) type for uploading to opengl, and also basic vector operations.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Vec2d {
    pub x: f64,
    pub y: f64,
}

impl Vec2d {
    pub const fn new(x: f64, y: f64) -> Vec2d {
        Vec2d { x, y }
    }
}

impl ops::Add<Vec2d> for Vec2d {
    type Output = Vec2d;

    fn add(self, rhs: Vec2d) -> Vec2d {
        Vec2d { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl ops::Sub<Vec2d> for Vec2d {
    type Output = Vec2d;

    fn sub(self, rhs: Vec2d) -> Vec2d {
        Vec2d { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}

impl ops::Mul<f64> for Vec2d {
    type Output = Vec2d;

    fn mul(self, rhs: f64) -> Vec2d {
        Vec2d { x: self.x * rhs, y: self.y * rhs }
    }
}

impl ops::Div<f64> for Vec2d {
    type Output = Vec2d;

    fn div(self, rhs: f64) -> Vec2d {
        Vec2d { x: self.x / rhs, y: self.y / rhs }
    }
}

impl std::convert::Into<Vec2> for Vec2d {
    fn into(self) -> Vec2 {
        Vec2 { x: self.x as f32, y: self.y as f32 }
    }
}

/// A Vertex type for our gpu vertex buffers.
#[repr(C)]
pub struct Vertex {
    pub pos: Vec2,
    pub uv: Vec2,
}
