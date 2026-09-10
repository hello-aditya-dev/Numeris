use numeris_core::describe::mean;
use numeris_stats::iv::fit_2sls;
use numeris_stats::linalg::Matrix;
use numeris_stats::vcov::VcovSpec;

fn cov(a: &[f64], b: &[f64]) -> f64 {
    let ma = mean(a).unwrap();
    let mb = mean(b).unwrap();
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - ma) * (y - mb))
        .sum::<f64>()
        / (a.len() as f64 - 1.0)
}

fn main() {
    // Exactly the test DGP.
    let n = 400;
    let mut z1 = Vec::new();
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut idx = 0u64;
    for i in 0..n {
        let zi = ((i * 37) % 100) as f64 / 10.0;
        let u1 = (((idx.wrapping_mul(6364136223846793005).wrapping_add(1)) >> 33) % 100) as f64
            / 50.0
            - 1.0;
        let u2 = (((idx
            .wrapping_mul(2862933555777941757)
            .wrapping_add(4)
            .wrapping_add(idx))
            >> 33)
            % 100) as f64
            / 50.0
            - 1.0;
        idx = idx.wrapping_add(1);
        let xi = 0.5 + 0.9 * zi + 0.25 * u1;
        let yi = 1.0 + 2.0 * xi + 0.5 * u2;
        z1.push(zi);
        x.push(xi);
        y.push(yi);
    }
    // Analytic exactly-identified IV: beta = Cov(z,y)/Cov(z,x).
    let czx = cov(&z1, &x);
    let czy = cov(&z1, &y);
    println!("analytic beta = {}", czy / czx);
    println!("cov(z,x) = {czx}, cov(z,u2) check:");

    let xmat = Matrix::from_rows(&x.iter().map(|&v| vec![v]).collect::<Vec<_>>()).unwrap();
    let zmat = Matrix::from_rows(&z1.iter().map(|&v| vec![v]).collect::<Vec<_>>()).unwrap();
    let empty_rows: Vec<Vec<f64>> = (0..n).map(|_| vec![]).collect();
    let empty = Matrix::from_rows(&empty_rows).unwrap();
    let names = vec!["x".to_string()];
    match fit_2sls(&y, &xmat, &empty, &zmat, &names, &VcovSpec::Classical) {
        Ok(r) => println!(
            "fit_2sls beta = {}, first-stage F = {}",
            r.coef[0], r.first_stage_f[0]
        ),
        Err(e) => println!("ERR {}", e.what),
    }
}
