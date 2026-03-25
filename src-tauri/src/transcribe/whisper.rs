use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub fn load_model(path: &Path) -> Result<WhisperContext, String> {
    let path_str = path.to_str().ok_or("Invalid model path")?;

    let file_size = std::fs::metadata(path)
        .map(|m| m.len())
        .unwrap_or(0);

    log::info!(
        "[whisper] loading model from {} ({:.1} MB)",
        path.display(),
        file_size as f64 / 1_048_576.0
    );

    WhisperContext::new_with_params(path_str, WhisperContextParameters::default()).map_err(|e| {
        format!(
            "Failed to load model: {:?}. Path: {}, size: {} bytes. \
             The file may be corrupted — try deleting and re-downloading it.",
            e,
            path.display(),
            file_size
        )
    })
}

pub fn transcribe(ctx: &WhisperContext, samples: &[f32], language: &str) -> Result<String, String> {
    let mut state = ctx.create_state().map_err(|e| format!("{:?}", e))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_single_segment(false);

    // "auto" → empty string triggers whisper.cpp auto-detect
    if language != "auto" && !language.is_empty() {
        params.set_language(Some(language));
    }

    state
        .full(params, samples)
        .map_err(|e| format!("Transcription failed: {:?}", e))?;

    let num_segments = state
        .full_n_segments()
        .map_err(|e| format!("{:?}", e))?;

    let mut text = String::new();
    for i in 0..num_segments {
        if let Ok(segment) = state.full_get_segment_text(i) {
            text.push_str(&segment);
        }
    }

    Ok(text.trim().to_string())
}
