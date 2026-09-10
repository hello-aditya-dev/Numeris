use numeris_core::csv::read_csv;
use numeris_core::frame::DataFrame;
fn main() {
    let text = "name,age,note\n\"Doe, Jane\",34,\"said \"\"hi\"\" twice\"\nBob,29,plain\n";
    let (df, report) = read_csv(text).unwrap();
    println!(
        "rows={} report_rows={} cols={}",
        df.n_rows(),
        report.rows,
        df.n_cols()
    );
    for v in &df.variables {
        println!("var: {} storage={:?}", v.name, v.storage);
    }

    let mut df2 = DataFrame::new();
    df2.add_numeric("x", vec![Some(3.0), None, Some(1.0), Some(2.0)])
        .unwrap();
    df2.add_text(
        "g",
        vec![Some("b".into()), Some("a".into()), None, Some("a".into())],
    )
    .unwrap();
    match df2.variable("nope") {
        Ok(_) => println!("?? found"),
        Err(e) => println!("ERR what: [{}] why: [{}]", e.what, e.why),
    }
}
