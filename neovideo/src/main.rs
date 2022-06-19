#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui_app::{
    egui_app::{AppState, EguiApp},
    winit_egui_event_listener::WinitEguiEventListener,
};
use glutin::event_loop::ControlFlow;

mod egui_app;

struct NeovideoApp {}

impl EguiApp for NeovideoApp {
    fn update(&mut self, ctx: &egui::Context, app_state: &mut AppState) {
        egui::SidePanel::left("my_side_panel").show(ctx, |ui| {
            ui.heading("Hello World!");
            if ui.button("Quit").clicked() {
                *app_state = AppState::Exit;
            }
            ui.color_edit_button_rgb(&mut [0.1, 0.1, 0.1]);
        });
    }
}

impl NeovideoApp {
    pub fn new() -> Self {
        Self {}
    }
}

fn main() {
    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let app = Box::new(NeovideoApp::new());

    let window_builder = glutin::window::WindowBuilder::new()
        .with_resizable(true)
        .with_title("Neovideo");

    let mut egui_listener = WinitEguiEventListener::new(&event_loop, window_builder, app);

    event_loop.run(move |event, _, control_flow| match event {
        glutin::event::Event::RedrawEventsCleared if cfg!(windows) => {
            *control_flow = egui_listener.process_redraw();
            egui_listener.swap_buffers();
        }
        glutin::event::Event::RedrawRequested(_) if !cfg!(windows) => {
            *control_flow = egui_listener.process_redraw();
            egui_listener.swap_buffers();
        }

        glutin::event::Event::WindowEvent { event, .. } => {
            use glutin::event::WindowEvent;
            if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
                *control_flow = ControlFlow::Exit;
            }

            egui_listener.process_event(&event);

            egui_listener.window().request_redraw();
        }
        glutin::event::Event::LoopDestroyed => {
            egui_listener.process_destroy();
        }

        _ => (),
    });
}
