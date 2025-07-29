use eframe::egui;
use std::fs::File;
use serde_json::{Value, json};

mod request;
use crate::request::request;

fn main() -> eframe::Result<()>  {
    let file = File::open("chat_history.json").expect("file not found!");
    let json: serde_json::Value = serde_json::from_reader(file).expect("file should be proper JSON");

    let options = eframe::NativeOptions{
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ASSistant",
        options,
        Box::new(|_cc: &eframe::CreationContext<'_>| {
            Ok(Box::new(MyAssistantApp {chat_history: json}))
        })
    )
}

struct MyAssistantApp {
    chat_history: serde_json::Value,
}

impl MyAssistantApp {
    fn display_chat(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            if let Some(messages) = self.chat_history["messages"].as_array() {
                for i in messages {
                    if i["role"] != "system" { 
                        ui.label(format!("{}: {}", i["role"], i["content"]));
                    } else {
                        continue;
                    }
                }
            }
        });
    }
    fn chat_input(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            if ui.button("send").clicked() {
                let buffer = request(&mut self.chat_history).expect("failed"); 
                println!("{}", buffer["choices"][0]["message"]["content"]);

                if let Some(Value::Array(messages)) = self.chat_history.get_mut("messages") {
                    // Create new message object from buffer
                    let new_message = json!({
                        "role": buffer["choices"][0]["message"]["role"],
                        "content": buffer["choices"][0]["message"]["content"]
                    });

                    // Append new message
                    messages.push(new_message);
                    ctx.request_repaint();
                }
                println!("{}", self.chat_history);
            }
            ui.label("placeholder");
        });
    }
}

impl eframe::App for MyAssistantApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui: &mut egui::Ui| {
            self.display_chat(ui); 
            self.chat_input(ctx, ui);
        });
    }
}
