use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{Sender, channel};
use log::{error, info};
use std::time::Duration;

pub struct AudioCapture {
    _stream: cpal::Stream,
}

impl AudioCapture {
    pub fn new(sample_rate: u32, chunk_size: usize) -> (Self, std::sync::mpsc::Receiver<Vec<i16>>) {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .expect("Nenhum dispositivo de entrada disponível");

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let (tx, rx) = channel::<Vec<i16>>();

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let mut buffer = Vec::with_capacity(chunk_size);
                    for &sample in data.iter().take(chunk_size) {
                        buffer.push(sample);
                        if buffer.len() == chunk_size {
                            if tx.send(buffer.clone()).is_err() {
                                error!("Falha ao enviar chunk de áudio");
                            }
                            buffer.clear();
                        }
                    }
                },
                move |err| {
                    error!("Erro na captura de áudio: {:?}", err);
                },
                Some(Duration::from_millis(100)),
            )
            .expect("Falha ao construir o stream de entrada");

        stream.play().expect("Falha ao iniciar a captura de áudio");
        info!("Captura de áudio iniciada.");

        (
            Self { _stream: stream },
            rx,
        )
    }
}