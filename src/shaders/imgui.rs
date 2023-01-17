// Based on https://github.com/not-fl3/imgui-miniquad-render.
use miniquad::*;

pub const VERTEX: &str = r#"
    #version 100

    attribute vec2 position;
    attribute vec2 texcoord;
    attribute vec4 color0;
    varying lowp vec2 uv;
    varying lowp vec4 color;

    uniform mat4 Projection;
    void main() {
        gl_Position = Projection * vec4(position, 0, 1);
        gl_Position.z = 0.;
        color = color0 / 255.0;
        uv = texcoord;
    }
"#;

pub const FRAGMENT: &str = r#"
    #version 100

    varying lowp vec4 color;
    varying lowp vec2 uv;

    uniform sampler2D Texture;

    void main() {
        gl_FragColor = color * texture2D(Texture, uv);
    }
"#;

pub fn meta() -> ShaderMeta {
    ShaderMeta {
        images: vec!["Texture".to_string()],
        uniforms: UniformBlockLayout {
            uniforms: vec![UniformDesc::new("Projection", UniformType::Mat4)],
        },
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Uniforms {
    pub projection: glam::Mat4,
}
