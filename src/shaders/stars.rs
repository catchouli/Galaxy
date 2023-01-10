use miniquad::*;

pub const _VERTEX: &str = r#"
    #version 100

    attribute vec2 pos;

    void main() {
        gl_Position = vec4(pos, 0, 1);
    }
"#;

pub const _FRAGMENT: &str = r#"
    #version 100

    varying lowp vec2 texcoord;

    void main() {
        gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
    }
"#;

pub fn _meta() -> ShaderMeta {
    ShaderMeta {
        images: Vec::new(),
        uniforms: UniformBlockLayout {
            uniforms: vec![UniformDesc::new("offset", UniformType::Float2)],
        },
    }
}

#[repr(C)]
pub struct _Uniforms {
    pub offset: (f32, f32),
}
