use crate::{
    error::KvError,
    pb::abi::{CommandResponse, Hget},
    storage::Storage,
    Hgetall, Hset, Value,
};

use super::CommandService;

impl CommandService for Hget {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        match store.get(&self.table, &self.key) {
            Ok(Some(v)) => v.into(),
            Ok(None) => KvError::NotFound(format!("table {}, key {}", self.table, self.key)).into(),
            Err(e) => e.into(),
        }
    }
}

impl CommandService for Hgetall {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        match store.get_all(&self.table) {
            Ok(v) => v.into(),
            Err(e) => e.into(),
        }
    }
}

impl CommandService for Hset {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        match self.pair {
            Some(v) => match store.set(&self.table, v.key, v.value.unwrap_or_default()) {
                Ok(Some(v)) => v.into(),
                Ok(None) => Value::default().into(),
                Err(e) => e.into(),
            },
            None => Value::default().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{dispatch, CommandRequest, Kvpair, MemTable};

    use super::*;

    #[test]
    fn hset_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hset("t1", "k1", "v1".into());
        let resp = dispatch(cmd.clone(), &store);
        assert_res_ok(resp, &[Value::default()], &[]);
        let res = dispatch(cmd, &store);
        assert_res_ok(res, &["v1".into()], &[]);
    }

    #[test]
    fn hget_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hset("score", "u1", 10.into());
        dispatch(cmd, &store);
        let cmd = CommandRequest::new_hget("score", "u1");
        let res = dispatch(cmd, &store);
        assert_res_ok(res, &[10.into()], &[]);
    }

    #[test]
    fn hget_with_no_exist_key_should_return_404() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hget("score", "u1");
        let res = dispatch(cmd, &store);
        println!("res: {:?}", res);
        assert_res_error(res, 404, "Not Found");
    }

    #[test]
    fn hget_all_should_work() {
        let store = MemTable::new();
        let cmds = vec![
            CommandRequest::new_hset("score", "u1", 10.into()),
            CommandRequest::new_hset("score", "u2", 8.into()),
            CommandRequest::new_hset("score", "u3", 11.into()),
            CommandRequest::new_hset("score", "u4", 6.into()),
        ];
        for cmd in cmds {
            dispatch(cmd, &store);
        }
        let cmd = CommandRequest::new_hgetall("score");
        let res = dispatch(cmd, &store);
        let pairs = &[
            Kvpair::new("u1", 6.into()),
            Kvpair::new("u2", 8.into()),
            Kvpair::new("u3", 11.into()),
        ];
        assert_res_ok(res, &[], pairs);
    }

    // fn dispath(cmd: CommandRequest, store: &impl Storage) -> CommandResponse {
    //     match cmd.request_data.unwrap() {
    //         RequestData::Hget(v) => v.execute(store),
    //         RequestData::Hset(v) => v.execute(store),
    //         // RequestData::Hdel(v) => v.execute(store),
    //         RequestData::Hgetall(v) => v.execute(store),
    //         // _ => KvError::InvalidCommand("Not Unimplemented".into()).into(),
    //         _ => todo!(),
    //     }
    // }

    fn assert_res_ok(mut res: CommandResponse, values: &[Value], _: &[Kvpair]) {
        res.pairs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(res.status, 200);
        assert!(res.message.is_empty());
        assert_eq!(res.values, values);
    }

    fn assert_res_error(res: CommandResponse, code: u32, msg: &str) {
        assert_eq!(res.status, code);
        assert!(res.message.contains(msg));
        assert_eq!(res.values, &[]);
        assert_eq!(res.pairs, &[]);
    }
}
