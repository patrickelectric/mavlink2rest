use wasm::App;

use tokio_with_wasm::alias as tokio;
use ewebsock::connect;
use url::Url;
use std::sync::mpsc;

// #[tokio::main(flavor = "current_thread")]
fn main() {
    /*
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
    */
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        connect_and_receive_messages(tx);
    });

    let app = App::new(rx);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    println!("What");
    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Box::new(app)),
    );
    println!("Potato");
}

// todo: We need a better WS handler, maybe an abstraction over two different ws implementations
fn connect_and_receive_messages(mut tx: mpsc::Sender<String>) {
    let (mut sender, receiver) = {
        let url = Url::parse("ws://192.168.31.11:6040/ws/mavlink").unwrap().to_string();
        connect(url, ewebsock::Options::default()).expect("Can't connect")
    };
    println!("Waiting...");
    loop {
        while let Some(message) = receiver.try_recv() {
            // println!("message: {message:#?}");
            if let ewebsock::WsEvent::Message(message) = message {
                if let ewebsock::WsMessage::Text(message) = message {
                    if let Err(error) = tx.send(message) {
                        println!("{error:#?}");
                    }
                }
            }
        }
    }
}