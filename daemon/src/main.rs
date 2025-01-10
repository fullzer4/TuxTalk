mod commands;
mod config;
mod ipc;
mod whisper;
mod audio_capture;

use config::{Config, ensure_whisper_model};
use audio_capture::AudioCapture;
use whisper::WhisperModel;
use log::{info, error};
use tokio::sync::Mutex;
use std::sync::Arc;
use ipc::IpcServer;
use tokio::task;
use env_logger;

const SOCKET_PATH: &str = "/tmp/tuxtalk.sock";

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Iniciando TuxTalk...");

    let config = Arc::new(Mutex::new(
        Config::load().expect("Falha ao carregar a configuração")
    ));

    config::watch_config({
        let config = Arc::clone(&config);
        move || {
            let config = Arc::clone(&config);
            tokio::spawn(async move {
                match Config::load() {
                    Ok(new_config) => {
                        let mut cfg = config.lock().await;
                        *cfg = new_config;
                        info!("Configuração recarregada com sucesso.");
                    }
                    Err(e) => error!("Falha ao recarregar a configuração: {:?}", e),
                }
            });
        }
    });

    {
        let cfg = config.lock().await;
        let assistant_name = &cfg.default.prefix;
        info!("{}: Configuração carregada com sucesso.", assistant_name);

        info!("{}: Verificando modelo Whisper...", assistant_name);
        ensure_whisper_model(&cfg.whisper.model)
            .expect("Falha ao baixar ou localizar o modelo Whisper");

        if cfg.daemon.audio_capture_enabled {
            info!(
                "{}: Captura de áudio está habilitada. Usando dispositivo: {}",
                assistant_name, cfg.daemon.audio_device
            );
        }
    }

    let assistant_name = {
        let cfg = config.lock().await;
        cfg.default.prefix.clone()
    };

    info!("{}: Inicializando servidor IPC...", assistant_name);
    let ipc_server = IpcServer::new(SOCKET_PATH)
        .await
        .expect("Falha ao inicializar o servidor IPC");

    info!("{}: Daemon iniciado. Escutando em {}", assistant_name, SOCKET_PATH);

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
            .expect("Erro no loop do servidor IPC");
    });

    {
        let cfg = config.lock().await;
        if cfg.daemon.audio_capture_enabled {
            let model_path = cfg.whisper.model.clone();
            let whisper_model = WhisperModel::new(&model_path).await
                .expect("Falha ao inicializar o modelo Whisper");

            let sample_rate = 16000;
            let chunk_size = 16000;

            let (audio_capture, rx) = AudioCapture::new(sample_rate, chunk_size);

            let whisper_model = Arc::new(whisper_model);

            let whisper_handle = tokio::spawn(process_audio_stream(rx, Arc::clone(&whisper_model)));

            info!("{}: Captura de áudio e transcrição em tempo real iniciadas.", assistant_name);
        }
    }

    let _ = tokio::join!(ipc_handle);
}

async fn process_audio_stream(rx: std::sync::mpsc::Receiver<Vec<i16>>, whisper_model: Arc<WhisperModel>) {
    loop {
        match rx.recv() {
            Ok(chunk) => {
                let whisper_model: Arc<WhisperModel> = Arc::clone(&whisper_model);
                tokio::spawn(async move {
                    match whisper_model.process_audio_chunk(&chunk).await {
                        Ok(transcription) => {
                            if !transcription.trim().is_empty() {
                                info!("Transcrição: {}", transcription.trim());
                            }
                        }
                        Err(e) => {
                            error!("Erro na transcrição: {:?}", e);
                        }
                    }
                });
            }
            Err(e) => {
                error!("Erro recebendo chunk de áudio: {:?}", e);
                break;
            }
        }
    }
}