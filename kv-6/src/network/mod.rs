mod frame;
mod stream;
mod tls;

pub use frame::FrameCoder;
use futures::{SinkExt, StreamExt};
use stream::ProstStream;
pub use tls::{TlsClientConnector, TlsServerAcceptor};
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::info;

use crate::{CommandRequest, CommandResponse, KvError, Service};

pub struct ProstServerStream<S> {
    // inner: S,
    inner: ProstStream<S, CommandRequest, CommandResponse>,
    service: Service,
}

pub struct ProstClientStream<S> {
    // inner: S,
    inner: ProstStream<S, CommandResponse, CommandRequest>,
}

impl<S> ProstServerStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S, service: Service) -> Self {
        Self {
            // inner: stream,
            inner: ProstStream::new(stream),
            service,
        }
    }

    pub async fn process(mut self) -> Result<(), KvError> {
        let stream = &mut self.inner;
        // while let Ok(cmd) = self.recv().await {
        while let Some(Ok(cmd)) = stream.next().await {
            info!("Got a new command: {:?}", cmd);
            let res = self.service.execute(cmd);
            stream.send(res).await?;
        }
        Ok(())
    }

    // async fn send(&mut self, msg: CommandResponse) -> Result<(), KvError> {
    //     let mut buf = BytesMut::new();
    //     msg.encode_frame(&mut buf)?;
    //     let encode = buf.freeze();
    //     self.inner.write_all(&encode[..]).await?;
    //     Ok(())
    // }
    //
    // async fn recv(&mut self) -> Result<CommandRequest, KvError> {
    //     let mut buf = BytesMut::new();
    //     let stream = &mut self.inner;
    //     read_frame(stream, &mut buf).await?;
    //     CommandRequest::decode_frame(&mut buf)
    // }
}

impl<S> ProstClientStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S) -> Self {
        // Self { inner: stream }
        Self {
            inner: ProstStream::new(stream),
        }
    }

    pub async fn execute(&mut self, cmd: CommandRequest) -> Result<CommandResponse, KvError> {
        // self.send(cmd).await?;
        // Ok(self.recv().await?)
        let stream = &mut self.inner;
        stream.send(cmd).await?;
        match stream.next().await {
            Some(v) => v,
            None => Err(KvError::Internal("Didn't get any response".into())),
        }
    }

    // async fn send(&mut self, msg: CommandRequest) -> Result<(), KvError> {
    //     let mut buf = BytesMut::new();
    //     msg.encode_frame(&mut buf)?;
    //     let encoded = buf.freeze();
    //     self.inner.write_all(&encoded[..]).await?;
    //     Ok(())
    // }
    //
    // async fn recv(&mut self) -> Result<CommandResponse, KvError> {
    //     let mut buf = BytesMut::new();
    //     let stream = &mut self.inner;
    //     read_frame(stream, &mut buf).await?;
    //     CommandResponse::decode_frame(&mut buf)
    // }
}

#[cfg(test)]
mod tests {

    use std::net::SocketAddr;

    use anyhow::Result;
    use bytes::Bytes;
    use tokio::net::TcpListener;
    use tokio::net::TcpStream;

    use crate::{assert_res_ok, MemTable, ServiceInner, Value};

    use super::*;

    #[tokio::test]
    async fn client_server_basic_communication_should_work() -> anyhow::Result<()> {
        let addr = start_server().await?;
        let stream = TcpStream::connect(addr).await?;
        let mut client = ProstClientStream::new(stream);
        let cmd = CommandRequest::new_hset("t1", "k1", "v1".into());
        let res = client.execute(cmd).await.unwrap();

        assert_res_ok(res, &[Value::default()], &[]);

        let cmd = CommandRequest::new_hget("t1", "k1");
        let res = client.execute(cmd).await?;

        assert_res_ok(res, &["v1".into()], &[]);
        Ok(())
    }

    #[tokio::test]
    async fn client_server_compression_should_work() -> anyhow::Result<()> {
        let addr = start_server().await?;
        let stream = TcpStream::connect(addr).await?;
        let mut client = ProstClientStream::new(stream);
        let v: Value = Bytes::from(vec![0u8; 16384]).into();
        let cmd = CommandRequest::new_hset("t2", "k2", v.clone().into());
        let res = client.execute(cmd).await?;

        assert_res_ok(res, &[Value::default()], &[]);

        let cmd = CommandRequest::new_hget("t2", "k2");
        let res = client.execute(cmd).await?;
        assert_res_ok(res, &[v.into()], &[]);
        Ok(())
    }

    async fn start_server() -> Result<SocketAddr> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let service: Service = ServiceInner::new(MemTable::new()).into();
                let server = ProstServerStream::new(stream, service);
                tokio::spawn(server.process());
            }
        });
        Ok(addr)
    }
}

#[cfg(test)]
pub mod utils {
    use std::task::Poll;

    use bytes::{BufMut, BytesMut};

    use super::*;

    pub struct DummyStream {
        pub buf: BytesMut,
    }

    impl AsyncRead for DummyStream {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            let len = buf.capacity();
            let data = self.get_mut().buf.split_to(len);
            buf.put_slice(&data);
            std::task::Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for DummyStream {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<Result<usize, std::io::Error>> {
            self.get_mut().buf.put_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), std::io::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), std::io::Error>> {
            Poll::Ready(Ok(()))
        }
    }
}
