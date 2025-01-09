use std::{ascii::AsciiExt, ops::RangeBounds};

use sqlparser::dialect::Dialect;

#[derive(Debug, Default)]
pub struct TyrDialect;

impl Dialect for TyrDialect {
    fn is_identifier_start(&self, ch: char) -> bool {
        ch.is_ascii_lowercase() || ch.is_ascii_uppercase() || ch == '_'
    }

    fn is_identifier_part(&self, ch: char) -> bool {
        ch.is_ascii_lowercase()
            || ch.is_ascii_uppercase()
            || ch.is_ascii_digit()
            || [':', '/', '?', '&', '=', '-', '_', '.'].contains(&ch)
    }
}

pub fn example_sql() -> String {
    let url = "https://raw.githubusercontent.com/owid/covid-19-data/master/public/data/latest/owid-covid-latest.csv";
    let sql = format!(
        // "SELECT location name, total_cases, new_cases, total_deaths, new_deaths \
        // FROM {} where new_deaths >= 500 ORDER BY new_cases DESC LIMIT 6 OFFSET 5",
        // url
        "SELECT location name, total_cases, new_cases, total_deaths, new_deaths \
        FROM {} where new_deaths >= 500 ORDER BY new_cases DESC",
        url
    );

    sql
}

#[cfg(test)]
mod tests {
    use sqlparser::parser::Parser;

    use super::*;

    #[test]
    fn is_works() {
        assert!(Parser::parse_sql(&TyrDialect, &example_sql()).is_ok());
    }
}
