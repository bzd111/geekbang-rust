mod command_service;
mod topic;
mod topic_service;

use std::sync::Arc;

use futures::stream;
pub use topic::{Broadcaster, Topic};
pub use topic_service::{StreamingResponse, TopicService};

use tracing::{debug, instrument};

use crate::{
    error::KvError,
    pb::abi::{command_request::RequestData, CommandRequest, CommandResponse},
    storage::Storage,
    MemTable,
};

pub trait CommandService {
    fn execute(self, store: &impl Storage) -> CommandResponse;
}

pub trait Notify<Arg> {
    fn notify(&self, arg: &Arg);
}

pub trait NotifyMut<Arg> {
    fn notify(&self, arg: &mut Arg);
}

impl<Arg> Notify<Arg> for Vec<fn(&Arg)> {
    #[inline]
    fn notify(&self, arg: &Arg) {
        for f in self {
            f(arg);
        }
    }
}

impl<Arg> NotifyMut<Arg> for Vec<fn(&mut Arg)> {
    fn notify(&self, arg: &mut Arg) {
        for f in self {
            f(arg);
        }
    }
}

pub fn dispatch(cmd: CommandRequest, store: &impl Storage) -> CommandResponse {
    match cmd.request_data {
        Some(RequestData::Hget(param)) => param.execute(store),
        Some(RequestData::Hgetall(param)) => param.execute(store),
        Some(RequestData::Hset(param)) => param.execute(store),
        // Some(RequestData::Hmget(param)) => param.execute(store),
        // Some(RequestData::Hmset(param)) => param.execute(store),
        // Some(RequestData::Hdel(param)) => param.execute(store),
        // Some(RequestData::Hmdel(param)) => param.execute(store),
        // Some(RequestData::Hexist(param)) => param.execute(store),
        // Some(RequestData::Hmexist(param)) => param.execute(store),
        None => KvError::InvalidCommand("Request has no data".into()).into(),
        // _ => KvError::InvalidCommand("Not Unimplemented".into()).into(),
        _ => CommandResponse::default(),
    }
}

pub fn dispatch_stream(cmd: CommandRequest, topic: impl Topic) -> StreamingResponse {
    match cmd.request_data {
        Some(RequestData::Publish(param)) => param.execute(topic),
        Some(RequestData::Subscribe(param)) => param.execute(topic),
        Some(RequestData::Unsubscribe(param)) => param.execute(topic),
        _ => unreachable!(),
    }
}

pub struct Service<Store = MemTable> {
    inner: Arc<ServiceInner<Store>>,
    broadcaster: Arc<Broadcaster>,
}

impl<Store> Clone for Service<Store> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            broadcaster: Arc::clone(&self.broadcaster),
        }
    }
}

impl<Store: Storage> Service<Store> {
    // pub fn new(store: Store) -> Self {
    //     Self {
    //         inner: Arc::new(ServiceInner { store }),
    //     }
    // }

    #[instrument(name = "service_execute", skip_all)]
    pub fn execute(&self, cmd: CommandRequest) -> StreamingResponse {
        debug!("Got request: {:?}", cmd);
        self.inner.on_received.notify(&cmd);
        let mut res = dispatch(cmd.clone(), &self.inner.store);
        if res == CommandResponse::default() {
            dispatch_stream(cmd, Arc::clone(&self.broadcaster))
        } else {
            debug!("Executed response: {:?}", res);
            self.inner.on_executed.notify(&res);
            self.inner.on_before_send.notify(&mut res);
            if !self.inner.on_before_send.is_empty() {
                debug!("Modified response: {:?}", res);
            }
            Box::pin(stream::once(async { Arc::new(res) }))
        }
    }
}

pub struct ServiceInner<Store> {
    store: Store,
    on_received: Vec<fn(&CommandRequest)>,
    on_executed: Vec<fn(&CommandResponse)>,
    on_before_send: Vec<fn(&mut CommandResponse)>,
    on_after_send: Vec<fn()>,
}

impl<Store> ServiceInner<Store> {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            on_received: vec![],
            on_executed: vec![],
            on_before_send: Vec::new(),
            on_after_send: vec![],
        }
    }

    pub fn fn_received(mut self, f: fn(&CommandRequest)) -> Self {
        self.on_received.push(f);
        self
    }

    pub fn fn_executed(mut self, f: fn(&CommandResponse)) -> Self {
        self.on_executed.push(f);
        self
    }

    // pub fn fn_before_send(mut self, f: fn(&mut CommandResponse)) -> Self {
    //     self.on_before_send.push(f);
    //     self
    // }

    pub fn fn_before_send(mut self, f: fn(&mut CommandResponse)) -> Self {
        self.on_before_send.push(f);
        self
    }

    pub fn fn_after_send(mut self, f: fn()) -> Self {
        self.on_after_send.push(f);
        self
    }
}

impl<Store: Storage> From<ServiceInner<Store>> for Service<Store> {
    fn from(value: ServiceInner<Store>) -> Self {
        Self {
            inner: Arc::new(value),
            broadcaster: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use http::StatusCode;
    use tracing::info;

    use crate::{MemTable, Value};

    use super::*;
    #[tokio::test]
    async fn service_should_works() {
        let service: Service = ServiceInner::new(MemTable::default()).into();
        let cloned = service.clone();

        // let handle = thread::spawn(async move {
        //     let mut res = cloned.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
        //     let data = res.next().await.unwrap();
        //     assert_res_ok(&data, &[Value::default()], &[]);
        // });
        // handle.join().unwrap();

        tokio::spawn(async move {
            let mut res = cloned.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
            let data = res.next().await.unwrap();
            assert_res_ok(&data, &[Value::default()], &[]);
        })
        .await
        .unwrap();

        let mut res = service.execute(CommandRequest::new_hget("t1", "k1"));
        let data = res.next().await.unwrap();
        assert_res_ok(&data, &["v1".into()], &[]);
    }

    #[tokio::test]
    async fn event_registration_should_work() {
        fn b(cmd: &CommandRequest) {
            info!("Got {:?}", cmd);
        }

        fn c(res: &CommandResponse) {
            info!("{:?}", res);
        }

        fn d(res: &mut CommandResponse) {
            res.status = StatusCode::CREATED.as_u16() as _;
        }

        fn e() {
            info!("Data is send")
        }

        let service: Service = ServiceInner::new(MemTable::default())
            .fn_received(|_: &CommandRequest| {})
            .fn_received(b)
            .fn_executed(c)
            .fn_before_send(d)
            .fn_after_send(e)
            .into();
        let mut res = service.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
        let data = res.next().await.unwrap();
        assert_eq!(data.status, StatusCode::CREATED.as_u16() as _);
        assert_eq!(data.message, "");
        assert_eq!(data.values, vec![Value::default()]);
        // assert_eq!(res.pairs, vec![Value::default()]);
    }
}

#[cfg(test)]
use crate::{Kvpair, Value};

#[cfg(test)]
pub fn assert_res_ok(res: &CommandResponse, values: &[Value], pairs: &[Kvpair]) {
    let mut sorted_pairs = res.pairs.clone();
    sorted_pairs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // res.pairs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(res.status, 200);
    assert_eq!(res.message, "");
    assert_eq!(res.values, values);
    assert_eq!(res.pairs, pairs);
}

#[cfg(test)]
pub fn assert_res_error(res: &CommandResponse, code: u32, msg: &str) {
    assert_eq!(res.status, code);
    assert!(res.message.contains(msg));
    assert_eq!(res.values, &[]);
    assert_eq!(res.pairs, &[]);
}
