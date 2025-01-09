use anyhow::Result;
use queryer::query;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let url = "https://raw.githubusercontent.com/owid/covid-19-data/master/public/data/latest/owid-covid-latest.csv";
    // let data = reqwest::get(url).await?.text().await?;
    // let cursor = Cursor::new(data.clone());
    // let df = CsvReader::new(cursor).finish()?;
    // let mask = df.column("new_deaths")?.gt(500)?;
    // let filtered = df.filter(&mask)?;
    //
    // println!(
    //     "{:?}",
    //     filtered.select([
    //         "location",
    //         "total_cases",
    //         "new_cases",
    //         "total_deaths",
    //         "new_deaths"
    //     ])
    // );

    // let sql = format!(
    //     "SELECT location name, total_cases, new_cases, total_deaths, new_deaths \
    //     FROM {} where new_deaths >= 500 ORDER BY new_cases DESC",
    //     url
    // );
    let sql = format!(
        "SELECT location name, total_cases, new_cases, total_deaths, new_deaths \
        FROM {} where new_deaths >= 500 ORDER BY new_cases DESC LIMIT 2 OFFSET 3",
        url
    );

    let df1 = query(sql).await?;
    println!("{:?}", df1);

    Ok(())
}
