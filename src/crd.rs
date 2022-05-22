use std::collections::BTreeMap;

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
    group = "maheshrayas.com",
    version = "v1",
    kind = "Upscaler",
    plural = "upscalers",
    derive = "PartialEq",
    namespaced
)]
pub struct UpscalerSpec {
    pub tags: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replicas: Option<i32>,
}
