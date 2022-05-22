use core::time;
use kube::core::object::HasSpec;
use kube_saver::deployment;
use kube_saver::is_downscale_time;
use kube_saver::Error;
use std::fs::File;
use std::sync::Arc;
use std::thread;
use tracing::*;

use futures::stream::StreamExt;

use kube::Resource;
use kube::ResourceExt;
use kube::{api::ListParams, client::Client, Api};
use kube_runtime::controller::{Action, Context};
use kube_runtime::Controller;
use serde::{Deserialize, Serialize};
use tokio::time::Duration;

use crate::crd::Upscaler;

pub mod crd;
mod finalizer;
mod upscaler;

#[tokio::main]
//TODO: clap input args
async fn main() {
    // First, a Kubernetes client must be obtained using the `kube` crate
    // The client will later be moved to the custom controller
    let kubernetes_client: Client = Client::try_default()
        .await
        .expect("Expected a valid KUBECONFIG environment variable.");

    // Preparation of resources used by the `kube_runtime::Controller`
    let crd_api: Api<Upscaler> = Api::all(kubernetes_client.clone());
    let context: Context<ContextData> = Context::new(ContextData::new(kubernetes_client.clone()));

    // The controller comes from the `kube_runtime` crate and manages the reconciliation process.
    // It requires the following information:
    // - `kube::Api<T>` this controller "owns". In this case, `T = Echo`, as this controller owns the `Echo` resource,
    // - `kube::api::ListParams` to select the `Echo` resources with. Can be used for Echo filtering `Echo` resources before reconciliation,
    // - `reconcile` function with reconciliation logic to be called each time a resource of `Echo` kind is created/updated/deleted,
    // - `on_error` function to call whenever reconciliation fails.

    let z = Controller::new(crd_api.clone(), ListParams::default())
        .run(reconcile, on_error, context)
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()));

    let m = down_scale();
    tokio::select! {
        _ = z => info!("contrilled failed"),
        //_ = m => info!("downscaler exited"),
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Rules {
    rules: Vec<Rule>,
}

#[derive(Serialize, Deserialize, Debug)]
enum Value {
    String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Rule {
    id: String,
    uptime: String,
    jmespath: String,
    resource: Vec<String>,
    replicas: String,
}

impl Rule {
    fn is_downscale_time(&self) {}
}

fn on_error(error: &Error, _context: Context<ContextData>) -> Action {
    eprintln!("Reconciliation error:\n{:?}", error);
    Action::requeue(Duration::from_secs(5))
}

async fn down_scale() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
    loop {
        let ten_millis = time::Duration::from_millis(10000);
        let client = Client::try_default().await?;
        // read rules yaml file
        //let f = File::open("/config/rules.yaml").unwrap();
        let f = File::open("./k8s/rules.yaml").unwrap();
        let r: Rules = serde_yaml::from_reader(f).unwrap();
        for e in r.rules {
            println!("id : {}", e.id);
            println!("downtime : {}", e.uptime);
            println!("jmespath : {}", e.jmespath);
            // check if the time is downtime
            let z = is_downscale_time(&e.uptime).unwrap();
            for r in &e.resource {
                if r.eq("Deployment") {
                    println!("{}", r);
                    deployment(client.clone(), z).await?;
                } else if r.eq("StatefulSet") {
                    println!("{}", r)
                } else {
                    println!("{}", r)
                }
            }
        }
        //TODO: input cli args
        thread::sleep(ten_millis);
    }
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
    /// Constructs a new instance of ContextData.
    ///
    /// # Arguments:
    /// - `client`: A Kubernetes client to make Kubernetes REST API requests with. Resources
    /// will be created and deleted with this client.
    pub fn new(client: Client) -> Self {
        ContextData { client }
    }
}

/// Action to be taken upon an `Echo` resource during reconciliation
enum EchoAction {
    /// Create the subresources, this includes spawning `n` pods with Echo service
    Create,
    /// Delete all subresources created in the `Create` phase
    Delete,
    /// This `Echo` resource is in desired state and requires no actions to be taken
    NoOp,
}

async fn reconcile(
    upscaler: Arc<Upscaler>,
    context: Context<ContextData>,
) -> Result<Action, Error> {
    let client: Client = context.get_ref().client.clone(); // The `Client` is shared -> a clone from the reference is obtained

    // The resource of `Echo` kind is required to have a namespace set. However, it is not guaranteed
    // the resource will have a `namespace` set. Therefore, the `namespace` field on object's metadata
    // is optional and Rust forces the programmer to check for it's existence first.
    let namespace: String = match upscaler.namespace() {
        None => {
            // If there is no namespace to deploy to defined, reconciliation ends with an error immediately.
            return Err(Error::UserInputError(
                "Expected Echo resource to be namespaced. Can't deploy to an unknown namespace."
                    .to_owned(),
            ));
        }
        // If namespace is known, proceed. In a more advanced version of the operator, perhaps
        // the namespace could be checked for existence first.
        Some(namespace) => namespace,
    };

    // Performs action as decided by the `determine_action` function.
    return match determine_action(&upscaler) {
        EchoAction::Create => {
            // Creates a deployment with `n` Echo service pods, but applies a finalizer first.
            // Finalizer is applied first, as the operator might be shut down and restarted
            // at any time, leaving subresources in intermediate state. This prevents leaks on
            // the `Echo` resource deletion.
            let name = upscaler.name(); // Name of the Echo resource is used to name the subresources as well.

            // Apply the finalizer first. If that fails, the `?` operator invokes automatic conversion
            // of `kube::Error` to the `Error` defined in this crate.
            finalizer::add(client.clone(), &name, &namespace).await?;
            // Invoke creation of a Kubernetes built-in resource named deployment with `n` echo service pods.
            upscaler::deploy(
                client,
                &upscaler.name(),
                upscaler.spec.replicas,
                &upscaler.spec.tags,
                &namespace,
            )
            .await?;
            Ok(Action::requeue(Duration::from_secs(10)))
        }
        EchoAction::Delete => {
            // Deletes any subresources related to this `Echo` resources. If and only if all subresources
            // are deleted, the finalizer is removed and Kubernetes is free to remove the `Echo` resource.

            //First, delete the deployment. If there is any error deleting the deployment, it is
            // automatically converted into `Error` defined in this crate and the reconciliation is ended
            // with that error.
            // Note: A more advanced implementation would for the Deployment's existence.
            upscaler::delete(client.clone(), &upscaler.name(), &namespace).await?;

            // Once the deployment is successfully removed, remove the finalizer to make it possible
            // for Kubernetes to delete the `Echo` resource.
            finalizer::delete(client, &upscaler.name(), &namespace).await?;
            Ok(Action::await_change()) // Makes no sense to delete after a successful delete, as the resource is gone
        }
        // The resource is already in desired state, do nothing and re-check after 10 seconds
        EchoAction::NoOp => Ok(Action::requeue(Duration::from_secs(10))),
    };
}

/// Resources arrives into reconciliation queue in a certain state. This function looks at
/// the state of given `Echo` resource and decides which actions needs to be performed.
/// The finite set of possible actions is represented by the `Action` enum.
///
/// # Arguments
/// - `echo`: A reference to `Echo` being reconciled to decide next action upon.
fn determine_action(echo: &Upscaler) -> EchoAction {
    return if echo.meta().deletion_timestamp.is_some() {
        EchoAction::Delete
    } else if echo
        .meta()
        .finalizers
        .as_ref()
        .map_or(true, |finalizers| finalizers.is_empty())
    {
        EchoAction::Create
    } else {
        EchoAction::NoOp
    };
}
