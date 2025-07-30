use eframe::egui;
use std::fs::{File, OpenOptions};
use std::io::BufWriter;
use serde_json::{Value, json};
use std::process::Command;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

mod request;
use crate::request::request;

fn main() -> eframe::Result<()>  {
    let file = File::open("chat_history.json").expect("file not found!");
    let json: serde_json::Value = serde_json::from_reader(file).expect("file should be proper JSON");

    let model_path: String = String::from("$HOME/llm/gemma-3-4b-it-q4_k_m.gguf");
    let mut child = Command::new("sh").arg("-c").arg(format!("llama-server -m {} --port 8080", model_path)).spawn().expect("Might be invalid path!");

    let options = eframe::NativeOptions{
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 800.0]),
        ..Default::default()
    };
    let result = eframe::run_native(
        "ASSistant",
        options,
        Box::new(|_cc: &eframe::CreationContext<'_>| {
            Ok(Box::new(MyAssistantApp {chat_history: json, user_input: String::new()}))
        })
    );

    let _ = child.kill(); // stop the llama-server
    result
}

struct MyAssistantApp {
    chat_history: serde_json::Value,
    user_input: String,
}

impl Drop for MyAssistantApp {
    fn drop(&mut self) {
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open("chat_history.json")
            .expect("Failed to open file for writing!");
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self.chat_history).expect("Failed to write JSON");
        println!("Successfully save to JSON!");
    }
}

impl MyAssistantApp {
    fn display_chat(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(messages) = self.chat_history["messages"].as_array() {
                let mut cache = CommonMarkCache::default();
                for i in messages {
                    if i["role"] != "system" { 
                        if i["role"] != "user" {
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.group(|ui| {
                                    CommonMarkViewer::new().show(ui, &mut cache, i["content"].as_str().unwrap());
                                });
                            });
                        } else {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                ui.group(|ui| {
                                    ui.label(format!("{}", i["content"].as_str().unwrap()));
                                });
                            });
                        }
                    } else {
                        continue;
                    }
                }
            }
        });
    }
    fn chat_input(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::multiline(&mut self.user_input));
                if ui.button("send").clicked() {
                    if let Some(Value::Array(messages)) = self.chat_history.get_mut("messages") {
                        let new_message = json!({
                            "role": "user",
                            "content": self.user_input
                        });
                        self.user_input = String::from("");
                        messages.push(new_message);
                    }
                    let buffer = request(&mut self.chat_history).expect("Request failed..."); 

                    if let Some(Value::Array(messages)) = self.chat_history.get_mut("messages") {
                        // Create new message object from buffer
                        let res_message = json!({
                            "role": buffer["choices"][0]["message"]["role"],
                            "content": buffer["choices"][0]["message"]["content"]
                        });

                        // Append new message
                        messages.push(res_message);
                    }
                }
            });
        });
    }
}

impl eframe::App for MyAssistantApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("input_holder").show(ctx, |ui| {
            self.chat_input(ui);
        });
        egui::CentralPanel::default().show(ctx, |ui: &mut egui::Ui| {
            self.display_chat(ui); 
        });
    }
}
