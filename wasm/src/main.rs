use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tungstenite::connect;
use url::Url;
use std::collections::BTreeMap;

struct App {
    receiver: mpsc::Receiver<String>,
    vehicles: BTreeMap<u8, BTreeMap<u8, BTreeMap<String, serde_json::Value>>>,
}

impl Default for App {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel(32);
        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async move {
                connect_and_receive_messages(tx).await;
            });
        });
        Self {
            receiver: rx,
            vehicles: Default::default(),
        }
    }
}

async fn connect_and_receive_messages(mut tx: mpsc::Sender<String>) {
    let (mut socket, _response) =
        connect(Url::parse("wss://192.168.31.63:6040/ws/mavlink").unwrap()).expect("Can't connect");

    while let Ok(message) = socket.read() {
        if message.is_text() {
            tx.send(message.to_text().unwrap().to_string())
                .await
                .unwrap();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.request_repaint();
            ui.label("Potato");
            while let Ok(message) = self.receiver.try_recv() {
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
                self.vehicles.get_mut(&system_id).unwrap().get_mut(&component_id).unwrap().insert(message_name, message);
            }

            for (system_id, components) in &self.vehicles {
                egui::CollapsingHeader::new(format!("Vehicle {system_id}")).default_open(true).show(ui, |ui| {
                    for (component_id, messages) in components {
                        egui::CollapsingHeader::new(format!("Component {component_id}")).default_open(true).show(ui, |ui| {
                            for (name, message) in messages {
                                ui.collapsing(name, |ui| {
                                    ui.label(serde_json::to_string_pretty(&message).unwrap());
                                });
                            }
                        });
                    }
                });
            }
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
