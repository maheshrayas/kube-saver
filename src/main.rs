use clap::Parser;
use futures::stream::StreamExt;
use kube::{api::DeleteParams, api::ListParams, client::Client, Api};
use kube::{Resource, ResourceExt};
use kube_runtime::controller::{Action, Controller};
use kube_saver::controller::{finalizer, upscaler, Upscaler};
use kube_saver::downscaler::processor::processor;
use kube_saver::init_logger;
use kube_saver::{ContextData, Error, Resources};
use log::error;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::Duration;

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

#[cfg(not(tarpaulin_include))]
fn on_error(error: &Error, _context: Arc<ContextData>) -> Action {
    error!("Reconciliation error:\n{:?}", error);
    Action::requeue(Duration::from_secs(5))
}

/// Action to be taken upon an `Upscaler` resource during reconciliation
enum UpscalerAction {
    Create,
    Delete,
    NoOp,
}

#[cfg(not(tarpaulin_include))]
async fn reconcile(upscaler: Arc<Upscaler>, context: Arc<ContextData>) -> Result<Action, Error> {
    use kube_saver::check_input_resource;

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
    match determine_action(&upscaler) {
        UpscalerAction::Create => {
            let name = upscaler.name_any(); // Name of the Upscaler resource is used to name the subresources as well.
            finalizer::add(client.clone(), &name, &namespace).await?;
            // Invoke creation of a Kubernetes built-in resource named deployment with `n` Upscaler service pods.
            // loop thru the scale
            for res in &upscaler.spec.scale {
                // for each resources in spec
                for r in &res.resource {
                    let f = check_input_resource(r);
                    if f.is_some() {
                        match f.unwrap() {
                            Resources::Deployment => {
                                upscaler::upscale_deploy(
                                    client.clone(),
                                    res.replicas,
                                    &res.jmespath,
                                )
                                .await?
                            }
                            Resources::StatefulSet => {
                                upscaler::upscale_statefulset(
                                    client.clone(),
                                    res.replicas,
                                    &res.jmespath,
                                )
                                .await?
                            }
                            Resources::Namespace => {
                                upscaler::upscale_ns(client.clone(), res.replicas, &res.jmespath)
                                    .await?
                            }
                            Resources::CronJob => {
                                upscaler::enable_cronjob(client.clone(), &res.jmespath).await?
                            }
                            Resources::Hpa => {
                                upscaler::upscale_hpa(client.clone(), res.replicas, &res.jmespath)
                                    .await?
                            }
                        };
                    }
                }
            }
            let api: Api<Upscaler> = Api::namespaced(client, &namespace);
            // delete the upscaler resource after creation as there is no use
            api.delete(&name, &DeleteParams::default()).await?;
            Ok(Action::requeue(Duration::from_secs(10)))
        }
        UpscalerAction::Delete => {
            // for Kubernetes to delete the `Upscaler` resource.
            finalizer::delete(client, &upscaler.name_any(), &namespace).await?;
            Ok(Action::await_change())
        }
        // The resource is already in desired state, do nothing and re-check after 10 seconds
        UpscalerAction::NoOp => Ok(Action::requeue(Duration::from_secs(10))),
    }
}

/// # Arguments
/// - `Upscaler`: A reference to `Upscaler` being reconciled to decide next action upon.
#[cfg(not(tarpaulin_include))]
fn determine_action(upscaler: &Upscaler) -> UpscalerAction {
    if upscaler.meta().deletion_timestamp.is_some() {
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
    }
}
