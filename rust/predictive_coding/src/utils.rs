use std::{fs::File, path::Path};

use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

pub fn bin_init_env() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
}

pub fn open_file_for_write(path: impl AsRef<Path>) -> File {
    let path = path.as_ref();

    if path.exists() {
        assert!(path.is_file());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .unwrap()
}
