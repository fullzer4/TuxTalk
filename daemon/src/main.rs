mod audio_capture;
mod commands;
mod whisper;
mod config;
mod ipc;

use config::{Config, ensure_whisper_model, get_model_path};
use tokio::sync::mpsc::Receiver;
use audio_capture::AudioCapture;
use whisper::WhisperModel;
use log::{info, error};
use tokio::sync::Mutex;
use std::sync::Arc;
use ipc::IpcServer;
use env_logger;

const SOCKET_PATH: &str = "/tmp/tuxtalk.sock";

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Starting TuxTalk...");
    println!("Starting TuxTalk...");

    let config = match Config::load() {
        Ok(cfg) => Arc::new(Mutex::new(cfg)),
        Err(e) => {
            error!("Failed to load configuration: {:?}", e);
            println!("Failed to load configuration: {:?}", e);
            return;
        }
    };
    info!("Configuration loaded successfully.");
    println!("Configuration loaded successfully.");

    {
        let cfg = config.lock().await;
        let assistant_name = &cfg.default.prefix;
        info!("{}: Configuration loaded successfully.", assistant_name);
        println!("{}: Configuration loaded successfully.", assistant_name);

        info!("{}: Checking Whisper model...", assistant_name);
        println!("{}: Checking Whisper model...", assistant_name);
        if let Err(e) = ensure_whisper_model(&cfg.whisper.model) {
            error!("Failed to download or locate Whisper model: {:?}", e);
            println!("Failed to download or locate Whisper model: {:?}", e);
            return;
        }
        info!("{}: Whisper model verified successfully.", assistant_name);
        println!("{}: Whisper model verified successfully.", assistant_name);

        if cfg.daemon.audio_capture_enabled {
            info!(
                "{}: Audio capture is enabled. Using device: {}",
                assistant_name, cfg.daemon.audio_device
            );
            println!(
                "{}: Audio capture is enabled. Using device: {}",
                assistant_name, cfg.daemon.audio_device
            );
        } else {
            info!("{}: Audio capture is disabled.", assistant_name);
            println!("{}: Audio capture is disabled.", assistant_name);
        }
    }

    config::watch_config({
        let config = Arc::clone(&config);
        move || {
            let config = Arc::clone(&config);
            tokio::spawn(async move {
                match Config::load() {
                    Ok(new_config) => {
                        let mut cfg = config.lock().await;
                        *cfg = new_config;
                        info!("Configuration reloaded successfully.");
                        println!("Configuration reloaded successfully.");
                    }
                    Err(e) => {
                        error!("Failed to reload configuration: {:?}", e);
                        println!("Failed to reload configuration: {:?}", e);
                    },
                }
            });
        }
    });
    info!("Configuration watcher configured.");
    println!("Configuration watcher configured.");

    let assistant_name = {
        let cfg = config.lock().await;
        cfg.default.prefix.clone()
    };

    info!("{}: Initializing IPC server...", assistant_name);
    println!("{}: Initializing IPC server...", assistant_name);
    let ipc_server = match IpcServer::new(SOCKET_PATH).await {
        Ok(server) => server,
        Err(e) => {
            error!("Failed to initialize IPC server: {:?}", e);
            println!("Failed to initialize IPC server: {:?}", e);
            return;
        }
    };
    info!("{}: IPC server initialized.", assistant_name);
    println!("{}: IPC server initialized.", assistant_name);

    info!("{}: Daemon started. Listening on {}", assistant_name, SOCKET_PATH);
    println!("{}: Daemon started. Listening on {}", assistant_name, SOCKET_PATH);

    let config_clone = Arc::clone(&config);
    let ipc_handle = tokio::spawn(async move {
        ipc_server
            .start(move |message| {
                let config_clone = Arc::clone(&config_clone);
                async move {
                    let cfg = config_clone.lock().await;
                    commands::execute(
                        &message,
                        &cfg.shell.r#type,
                        &cfg.actions,
                        &cfg.commands,
                    )
                }
            })
            .await
            .expect("Error in IPC server loop");
    });
    info!("{}: IPC server running.", assistant_name);
    println!("{}: IPC server running.", assistant_name);

    let (audio_capture, whisper_model_option) = {
        let cfg = config.lock().await;
        if cfg.daemon.audio_capture_enabled {
            info!("{}: Initializing Whisper model...", assistant_name);
            println!("{}: Initializing Whisper model...", assistant_name);
            
            let model_path = get_model_path(&cfg.whisper.model);
            println!("{}: Path: {:?}", assistant_name, model_path);

            let whisper_model = match WhisperModel::new(model_path.to_str().unwrap()).await {
                Ok(model) => Arc::new(model),
                Err(e) => {
                    error!("Failed to initialize Whisper model: {:?}", e);
                    println!("Failed to initialize Whisper model: {:?}", e);
                    return;
                }
            };
            info!("{}: Whisper model initialized.", assistant_name);
            println!("{}: Whisper model initialized.", assistant_name);

            let sample_rate = 16000;
            let chunk_size = 16000;

            let (ac, rx) = AudioCapture::new(sample_rate, chunk_size);
            info!("{}: AudioCapture initialized.", assistant_name);
            println!("{}: AudioCapture initialized.", assistant_name);

            tokio::spawn(process_audio_stream(rx, Arc::clone(&whisper_model)));
            info!("{}: Real-time audio capture and transcription started.", assistant_name);
            println!("{}: Real-time audio capture and transcription started.", assistant_name);

            (Some(ac), Some(whisper_model))
        } else {
            (None, None)
        }
    };

    let mut audio_captures_store = Vec::new();
    if let Some(ac) = audio_capture {
        audio_captures_store.push(ac);
    }

    if let Err(e) = ipc_handle.await {
        error!("IPC server error: {:?}", e);
        println!("IPC server error: {:?}", e);
    }
}

async fn process_audio_stream(mut rx: Receiver<Vec<i16>>, whisper_model: Arc<WhisperModel>) {
    while let Some(chunk) = rx.recv().await {
        let whisper_model = Arc::clone(&whisper_model);
        tokio::spawn(async move {
            match whisper_model.process_audio_chunk(&chunk).await {
                Ok(transcription) => {
                    if !transcription.trim().is_empty() {
                        info!("Transcription: {}", transcription.trim());
                        println!("Transcription: {}", transcription.trim());
                    }
                }
                Err(e) => {
                    error!("Transcription error: {:?}", e);
                }
            }
        });
    }
}
