mod convert;
mod dialect;
mod fetcher;
mod loader;

use std::ops::{Deref, DerefMut};

use anyhow::{anyhow, Result};
use convert::Sql;
pub use dialect::{example_sql, TyrDialect};
use fetcher::retrieve_data;
use loader::detect_content;
use polars::prelude::*;
use polars::{
    frame::DataFrame,
    io::{csv::write::CsvWriter, SerWriter},
};
use sqlparser::parser::Parser;
use tracing::info;

#[derive(Debug)]
pub struct DataSet(DataFrame);

impl Deref for DataSet {
    type Target = DataFrame;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DataSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DataSet {
    pub fn to_csv(&mut self) -> Result<String> {
        let mut buf = Vec::new();
        let mut writer = CsvWriter::new(&mut buf);
        writer.finish(&mut self.0)?;
        Ok(String::from_utf8(buf)?)
    }
}

pub async fn query<T: AsRef<str>>(sql: T) -> Result<DataSet> {
    let ast = Parser::parse_sql(&TyrDialect::default(), sql.as_ref())?;
    if ast.len() != 1 {
        return Err(anyhow!("Only support single sql at the moment"));
    }

    let sql = &ast[0];

    let Sql {
        source,
        condition,
        selection,
        offset,
        limit,
        order_by,
    } = sql.try_into()?;

    info!("retrieving data from {}", source);

    let ds = detect_content(retrieve_data(source).await?).load()?;

    let mut filtered = match condition {
        Some(expr) => ds.0.lazy().filter(expr),
        None => ds.0.lazy(),
    };

    // filtered = order_by
    //     .into_iter()
    //     .fold(filtered, |acc, (col, desc)| acc.sort(col.as_str(), desc));
    if offset.is_some() || limit.is_some() {
        // filtered = filtered.slice(offset.unwrap_or(0), limit.unwrap_or(usize::MAX));
        filtered = filtered.slice(
            offset.unwrap_or(0),
            limit.unwrap_or(usize::MAX).try_into().unwrap(),
        );
    }
    Ok(DataSet(filtered.select(selection).collect()?))
}
