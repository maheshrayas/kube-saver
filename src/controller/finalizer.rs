use crate::controller::Upscaler;
use kube::api::{Patch, PatchParams};
use kube::{Api, Client, Error};
use serde_json::{json, Value};
use tracing::info;

/// Adds a finalizer record into an `Upscaler` kind of resource. If the finalizer already exists,
/// this action has no effect.
///
/// # Arguments:
/// - `client` - Kubernetes client to modify the `Upscaler` resource with.
/// - `name` - Name of the `Upscaler` resource to modify. Existence is not verified
/// - `namespace` - Namespace where the `Upscaler` resource with given `name` resides.
///
/// Note: Does not check for resource's existence for simplicity.
pub async fn add(client: Client, name: &str, namespace: &str) -> Result<Upscaler, Error> {
    let api: Api<Upscaler> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": ["upscalers.kubesaver.com/finalizer"]
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    api.patch(name, &PatchParams::default(), &patch).await
}

/// Removes all finalizers from an `Upscaler` resource. If there are no finalizers already, this
/// action has no effect.
///
/// # Arguments:
/// - `client` - Kubernetes client to modify the `Upscaler` resource with.
/// - `name` - Name of the `Upscaler` resource to modify. Existence is not verified
/// - `namespace` - Namespace where the `Upscaler` resource with given `name` resides.
///
/// Note: Does not check for resource's existence for simplicity.
pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<Upscaler, Error> {
    let api: Api<Upscaler> = Api::namespaced(client, namespace);
    info!(
        "Deleting the Upscaler resource {} in namespace {}",
        name, namespace
    );
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": null
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    api.patch(name, &PatchParams::default(), &patch).await
}
