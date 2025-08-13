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
use std::time::{Instant, Duration};

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
        Box::new(|cc: &eframe::CreationContext<'_>| {
            // Load and setup emoji font here:
            let mut fonts = egui::FontDefinitions::default();

            // Add NotoColorEmoji.ttf font (adjust path as needed)
            fonts.font_data.insert(
                "NotoEmoji".to_owned(),
                egui::FontData::from_static(include_bytes!("../NotoColorEmoji.ttf")).into(),
            );

            // Insert emoji font as highest priority fallback for proportional family
            fonts.families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("NotoEmoji".to_owned());

            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(MyAssistantApp {
                select_file: SelectFile::default(),
                state: AppState::ChooseFile,
                chat_history: json, 
                user_input: String::new(),
                tx, rx,
                is_thinking: false,
                commonmark_cache: CommonMarkCache::default(),
                child_process: None,
                animated_assistant_msg: None,
                can_send: true,
            }))
        })
    );

    result
}

struct AnimatedMessage {
    full_text: String,
    visible_text: String,
    index: usize,
    last_update: Instant,
    finished: bool
}

impl AnimatedMessage {
    fn new(full_text: String) -> Self {
        Self {
            full_text,
            visible_text: String::new(),
            index: 0,
            last_update: Instant::now(),
            finished: false,
        }
    }

    fn update(&mut self) {
        if self.finished {
            return;
        }

        let now = Instant::now();
        if now.duration_since(self.last_update) > Duration::from_millis(20) {
            self.index += 1;

            // Use char indexing safely:
            if self.index >= self.full_text.chars().count() {
                self.visible_text = self.full_text.clone();
                self.finished = true;
            } else {
                self.visible_text = self.full_text.chars().take(self.index).collect();
            }

            self.last_update = now;
        }
    }
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
    animated_assistant_msg: Option<AnimatedMessage>,
    can_send: bool,

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
            // 1. Show chat history
            if let Some(messages) = self.chat_history["messages"].as_array() {
                for msg in messages {
                    if msg["role"] == "system" {
                        continue;
                    }

                    let content = msg["content"].as_str().unwrap_or("<missing content>");
                    let role = msg["role"].as_str().unwrap_or("unknown");

                    ui.with_layout(
                        if role == "user" {
                            egui::Layout::right_to_left(egui::Align::TOP)
                        } else {
                            egui::Layout::left_to_right(egui::Align::TOP)
                        },
                        |ui| {
                            ui.group(|ui| {
                                if role == "user" {
                                    ui.label(content);
                                } else {
                                    CommonMarkViewer::new()
                                        .show(ui, &mut self.commonmark_cache, content);
                                }
                            });
                        },
                    );
                }
            }

            // 2. Show typing animation (if exists)
            if let Some(anim) = &mut self.animated_assistant_msg {
                anim.update();

                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.group(|ui| {
                        CommonMarkViewer::new()
                            .show(ui, &mut self.commonmark_cache, &anim.visible_text);
                    });
                });


                if anim.finished {
                    // Move to permanent chat history
                    if let Some(Value::Array(messages)) = self.chat_history.get_mut("messages") {
                        let res_message = json!({
                            "role": "assistant",
                            "content": anim.full_text
                        });
                        messages.push(res_message);
                    }
                    self.animated_assistant_msg = None;
                    self.can_send = true;
                }
            } else if self.is_thinking {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.group(|ui| {
                        ui.label("Thinking...");
                    });
                });
            }
        });
    }
    fn chat_input(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
            ui.add(egui::TextEdit::multiline(&mut self.user_input));
            if self.can_send {
                if ui.button("send").clicked() {
                    self.is_thinking = true;
                    self.can_send = false;

                    // Add the new user message to chat history
                    if let Some(Value::Array(messages)) = self.chat_history.get_mut("messages") {
                        let new_message = json!({
                            "role": "user",
                            "content": self.user_input
                        });
                        self.user_input.clear();
                        messages.push(new_message);
                    }

                    // Prepare the messages to send: first (system) + last 10
                    let messages_fallback = Vec::new();
                    let all_messages = self.chat_history["messages"]
                        .as_array()
                        .unwrap_or(&messages_fallback);

                    let mut selected_messages = Vec::new();

                    // Always include the system prompt if present
                    if let Some(first_msg) = all_messages.first() {
                        selected_messages.push(first_msg.clone());
                    }

                    // Take the last 10 messages (after the first)
                    let last_10 = all_messages
                        .iter()
                        .skip(1) // skip system message
                        .rev()
                        .take(10)
                        .cloned()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>();

                    selected_messages.extend(last_10);

                    let mut chat_history_clone = json!({ "messages": selected_messages });

                    let tx = self.tx.clone(); // safe because set in main()

                    // Background request
                    std::thread::spawn(move || {
                        let response = request(&mut chat_history_clone);
                        if let Ok(buffer) = response {
                            let _ = tx.send(buffer); // send back to main thread
                        }
                    });
                }
            } else {
                if ui.button("skip").clicked() {
                    if let Some(anim) = &mut self.animated_assistant_msg {
                        anim.finished = true;
                    }
                }
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

                    match buffer["choices"][0]["message"]["content"].as_str() {
                        Some(full_text) => {
                            //println!("Received assistant response: {}", full_text);
                            self.animated_assistant_msg = Some(AnimatedMessage::new(full_text.to_string()));
                        }
                        None => {
                            println!("ERROR: Invalid response from model:\n{:#?}", buffer);
                        }
                    }
                }
            }
        }
        ctx.request_repaint();
    }
}
