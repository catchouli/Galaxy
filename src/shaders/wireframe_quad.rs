use miniquad::*;

pub const VERTEX: &str = r#"
    #version 100

    attribute vec2 pos;
    attribute vec2 uv;

    uniform vec4 min_max;

    void main() {
        // Scale from (0..1) to (min..max)
        vec2 pos_scaled = pos * (min_max.zw - min_max.xy) + min_max.xy;
        gl_Position = vec4(pos_scaled, 0, 1);
    }
"#;

pub const FRAGMENT: &str = r#"
    #version 100

    varying lowp vec2 texcoord;

    uniform sampler2D tex;

    void main() {
        gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
    }
"#;

pub fn meta() -> ShaderMeta {
    ShaderMeta {
        images: Vec::new(),
        uniforms: UniformBlockLayout {
            uniforms: vec![UniformDesc::new("min_max", UniformType::Float4)],
        },
    }
}

#[repr(C)]
pub struct Uniforms {
    pub min_max: (f32, f32, f32, f32),
}
