use notify::{Watcher, RecursiveMode, Result as NotifyResult, Config as NotifyConfig, Event, RecommendedWatcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::process::Command;
use std::path::PathBuf;
use std::fs;
use std::io;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub default: DefaultConfig,
    pub daemon: DaemonConfig,
    pub shell: ShellConfig,
    pub whisper: WhisperConfig,
    pub actions: HashMap<String, String>,
    pub commands: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DefaultConfig {
    pub prefix: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DaemonConfig {
    pub audio_capture_enabled: bool,
    pub audio_device: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WhisperConfig {
    pub model: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ShellConfig {
    pub r#type: String,
}

impl Config {
    pub fn load() -> io::Result<Config> {
        let config_path = get_config_path();

        if !config_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Configuration file not found at {:?}. Ensure the application is installed correctly.",
                    config_path
                ),
            ));
        }

        let content = fs::read_to_string(&config_path)
            .expect("Failed to read the configuration file");
        let config: Config = toml::from_str(&content)
            .expect("Invalid configuration format. Check your config.toml file.");
        Ok(config)
    }
}

fn get_config_path() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("tuxtalk").join("config.toml")
    } else {
        PathBuf::from("/etc/tuxtalk/config.toml")
    }
}

pub fn get_model_path(model_name: &str) -> PathBuf {
    get_model_dir().join(format!("ggml-{}.bin", model_name))
}

pub fn ensure_whisper_model(model_name: &str) -> io::Result<()> {
    let model_dir = get_model_dir();
    let model_path = model_dir.join(format!("ggml-{}.bin", model_name));

    if model_path.exists() {
        println!("Whisper model '{}' already exists at {:?}", model_name, model_path);
        return Ok(());
    }

    println!("Downloading Whisper model '{}'...", model_name);

    let status = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "curl -L -o {} https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
            model_path.display(),
            model_name
        ))
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to download Whisper model",
        ));
    }

    println!("Whisper model '{}' downloaded successfully.", model_name);
    Ok(())
}

fn get_model_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .expect("Failed to locate the configuration directory")
        .join("tuxtalk")
        .join("models");

    if !dir.exists() {
        std::fs::create_dir_all(&dir).expect("Failed to create models directory");
    }

    dir
}

pub fn watch_config<F: Fn() + Send + 'static>(reload_callback: F) {
    let config_path = get_config_path();
    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = match RecommendedWatcher::new(
        move |res: NotifyResult<Event>| {
            match res {
                Ok(event) => {
                    tx.send(event).unwrap();
                }
                Err(e) => eprintln!("Watch error: {:?}", e),
            }
        },
        NotifyConfig::default(),
    ) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to create watcher: {:?}", e);
            return;
        }
    };

    watcher
        .watch(&config_path, RecursiveMode::NonRecursive)
        .expect("Failed to watch configuration file");

    std::thread::spawn(move || {
        for _event in rx {
            reload_callback();
        }
    });
}