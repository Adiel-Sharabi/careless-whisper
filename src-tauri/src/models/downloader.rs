use futures_util::StreamExt;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub disk_size_mb: u32,
    pub ram_mb: u32,
    pub is_downloaded: bool,
}

const MODELS: &[(&str, u32, u32)] = &[
    ("tiny", 75, 390),
    ("base", 142, 500),
    ("small", 466, 1024),
    ("medium", 1500, 2600),
    ("large-v3", 3000, 5120),
];

pub fn models_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_default()
        .join("careless-whisper")
        .join("models")
}

pub fn model_path(name: &str) -> PathBuf {
    models_dir().join(format!("ggml-{}.bin", name))
}

pub fn list_models() -> Vec<ModelInfo> {
    MODELS
        .iter()
        .map(|(name, disk_mb, ram_mb)| ModelInfo {
            name: name.to_string(),
            disk_size_mb: *disk_mb,
            ram_mb: *ram_mb,
            is_downloaded: model_path(name).exists(),
        })
        .collect()
}

pub async fn download_model(app: AppHandle, name: String) -> Result<(), String> {
    let url = format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
        name
    );

    let dir = models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let part_path = dir.join(format!("ggml-{}.bin.part", name));
    let final_path = model_path(&name);

    let response = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Download failed: HTTP {}", response.status()));
    }

    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut file = tokio::fs::File::create(&part_path)
        .await
        .map_err(|e| e.to_string())?;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        let percent = if total > 0 {
            (downloaded * 100 / total) as u32
        } else {
            0
        };
        let _ = app.emit(
            "download-progress",
            serde_json::json!({ "model": name, "percent": percent }),
        );
    }

    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(&part_path, &final_path).map_err(|e| e.to_string())?;

    Ok(())
}

pub fn delete_model(name: &str) -> Result<(), String> {
    let path = model_path(name);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
