use std::error::Error;

use miniquad::*;
use crate::types::*;
use crate::shaders::*;

pub struct TexturedQuad {
    pipeline: Pipeline,
    bindings: Bindings,
    pub texture: Texture,
    pub width: usize,
    pub height: usize,
}

impl TexturedQuad {
    pub fn new(ctx: &mut Context, width: usize, height: usize) -> Result<Self, Box<dyn Error>> {
        let vertices: [Vertex; 4] = [
            Vertex { pos: Vec2::new(-1.0, -1.0), uv: Vec2::new(0.0, 0.0) },
            Vertex { pos: Vec2::new( 1.0, -1.0), uv: Vec2::new(1.0, 0.0) },
            Vertex { pos: Vec2::new( 1.0,  1.0), uv: Vec2::new(1.0, 1.0) },
            Vertex { pos: Vec2::new(-1.0,  1.0), uv: Vec2::new(0.0, 1.0) },
        ];

        let vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices);

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &indices);

        let texture_size = usize::try_from(width * height * 4).unwrap();
        let pixels = vec![0x00; texture_size];
        let texture = Texture::from_data_and_format(
            ctx,
            &pixels,
            TextureParams {
                width: width.try_into().unwrap(),
                height: height.try_into().unwrap(),
                format: TextureFormat::RGBA8,
                wrap: TextureWrap::Clamp,
                filter: FilterMode::Nearest,
            });

        let bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            images: vec![texture],
            index_buffer,
        };

        let shader = Shader::new(ctx,
            basic_textured::VERTEX,
            basic_textured::FRAGMENT,
            basic_textured::meta()).unwrap();

        let pipeline = Pipeline::new(
            ctx,
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("pos", VertexFormat::Float2),
                VertexAttribute::new("uv", VertexFormat::Float2),
            ],
            shader,
        );

        Ok(Self {
            pipeline,
            bindings,
            texture,
            width,
            height,
        })
    }

    pub fn draw(&self, ctx: &mut Context) {
        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);

        ctx.apply_uniforms(&basic_textured::Uniforms {
            offset: (0.0, 0.0),
        });
        ctx.draw(0, 6, 1);
    }
}
