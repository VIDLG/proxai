use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Reading the config file from disk failed.
    #[error("read config file `{path}`", path = .path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The config file was readable, but its contents are invalid.
    #[error(
        "invalid config file `{path}`\n\n{message}\n\nFix this file, compare it with `config.example.toml` in the same directory, or delete it to regenerate defaults.",
        path = .path.display()
    )]
    Invalid { path: PathBuf, message: String },
}
