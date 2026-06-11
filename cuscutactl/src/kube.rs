use std::{pin::Pin, process::Stdio};

use base64::Engine;
use tokio::process::{Child, Command};

use crate::{
    Handler,
    url::{K8sService, analyze_k8s_url},
};

pub struct K8sPortForwardHandler {
    pub service: K8sService,
    pub child: Child,
}

impl Handler for K8sPortForwardHandler {
    fn get_url(&self) -> String {
        self.service.url.to_string()
    }

    fn kill(&mut self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async {
            let _ = self.child.kill().await;
        })
    }
}

pub async fn k8s_port_forward(
    namespace: &str,
    secret: &str,
    secret_key: &str,
    cluster_domain: &str,
    forward_port: &u16,
    port: &u16,
) -> anyhow::Result<K8sPortForwardHandler> {
    async fn get_secret(namespace: &str, secret: &str, key: &str) -> anyhow::Result<String> {
        let bytes = Command::new("kubectl")
            .args([
                "get",
                "secret",
                secret,
                &format!("--namespace={namespace}"),
                &format!("-o=jsonpath={{.data.{key}}}"),
            ])
            .stdin(Stdio::piped())
            .output()
            .await?
            .stdout;
        let base64 = String::from_utf8(bytes)?;
        Ok(String::from_utf8(
            base64::prelude::BASE64_STANDARD.decode(base64)?,
        )?)
    }
    let addr = get_secret(namespace, secret, secret_key).await?;
    let service = analyze_k8s_url(&addr, cluster_domain, namespace, *forward_port)?;
    let handle = K8sPortForwardHandler {
        child: port_forward(&service, *forward_port, *port)?,
        service,
    };
    Ok(handle)
}

fn port_forward(
    K8sService {
        service_name,
        namespace,
        ..
    }: &K8sService,
    host_port: u16,
    src_port: u16,
) -> anyhow::Result<Child> {
    Ok(Command::new("kubectl")
        .args([
            "port-forward",
            &format!("svc/{service_name}"),
            &format!("{host_port}:{src_port}"),
            &format!("-n={namespace}"),
        ])
        .stdout(Stdio::piped())
        .spawn()?)
}
