use std::sync::Arc;
use tokio::sync::Mutex;
use whisper_rs::{WhisperContext, FullParams, SamplingStrategy};
use log::error;

pub struct WhisperModel {
    ctx: Arc<Mutex<WhisperContext>>,
    default_params: FullParams<'static, 'static>,
}

impl WhisperModel {
    pub async fn new(model_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        let ctx = WhisperContext::new(model_path)?;

        Ok(Self {
            ctx: Arc::new(Mutex::new(ctx)),
            default_params: params,
        })
    }

    pub async fn process_audio_chunk(&self, samples: &[i16]) -> Result<String, Box<dyn std::error::Error>> {
        let ctx_guard = self.ctx.lock().await;
        let mut state = ctx_guard.create_state()?;
    
        let float_samples = whisper_rs::convert_integer_to_float_audio(samples);
    
        state.full(FullParams::new(SamplingStrategy::Greedy { best_of: 1 }), &float_samples)?;
    
        let num_segments = state.full_n_segments()?;
    
        let mut transcription = String::new();
    
        for i in 0..num_segments {
            let segment_text = state.full_get_segment_text(i)?;
            transcription.push_str(segment_text.as_str());
            transcription.push('\n');
        }
    
        Ok(transcription)
    }
}
