use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, JsonSchema)]
#[kube(
    group = "kubesaver.com",
    version = "v1",
    kind = "Upscaler",
    plural = "upscalers",
    derive = "PartialEq",
    namespaced
)]
pub struct UpscalerSpec {
    pub scale: Vec<Resource>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, JsonSchema)]
pub struct Resource {
    pub resource: String,
    pub expression: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replicas: Option<i32>,
}
