use clap::Parser;
use futures::stream::StreamExt;
use kube::{api::DeleteParams, api::ListParams, client::Client, Api};
use kube::{Resource, ResourceExt};
use kube_runtime::controller::{Action, Controller};
use kube_saver::controller::{finalizer, upscaler, Upscaler};
use kube_saver::downscaler::processor::processor;
use kube_saver::{init_logger, Error};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::error;

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

#[tokio::main]
async fn main() {
    init_logger();
    let cli = KubeSaver::parse();
    match cli.debug {
        1 => {
            std::env::set_var("RUST_LOG", "debug");
        }
        _ => std::env::set_var("RUST_LOG", "info"),
    }
    let kubernetes_client: Client = Client::try_default()
        .await
        .expect("Expected a valid KUBECONFIG environment variable.");

    let crd_api: Api<Upscaler> = Api::all(kubernetes_client.clone());
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

#[derive(Serialize, Deserialize, Debug)]
enum Value {
    String,
}

fn on_error(error: &Error, _context: Arc<ContextData>) -> Action {
    error!("Reconciliation error:\n{:?}", error);
    Action::requeue(Duration::from_secs(5))
}
//

// REGEX to check downtime
// ^([a-zA-Z]{3})-([a-zA-Z]{3}) (\d\d):(\d\d)-(\d\d):(\d\d) (?P<tz>[a-zA-Z/_]+)$
// Mon-Fri 06:30-20:30 Europe/Berlin
// check if the current time between the downtime
// igore if there is annotaion is_downscaled=true
// read all the deploy from the namespace with matching labels
// put the annotation as is_downscaled=true
// read the current replica and add it a annotation to that resource kube/original-replicas = count
// downscale it to 0 or configured replicas in config-map
// when the current time is greater than configured replicas, scale the replicas = original count and set the annotation is_downscaled = false

/// Context injected with each `reconcile` and `on_error` method invocation.
struct ContextData {
    /// Kubernetes client to make Kubernetes API requests with. Required for K8S resource management.
    client: Client,
}

impl ContextData {
    pub fn new(client: Client) -> Self {
        ContextData { client }
    }
}

/// Action to be taken upon an `Upscaler` resource during reconciliation
enum UpscalerAction {
    /// Create the subresources, this includes spawning `n` pods with Upscaler service
    Create,
    /// Delete all subresources created in the `Create` phase
    Delete,
    /// This `Upscaler` resource is in desired state and requires no actions to be taken
    NoOp,
}

async fn reconcile(upscaler: Arc<Upscaler>, context: Arc<ContextData>) -> Result<Action, Error> {
    let client: Client = context.client.clone();
    let namespace: String = match upscaler.namespace() {
        None => {
            // If there is no namespace to deploy to defined, reconciliation ends with an error immediately.
            return Err(Error::UserInputError(
                "Expected Upscaler resource to be namespaced. Can't deploy to an unknown namespace."
                    .to_owned(),
            ));
        }
        Some(namespace) => namespace,
    };

    // Performs action as decided by the `determine_action` function.
    return match determine_action(&upscaler) {
        UpscalerAction::Create => {
            let name = upscaler.name(); // Name of the Upscaler resource is used to name the subresources as well.
            finalizer::add(client.clone(), &name, &namespace).await?;
            // Invoke creation of a Kubernetes built-in resource named deployment with `n` Upscaler service pods.
            upscaler::upscale(
                client.clone(),
                &upscaler.name(),
                upscaler.spec.replicas,
                &upscaler.spec.tags,
                &namespace,
            )
            .await?;
            let api: Api<Upscaler> = Api::namespaced(client, &namespace);
            // delete the upscaler resource after creation as there is no use
            api.delete(&upscaler.name(), &DeleteParams::default())
                .await?;
            Ok(Action::requeue(Duration::from_secs(10)))
        }
        UpscalerAction::Delete => {
            // for Kubernetes to delete the `Upscaler` resource.
            finalizer::delete(client, &upscaler.name(), &namespace).await?;
            Ok(Action::await_change())
        }
        // The resource is already in desired state, do nothing and re-check after 10 seconds
        UpscalerAction::NoOp => Ok(Action::requeue(Duration::from_secs(10))),
    };
}

/// # Arguments
/// - `Upscaler`: A reference to `Upscaler` being reconciled to decide next action upon.
fn determine_action(upscaler: &Upscaler) -> UpscalerAction {
    return if upscaler.meta().deletion_timestamp.is_some() {
        UpscalerAction::Delete
    } else if upscaler
        .meta()
        .finalizers
        .as_ref()
        .map_or(true, |finalizers| finalizers.is_empty())
    {
        UpscalerAction::Create
    } else {
        UpscalerAction::NoOp
    };
}
