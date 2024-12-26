mod config;
mod ipc;
mod commands;

use config::{Config, ensure_whisper_model};
use ipc::IpcServer;
use std::process::Command;

const SOCKET_PATH: &str = "/tmp/tuxtalk.sock";

fn main() {
    println!("Loading configuration...");
    let config = Config::load().expect("Failed to load configuration");

    let assistant_name = &config.default.prefix;
    println!("{}: Configuration loaded successfully.", assistant_name);

    println!("{}: Checking Whisper model...", assistant_name);
    ensure_whisper_model(&config.whisper.model)
        .expect("Failed to download or locate Whisper model");

    if config.daemon.audio_capture_enabled {
        println!(
            "{}: Audio capture is enabled. Using device: {}",
            assistant_name, config.daemon.audio_device
        );
        start_audio_capture(&config.daemon.audio_device, assistant_name);
    }

    println!("{}: Initializing IPC server...", assistant_name);
    let ipc_server = IpcServer::new(SOCKET_PATH).expect("Failed to initialize IPC server");

    println!("{}: Daemon started. Listening at {}", assistant_name, SOCKET_PATH);

    ipc_server
        .start(|message| {
            println!("{}: Received command: {}", assistant_name, message);
            commands::execute(
                &message,
                &config.shell.r#type,
                &config.actions,
                &config.commands,
            )
        })
        .expect("Error in IPC server loop");
}

fn start_audio_capture(audio_device: &str, assistant_name: &str) {
    println!("{}: Starting audio capture on device: {}", assistant_name, audio_device);

    match Command::new("parec")
        .arg("--device")
        .arg(audio_device)
        .arg("/tmp/audio_capture.raw")
        .spawn()
    {
        Ok(_) => println!("{}: Audio capture started successfully.", assistant_name),
        Err(e) => eprintln!("{}: Failed to start audio capture: {:?}", assistant_name, e),
    }
}

