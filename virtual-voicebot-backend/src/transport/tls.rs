use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use rustls::server::AllowAnyAuthenticatedClient;
use rustls::{Certificate, PrivateKey, RootCertStore, ServerConfig};
use tokio_rustls::TlsAcceptor;

use crate::config::TlsSettings;

pub fn build_tls_acceptor(settings: &TlsSettings) -> Result<TlsAcceptor> {
    let certs = load_cert_chain(&settings.cert_path)?;
    let key = load_private_key(&settings.key_path)?;

    let builder = ServerConfig::builder().with_safe_defaults();
    let config = if let Some(ca_path) = &settings.ca_path {
        let roots = load_ca_store(ca_path)?;
        let verifier = AllowAnyAuthenticatedClient::new(roots).boxed();
        builder
            .with_client_cert_verifier(verifier)
            .with_single_cert(certs, key)?
    } else {
        builder.with_no_client_auth().with_single_cert(certs, key)?
    };

    Ok(TlsAcceptor::from(Arc::new(config)))
}

fn load_cert_chain(path: &str) -> Result<Vec<Certificate>> {
    let file = File::open(path).with_context(|| format!("open cert file {}", path))?;
    let mut reader = BufReader::new(file);
    let certs =
        rustls_pemfile::certs(&mut reader).with_context(|| format!("read certs from {}", path))?;
    if certs.is_empty() {
        return Err(anyhow!("no certificates found in {}", path));
    }
    Ok(certs.into_iter().map(Certificate).collect())
}

fn load_private_key(path: &str) -> Result<PrivateKey> {
    let file = File::open(path).with_context(|| format!("open key file {}", path))?;
    let mut reader = BufReader::new(file);
    let mut keys = rustls_pemfile::pkcs8_private_keys(&mut reader)
        .with_context(|| format!("read pkcs8 keys from {}", path))?;

    if keys.is_empty() {
        let file = File::open(path).with_context(|| format!("open key file {}", path))?;
        let mut reader = BufReader::new(file);
        keys = rustls_pemfile::rsa_private_keys(&mut reader)
            .with_context(|| format!("read rsa keys from {}", path))?;
    }

    if keys.is_empty() {
        return Err(anyhow!("no private keys found in {}", path));
    }

    Ok(PrivateKey(keys.remove(0)))
}

fn load_ca_store(path: &str) -> Result<RootCertStore> {
    let file = File::open(path).with_context(|| format!("open ca file {}", path))?;
    let mut reader = BufReader::new(file);
    let certs = rustls_pemfile::certs(&mut reader)
        .with_context(|| format!("read ca certs from {}", path))?;
    if certs.is_empty() {
        return Err(anyhow!("no ca certificates found in {}", path));
    }
    let mut roots = RootCertStore::empty();
    let (added, _) = roots.add_parsable_certificates(&certs);
    if added == 0 {
        return Err(anyhow!("no valid ca certificates found in {}", path));
    }
    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_tls_acceptor_from_pem() -> Result<()> {
        let rcgen::CertifiedKey { cert, key_pair } =
            rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        let dir = tempfile::tempdir()?;
        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");
        std::fs::write(&cert_path, cert_pem)?;
        std::fs::write(&key_path, key_pem)?;

        let settings = TlsSettings {
            bind_ip: "127.0.0.1".to_string(),
            port: 5061,
            cert_path: cert_path.to_string_lossy().to_string(),
            key_path: key_path.to_string_lossy().to_string(),
            ca_path: None,
        };

        let _acceptor = build_tls_acceptor(&settings)?;
        Ok(())
    }

    #[test]
    fn build_tls_acceptor_with_ca() -> Result<()> {
        let rcgen::CertifiedKey { cert, key_pair } =
            rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        let dir = tempfile::tempdir()?;
        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");
        let ca_path = dir.path().join("ca.pem");
        std::fs::write(&cert_path, &cert_pem)?;
        std::fs::write(&key_path, key_pem)?;
        std::fs::write(&ca_path, cert_pem)?;

        let settings = TlsSettings {
            bind_ip: "127.0.0.1".to_string(),
            port: 5061,
            cert_path: cert_path.to_string_lossy().to_string(),
            key_path: key_path.to_string_lossy().to_string(),
            ca_path: Some(ca_path.to_string_lossy().to_string()),
        };

        let _acceptor = build_tls_acceptor(&settings)?;
        Ok(())
    }
}
