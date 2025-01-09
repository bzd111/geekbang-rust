use anyhow::Result;
use bytes::BytesMut;
use futures::{SinkExt, StreamExt};
use kv::{CommandRequest, Service, ServiceInner, SledDb};
use prost::Message;
use tokio::net::TcpListener;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    // let service = Service::new(MemTable::new());
    let service: Service<SledDb> = ServiceInner::new(SledDb::new("/tmp/kvserver"))
        .fn_before_send(|res| match res.message.as_ref() {
            "" => res.message = "altered. Original message is empty.".into(),
            s => res.message = format!("altered: {}", s),
        })
        .into();
    let addr = "127.0.0.1:9527";
    let listener = TcpListener::bind(addr).await?;
    info!("Start dummy server at: {}", addr);
    let svc = service.clone();
    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Client {:?} connected", addr);
        let svc = svc.clone();
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
                let res = svc.execute(cmd);
                let mut buf = BytesMut::new();
                res.encode(&mut buf).unwrap();
                if let Err(e) = stream.send(buf.freeze()).await {
                    info!("Failed to send response: {:?}", e);
                    break;
                }
            }
            info!("Client {:?} disconnected", addr);
        });
    }
}
