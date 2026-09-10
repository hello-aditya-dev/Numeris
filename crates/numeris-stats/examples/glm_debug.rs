use numeris_stats::glm::{fit_glm, Family};
use numeris_stats::linalg::Matrix;

fn main() {
    let text = "employed,education,experience\n1,16,4\n0,12,2\n1,15,6\n0,11,1\n1,14,5\n0,13,2\n1,17,8\n0,12,3\n1,16,7\n0,10,2\n";
    let (df, _) = numeris_core::csv::read_csv(text).unwrap();
    let idx = df.complete_case_indices(&["education"]).unwrap();
    let y = df.numeric_vector("employed", &idx).unwrap();
    let x = Matrix::from_rows(&df.numeric_matrix(&["education"], &idx).unwrap()).unwrap();
    match fit_glm(&y, &x, &["education".to_string()], Family::Probit, false) {
        Ok(r) => println!(
            "probit OK: iters={} coef={:?} ll={}",
            r.iterations, r.coef, r.loglik
        ),
        Err(e) => println!("probit ERR: {}", e.what),
    }
    match fit_glm(&y, &x, &["education".to_string()], Family::Logit, false) {
        Ok(r) => println!(
            "logit OK: iters={} coef={:?} ll={}",
            r.iterations, r.coef, r.loglik
        ),
        Err(e) => println!("logit ERR: {}", e.what),
    }
}
