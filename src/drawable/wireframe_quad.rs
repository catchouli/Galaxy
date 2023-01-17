use std::error::Error;

use miniquad::*;
use crate::types::*;
use crate::shaders::*;

pub struct WireframeQuad {
    pipeline: Pipeline,
    bindings: Bindings,
}

impl WireframeQuad {
    pub fn new(ctx: &mut Context) -> Result<Self, Box<dyn Error>> {
        let vertices: [Vertex; 4] = [
            Vertex { pos: Vec2::new(0.0, 0.0), uv: Vec2::new(0.0, 0.0) },
            Vertex { pos: Vec2::new(1.0, 0.0), uv: Vec2::new(1.0, 0.0) },
            Vertex { pos: Vec2::new(1.0, 1.0), uv: Vec2::new(1.0, 1.0) },
            Vertex { pos: Vec2::new(0.0, 1.0), uv: Vec2::new(0.0, 1.0) },
        ];

        let vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices);

        let indices: [u16; 8] = [0, 1, 1, 2, 2, 3, 3, 0];
        let index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &indices);

        let bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            images: Vec::new(),
            index_buffer,
        };

        let shader = Shader::new(ctx,
            wireframe_quad::VERTEX,
            wireframe_quad::FRAGMENT,
            wireframe_quad::meta()).unwrap();

        let pipeline_params = PipelineParams {
            primitive_type: PrimitiveType::Lines,
            ..Default::default()
        };

        let pipeline = Pipeline::with_params(
            ctx,
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("pos", VertexFormat::Float2),
                VertexAttribute::new("uv", VertexFormat::Float2),
            ],
            shader,
            pipeline_params,
        );

        Ok(Self {
            pipeline,
            bindings,
        })
    }

    pub fn draw(&self, ctx: &mut Context, min: &Vec2, max: &Vec2) {
        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);

        ctx.apply_uniforms(&wireframe_quad::Uniforms {
            min_max: (min.x, min.y, max.x, max.y),
        });
        ctx.draw(0, 8, 1);
    }
}
