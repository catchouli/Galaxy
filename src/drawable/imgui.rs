use std::{cell::{RefCell, RefMut}, rc::Rc};

// Based on https://github.com/not-fl3/imgui-miniquad-render.
use miniquad::*;
use imgui::{DrawCmd, DrawCmdParams, DrawVert};
use crate::shaders::imgui as shader;

const MAX_VERTICES: usize = 30000;
const MAX_INDICES: usize = 50000;

/// An ImguiRenderer, which owns an instance of imgui and responds to miniquad events for input and rendering.
pub struct ImguiRenderer {
    last_frame: std::time::Instant,
    imgui: Rc<RefCell<imgui::Context>>,
    pipeline: Pipeline,
    font_texture: Texture,
    draw_calls: Vec<Bindings>,
}

impl ImguiRenderer {
    /// Create a new imgui renderer for the given miniquad context.
    pub fn new(ctx: &mut miniquad::Context, imgui: Rc<RefCell<imgui::Context>>) -> Self {
        let shader = Shader::new(ctx, shader::VERTEX, shader::FRAGMENT, shader::meta()).unwrap();

        let pipeline = Pipeline::with_params(
            ctx,
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("position", VertexFormat::Float2),
                VertexAttribute::new("texcoord", VertexFormat::Float2),
                VertexAttribute::new("color0", VertexFormat::Byte4),
            ],
            shader,
            PipelineParams {
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        );

        {
            use imgui::*;

            let mut imgui = imgui.borrow_mut();
            imgui.fonts().add_font(&[FontSource::DefaultFontData {
                config: Some(FontConfig {
                    rasterizer_multiply: 1.75,
                    ..FontConfig::default()
                }),
            }]);

            let (w, h) = ctx.screen_size();
            let mut io = imgui.io_mut();

            io[Key::Tab] = KeyCode::Tab as _;
            io[Key::LeftArrow] = KeyCode::Left as _;
            io[Key::RightArrow] = KeyCode::Right as _;
            io[Key::UpArrow] = KeyCode::Up as _;
            io[Key::DownArrow] = KeyCode::Down as _;
            io[Key::PageUp] = KeyCode::PageUp as _;
            io[Key::PageDown] = KeyCode::PageDown as _;
            io[Key::Home] = KeyCode::Home as _;
            io[Key::End] = KeyCode::End as _;
            io[Key::Insert] = KeyCode::Insert as _;
            io[Key::Delete] = KeyCode::Delete as _;
            io[Key::Backspace] = KeyCode::Backspace as _;
            io[Key::Space] = KeyCode::Space as _;
            io[Key::Enter] = KeyCode::Enter as _;
            io[Key::Escape] = KeyCode::Escape as _;
            io[Key::KeypadEnter] = KeyCode::KpEnter as _;
            io[Key::A] = KeyCode::A as _;
            io[Key::C] = KeyCode::C as _;
            io[Key::V] = KeyCode::V as _;
            io[Key::X] = KeyCode::X as _;
            io[Key::Y] = KeyCode::Y as _;
            io[Key::Z] = KeyCode::Z as _;

            io.font_global_scale = 1.0;
            io.display_size = [w, h];
            io.mouse_pos = [0., 0.];
        }

        let font_texture = {
            let mut imgui = imgui.borrow_mut();
            let mut fonts = imgui.fonts();
            let texture = fonts.build_rgba32_texture();

            Texture::from_rgba8(
                ctx,
                texture.width as u16,
                texture.height as u16,
                texture.data,
            )
        };

        Self {
            imgui,
            pipeline,
            font_texture,
            last_frame: std::time::Instant::now(),
            draw_calls: Vec::with_capacity(200),
        }
    }
}

impl EventHandler for ImguiRenderer {
    fn resize_event(&mut self, _ctx: &mut miniquad::Context, width: f32, height: f32) {
        let mut imgui = self.imgui.borrow_mut();
        let mut io = imgui.io_mut();
        io.display_size = [width, height];
    }

    fn char_event(&mut self, _ctx: &mut miniquad::Context, character: char, mods: KeyMods, _: bool) {
        let mut imgui = self.imgui.borrow_mut();
        let io = imgui.io_mut();

        io.key_ctrl = mods.ctrl;
        io.key_alt = mods.alt;
        io.key_shift = mods.shift;

        io.add_input_character(character);
    }

    fn key_down_event(&mut self, _ctx: &mut miniquad::Context, keycode: KeyCode, mods: KeyMods, _: bool) {
        let mut imgui = self.imgui.borrow_mut();
        let mut io = imgui.io_mut();

        // when the keycode is the modifier itself - mods.MODIFIER is false yet, however the modifier button is just pressed and is actually true
        io.key_ctrl = mods.ctrl;
        io.key_alt = mods.alt;
        io.key_shift = mods.shift;

        io.keys_down[keycode as usize] = true;
    }

    fn key_up_event(&mut self, _ctx: &mut miniquad::Context, keycode: KeyCode, mods: KeyMods) {
        let mut imgui = self.imgui.borrow_mut();
        let mut io = imgui.io_mut();

        // when the keycode is the modifier itself - mods.MODIFIER is true, however the modifier is actually released
        io.key_ctrl =
            keycode != KeyCode::LeftControl && keycode != KeyCode::RightControl && mods.ctrl;
        io.key_alt = keycode != KeyCode::LeftAlt && keycode != KeyCode::RightAlt && mods.alt;
        io.key_shift =
            keycode != KeyCode::LeftShift && keycode != KeyCode::RightShift && mods.shift;

        io.keys_down[keycode as usize] = false;
    }

    fn mouse_motion_event(&mut self, _ctx: &mut miniquad::Context, x: f32, y: f32) {
        let mut imgui = self.imgui.borrow_mut();
        let mut io = imgui.io_mut();
        io.mouse_pos = [x, y];
    }
    fn mouse_wheel_event(&mut self, _ctx: &mut miniquad::Context, _x: f32, y: f32) {
        let mut imgui = self.imgui.borrow_mut();
        let mut io = imgui.io_mut();
        io.mouse_wheel = y;
    }
    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut miniquad::Context,
        button: MouseButton,
        _x: f32,
        _y: f32,
    ) {
        let mut imgui = self.imgui.borrow_mut();
        let mut io = imgui.io_mut();
        let mouse_left = button == MouseButton::Left;
        let mouse_right = button == MouseButton::Right;
        io.mouse_down = [mouse_left, mouse_right, false, false, false];
    }
    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut miniquad::Context,
        _button: MouseButton,
        _x: f32,
        _y: f32,
    ) {
        let mut imgui = self.imgui.borrow_mut();
        let mut io = imgui.io_mut();
        io.mouse_down = [false, false, false, false, false];
    }

    /// Unused
    fn update(&mut self, _ctx: &mut miniquad::Context) {}

    /// Render imgui.
    fn draw(&mut self, ctx: &mut miniquad::Context) {
        let mut imgui = self.imgui.borrow_mut();
        let draw_data = {
            let io = imgui.io_mut();
            let now = std::time::Instant::now();
            io.update_delta_time(now.duration_since(self.last_frame));
            self.last_frame = now;
            let ui = imgui.frame();
            ui.window("test")
                .size([300.0, 300.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    ui.text("Hello world");
                });
            imgui.render()
        };

        let (width, height) = ctx.screen_size();
        let projection = glam::Mat4::orthographic_rh_gl(0., width, height, 0., -1., 1.);

        ctx.begin_default_pass(PassAction::Nothing);

        let clip_off = draw_data.display_pos;
        let clip_scale = draw_data.framebuffer_scale;

        for (n, draw_list) in draw_data.draw_lists().enumerate() {
            let vertices = draw_list.vtx_buffer();
            let indices = draw_list.idx_buffer();

            if n >= self.draw_calls.len() {
                let vertex_buffer = Buffer::stream(
                    ctx,
                    BufferType::VertexBuffer,
                    MAX_VERTICES * std::mem::size_of::<DrawVert>(),
                );
                let index_buffer = Buffer::stream(
                    ctx,
                    BufferType::IndexBuffer,
                    MAX_INDICES * std::mem::size_of::<u16>(),
                );
                let bindings = Bindings {
                    vertex_buffers: vec![vertex_buffer],
                    index_buffer,
                    images: vec![],
                };
                self.draw_calls.push(bindings);
            }

            let dc = &mut self.draw_calls[n];

            if vertices.len() * std::mem::size_of::<DrawVert>() > dc.vertex_buffers[0].size() {
                println!("imgui: Vertex buffer too small, reallocating");

                dc.vertex_buffers[0] = Buffer::stream(
                    ctx,
                    BufferType::VertexBuffer,
                    vertices.len() * std::mem::size_of::<DrawVert>(),
                );
            }

            if indices.len() * std::mem::size_of::<u16>() > dc.index_buffer.size() {
                println!("imgui: Index buffer too small, reallocating");

                dc.index_buffer = Buffer::stream(
                    ctx,
                    BufferType::IndexBuffer,
                    indices.len() * std::mem::size_of::<u16>() * std::mem::size_of::<u16>(),
                );
            }

            dc.vertex_buffers[0].update(ctx, vertices);
            dc.index_buffer.update(ctx, indices);
            dc.images = vec![self.font_texture];

            let mut slice_start = 0;
            for cmd in draw_list.commands() {
                match cmd {
                    DrawCmd::Elements {
                        count,
                        cmd_params: DrawCmdParams { clip_rect, .. },
                    } => {
                        let clip_rect = [
                            (clip_rect[0] - clip_off[0]) * clip_scale[0],
                            (clip_rect[1] - clip_off[1]) * clip_scale[1],
                            (clip_rect[2] - clip_off[0]) * clip_scale[0],
                            (clip_rect[3] - clip_off[1]) * clip_scale[1],
                        ];
                        ctx.apply_pipeline(&self.pipeline);
                        let h = clip_rect[3] - clip_rect[1];

                        ctx.apply_scissor_rect(
                            clip_rect[0] as i32,
                            height as i32 - (clip_rect[1] + h) as i32,
                            (clip_rect[2] - clip_rect[0]) as i32,
                            h as i32,
                        );

                        ctx.apply_bindings(&dc);
                        ctx.apply_uniforms(&shader::Uniforms { projection });
                        ctx.draw(slice_start, count as i32, 1);
                        slice_start += count as i32;
                    }
                    _ => {}
                }
            }
        }

        ctx.end_render_pass();

        ctx.commit_frame();
    }
}
