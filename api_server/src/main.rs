use std::{net::Ipv4Addr, sync::Arc};

use anyhow::{Context, Result};
use axum::{
    routing::{get, post},
    Extension, Router,
};
use config::Config;
use dashmap::DashSet;
use etcd::EtcdConfig;
use serde::Deserialize;

mod etcd;
mod handler;

#[derive(Debug, Clone, Deserialize)]
struct ServerConfig {
    log_level: String,
    etcd: EtcdConfig,
    metrics_server: String,
}

pub struct AppState {
    etcd_pool: etcd::EtcdPool,
    service_ip_pool: DashSet<Ipv4Addr>,
    config: ServerConfig,
}

#[tokio::main]
async fn main() -> Result<()> {
    // read config
    let config = Config::builder()
        .add_source(config::File::with_name("./examples/api-server/config.yaml"))
        .build()?
        .try_deserialize::<ServerConfig>()
        .with_context(|| "Failed to parse config".to_string())?;

    // init tracing
    std::env::set_var("RUST_LOG", format!("api_server={}", config.log_level));
    tracing_subscriber::fmt::init();

    // init app state
    let app_state = AppState::from_config(&config).await?;
    let shared_state = Arc::new(app_state);

    #[rustfmt::skip]
    let pod_routes = Router::new().nest(
        "/pods",
        Router::new()
            .route("/",
                   get(handler::pod::list)
                       .post(handler::pod::create))
            .route("/:name",
                   get(handler::pod::get)
                       .put(handler::pod::replace)
                       .delete(handler::pod::delete),
            ),
    );

    #[rustfmt::skip]
    let rs_routes = Router::new().nest(
        "/replicasets",
        Router::new()
            .route("/",
                   get(handler::replica_set::list)
                       .post(handler::replica_set::create))
            .route("/:name",
                   get(handler::replica_set::get)
                       .put(handler::replica_set::update)
                       .patch(handler::replica_set::patch)
                       .delete(handler::replica_set::delete),
            ),
    );

    #[rustfmt::skip]
    let service_routes = Router::new().nest(
        "/services",
        Router::new()
            .route("/",
                   get(handler::service::list)
                       .post(handler::service::create))
            .route("/:name",
                   get(handler::service::get)
                       .put(handler::service::update)
                       .delete(handler::service::delete),
            ),
    );

    #[rustfmt::skip]
    let hpa_routes = Router::new().nest(
        "/horizontalpodautoscalers",
        Router::new()
            .route("/",
                   get(handler::hpa::list)
                       .post(handler::hpa::create))
            .route("/:name",
                   get(handler::hpa::get)
                       .put(handler::hpa::update)
                       .patch(handler::hpa::patch)
                       .delete(handler::hpa::delete),
            ),
    );

    #[rustfmt::skip]
    let watch_routes = Router::new().nest(
        "/watch",
        Router::new()
            .route("/nodes", get(handler::node::watch_all))
            .route("/pods", get(handler::pod::watch_all))
            .route("/replicasets", get(handler::replica_set::watch_all))
            .route("/services", get(handler::service::watch_all))
            .route("/horizontalpodautoscalers", get(handler::hpa::watch_all)),
    );

    #[rustfmt::skip]
    let metrics_routes = Router::new().nest(
        "/metrics",
        Router::new()
            .route("/pods", get(handler::metrics::list))
    );

    let app = Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .merge(pod_routes)
                .merge(rs_routes)
                .merge(service_routes)
                .merge(hpa_routes)
                .merge(watch_routes)
                .merge(metrics_routes)
                .route("/nodes", get(handler::node::list))
                .route("/bindings", post(handler::binding::bind)),
        )
        .layer(Extension(shared_state));

    tracing::info!("Listening at 0.0.0.0:8080");
    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown())
        .await
        .unwrap();

    Ok(())
}

impl AppState {
    async fn from_config(config: &ServerConfig) -> Result<AppState> {
        let pool = config
            .etcd
            .create_pool()
            .await
            .with_context(|| "Failed to create etcd client pool".to_string())?;

        Ok(AppState {
            etcd_pool: pool,
            service_ip_pool: DashSet::new(),
            config: (*config).to_owned(),
        })
    }
}

async fn shutdown() {
    tokio::signal::ctrl_c()
        .await
        .expect("expect tokio signal ctrl-c");
    tracing::info!("Shutting Down");
}
