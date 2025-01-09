use futures::{stream, Stream};
use std::{pin::Pin, sync::Arc};
use tokio_stream::wrappers::ReceiverStream;

use crate::{CommandResponse, Publish, Subscribe, Topic, Unsubscribe};

pub type StreamingResponse = Pin<Box<dyn Stream<Item = Arc<CommandResponse>> + Send>>;

// use std::{pin::Pin, sync::Arc};
//
// use futures::{stream, Stream};
// use tokio_stream::wrappers::ReceiverStream;
//
// use crate::{CommandResponse, Publish, Subscribe, Topic, Unsubscribe};

// pub type StreamingResponse = Pin<Box<dyn Stream<Item = Arc<CommandResponse>> + Send>>;

pub trait TopicService {
    fn execute(self, topic: impl Topic) -> StreamingResponse;
    // fn execute(self, topic: impl Topic) -> StreamingResponse;

    // fn execute(self, topic: impl Topic) -> StreamingResponse;
}

impl TopicService for Subscribe {
    fn execute(self, topic: impl Topic) -> StreamingResponse {
        // let rx = topic.subscribe(self.topic);
        // Box::pin(ReceiverStream::new(rx))
        let rx = topic.subscribe(self.topic);
        Box::pin(ReceiverStream::new(rx))
    }
}

impl TopicService for Unsubscribe {
    fn execute(self, topic: impl Topic) -> StreamingResponse {
        // topic.unsubscribe(self.topic, self.id);
        // Box::pin(stream::once(async { Arc::new(CommandResponse::ok()) }))
        let res = match topic.unsubscribe(self.topic, self.id) {
            Ok(_) => CommandResponse::ok(),
            Err(e) => e.into(),
        };
        Box::pin(stream::once(async { Arc::new(res) }))
    }
}

impl TopicService for Publish {
    fn execute(self, topic: impl Topic) -> StreamingResponse {
        // topic.publish(self.topic, Arc::new(self.dat.into()));
        // Box::pin(stream::once(async { Arc::new(CommandResponse::ok()) }))

        topic.publish(self.topic, Arc::new(self.dat.into()));
        Box::pin(stream::once(async { Arc::new(CommandResponse::ok()) }))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time;

    use crate::{assert_res_ok, dispatch_stream, service::topic::Broadcaster, CommandRequest};
    use futures::StreamExt;

    use super::*;

    #[tokio::test]
    async fn dispatch_publish_should_work() {
        let topic = Arc::new(Broadcaster::default());
        let cmd = CommandRequest::new_publish("t1", vec!["hello".into()]);
        let mut res = dispatch_stream(cmd, topic);
        let data = res.next().await.unwrap();
        assert_res_ok(&data, &[], &[]);
    }

    #[tokio::test]
    async fn dispatch_subscribe_should_work() {
        let topic = Arc::new(Broadcaster::default());
        let cmd = CommandRequest::new_subscribe("t1");
        let mut res = dispatch_stream(cmd, topic);
        let id: i64 = res.next().await.unwrap().as_ref().try_into().unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn dispatch_subscribe_abnormal_quit_should_be_removed_on_next_publish() {
        let topic = Arc::new(Broadcaster::default());
        let id = {
            let cmd = CommandRequest::new_subscribe("t1");
            let mut res = dispatch_stream(cmd, topic.clone());
            // let id: i64 = res.next().await.unwrap().as_ref().try_into().unwrap();
            let id = get_id(&mut res).await;
            drop(res);
            id as u32
        };

        let cmd = CommandRequest::new_publish("t1", vec!["hello".into()]);
        let _ = dispatch_stream(cmd, topic.clone());
        time::sleep(Duration::from_millis(10)).await;

        println!("id: {:?}", id);
        let result = topic.unsubscribe("t1".into(), id);
        // match result {
        //     Ok(_) => {
        //         println!("success");
        //     }
        //     Err(e) => {
        //         println!("{}", e)
        //     }
        // }
        assert!(result.is_err());
        // assert!(result.is_ok());
    }

    #[tokio::test]
    async fn dispatch_unsubscribe_should_work() {
        let topic = Arc::new(Broadcaster::default());
        let cmd = CommandRequest::new_subscribe("t1");
        let mut res = dispatch_stream(cmd, topic.clone());
        let id: i64 = res.next().await.unwrap().as_ref().try_into().unwrap();
        let cmd = CommandRequest::new_unsubscribe("t1", id as _);
        let mut res = dispatch_stream(cmd, topic);
        let data = res.next().await.unwrap();
        assert_res_ok(&data, &[], &[]);
    }

    #[tokio::test]
    async fn dispatch_unsubscribe_random_id_should_error() {
        let topic = Arc::new(Broadcaster::default());
        let cmd = CommandRequest::new_unsubscribe("t1", 1230);
        let _ = dispatch_stream(cmd, topic);
    }

    pub async fn get_id(res: &mut StreamingResponse) -> u32 {
        let id: i64 = res.next().await.unwrap().as_ref().try_into().unwrap();
        id as u32
    }
}
