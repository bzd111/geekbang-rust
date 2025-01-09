use anyhow::Result;
use kv::{CommandRequest, ProstClientStream, TlsClientConnector};
use tokio::net::TcpStream;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let ca_cert = include_str!("../fixtures/ca.cert");

    let addr = "127.0.0.1:9527";
    let connector = TlsClientConnector::new("kvserver.acme.inc", None, Some(ca_cert))?;
    let stream = TcpStream::connect(addr).await?;
    let stream = connector.connect(stream).await?;
    let mut client = ProstClientStream::new(stream);

    let cmd = CommandRequest::new_hset("t1", "k1", "v1".into());
    let data = client.execute(cmd).await?;
    info!("Got response {:?}", data);
    Ok(())
}
