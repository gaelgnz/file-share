use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    thread,
};

use eframe::egui;
use egui_file_dialog::FileDialog;
use local_ip_address::local_ip;
use egui_notify::{Toasts};
use std::sync::{Arc, Mutex};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "File Share",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
}

#[derive(Default)]
struct MyApp {
    desired_ip: String,
    file_dialog: FileDialog,
    picked_file: Option<PathBuf>,
    toasts: Toasts,
}

impl MyApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let toasts: Arc<Mutex<Toasts>> = Arc::new(Mutex::new(Toasts::default()));
        let toast_sender: Arc<Mutex<Toasts>> = Arc::clone(&toasts);

        thread::spawn(move || {
            let listener: TcpListener = match TcpListener::bind("0.0.0.0:8080") {
                Ok(t) => t,
                Err(e) => {
                    toast_sender.lock().unwrap().error(format!("Failed to listen for files: {:?} Please restart app", e));
                    panic!()
                }
            };

            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        let mut buffer = Vec::new();
                        if let Err(e) = stream.read_to_end(&mut buffer) {
                            eprintln!("Error reading stream: {}", e);
                            if let Ok(mut toasts) = toast_sender.lock() {
                                toasts.error(format!("Error reading stream: {}", e));
                            }
                            continue;
                        }

                        if let Err(e) = fs::write("received_file", &buffer) {
                            eprintln!("Error writing file: {}", e);
                            if let Ok(mut toasts) = toast_sender.lock() {
                                toasts.error(format!("Error saving file: {}", e));
                            }
                        } else {
                            if let Ok(mut toasts) = toast_sender.lock() {
                                toasts.success("✅ File received as 'received_file'");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Connection failed: {}", e);
                        if let Ok(mut toasts) = toast_sender.lock() {
                            toasts.error(format!("Connection failed: {}", e));
                        }
                    }
                }
            }
        });

        Self {
            toasts: Arc::try_unwrap(toasts).unwrap_or_default().into_inner().unwrap_or_default(),
            ..Default::default()
        }
        
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Show local IP
            match local_ip() {
                Ok(ip) => {
                    ui.label(format!("Your IP: {}", ip));
                }
                Err(_) => {
                    ui.label("Could not detect local IP.");
                }
            }

            // IP input
            ui.text_edit_singleline(&mut self.desired_ip);

            // File picker
            if ui.button("Pick file").clicked() {
                self.file_dialog.pick_file();
            }

            self.file_dialog.update(ctx);
            if let Some(path) = self.file_dialog.take_picked() {
                self.picked_file = Some(path.to_path_buf());
            }

            // Show picked file
            if let Some(path) = &self.picked_file {
                ui.label(format!("Picked file: {}", path.display()));
            }

            // Send button
            if self.picked_file.is_some() && !self.desired_ip.trim().is_empty() {
                if ui.button(format!("Send file to {}", self.desired_ip)).clicked() {
                    let path = self.picked_file.as_ref().unwrap();
                    let data = match fs::read(path) {
                        Ok(d) => d,
                        Err(e) => {
                            eprintln!("Failed to read file: {}", e);
                            return;
                        }
                    };

                    // Connect and send
                    let ip = format!("{}:8080", self.desired_ip.trim());
                    match TcpStream::connect(ip) {
                        Ok(mut stream) => {
                            match stream.write_all(&data) {
                                Ok(_) => { self.toasts.success("✅ File sent!"); }
                                Err(e) => { self.toasts.error(format!("❌ Failed to send: {}", e)); }
                            };
                        }
                        Err(e) => { self.toasts.error(format!("❌ Failed to connect: {}", e)); }
                    }

                }
            }
            self.toasts.show(ctx);

        });
    }
}
