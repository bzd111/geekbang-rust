use anyhow::Result;
use bytes::BytesMut;
use futures::{SinkExt, StreamExt};
use kv::{CommandRequest, CommandResponse};
use prost::Message;
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let addr = "127.0.0.1:9527";
    let stream = TcpStream::connect(addr).await?;

    let mut stream = Framed::new(stream, LengthDelimitedCodec::new());

    let cmd = CommandRequest::new_hset("t1", "k1", "v1".into());
    let mut buf = BytesMut::new();
    cmd.encode(&mut buf)?;
    stream.send(buf.freeze()).await?;

    if let Some(Ok(data)) = stream.next().await {
        let resp = CommandResponse::decode(data)?;
        info!("Got response {:?}", resp);
    }

    let cmd2 = CommandRequest::new_hget("t1", "k1");
    let mut buf = BytesMut::new();
    cmd2.encode(&mut buf)?;
    stream.send(buf.freeze()).await?;

    if let Some(Ok(data)) = stream.next().await {
        let resp = CommandResponse::decode(data)?;
        info!("Got response {:?}", resp);
    }

    Ok(())
}
