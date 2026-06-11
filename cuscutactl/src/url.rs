use std::str::FromStr;

use url::Url;

pub struct K8sService {
    pub url: Url,
    pub service_name: String,
    pub namespace: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no host found in url")]
    NoHost,
    #[error("invalid host")]
    InvalidHost,
}

pub fn analyze_k8s_url(
    url_str: &str,
    cluster_domain: &str,
    namespace: &str,
    port: u16,
) -> anyhow::Result<K8sService> {
    let mut url = Url::from_str(url_str)?;
    let host = url.host_str().ok_or(Error::NoHost)?;
    let host = host.replace(&format!(".svc.{cluster_domain}",), "");
    let split: Vec<_> = host.split('.').collect();
    let service_name = split.first().ok_or(Error::InvalidHost)?;
    let namespace = split.get(1).unwrap_or(&namespace);
    url.set_host("127.0.0.1".into())?;
    let _ = url.set_port(port.into());
    Ok(K8sService {
        url,
        service_name: service_name.to_string(),
        namespace: namespace.to_string(),
    })
}
