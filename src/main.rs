use eframe::egui;



fn main() -> eframe::Result<()>  {
    let options = eframe::NativeOptions{
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ASSistant",
        options,
        Box::new(|_cc: &eframe::CreationContext<'_>| {
            Ok(Box::new(MyAssistantApp::default()))
        })
    )
}



struct MyAssistantApp {

}

impl Default for MyAssistantApp {
    fn default() -> Self {
        Self {

        }
    }
}

impl eframe::App for MyAssistantApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui: &mut egui::Ui| {

        });
    }
}
