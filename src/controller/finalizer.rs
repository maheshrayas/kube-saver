use crate::controller::Upscaler;
use kube::api::{Patch, PatchParams};
use kube::{Api, Client, Error};
use log::info;
use serde_json::{json, Value};

/// Adds a finalizer record into an `Upscaler` kind of resource. If the finalizer already exists,
/// this action has no effect.
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
