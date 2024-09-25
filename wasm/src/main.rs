use chrono::prelude::*;
use humantime::format_duration;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use ewebsock::connect;
use url::Url;
use std::collections::BTreeMap;

struct MessageInfo {
    value: serde_json::Value,
    previous_value: Option<serde_json::Value>,
    last_sample_time: DateTime<chrono::Utc>,
}

impl MessageInfo {
    pub fn update(&mut self, value: serde_json::Value) {

    }
}

struct App {
    receiver: mpsc::Receiver<String>,
    vehicles: BTreeMap<u8, BTreeMap<u8, BTreeMap<String, MessageInfo>>>,
}

impl Default for App {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel(32);
        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async move {
                println!("Connect!");
                connect_and_receive_messages(tx).await;
                println!("Done!");
            });
        });
        Self {
            receiver: rx,
            vehicles: Default::default(),
        }
    }
}

// todo: We need a better WS handler, maybe an abstraction over two different ws implementations
async fn connect_and_receive_messages(mut tx: mpsc::Sender<String>) {
    let (mut sender, receiver) = {
        let url = Url::parse("ws://192.168.31.11:6040/ws/mavlink").unwrap().to_string();
        connect(url, ewebsock::Options::default()).expect("Can't connect")
    };

    loop {
        while let Some(message) = receiver.try_recv() {
            if let ewebsock::WsEvent::Message(message) = message {
                if let ewebsock::WsMessage::Text(message) = message {
                    tx.send(message)
                        .await
                        .unwrap();
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            print!(".");
            ui.label("Potato");
            while let Ok(message) = self.receiver.try_recv() {
                println!("{message:#?}");
                let Ok(message) = serde_json::from_str::<serde_json::Value>(&message) else {
                    continue;
                };
                let system_id: u8 = message["header"]["system_id"].as_u64().unwrap() as u8;
                let component_id: u8 = message["header"]["component_id"].as_u64().unwrap() as u8;
                let message_name: String = message["message"]["type"].to_string().trim_matches(['"']).to_string();
                if !self.vehicles.contains_key(&system_id) {
                    self.vehicles.insert(system_id, Default::default());
                }
                if !self.vehicles[&system_id].contains_key(&component_id) {
                    self.vehicles.get_mut(&system_id).unwrap().insert(component_id, Default::default());
                }
                let previous_message = match self.vehicles.get(&system_id).unwrap().get(&component_id).unwrap().get(&message_name) {
                    Some(previous_message) => Some(previous_message.value.clone()),
                    None => None,
                };

                self.vehicles.get_mut(&system_id).unwrap().get_mut(&component_id).unwrap().insert(message_name, MessageInfo {
                    value: message,
                    previous_value: previous_message,
                    last_sample_time: Utc::now(),
                });
            }

            for (system_id, components) in &self.vehicles {
                egui::CollapsingHeader::new(format!("Vehicle {system_id}")).default_open(true).show(ui, |ui| {
                    for (component_id, messages) in components {
                        egui::CollapsingHeader::new(format!("Component {component_id}")).default_open(true).show(ui, |ui| {
                            for (name, message) in messages {
                                ui.collapsing(name, |ui| {
                                    ui.label(serde_json::to_string_pretty(&message.value).unwrap());
                                    ui.label(format_duration((Utc::now() - message.last_sample_time).to_std().unwrap()).to_string() + " Ago");
                                });
                            }
                        });
                    }
                });
            }
            ctx.request_repaint();
        });
    }
}

fn main() -> eframe::Result<()> {
    let app = App::default();
    /*
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
    */

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]), /*
                                                  .with_icon(
                                                      // NOTE: Adding an icon is optional
                                                      eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                                                          .expect("Failed to load icon"),
                                                  ) */
        ..Default::default()
    };
    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Box::new(app)),
    )
}
