use glutin::{PossiblyCurrent, ContextError, ContextWrapper};
use winit::window::{Window, WindowBuilder};

use super::egui_app::{AppState, EguiApp};

pub struct WinitEguiEventListener {
    egui_glow: egui_glow::EguiGlow,
    gl: std::sync::Arc<glow::Context>,
    app: Box<dyn EguiApp>,
    gl_context: Option<glutin::WindowedContext<PossiblyCurrent>>,
}

impl WinitEguiEventListener {
    pub fn new(
        event_loop: &glutin::event_loop::EventLoop<()>,
        window_builder: WindowBuilder,
        app: Box<dyn EguiApp>,
    ) -> Self {
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
        let gl = std::sync::Arc::new(gl);
        let egui_glow = egui_glow::EguiGlow::new(event_loop, gl.clone());
        Self {
            egui_glow,
            gl,
            app,
            gl_context: Some(gl_window),
        }
    }

    #[inline]
    pub unsafe fn make_current(&mut self){
        self.gl_context.take().map(|ctx| {
            self.gl_context = Some(ctx.make_current().expect("make_current error"));
        });
    }

    #[inline]
    pub fn process_event(&mut self, event: &glutin::event::WindowEvent) {
        let context = self.gl_context.as_ref().unwrap();
        if let glutin::event::WindowEvent::Resized(physical_size) = &event {
            context.resize(*physical_size);
        } else if let glutin::event::WindowEvent::ScaleFactorChanged { new_inner_size, .. } = &event
        {
            context.resize(**new_inner_size);
        }
        self.egui_glow.on_event(event);
    }

    pub fn process_redraw(&mut self) -> glutin::event_loop::ControlFlow {
        let mut app_state = AppState::Running;
        let clear_color = [0.1, 0.1, 0.1];
        unsafe {
            self.make_current();
        }
        let window = self.gl_context.as_ref().unwrap().window();
        let needs_repaint = self.egui_glow.run(window, |egui_ctx| {
            self.app.update(egui_ctx, &mut app_state);
        });

        {
            unsafe {
                use glow::HasContext as _;
                self.gl
                    .clear_color(clear_color[0], clear_color[1], clear_color[2], 1.0);
                self.gl.clear(glow::COLOR_BUFFER_BIT);
            }

            // draw things behind egui here
            self.egui_glow.paint(window);
        }
        match app_state {
            AppState::Exit => glutin::event_loop::ControlFlow::Exit,
            AppState::Running => {
                if needs_repaint {
                    window.request_redraw();
                    glutin::event_loop::ControlFlow::Poll
                } else {
                    glutin::event_loop::ControlFlow::Wait
                }
            }
        }
    }

    pub fn process_destroy(&mut self) {
        self.egui_glow.destroy();
    }

    pub fn swap_buffers(&mut self) {
        self.gl_context.as_ref().unwrap().swap_buffers().unwrap();
    }

    pub fn window(&mut self) -> &Window {
        self.gl_context.as_mut().unwrap().window()
    }
}
