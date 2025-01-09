use pyo3::prelude::*;

#[pyfunction]
pub fn example_sql() -> PyResult<String> {
    Ok(queryer::example_sql())
}

#[pyfunction]
pub fn query(sql: &str, output: Option<&str>) -> PyResult<String> {
    let rt = tokio::runtime::Runtime::new()?;
    let mut data = rt.block_on(async { queryer::query(sql).await.unwrap() });
    match output {
        Some("csv") | None => Ok(data.to_csv().unwrap()),
        // Some(v) => Err(PyTypeError::new_err(format!(
        //     "Output type {} is not supported",
        //     v
        // ),
        Some(&_) => todo!(),
    }
}

#[pymodule]
fn queryer_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(example_sql, m)?)?;
    m.add_function(wrap_pyfunction!(query, m)?)?;
    Ok(())
}
