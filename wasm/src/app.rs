use chrono::prelude::*;
use humantime::format_duration;

use tokio_with_wasm::alias as tokio;
// use tokio::runtime::Runtime;
use std::sync::mpsc;
// use tokio::runtime::Runtime;
// use tokio::sync::mpsc;
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

pub struct App {
    receiver: mpsc::Receiver<String>,
    vehicles: BTreeMap<u8, BTreeMap<u8, BTreeMap<String, MessageInfo>>>,
}

impl App {
    pub fn new(rx: mpsc::Receiver<String>) -> Self {
        /*
        std::thread::spawn(move || {
            //TODO: Use multthread runtime if running in desktop
            let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
            rt.block_on(async move {
                println!("Connect!");
                connect_and_receive_messages(tx).await;
                println!("Done!");
            });
        });
         */
        Self {
            receiver: rx,
            vehicles: Default::default(),
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