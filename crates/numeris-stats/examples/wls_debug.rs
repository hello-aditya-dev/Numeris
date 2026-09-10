use numeris_stats::linalg::Matrix;
use numeris_stats::Ols;
fn main() {
    let w: Vec<f64> = (0..20).map(|i| 1.0 + i as f64 * 0.5).collect();
    let rows: Vec<Vec<f64>> = (0..20).map(|i| vec![i as f64]).collect();
    let x = Matrix::from_rows(&rows).unwrap();
    let y: Vec<f64> = (0..20).map(|i| 1.0 + i as f64).collect();
    let names = vec!["x".to_string()];
    let reg = Ols::new(&y, &x, &names).unwrap().weights(w).fit().unwrap();
    println!("coef = {:?} se = {:?} r2 = {}", reg.coef, reg.se, reg.r2);
    let ols = Ols::new(&y, &x, &names).unwrap().fit().unwrap();
    println!("ols coef = {:?}", ols.coef);
}
