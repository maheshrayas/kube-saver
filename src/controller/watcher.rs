use crate::controller::{finalizer, upscaler, Upscaler};
use crate::util::{ContextData, Error};
use kube::{Resource, ResourceExt};
use kube_runtime::controller::Action;
use log::error;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::Duration;
#[derive(Serialize, Deserialize, Debug)]
enum Value {
    String,
}

#[cfg(not(tarpaulin_include))]
pub fn on_error(error: &Error, _context: Arc<ContextData>) -> Action {
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
pub async fn reconcile(
    upscaler: Arc<Upscaler>,
    context: Arc<ContextData>,
) -> Result<Action, Error> {
    use kube::{api::DeleteParams, Api, Client};

    use crate::{downscaler::Resources, util::check_input_resource};

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
