use std::{collections::HashMap, str::FromStr};

use anyhow::Result;
use clap::Parser;
use colored::*;
use mime::Mime;
use reqwest::{
    header::{self, HeaderMap, HeaderName, HeaderValue},
    Client, Response, Url,
};

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

#[derive(Parser, Debug)]
struct Opts {
    #[command(subcommand)]
    subcmd: SubCommand,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum SubCommand {
    Get(Get),
    Post(Post),
}

#[derive(Debug, Clone, Parser)]
struct Get {
    #[clap(value_parser = parse_url)]
    url: String,
}

#[derive(Debug, Clone, Parser)]
struct Post {
    #[clap(value_parser = parse_url)]
    url: String,
    #[clap(value_parser = parse_kv_pair)]
    body: Vec<KvPair>,
}

#[derive(Debug, Clone, Parser, PartialEq, Eq)]
struct KvPair {
    key: String,
    value: String,
}

impl FromStr for KvPair {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.splitn(2, '=');
        let key = split
            .next()
            .ok_or_else(|| anyhow::anyhow!("no key in {}", s))?
            .to_string();
        let value = split
            .next()
            .ok_or_else(|| anyhow::anyhow!("no value in {}", s))?
            .to_string();
        Ok(KvPair { key, value })
    }
}

fn parse_url(s: &str) -> Result<String> {
    let _url: Url = s.parse()?;
    Ok(s.into())
}

fn parse_kv_pair(s: &str) -> Result<KvPair> {
    s.parse()
}

async fn get(client: Client, args: &Get) -> Result<()> {
    let resp = client.get(&args.url).send().await?;
    println!("{:?}", resp.text().await?);
    Ok(())
}

async fn post(client: Client, args: &Post) -> Result<()> {
    let mut body = HashMap::new();
    for pair in args.body.iter() {
        body.insert(&pair.key, &pair.value);
    }
    let resp = client.post(&args.url).json(&body).send().await?;
    print_resp(resp).await?;
    Ok(())
}

fn print_status(resp: &reqwest::Response) {
    let status = format!("{:?} {}", resp.version(), resp.status()).blue();
    println!("{}\n", status);
}

fn print_headers(resp: &reqwest::Response) {
    let mut headers_vec: Vec<(HeaderName, HeaderValue)> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    headers_vec.sort_by(|(a, _), (b, _)| a.as_str().cmp(b.as_str()));

    for (key, value) in headers_vec {
        let mut key = key.to_string();
        key.replace_range(..1, &key[..1].to_uppercase());
        println!("{}: {:?}", key, value);
    }
    // for (name, value) in resp.headers() {
    //     println!("{}: {:?}", name, value);
    // }
    println!("\n");
}

fn print_syntect(s: &str) -> Result<()> {
    // Load these once at the start of your program
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ps.find_syntax_by_extension("json").unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    for line in LinesWithEndings::from(s) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        print!("{}", escaped);
    }
    Ok(())
}

fn print_body(m: Option<Mime>, body: &String) {
    match m {
        Some(v) => {
            if v == mime::APPLICATION_JSON {
                print_syntect(body).unwrap();
                // println!("{}", jsonxf::pretty_print(body).unwrap().cyan());
            }
        }
        _ => println!("{}\n", body),
    }
}

async fn print_resp(resp: Response) -> Result<()> {
    print_status(&resp);
    print_headers(&resp);
    let mime = get_content_type(&resp);
    let body = resp.text().await?;
    print_body(mime, &body);
    Ok(())
}

fn get_content_type(resp: &Response) -> Option<Mime> {
    resp.headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap().parse().unwrap())
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();
    let mut headers = HeaderMap::new();
    headers.insert("X-POWERED-BY", "Rust".parse()?);
    headers.insert(header::USER_AGENT, "Rust Httpie".parse()?);
    let client = reqwest::Client::new();
    match opts.subcmd {
        SubCommand::Get(ref subcmd) => {
            get(client, subcmd).await?;
        }
        SubCommand::Post(ref subcmd) => {
            post(client, subcmd).await?;
        }
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_works() {
        assert!(parse_url("abc").is_err());
        assert!(parse_url("http://123.com").is_ok());
        assert!(parse_url("https://123.com").is_ok());
    }

    #[test]
    fn parse_kv_pair_works() {
        assert!(parse_kv_pair("a").is_err());
        assert_eq!(
            parse_kv_pair("a=1").unwrap(),
            KvPair {
                key: "a".into(),
                value: "1".into()
            }
        );
        assert_eq!(
            parse_kv_pair("b=").unwrap(),
            KvPair {
                key: "b".into(),
                value: "".into()
            }
        );
    }
}
