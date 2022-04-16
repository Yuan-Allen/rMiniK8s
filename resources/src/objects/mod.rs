use serde::{Deserialize, Serialize};
use strum::Display;

pub mod pod;

#[derive(Debug, Serialize, Deserialize)]
pub struct KubeObject {
    pub metadata: Metadata,
    pub spec: KubeSpec,
    pub status: Option<KubeStatus>,
}

#[derive(Debug, Serialize, Deserialize, Display)]
#[serde(tag = "kind", content = "spec")]
pub enum KubeSpec {
    Pod(pod::PodSpec),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "status")]
pub enum KubeStatus {
    Pod(pod::PodStatus),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    /// Name must be unique within a namespace.
    /// Is required when creating resources,
    /// although some resources may allow a client
    /// to request the generation of an appropriate name automatically.
    /// Name is primarily intended for creation idempotence
    /// and configuration definition. Cannot be updated.
    pub name: String,
}
