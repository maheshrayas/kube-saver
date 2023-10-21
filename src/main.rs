use futures::stream::StreamExt;
use kube::{client::Client, Api};
use kube_runtime::controller::Controller;
use log::error;
use prometheus::{Encoder, TextEncoder};
use saver::error::Error;
use saver::ScaleState;
use saver::{
    controller::watcher::{on_error, reconcile},
    parser::Args,
    processor::Process,
};
use std::sync::Arc;

use actix_web::{get, App, HttpResponse, HttpServer};

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() -> Result<(), Error> {
    use actix_web::web;
    use kube_runtime::watcher::Config;

    let cli_parser = Args::new();
    saver::parser::init_logger();

    let kubernetes_client: Client = Client::try_default()
        .await
        .expect("Expected a valid KUBECONFIG environment variable.");

    let crd_api: Api<saver::controller::Upscaler> = Api::all(kubernetes_client.clone());
    let context: Arc<saver::parser::ContextData> =
        Arc::new(saver::parser::ContextData::new(kubernetes_client.clone()));

    let controller = Controller::new(crd_api.clone(), Config::default())
        .run(reconcile, on_error, context)
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()));
    let p: Process = cli_parser.into();

    // metrics
    let prom_state = Arc::new(ScaleState::new());
    let downscaler = p.processor(Arc::clone(&prom_state));

    let server = HttpServer::new(move || {
        let app_state = prom_state.clone();
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .service(export_metrics)
    })
    .bind("0.0.0.0:8085")?
    .shutdown_timeout(5);

    tokio::select! {
        _ = controller => error!("controller failed"),
       _ = downscaler => error!("downscaler exited"),
       _ = server.run() => error!("mertics server failed"),
    }
    Ok(())
}

#[get("/metrics")]
async fn export_metrics() -> HttpResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    HttpResponse::Ok()
        .content_type(encoder.format_type())
        .body(buffer)
}
