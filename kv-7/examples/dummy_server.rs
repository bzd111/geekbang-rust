use anyhow::Result;
use bytes::BytesMut;
use futures::{SinkExt, StreamExt};
use kv::{CommandRequest, MemTable, Service, ServiceInner};
use prost::Message;
use tokio::net::TcpListener;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let service: Service = ServiceInner::new(MemTable::new()).into();
    let addr = "127.0.0.1:9527";
    let listener = TcpListener::bind(addr).await?;
    info!("Start dummy server at: {}", addr);
    let svc = service.clone();
    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Client {:?} connected", addr);
        // let svc = svc.clone();
        tokio::spawn(async move {
            let mut stream = Framed::new(stream, LengthDelimitedCodec::new());
            while let Some(Ok(bytes)) = stream.next().await {
                let cmd = match CommandRequest::decode(bytes) {
                    Ok(cmd) => cmd,
                    Err(e) => {
                        info!("Failed to decode request: {:?}", e);
                        continue;
                    }
                };
                info!("Got a new command: {:?}", cmd);
                // let res = svc.execute(cmd);
                // let mut buf = BytesMut::new();
                // // res.encode(&mut buf).unwrap();
                // if let Err(e) = stream.send(buf.freeze()).await {
                //     info!("Failed to send response: {:?}", e);
                //     break;
                // }
            }
            info!("Client {:?} disconnected", addr);
        });
    }
}
