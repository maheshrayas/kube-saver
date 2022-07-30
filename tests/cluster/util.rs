use kube::{
    api::{Api, DynamicObject, Patch, PatchParams, ResourceExt},
    core::GroupVersionKind,
    discovery::{ApiCapabilities, ApiResource, Discovery, Scope},
    Client,
};

use anyhow::{bail, Result};
//"tests/upscaler/upscaler-scaleup13.yaml"
pub async fn kubectl_appy(yaml_file: &str) -> Result<()> {
    let client = Client::try_default()
        .await
        .expect("Failed to read kubeconfig");
    let ssapply = PatchParams::apply("kubectl-light").force();
    let yaml = std::fs::read_to_string(yaml_file).unwrap();
    let namespace = "kuber13";

    for doc in multidoc_deserialize(&yaml)? {
        let obj: DynamicObject = serde_yaml::from_value(doc)?;
        let gvk = if let Some(tm) = &obj.types {
            GroupVersionKind::try_from(tm)?
        } else {
            bail!("cannot apply object without valid TypeMeta {:?}", obj);
        };
        let name = obj.name_any();
        let discovery = Discovery::new(client.clone()).run().await?;
        if let Some((ar, caps)) = discovery.resolve_gvk(&gvk) {
            let api = dynamic_api(
                ar,
                caps,
                client.clone(),
                &Some(namespace.to_string()),
                false,
            );
            let data: serde_json::Value = serde_json::to_value(&obj)?;
            let _r = api.patch(&name, &ssapply, &Patch::Apply(data)).await?;
        } else {
            println!("Cannot apply document for unknown {:?}", gvk);
        }
    }
    Ok(())
}
fn multidoc_deserialize(data: &str) -> Result<Vec<serde_yaml::Value>> {
    use serde::Deserialize;
    let mut docs = vec![];
    for de in serde_yaml::Deserializer::from_str(data) {
        docs.push(serde_yaml::Value::deserialize(de)?);
    }
    Ok(docs)
}

fn dynamic_api(
    ar: ApiResource,
    caps: ApiCapabilities,
    client: Client,
    ns: &Option<String>,
    all: bool,
) -> Api<DynamicObject> {
    if caps.scope == Scope::Cluster || all {
        Api::all_with(client, &ar)
    } else if let Some(namespace) = ns {
        Api::namespaced_with(client, namespace, &ar)
    } else {
        Api::default_namespaced_with(client, &ar)
    }
}
