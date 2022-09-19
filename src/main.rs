use futures::stream::StreamExt;
use kube::{api::ListParams, client::Client, Api};
use kube_runtime::controller::Controller;
use log::error;
use saver::{
    controller::watcher::{on_error, reconcile},
    processor::Process,
    KubeSaver,
};
use std::sync::Arc;

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() {
    saver::util::init_logger();
    let cli_parser = KubeSaver::new();
    let kubernetes_client: Client = Client::try_default()
        .await
        .expect("Expected a valid KUBECONFIG environment variable.");

    let crd_api: Api<saver::controller::Upscaler> = Api::all(kubernetes_client.clone());
    let context: Arc<saver::util::ContextData> =
        Arc::new(saver::util::ContextData::new(kubernetes_client.clone()));

    let controller = Controller::new(crd_api.clone(), ListParams::default())
        .run(reconcile, on_error, context)
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()));
    let p: Process = cli_parser.into();
    let downscaler = p.processor();
    tokio::select! {
        _ = controller => error!("controlled failed"),
       _ = downscaler => error!("downscaler exited"),
    }
}
