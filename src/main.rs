use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use egui_file::FileDialog;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf}
};
use std::fs::{File, OpenOptions};
use std::io::BufWriter;
use serde_json::{Value, json};
use std::process::{Child, Command};
use std::sync::mpsc::{self, Sender, Receiver};

mod request;
use crate::request::request;

fn main() -> eframe::Result<()>  {
    let file = File::open("chat_history.json").expect("file not found!");
    let json: serde_json::Value = serde_json::from_reader(file).expect("file should be proper JSON");

    let options = eframe::NativeOptions{
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 800.0]),
        ..Default::default()
    };

    let (tx, rx) = mpsc::channel();
    
    let result = eframe::run_native(
        "ASSistant",
        options,
        Box::new(|_cc: &eframe::CreationContext<'_>| {
            Ok(Box::new(MyAssistantApp {
                select_file: SelectFile::default(),
                state: AppState::ChooseFile,
                chat_history: json, 
                user_input: String::new(),
                tx, rx,
                is_thinking: false,
                commonmark_cache: CommonMarkCache::default(),
                child_process: None
            }))
        })
    );

    result
}

enum AppState {
    ChooseFile,
    Running,
}

#[derive(Default)]
struct SelectFile {
    opened_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,
}

struct MyAssistantApp {
    select_file: SelectFile,
    state: AppState,
    chat_history: serde_json::Value,
    user_input: String,
    tx: Sender<Value>,
    rx: Receiver<Value>,
    is_thinking: bool,
    commonmark_cache: CommonMarkCache,
    child_process: Option<Child>,
}

impl Drop for MyAssistantApp {
    fn drop(&mut self) {
        if let Some(mut child) = self.child_process.take() {
            // Try to kill it gracefully
            if let Err(e) = child.kill() {
                eprintln!("Failed to kill child process: {}", e);
            } else {
                let _ = child.wait(); // Wait for it to exit
            }
        }
        println!("Successfully stop llama-server");

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
        egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
            if let Some(messages) = self.chat_history["messages"].as_array() {
                for i in messages {
                    if i["role"] != "system" { 
                        if i["role"] != "user" {
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.group(|ui| {
                                    if let Some(content) = i["content"].as_str() {
                                        CommonMarkViewer::new().show(ui, &mut self.commonmark_cache, content);
                                    } else {
                                        ui.label("<missing content>");
                                    }
                                    //CommonMarkViewer::new().show(ui, &mut cache, i["content"].as_str().unwrap());
                                });
                            });
                        } else {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                ui.group(|ui| {
                                    if let Some(content) = i["content"].as_str() {
                                        ui.label(content);
                                    } else {
                                        ui.label("<missing content>");
                                    }
                                    //ui.label(format!("{}", i["content"].as_str().unwrap()));
                                });
                            });
                        }
                    } else {
                        continue;
                    }
                }
                if self.is_thinking {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                        ui.group(|ui| {
                            ui.label("Thinking...");
                        });
                    });
                }
            }
        });
    }
    fn chat_input(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
            ui.add(egui::TextEdit::multiline(&mut self.user_input));
            if ui.button("send").clicked() {
                self.is_thinking = true;
                if let Some(Value::Array(messages)) = self.chat_history.get_mut("messages") {
                    let new_message = json!({
                        "role": "user",
                        "content": self.user_input
                    });
                    self.user_input.clear();
                    messages.push(new_message);
                }

                let mut chat_history_clone = self.chat_history.clone();
                let tx = self.tx.clone(); // safe because set in main()

                std::thread::spawn(move || {
                    let response = request(&mut chat_history_clone);
                    if let Ok(buffer) = response {
                        let _ = tx.send(buffer); // send the buffer back to main thread
                    }
                });
            }
        });
    }
}

impl eframe::App for MyAssistantApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.state {
            AppState::ChooseFile => {
                egui::CentralPanel::default().show(ctx, |ui: &mut egui::Ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(300.0);
                        ui.label("Import .gguf");
                        ui.add_space(10.0);
                        if ui.button("import").clicked() {
                            let filter = Box::new({
                                let ext = Some(OsStr::new("gguf"));
                                move |path: &Path| -> bool {path.extension() == ext}
                            });
                            let mut dialog = FileDialog::open_file(self.select_file.opened_file.clone()).show_files_filter(filter);
                            dialog.open();
                            self.select_file.open_file_dialog = Some(dialog);
                        }
                        if let Some(dialog) = &mut self.select_file.open_file_dialog {
                            if dialog.show(ctx).selected() {
                                if let Some(file) = dialog.path() {
                                    self.select_file.opened_file = Some(file.to_path_buf());

                                    let model_path = self.select_file.opened_file.clone().expect("some").display().to_string();
                                    let child = Command::new("llama-server")
                                        .arg("-m")
                                        .arg(&model_path)
                                        .arg("--port")
                                        .arg("8080")
                                        .spawn();
                                    match child {
                                        Ok(child) => {
                                            self.child_process = Some(child);
                                            self.state = AppState::Running;
                                        },
                                        Err(e) => {
                                            eprintln!("Failed to start llama-server: {}", e);
                                            // Decide: exit or continue without server
                                            std::process::exit(1);
                                        }
                                    };
                                }
                            }
                        }
                    });
                });
            },
            AppState::Running => {
                egui::TopBottomPanel::bottom("input_holder").show(ctx, |ui| {
                    self.chat_input(ui);
                });
                egui::CentralPanel::default().show(ctx, |ui: &mut egui::Ui| {
                    self.display_chat(ui); 
                });

                // Check for any new assistant response
                if let Ok(buffer) = self.rx.try_recv() {
                    self.is_thinking = false;
                    if let Some(Value::Array(messages)) = self.chat_history.get_mut("messages") {
                        let res_message = json!({
                            "role": buffer["choices"][0]["message"]["role"],
                            "content": buffer["choices"][0]["message"]["content"]
                        });
                        messages.push(res_message);
                    }
                }
            }
        }

        // Tell egui to repaint periodically to keep UI responsive
        //ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}
