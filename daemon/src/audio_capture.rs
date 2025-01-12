use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::sync::mpsc::{Sender, Receiver, channel};
use dasp_sample::conv::ToSample;
use std::time::Duration;
use log::{error, info};
use cpal::Sample;
use std::mem;

pub struct AudioCapture {
    _stream: cpal::Stream,
}

impl AudioCapture {
    pub fn new(_sample_rate: u32, chunk_size: usize) -> (Self, Receiver<Vec<i16>>) {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .expect("No input device available");

        let config = device.default_input_config().expect("Failed to get input configuration");
        info!("Input format: {:?}", config);

        let (tx, rx) = channel::<Vec<i16>>(100);

        let stream = match config.sample_format() {
            cpal::SampleFormat::I16 => Self::build_stream::<i16>(&device, &config.into(), tx, chunk_size),
            cpal::SampleFormat::U16 => Self::build_stream::<u16>(&device, &config.into(), tx, chunk_size),
            cpal::SampleFormat::F32 => Self::build_stream::<f32>(&device, &config.into(), tx, chunk_size),
            _ => {
                error!("Unsupported audio format");
                panic!("Unsupported audio format");
            }
        };

        stream.play().expect("Failed to start audio capture");
        info!("Audio capture started.");

        (
            Self { _stream: stream },
            rx,
        )
    }

    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        tx: Sender<Vec<i16>>,
        chunk_size: usize,
    ) -> cpal::Stream
    where
        T: cpal::Sample + cpal::SizedSample + ToSample<i16>,
    {
        device
            .build_input_stream(
                config,
                move |data: &[T], _: &cpal::InputCallbackInfo| {
                    let mut buffer: Vec<i16> = Vec::with_capacity(chunk_size);
                    for &sample in data.iter() {
                        let sample_i16 = sample.to_sample::<i16>();
                        buffer.push(sample_i16);
                        if buffer.len() == chunk_size {
                            if tx.blocking_send(mem::take(&mut buffer)).is_err() {
                                error!("Failed to send audio chunk");
                            }
                            buffer.reserve(chunk_size);
                        }
                    }

                    if !buffer.is_empty() {
                        if tx.blocking_send(mem::take(&mut buffer)).is_err() {
                            error!("Failed to send audio chunk");
                        }
                    }
                },
                move |err| {
                    error!("Audio capture error: {:?}", err);
                },
                Some(Duration::from_millis(100)),
            )
            .expect("Failed to build input stream")
    }    
}
