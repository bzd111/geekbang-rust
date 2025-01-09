use std::io::Cursor;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::rustls::{internal::pemfile, Certificate, ClientConfig, ServerConfig};
use tokio_rustls::rustls::{AllowAnyAuthenticatedClient, NoClientAuth, PrivateKey, RootCertStore};
use tokio_rustls::webpki::DNSNameRef;
use tokio_rustls::TlsConnector;
use tokio_rustls::{
    client::TlsStream as ClientTlsStream, server::TlsStream as ServerTlsStream, TlsAcceptor,
};
use tracing::instrument;

use crate::KvError;

const ALPN_KV: &str = "kv";

#[derive(Clone)]
pub struct TlsServerAcceptor {
    inner: Arc<ServerConfig>,
}

#[derive(Clone)]
pub struct TlsClientConnector {
    pub config: Arc<ClientConfig>,
    pub domain: Arc<String>,
}

impl TlsClientConnector {
    #[instrument(name = "tls_connector_new", skip_all)]
    pub fn new(
        domain: impl Into<String> + std::fmt::Debug,
        identity: Option<(&str, &str)>,
        server_ca: Option<&str>,
    ) -> Result<Self, KvError> {
        let mut config = ClientConfig::new();

        if let Some((cert, key)) = identity {
            let certs = load_certs(cert)?;
            let key = load_key(key)?;
            config.set_single_client_cert(certs, key)?;
        }

        // config.root_store = match rustls_native_certs::load_native_certs() {
        //     Ok(store) | Err((Some(store), _)) => store,
        //     Err((None, error)) => return Err(error.into()),
        // };
        // config.root_store = match rustls_native_certs::load_native_certs() {
        //     Ok(store) | Err((Some(store), _)) => store,
        //     Err((None, error)) => return Err(error.into()),
        // };

        if let Some(cert) = server_ca {
            let mut buf = Cursor::new(cert);
            config.root_store.add_pem_file(&mut buf).unwrap();
        }

        Ok(Self {
            config: Arc::new(config),
            domain: Arc::new(domain.into()),
        })
    }

    #[instrument(name = "tls_connector_connect", skip_all)]
    pub async fn connect<S>(&self, stream: S) -> Result<ClientTlsStream<S>, KvError>
    where
        S: AsyncRead + AsyncWrite + Send + Unpin,
    {
        let dns = DNSNameRef::try_from_ascii_str(&self.domain)
            .map_err(|_| KvError::Internal("Invalid DNS name".into()))?;
        // let stream = TlsClientConnector::from(self.config.clone())
        //     .connect(dns, stream)
        //     .await?;
        let stream = TlsConnector::from(self.config.clone())
            .connect(dns, stream)
            .await?;
        Ok(stream)
    }
}

impl TlsServerAcceptor {
    #[instrument(name = "tls_server_new", skip_all)]
    pub fn new(cert: &str, key: &str, client_ca: Option<&str>) -> Result<Self, KvError> {
        let certs = load_certs(cert)?;
        let key = load_key(key)?;

        let mut config = match client_ca {
            None => ServerConfig::new(NoClientAuth::new()),
            Some(cert) => {
                // 如果客户端证书是某个 CA 证书签发的，则把这个 CA 证书加载到信任链中
                let mut cert = Cursor::new(cert);
                let mut client_root_cert_store = RootCertStore::empty();
                client_root_cert_store
                    .add_pem_file(&mut cert)
                    .map_err(|_| KvError::CertifcateParseError("CA", "cert"))?;

                let client_auth = AllowAnyAuthenticatedClient::new(client_root_cert_store);
                ServerConfig::new(client_auth)
            }
        };

        // 加载服务器证书
        config
            .set_single_cert(certs, key)
            .map_err(|_| KvError::CertifcateParseError("server", "cert"))?;
        config.set_protocols(&[Vec::from(ALPN_KV)]);

        Ok(Self {
            inner: Arc::new(config),
        })
    }

    #[instrument(name = "tls_server_accept", skip_all)]
    pub async fn accept<S>(&self, stream: S) -> Result<ServerTlsStream<S>, KvError>
    where
        S: AsyncRead + AsyncWrite + Send + Unpin,
    {
        let acceptor = TlsAcceptor::from(self.inner.clone());
        Ok(acceptor.accept(stream).await?)
    }
}
fn load_certs(cert: &str) -> Result<Vec<Certificate>, KvError> {
    let mut cert = Cursor::new(cert);
    pemfile::certs(&mut cert).map_err(|_| KvError::CertifcateParseError("server", "cert"))
}

fn load_key(key: &str) -> Result<PrivateKey, KvError> {
    let mut cursor = Cursor::new(key);

    if let Ok(mut keys) = pemfile::pkcs8_private_keys(&mut cursor) {
        if !keys.is_empty() {
            return Ok(keys.remove(0));
        }
    }

    cursor.set_position(0);
    if let Ok(mut keys) = pemfile::rsa_private_keys(&mut cursor) {
        if !keys.is_empty() {
            return Ok(keys.remove(0));
        }
    }

    Err(KvError::CertifcateParseError("private", "key"))
}

#[cfg(test)]
pub mod tls_utils {
    use super::*;

    const CA_CERT: &str = include_str!("../../fixtures/ca.cert");
    const SERVER_CERT: &str = include_str!("../../fixtures/server.cert");
    const SERVER_KEY: &str = include_str!("../../fixtures/server.key");
    const CLIENT_CERT: &str = include_str!("../../fixtures/client.cert");
    const CLIENT_KEY: &str = include_str!("../../fixtures/client.key");

    pub fn tls_connector(client_cert: bool) -> Result<TlsClientConnector, KvError> {
        let ca = Some(CA_CERT);
        let client_identity = Some((CLIENT_CERT, CLIENT_KEY));

        match client_cert {
            false => TlsClientConnector::new("kvserver.acme.inc", None, ca),
            true => TlsClientConnector::new("kvserver.acme.inc", client_identity, ca),
        }
    }

    pub fn tls_acceptor(client_cert: bool) -> Result<TlsServerAcceptor, KvError> {
        let ca = Some(CA_CERT);
        match client_cert {
            true => TlsServerAcceptor::new(SERVER_CERT, SERVER_KEY, ca),
            false => TlsServerAcceptor::new(SERVER_CERT, SERVER_KEY, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use super::*;
    use anyhow::Result;
    use tls_utils::{tls_acceptor, tls_connector};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    // const CA_CERT: &str = include_str!("../../fixtures/ca.cert");
    // const SERVER_CERT: &str = include_str!("../../fixtures/server.cert");
    // const SERVER_KEY: &str = include_str!("../../fixtures/server.key");

    #[tokio::test]
    async fn tls_should_work() -> Result<()> {
        // let ca = Some(CA_CERT);
        let addr = start_server(false).await?;
        // let connector = TlsClientConnector::new("kvserver.acme.inc", None, ca)?;
        let connector = tls_connector(false)?;
        let stream = TcpStream::connect(addr).await?;
        // let mut stream = connector.connect("kvserver.acme.inc", stream).await?;
        let mut stream = connector.connect(stream).await?;
        stream.write_all(b"hello").await?;
        let mut buf = [0; 5];
        stream.read_exact(&mut buf).await?;
        assert_eq!(&buf, b"hello");
        Ok(())
    }

    #[tokio::test]
    async fn tls_with_client_cert_should_work() -> Result<()> {
        // let client_identity = Some((CLIENT_CERT, CLIENT_KEY));
        // let ca = Some(CA_CERT);
        let addr = start_server(true).await?;
        // let connector = TlsClientConnector::new("kvserver.acme.inc", client_identity, ca)?;
        let connector = tls_connector(true)?;
        let stream = TcpStream::connect(addr).await?;
        let mut stream = connector.connect(stream).await?;
        stream.write_all(b"hello").await?;
        let mut buf = [0; 5];
        stream.read_exact(&mut buf).await?;
        assert_eq!(&buf, b"hello");
        Ok(())
    }

    #[tokio::test]
    async fn tls_with_bad_domain_should_not_work() -> Result<()> {
        // let connector = TlsClientConnector::new("kvserver1.acme.inc", None, Some(CA_CERT))?;

        let addr = start_server(false).await?;
        let mut connector = tls_connector(false)?;
        connector.domain = Arc::new("kvserver1.acme.inc".into());
        let stream = TcpStream::connect(addr).await?;
        let result = connector.connect(stream).await;
        assert!(result.is_err());

        Ok(())
    }

    async fn start_server(client_cert: bool) -> Result<SocketAddr> {
        let acceptor = tls_acceptor(client_cert)?;
        // let acceptor = TlsServerAcceptor::new(SERVER_CERT, SERVER_KEY, ca)?;
        let echo = TcpListener::bind("127.0.0.1:0").await?;
        let addr = echo.local_addr()?;
        tokio::spawn(async move {
            let (stream, _) = echo.accept().await.unwrap();
            let mut stream = acceptor.accept(stream).await.unwrap();
            let mut buf = [0; 5];
            stream.read_exact(&mut buf).await.unwrap();
            stream.write_all(&buf).await.unwrap();
        });
        Ok(addr)
    }
}
