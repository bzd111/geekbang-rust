use clap::Parser;
use colored::*;
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use regex::Regex;
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read, Stdout, Write},
    ops::Range,
    path::Path,
};

mod error;
pub use error::GrepError;

pub type StrategyFn<W, R> = fn(&Path, BufReader<R>, &mut W, &Regex) -> Result<(), GrepError>;

// #[clap(version = "1.0", author = "bzd111")]
// #[clap(setting = AppSettings::ColoredHelp)]
#[derive(Parser)]
pub struct GrepConfig {
    pattern: String,
    glob: String,
}

impl GrepConfig {
    pub fn match_with_default_strategy(&self) -> Result<(), GrepError> {
        self.match_with(default_strategy)
    }

    pub fn match_with(&self, strategy: StrategyFn<Stdout, File>) -> Result<(), GrepError> {
        let regex = Regex::new(&self.pattern)?;
        let files: Vec<_> = glob::glob(&self.glob).unwrap().collect();

        files.into_par_iter().for_each(|v| {
            if let Ok(filename) = v {
                if let Ok(file) = File::open(&filename) {
                    let reader = BufReader::new(file);
                    let mut stdout = io::stdout();
                    if let Err(e) = strategy(filename.as_path(), reader, &mut stdout, &regex) {
                        println!("Internal error: {:?}", e);
                    }
                }
            }
        });
        Ok(())
    }
}

pub fn default_strategy<W: Write, R: Read>(
    path: &Path,
    reader: BufReader<R>,
    writer: &mut W,
    pattern: &Regex,
) -> Result<(), GrepError> {
    let matcher: String = reader
        .lines()
        .enumerate()
        .map(|(lineno, line)| {
            line.ok().and_then(|line| {
                pattern
                    .find(&line)
                    .map(|m| format_line(&line, lineno + 1, m.range()))
            })
        })
        .filter_map(|v| v.ok_or(()).ok())
        .join("\n");
    if !matcher.is_empty() {
        writer.write_all(path.display().to_string().green().as_bytes())?;
        writer.write_all(b":\n")?;
        writer.write_all(matcher.as_bytes())?;
        writer.write_all(b"\n")?;
    }
    Ok(())
}

pub fn format_line(line: &str, lineno: usize, range: Range<usize>) -> String {
    let Range { start, end } = range;
    let prefix = &line[..start];
    format! {
        "{0: >6}:{1: <3} {2}{3}{4}",
        lineno.to_string().blue(),
        (prefix.chars().count()+1).to_string().cyan(),
        prefix,
        &line[start..end].red(),
        &line[end..]
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn format_line_should_work() {
        let result = format_line("Hello, aaa~", 1000, 7..10);
        let expected = format!(
            "{0: >6}:{1: <3} Hello, {2}~",
            "1000".blue(),
            "8".cyan(),
            "aaa".red(),
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn default_strategy_should_work() {
        let path = Path::new("src/main.rs");
        let input = b"hello world!\nhey aaa";
        let reader = BufReader::new(&input[..]);
        let pattern = Regex::new(r"he\w+").unwrap();
        let mut writer = Vec::new();
        default_strategy(path, reader, &mut writer, &pattern).unwrap();
        let result = String::from_utf8(writer).unwrap();
        let expected = [
            String::from("src/main.rs:"),
            format_line("hello world!", 1, 0..11),
            format_line("hey aaa\n", 2, 0..3),
        ];
        assert_eq!(result, expected.join("\n"));
    }
}
