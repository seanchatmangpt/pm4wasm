// Drift Server – Real-time Process Drift Detection SaaS
// Copyright (C) 2024 Process Intelligence Solutions
// Apache License 2.0

//! Telemetry and tracing setup.

/// Initialize tracing and logging.
pub fn setup_telemetry() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("drift_server=debug".parse().unwrap())
                .add_directive("hyper=info".parse().unwrap())
                .add_directive("tokio=info".parse().unwrap())
        )
        .with_target(true)
        .with_thread_ids(true)
        .init();
}
