use crate::types::{Config, SessionMeta, Settings};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

fn home_dir() -> PathBuf {
    if let Ok(p) = std::env::var("CODEX_SESSION_HOME") {
        return PathBuf::from(p);
    }
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn config_dir() -> PathBuf {
    home_dir().join(".codex-sessions")
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.json")
}

pub fn load_config() -> Config {
    let path = config_file();
    if !path.exists() {
        return Config::default();
    }
    match fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn save_config(config: &Config) -> Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir).context("create config dir")?;
    let body = serde_json::to_string_pretty(config)?;
    fs::write(config_file(), body).context("write config")?;
    Ok(())
}

pub fn upsert_session_meta(session_id: &str, patch: SessionMeta) -> Result<()> {
    let mut cfg = load_config();
    let entry = cfg.sessions.entry(session_id.to_string()).or_default();
    if patch.name.is_some() {
        entry.name = patch.name;
    }
    if patch.description.is_some() {
        entry.description = patch.description;
    }
    if patch.auto_summary.is_some() {
        entry.auto_summary = patch.auto_summary;
    }
    if patch.storage_type.is_some() {
        entry.storage_type = patch.storage_type;
    }
    if patch.favorite.is_some() {
        entry.favorite = patch.favorite;
    }
    entry.updated_at = Some(chrono::Utc::now().to_rfc3339());
    save_config(&cfg)
}

pub fn delete_session_meta(session_id: &str) -> Result<()> {
    let mut cfg = load_config();
    cfg.sessions.remove(session_id);
    save_config(&cfg)
}

pub fn update_settings(patch: Settings) -> Result<()> {
    let mut cfg = load_config();
    if patch.locale.is_some() {
        cfg.settings.locale = patch.locale;
    }
    if patch.cloud_path.is_some() {
        cfg.settings.cloud_path = patch.cloud_path;
    }
    if patch.preferred_terminal.is_some() {
        cfg.settings.preferred_terminal = patch.preferred_terminal;
    }
    if patch.resume_flags.is_some() {
        cfg.settings.resume_flags = patch.resume_flags;
    }
    if patch.custom_terminal_program.is_some() {
        cfg.settings.custom_terminal_program = patch.custom_terminal_program;
    }
    if patch.custom_terminal_args.is_some() {
        cfg.settings.custom_terminal_args = patch.custom_terminal_args;
    }
    if patch.extra_project_dirs.is_some() {
        cfg.settings.extra_project_dirs = patch.extra_project_dirs;
    }
    if patch.wsl_auto_detect.is_some() {
        cfg.settings.wsl_auto_detect = patch.wsl_auto_detect;
    }
    if patch.excluded_scan_paths.is_some() {
        cfg.settings.excluded_scan_paths = patch.excluded_scan_paths;
    }
    save_config(&cfg)
}
