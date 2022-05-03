use anyhow::{Ok, Result};
use resources::{
    models,
    objects::{
        binding::Binding, object_reference::ObjectReference, KubeObject, KubeResource, Metadata,
    },
};
use tokio::sync::mpsc::Receiver;

use crate::cache::Cache;

pub struct Scheduler<T>
where
    T: Fn(&KubeObject, &Cache) -> ObjectReference,
{
    cache: Cache,
    algorithm: T,
    client: reqwest::Client,
}

impl<T> Scheduler<T>
where
    T: Fn(&KubeObject, &Cache) -> ObjectReference,
{
    pub fn new(algorithm: T, cache: Cache) -> Scheduler<T> {
        Scheduler {
            cache,
            algorithm,
            client: reqwest::Client::new(),
        }
    }

    pub async fn run(&self, mut pod_queue: Receiver<KubeObject>) -> Result<()> {
        while let Some(pod) = pod_queue.recv().await {
            tracing::info!("schedule pod: {}", pod.name());
            let node = (self.algorithm)(&pod, &self.cache);
            self.bind(pod, node).await?;
        }
        Ok(())
    }

    async fn bind(&self, pod: KubeObject, node: ObjectReference) -> Result<()> {
        let binding = KubeObject {
            metadata: Metadata {
                name: pod.name(),
                uid: pod.metadata.uid,
            },
            resource: KubeResource::Binding(Binding {
                target: node,
            }),
        };
        let _ = self
            .client
            .post("http://localhost:8080/api/v1/bindings")
            .json(&binding)
            .send()
            .await?
            .json::<models::Response<()>>()
            .await?;

        Ok(())
    }
}
