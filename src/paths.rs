use std::fs;
use std::io;
use std::path::PathBuf;

const APP_DIR_NAME: &str = ".proxai";
const CONFIG_FILE_NAME: &str = "config.toml";
const CONFIG_EXAMPLE_FILE_NAME: &str = "config.example.toml";
const LOGS_DIR_NAME: &str = "logs";
const CAPTURES_DIR_NAME: &str = "captures";
const DIAGNOSTICS_DIR_NAME: &str = "diagnostics";
const DEFAULT_CONFIG_TOML: &str = include_str!("../config.example.toml");

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub app_dir: PathBuf,
    pub config_path: PathBuf,
    pub config_example_path: PathBuf,
    pub logs_dir: PathBuf,
    pub captures_dir: PathBuf,
    pub diagnostics_dir: PathBuf,
    pub created_config: bool,
    pub updated_config_example: bool,
}

fn app_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .map(|dir| dir.join(APP_DIR_NAME))
}

pub fn ensure_app_paths() -> io::Result<AppPaths> {
    let app_dir = app_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "app directory unavailable"))?;
    fs::create_dir_all(&app_dir)?;

    let config_path = app_dir.join(CONFIG_FILE_NAME);
    let config_example_path = app_dir.join(CONFIG_EXAMPLE_FILE_NAME);
    let logs_dir = app_dir.join(LOGS_DIR_NAME);
    let captures_dir = app_dir.join(CAPTURES_DIR_NAME);
    let diagnostics_dir = app_dir.join(DIAGNOSTICS_DIR_NAME);

    let created_config = write_file_if_missing(&config_path, DEFAULT_CONFIG_TOML)?;
    let updated_config_example = write_file_if_changed(&config_example_path, DEFAULT_CONFIG_TOML)?;
    fs::create_dir_all(&logs_dir)?;
    fs::create_dir_all(&captures_dir)?;
    fs::create_dir_all(&diagnostics_dir)?;

    Ok(AppPaths {
        app_dir,
        config_path,
        config_example_path,
        logs_dir,
        captures_dir,
        diagnostics_dir,
        created_config,
        updated_config_example,
    })
}

fn write_file_if_missing(path: &PathBuf, content: &str) -> io::Result<bool> {
    if path.exists() {
        return Ok(false);
    }
    fs::write(path, content)?;
    Ok(true)
}

fn write_file_if_changed(path: &PathBuf, content: &str) -> io::Result<bool> {
    if path.exists() && fs::read_to_string(path)? == content {
        return Ok(false);
    }
    fs::write(path, content)?;
    Ok(true)
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
