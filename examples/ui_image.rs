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
            font_cache::FontSize, next_hot_active, Align, Axis, Board, BoardInput, BorderRadius,
            Button, ContainerId, DivStyle, DivTexture, HotActive, Id, Len, MainAlign, Slider, Text,
            Widget,
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
    app.add::<GraphicsSettingsController>();
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

        let mut parent = self.ui.add_div("Parent", None);
        parent.width = Len::PARENT;
        parent.height = Len::PARENT;
        parent.axis = Axis::X;
        parent.main_align = MainAlign::Center;
        parent.cross_align = Align::Center;
        parent.color = Color::RED.alpha(0.2);
        let parent = Some(parent.id);

        // show some image in the UI
        let mut img = self.ui.add_div("img", parent);
        img.width = Len::Px(200.0);
        img.height = Len::Px(200.0);
        img.texture = Some(DivTexture {
            texture: self.image_key.key(),
            uv: Aabb::UNIT,
        });

        // sow the slider
        self.ui
            .add(Slider::new(&mut self.value, 0.0, 400.0), "slider", parent);

        self.ui.end_frame(&mut self.deps.ui.fonts);
        self.deps.ui.ui_renderer.draw_ui_board(&self.ui);

        // compare to Egui Widget (obviously a bit more polished)
        let mut egui = self.deps.egui.context();
        egui::Window::new("Value").show(&mut egui, |ui| {
            ui.add(egui::Slider::new(&mut self.value, 0.0..=400.0));
        });
        // std::thread::sleep(Duration::from_millis(150));
    }
}
