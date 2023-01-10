use std::error::Error;

use miniquad::Context;
use noise::{Fbm, Perlin};
use noise::utils::{NoiseMapBuilder, PlaneMapBuilder};

use crate::primitives::TexturedQuad;

/// A structure representing the rendering of a patch of perlin noise.
pub struct PerlinMap {
    textured_quad: TexturedQuad,
}

impl PerlinMap {
    /// Create a new perlin map that renders via the given miniquad context.
    pub fn new(ctx: &mut Context) -> Result<Self, Box<dyn Error>> {
        const WIDTH: usize = 128;
        const HEIGHT: usize = 128;

        let textured_quad = TexturedQuad::new(ctx, WIDTH, HEIGHT)?;

        let fbm = Fbm::<Perlin>::default();
        let noise_map = PlaneMapBuilder::<_, 2>::new(&fbm)
            .set_size(textured_quad.width as usize, textured_quad.height as usize)
            .set_x_bounds(-5.0, 5.0)
            .set_y_bounds(-5.0, 5.0)
            .build();

        let data = noise_map.iter().flat_map(|&sample| {
            let sample = (sample * 256.0) as u8;
            [sample, sample, sample, 0xFF]
        }).collect::<Vec<u8>>();

        textured_quad.texture.update(ctx, &data);

        Ok(Self {
            textured_quad,
        })
    }

    /// Update the perlin map.
    pub fn update(&mut self, _ctx: &mut Context) {}

    /// Draw the perlin map.
    pub fn draw(&mut self, ctx: &mut Context) {
        self.textured_quad.draw(ctx);
    }
}
