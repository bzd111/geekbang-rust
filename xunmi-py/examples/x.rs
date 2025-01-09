use std::{str::FromStr, thread, time::Duration};
use xunmi::*;

fn main() {
    let config = IndexConfig::from_str(include_str!("../fixtures/config.yml")).unwrap();

    // let indexer = Indexer::new(&config).unwrap();
    let indexer = Indexer::open_or_create(config).unwrap();
    let content = include_str!("../fixtures/wiki_00.xml");
    let config = InputConfig::new(
        InputType::Xml,
        vec![("$value".into(), "content".into())],
        vec![("id".into(), (ValueType::String, ValueType::Number))],
    );

    let mut updater = indexer.get_updater();
    let mut updater1 = indexer.get_updater();

    updater.update(content, &config).unwrap();
    updater.commit().unwrap();

    thread::spawn(move || {
        let config = InputConfig::new(InputType::Yaml, vec![], vec![]);
        let text = include_str!("../fixtures/test.yml");
        updater1.update(text, &config).unwrap();
        updater1.commit().unwrap();
    });

    while indexer.num_docs() == 0 {
        thread::sleep(Duration::from_millis(100));
    }
    println!("total: {}", indexer.num_docs());

    let result = indexer.search("历史", &["title", "content"], 5, 0).unwrap();
    for (score, doc) in result.iter() {
        println!("score: {}, doc: {:?}", score, doc);
    }
}
