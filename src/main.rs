use clap::Parser;
use futures::stream::StreamExt;
use kube::{api::ListParams, client::Client, Api};
use kube_runtime::controller::Controller;
use kube_saver::{
    controller::watcher::{on_error, reconcile},
    downscaler::processor::processor,
    init_logger, ContextData,
};
use log::error;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct KubeSaver {
    // Loop interval in secs
    #[clap(short, long, default_value_t = 60)]
    interval: u64,
    /// rules yaml
    #[clap(short, long, default_value = "/config/rules.yaml")]
    rules: String,
    /// supply --debug to print the debug information
    #[clap(short, long, parse(from_occurrences))]
    debug: usize,
}

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() {
    let cli = KubeSaver::parse();
    match cli.debug {
        1 => {
            std::env::set_var("RUST_LOG", "debug,kube_client=off,tower=off,hyper=off");
        }
        _ => {
            std::env::set_var("RUST_LOG", "info,kube_client=off");
        }
    }
    init_logger();

    let kubernetes_client: Client = Client::try_default()
        .await
        .expect("Expected a valid KUBECONFIG environment variable.");

    let crd_api: Api<kube_saver::controller::Upscaler> = Api::all(kubernetes_client.clone());
    let context: Arc<ContextData> = Arc::new(ContextData::new(kubernetes_client.clone()));

    let controller = Controller::new(crd_api.clone(), ListParams::default())
        .run(reconcile, on_error, context)
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()));

    let downscaler = processor(cli.interval, &cli.rules);
    tokio::select! {
        _ = controller => error!("controlled failed"),
       _ = downscaler => error!("downscaler exited"),
    }
}
