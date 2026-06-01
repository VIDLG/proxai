use super::*;
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn creates_default_files_inside_app_dir() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let root = unique_temp_dir();
    let app_dir = root.join(".proxai");
    let original_unix_home = std::env::var_os("HOME");
    let original_home = std::env::var_os("USERPROFILE");
    unsafe { std::env::set_var("HOME", &root) };
    unsafe { std::env::set_var("USERPROFILE", &root) };

    let created = ensure_app_paths().unwrap();
    assert_eq!(created.app_dir, app_dir);
    assert!(created.config_path.exists());
    assert!(created.config_example_path.exists());
    assert!(created.created_config);
    assert!(created.created_config_example);

    let created_again = ensure_app_paths().unwrap();
    assert!(!created_again.created_config);
    assert!(!created_again.created_config_example);

    fs::write(&created_again.config_example_path, "user-local example").unwrap();
    let refreshed = ensure_app_paths().unwrap();
    assert!(!refreshed.created_config);
    assert!(!refreshed.created_config_example);
    assert_eq!(
        fs::read_to_string(&refreshed.config_example_path).unwrap(),
        "user-local example"
    );

    match original_unix_home {
        Some(value) => unsafe { std::env::set_var("HOME", value) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    match original_home {
        Some(value) => unsafe { std::env::set_var("USERPROFILE", value) },
        None => unsafe { std::env::remove_var("USERPROFILE") },
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn creates_logs_dir_inside_app_dir() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let root = unique_temp_dir();
    let logs_dir = root.join(".proxai").join("logs");
    let original_unix_home = std::env::var_os("HOME");
    let original_home = std::env::var_os("USERPROFILE");
    unsafe { std::env::set_var("HOME", &root) };
    unsafe { std::env::set_var("USERPROFILE", &root) };

    assert_eq!(ensure_app_paths().unwrap().logs_dir, logs_dir);
    assert!(logs_dir.exists());

    match original_unix_home {
        Some(value) => unsafe { std::env::set_var("HOME", value) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    match original_home {
        Some(value) => unsafe { std::env::set_var("USERPROFILE", value) },
        None => unsafe { std::env::remove_var("USERPROFILE") },
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn creates_captures_dir_inside_app_dir() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let root = unique_temp_dir();
    let captures_dir = root.join(".proxai").join("captures");
    let original_unix_home = std::env::var_os("HOME");
    let original_home = std::env::var_os("USERPROFILE");
    unsafe { std::env::set_var("HOME", &root) };
    unsafe { std::env::set_var("USERPROFILE", &root) };

    assert_eq!(ensure_app_paths().unwrap().captures_dir, captures_dir);
    assert!(captures_dir.exists());

    match original_unix_home {
        Some(value) => unsafe { std::env::set_var("HOME", value) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    match original_home {
        Some(value) => unsafe { std::env::set_var("USERPROFILE", value) },
        None => unsafe { std::env::remove_var("USERPROFILE") },
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn creates_diagnostics_dir_inside_app_dir() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let root = unique_temp_dir();
    let diagnostics_dir = root.join(".proxai").join("diagnostics");
    let original_unix_home = std::env::var_os("HOME");
    let original_home = std::env::var_os("USERPROFILE");
    unsafe { std::env::set_var("HOME", &root) };
    unsafe { std::env::set_var("USERPROFILE", &root) };

    assert_eq!(ensure_app_paths().unwrap().diagnostics_dir, diagnostics_dir);
    assert!(diagnostics_dir.exists());

    match original_unix_home {
        Some(value) => unsafe { std::env::set_var("HOME", value) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    match original_home {
        Some(value) => unsafe { std::env::set_var("USERPROFILE", value) },
        None => unsafe { std::env::remove_var("USERPROFILE") },
    }
    let _ = fs::remove_dir_all(root);
}

fn unique_temp_dir() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("proxai-paths-{}-{nanos}", std::process::id()))
}
