use sled::{Db, IVec};
use std::{convert::TryInto, path::Path, str};

use crate::{Kvpair, Storage, StorageIter, Value};

#[derive(Debug)]
pub struct SledDb(Db);

impl SledDb {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self(sled::open(path).unwrap())
    }

    fn get_full_key(table: &str, key: &str) -> String {
        format!("{}:{}", table, key)
    }

    fn get_table_prefix(table: &str) -> String {
        format!("{}:", table)
    }
}

fn flip<T, E>(x: Option<Result<T, E>>) -> Result<Option<T>, E> {
    x.map_or(Ok(None), |x| x.map(Some))
}

impl Storage for SledDb {
    fn get(&self, table: &str, key: &str) -> Result<Option<crate::Value>, crate::KvError> {
        let name = Self::get_full_key(table, key);
        let result = self.0.get(name.as_bytes())?.map(|v| v.as_ref().try_into());
        flip(result)
    }

    fn set(
        &self,
        table: &str,
        key: impl Into<String>,
        value: impl Into<Value>,
    ) -> Result<Option<crate::Value>, crate::KvError> {
        let key = key.into();
        let name = Self::get_full_key(table, &key);
        let data: Vec<u8> = value.into().try_into()?;
        let result = self.0.insert(name, data)?.map(|v| v.as_ref().try_into());
        flip(result)
    }

    fn contains(&self, table: &str, key: &str) -> Result<bool, crate::KvError> {
        let name = Self::get_full_key(table, key);
        Ok(self.0.contains_key(name)?)
    }

    fn del(&self, table: &str, key: &str) -> Result<Option<crate::Value>, crate::KvError> {
        let name = Self::get_full_key(table, key);
        let result = self.0.remove(name)?.map(|v| v.as_ref().try_into());
        flip(result)
    }

    fn get_all(&self, table: &str) -> Result<Vec<crate::Kvpair>, crate::KvError> {
        let prefix = Self::get_table_prefix(table);
        let result = self.0.scan_prefix(prefix).map(|v| v.into()).collect();
        Ok(result)
    }

    fn get_iter(
        &self,
        table: &str,
    ) -> Result<Box<dyn Iterator<Item = crate::Kvpair>>, crate::KvError> {
        let prefix = Self::get_table_prefix(table);
        let iter = StorageIter::new(self.0.scan_prefix(prefix));
        Ok(Box::new(iter))
    }
}

impl From<Result<(IVec, IVec), sled::Error>> for Kvpair {
    fn from(v: Result<(IVec, IVec), sled::Error>) -> Self {
        match v {
            Ok((k, v)) => match v.as_ref().try_into() {
                Ok(v) => Kvpair::new(ivec_to_key(k.as_ref()), v),
                Err(_) => Kvpair::default(),
            },
            _ => Kvpair::default(),
        }
    }
}

fn ivec_to_key(ivec: &[u8]) -> &str {
    let from_utf8 = str::from_utf8(ivec);
    let s = from_utf8.unwrap();
    let mut iter = s.split(":");
    iter.next();
    iter.next().unwrap()
}
