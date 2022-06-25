#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};

use egui_app::{
    egui_app::{AppState, EguiApp},
    winit_egui_event_listener::WinitEguiEventListener,
};
use glutin::{event_loop::ControlFlow, ContextWrapper, PossiblyCurrent};
use neovideo_vlc::vlcvideo::{TextureRender, VLCVideo};
use winit::window::{Window, WindowBuilder, WindowId};

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

struct PlayerWindow {
    texture_render: TextureRender,
    video_decoder: VLCVideo,
    window_context: Option<glutin::WindowedContext<PossiblyCurrent>>,
}

impl PlayerWindow {
    #[allow(unused)]
    pub fn new(
        event_loop: &glutin::event_loop::EventLoop<()>,
        window_builder: WindowBuilder,
    ) -> Self {
        let window_context = unsafe {
            glutin::ContextBuilder::new()
                .with_depth_buffer(0)
                .with_srgb(true)
                .with_stencil_buffer(0)
                .build_windowed(window_builder, event_loop)
                .unwrap()
                .make_current()
                .unwrap()
        };
        let texture_render = TextureRender::new(&window_context);
        let video_decoder = VLCVideo::new(window_context.context(), event_loop);
        PlayerWindow {
            texture_render,
            window_context: Some(window_context),
            video_decoder,
        }
    }

    pub fn window(&self) -> &Window {
        self.window_context.as_ref().unwrap().window()
    }

    pub fn play<T>(&mut self, path: T)
    where
        T: AsRef<Path>,
    {
        self.video_decoder.play_media(path);
    }

    #[allow(unused)]
    pub unsafe fn make_current(&mut self) {
        self.window_context.take().map(|ctx| {
            self.window_context = Some(
                ctx.make_current()
                    .expect("make_current error in PlayerWindow::make_current"),
            );
        });
    }

    #[allow(unused)]
    pub fn render_frame(&mut self) {
        unsafe {
            self.make_current();
        }
        let mut update: bool = false;
        let tex = self.video_decoder.get_video_frame(&mut update);
        if update {
            println!("render_frame: {}", tex);
            self.texture_render.draw_video_frame(tex);
        }
    }
}

fn main() {
    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let app = Box::new(NeovideoApp::new());

    let window_builder = glutin::window::WindowBuilder::new()
        .with_resizable(true)
        .with_decorations(true)
        .with_title("Egui Demo");

    let mut egui_listener = WinitEguiEventListener::new(&event_loop, window_builder, app);
    let egui_winid = egui_listener.window().id();

    let window_builder = glutin::window::WindowBuilder::new()
        .with_title("Neovideo")
        .with_resizable(true);

    let mut player_window = PlayerWindow::new(&event_loop, window_builder);

    let player_winid = player_window.window().id();
    player_window.play("file:///D:\\movie\\matrix\\matrix.mkv");

    event_loop.run(move |event, _, control_flow| {
        player_window.render_frame();
        player_window
            .window_context
            .as_ref()
            .unwrap()
            .swap_buffers()
            .unwrap();
        player_window.window().request_redraw();
        match event {
            glutin::event::Event::RedrawEventsCleared if cfg!(windows) => {
                *control_flow = egui_listener.process_redraw();
                egui_listener.swap_buffers();
            }
            glutin::event::Event::RedrawRequested(window_id) if !cfg!(windows) => {
                if window_id == egui_winid {
                    *control_flow = egui_listener.process_redraw();
                    egui_listener.swap_buffers();
                } else if window_id == player_winid {
                    println!("redraw request");
                }
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
        }
    });
}

fn create_display(
    event_loop: &glutin::event_loop::EventLoop<()>,
) -> (
    glutin::WindowedContext<glutin::PossiblyCurrent>,
    glow::Context,
) {
    let window_builder = glutin::window::WindowBuilder::new()
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize {
            width: 800.0,
            height: 600.0,
        })
        .with_title("egui_glow example");

    let gl_window = unsafe {
        glutin::ContextBuilder::new()
            .with_depth_buffer(0)
            .with_srgb(true)
            .with_stencil_buffer(0)
            .with_vsync(true)
            .build_windowed(window_builder, event_loop)
            .unwrap()
            .make_current()
            .unwrap()
    };

    let gl = unsafe { glow::Context::from_loader_function(|s| gl_window.get_proc_address(s)) };

    (gl_window, gl)
}
