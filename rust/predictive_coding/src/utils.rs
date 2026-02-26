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
