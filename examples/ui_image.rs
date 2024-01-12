use std::time::Duration;

use egui::Style;
use glam::{dvec2, vec2, vec3, Vec2};
use vert::{
    elements::{rect::Aabb, BindableTexture, Color, Rect, Texture, Transform},
    modules::{
        arenas::{Key, OwnedKey},
        batteries::{FlyCam, GraphicsSettingsController},
        renderer::main_pass_renderer::{text_renderer::DrawText, ui_rect::UiRect},
        ui::{
            board::{
                egui_inspect_board, Align, Axis, Board, BoardInput, BorderRadius, ContainerId,
                DivProps, DivStyle, DivTexture, Id, Len, MainAlign, Text,
            },
            font_cache::FontSize,
            widgets::{Button, Widget},
        },
        DefaultDependencies, DefaultModules, MainPassRenderer, Schedule,
    },
    utils::Timing,
    AppBuilder, Module,
};

fn main() {
    let mut app = AppBuilder::new();
    app.add_plugin(DefaultModules);
    app.add::<FlyCam>();
    // app.add::<GraphicsSettingsController>();
    app.add::<MyApp>();
    app.run().unwrap();
}

struct MyApp {
    deps: DefaultDependencies,
    ui: Board,
    value: f32,
    image_key: OwnedKey<BindableTexture>,
}

impl Module for MyApp {
    type Config = ();

    type Dependencies = DefaultDependencies;

    fn new(config: Self::Config, mut deps: Self::Dependencies) -> anyhow::Result<Self> {
        deps.bloom.settings_mut().activated = false;
        deps.ui
            .ui_renderer
            .watch_shader_file("./src/modules/ui/ui.wgsl");

        let img_bytes = std::fs::read("assets/test.png").unwrap();
        let rgba = image::load_from_memory(&img_bytes).unwrap().to_rgba8();
        let texture = Texture::from_image(&deps.ctx.device, &deps.ctx.queue, &rgba);

        let bindable_texture = BindableTexture::new(&deps.ctx.device, texture);
        let image_texture = deps.arenas.insert(bindable_texture);

        Ok(MyApp {
            deps,
            ui: Board::new(dvec2(800.0, 800.0)),
            image_key: image_texture,
            value: 300.0,
        })
    }

    fn intialize(handle: vert::Handle<Self>) -> anyhow::Result<()> {
        let scheduler = handle.deps.scheduler.get_mut();
        scheduler.register(handle, Schedule::Update, Timing::DEFAULT, Self::update);

        Ok(())
    }
}

impl MyApp {
    fn update(&mut self) {
        self.deps.gizmos.draw_xyz();
        self.deps
            .color_mesh
            .draw_cubes(&[Transform::new(1.0, 1.0, 1.0)], None);

        let size = self.deps.ctx.size;
        self.ui.start_frame(
            BoardInput::from_input_module(&self.deps.input),
            dvec2(size.width as f64, size.height as f64),
        );

        let parent = self
            .ui
            .add_non_text_div(
                DivProps {
                    width: Len::PARENT,
                    height: Len::PARENT,
                    axis: Axis::X,
                    main_align: MainAlign::Center,
                    cross_align: Align::Center,
                    absolute: false,
                },
                DivStyle {
                    color: Color::RED.alpha(0.2),
                    ..Default::default()
                },
                Id::from("Parent"),
                None,
            )
            .id;

        self.ui.add_non_text_div(
            DivProps {
                width: Len::Px(200.0),
                height: Len::Px(200.0),
                ..Default::default()
            },
            DivStyle {
                color: Color::RED,
                texture: Some(DivTexture {
                    texture: self.image_key.key(),
                    uv: Aabb::UNIT,
                }),
                ..Default::default()
            },
            Id::from("img"),
            Some(parent),
        );

        self.ui.add(
            Slider {
                value: &mut self.value,
                min: 0.0,
                max: 200.0,
            },
            Id::from("slider"),
            Some(parent),
        );
        self.ui.end_frame(&mut self.deps.ui.fonts);
        self.deps.ui.ui_renderer.draw_billboard(&self.ui);
        // std::thread::sleep(Duration::from_millis(150));
    }
}

pub struct Slider<'v> {
    value: &'v mut f32,
    min: f32,
    max: f32,
}

impl<'v> Widget for Slider<'v> {
    type Response<'a> = ();

    fn add_to_board<'a>(
        self,
        board: &'a mut Board,
        id: Id,
        parent: Option<ContainerId>,
    ) -> Self::Response<'a> {
        let hot_active = board.hot_active(id);

        let container = board.add_non_text_div(
            DivProps {
                width: Len::Px(100.0),
                height: Len::Px(20.0),
                axis: Axis::X,
                main_align: MainAlign::Start,
                cross_align: Align::Center,
                absolute: false,
            },
            DivStyle {
                color: Color::PURPLE,
                ..DivStyle::default()
            },
            id,
            parent,
        );

        let container_hovered = container.mouse_in_rect();
        let container = container.id;

        // slider bar
        board.add_non_text_div(
            DivProps {
                width: Len::PARENT,
                height: Len::Px(8.0),
                ..Default::default()
            },
            DivStyle {
                color: Color::GREY,
                border_color: Color::from_hex("#32a852"),
                border_radius: BorderRadius::all(4.0),
                border_thickness: 1.0,
                ..Default::default()
            },
            id + 1,
            Some(container),
        );

        // knob
        board.add_non_text_div(
            DivProps {
                width: Len::Px(16.0),
                height: Len::Px(16.0),
                absolute: true,
                ..Default::default()
            },
            DivStyle {
                color: Color::BLACK,
                border_color: if container_hovered {
                    Color::from_hex("#ffffff")
                } else {
                    Color::BLACK
                },

                border_radius: BorderRadius::all(8.0),
                border_thickness: 3.0,
                ..Default::default()
            },
            id + 2,
            Some(container),
        );
    }
}
