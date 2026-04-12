use std::path::PathBuf;use std::sync::atomic::{AtomicBool, Ordering};use std::sync::Arc;use tauri::{AppHandle, Emitter, Manager, State};use tauri_plugin_autostart::ManagerExt;use crate::config::settings::{OverlayPosition, Settings};use crate::models::downloader::{self, ModelInfo};use crate::output::paste::FocusTarget;use crate::AppState;fn position_overlay(app: &AppHandle, win: &tauri::WebviewWindow, position: &OverlayPosition) {    use tauri::LogicalPosition;    let monitor = win        .current_monitor()        .ok()        .flatten()        .or_else(|| app.primary_monitor().ok().flatten());    let monitor = match monitor {        Some(m) => m,        None => {            log::warn!("[overlay] no monitor found");            return;        }    };    let scale = monitor.scale_factor();    let screen_w = monitor.size().width as f64 / scale;    let screen_h = monitor.size().height as f64 / scale;    let win_width = 200.0;    let win_height = 44.0;    let margin = 16.0;
/// Format transcription output: if multiple sentences are detected,
/// convert to bullet points for better readability.
fn format_text_output(text: &str) -> String {
    let text = text.trim();
    let parts: Vec<&str> = text
        .split(". ")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() <= 1 {
        return text.to_string();
    }
    parts
        .iter()
        .map(|s| format!("\u{2022} {}", s))
        .collect::<Vec<_>>()
        .join("\n")
}
