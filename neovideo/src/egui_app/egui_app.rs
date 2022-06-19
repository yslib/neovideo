pub enum AppState{
    Running,
    Exit,
}

pub trait EguiApp {
    fn update(&mut self, ctx: &egui::Context, control_flow: &mut AppState);
}
