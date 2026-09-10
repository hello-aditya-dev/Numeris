//! # Probability distributions
//!
//! Independent implementations of the special functions and distributions used
//! by the statistical engine: log-gamma (Lanczos), regularized incomplete
//! gamma (series + continued fraction), regularized incomplete beta
//! (continued fraction), and the Normal / Student-t / F / chi-square
//! CDFs and quantiles.
//!
//! Accuracy: CDFs are validated against published reference values to
//! ~1e-10 or better; quantiles are computed by bracketed bisection on the
//! CDF to a width below 1e-12 (deterministic and platform-stable).

use crate::error::{NumerisError, Result};

const FPMIN: f64 = 1.0e-300;
const EPS: f64 = 1.0e-15;

/// Log-gamma via the Lanczos approximation (g = 7, n = 9).
/// Uses the reflection formula for x < 0.5.
pub fn ln_gamma(x: f64) -> f64 {
    if x < 0.5 {
        // Reflection: Γ(x)Γ(1-x) = π / sin(πx)
        let s = std::f64::consts::PI / (std::f64::consts::PI * x).sin();
        s.ln() - ln_gamma(1.0 - x)
    } else {
        const G: f64 = 7.0;
        const COEF: [f64; 9] = [
            0.999_999_999_999_809_9,
            676.5203681218851,
            -1259.1392167224028,
            771.323_428_777_653_1,
            -176.615_029_162_140_6,
            12.507343278686905,
            -0.13857109526572012,
            9.984_369_578_019_572e-6,
            1.5056327351493116e-7,
        ];
        let z = x - 1.0;
        let mut series = COEF[0];
        for (i, &c) in COEF.iter().enumerate().skip(1) {
            series += c / (z + i as f64);
        }
        let t = z + G + 0.5;
        0.5 * (2.0 * std::f64::consts::PI).ln() + (z + 0.5) * t.ln() - t + series.ln()
    }
}

/// Regularized lower incomplete gamma P(a, x).
/// Series for x < a+1; continued fraction for the complement otherwise.
pub fn gamma_p(a: f64, x: f64) -> f64 {
    if a <= 0.0 || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    let front = (-x + a * x.ln() - ln_gamma(a)).exp();
    if x < a + 1.0 {
        // Series representation.
        let mut ap = a;
        let mut sum = 1.0 / a;
        let mut del = sum;
        for _ in 0..1000 {
            ap += 1.0;
            del *= x / ap;
            sum += del;
            if del.abs() < sum.abs() * EPS {
                break;
            }
        }
        sum * front
    } else {
        // Continued fraction for Q(a, x), then P = 1 - Q.
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / FPMIN;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..=1000 {
            let an = -(i as f64) * ((i as f64) - a);
            b += 2.0;
            d = an * d + b;
            if d.abs() < FPMIN {
                d = FPMIN;
            }
            c = b + an / c;
            if c.abs() < FPMIN {
                c = FPMIN;
            }
            d = 1.0 / d;
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() < EPS {
                break;
            }
        }
        1.0 - front * h
    }
}

/// Continued fraction core for the regularized incomplete beta function.
fn beta_cf(a: f64, b: f64, x: f64) -> f64 {
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < FPMIN {
        d = FPMIN;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=500 {
        let m2 = 2 * m;
        let mut aa = (m as f64) * (b - m as f64) * x / ((qam + m2 as f64) * (a + m2 as f64));
        // Even step.
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        h *= d * c;
        aa = -((a + m as f64) * (qab + m as f64) * x) / ((a + m2 as f64) * (qap + m2 as f64));
        // Odd step.
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < EPS {
            break;
        }
    }
    h
}

/// Regularized incomplete beta function I_x(a, b).
pub fn beta_reg(a: f64, b: f64, x: f64) -> f64 {
    if a <= 0.0 || b <= 0.0 || !(0.0..=1.0).contains(&x) {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    if x == 1.0 {
        return 1.0;
    }
    let ln_front = ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln();
    let front = ln_front.exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        // Direct continued fraction.
        front * beta_cf(a, b, x) / a
    } else {
        // Symmetry transformation.
        1.0 - front * beta_cf(b, a, 1.0 - x) / b
    }
}

/// Standard normal PDF.
pub fn normal_pdf(x: f64) -> f64 {
    (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

/// Standard normal CDF, computed as 0.5 * erfc(-x/√2) through the
/// incomplete gamma function (erf(x) = P(1/2, x²)).
pub fn normal_cdf(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x >= 0.0 {
        0.5 * (1.0 + gamma_p(0.5, 0.5 * x * x))
    } else {
        0.5 * (1.0 - gamma_p(0.5, 0.5 * x * x))
    }
}

/// Standard normal quantile (inverse CDF). Acklam's rational approximation
/// followed by one Halley refinement step.
pub fn normal_quantile(p: f64) -> f64 {
    if !(0.0..=1.0).contains(&p) || p == 0.0 || p == 1.0 {
        return f64::NAN;
    }
    const A: [f64; 6] = [
        -3.969683028665376e+01,
        2.209460984245205e+02,
        -2.759285104469687e+02,
        1.383_577_518_672_69e2,
        -3.066479806614716e+01,
        2.506628277459239e+00,
    ];
    const B: [f64; 5] = [
        -5.447609879822406e+01,
        1.615858368580409e+02,
        -1.556989798598866e+02,
        6.680131188771972e+01,
        -1.328068155288572e+01,
    ];
    const C: [f64; 6] = [
        -7.784894002430293e-03,
        -3.223964580411365e-01,
        -2.400758277161838e+00,
        -2.549732539343734e+00,
        4.374664141464968e+00,
        2.938163982698783e+00,
    ];
    const D: [f64; 4] = [
        7.784695709041462e-03,
        3.224671290700398e-01,
        2.445134137142996e+00,
        3.754408661907416e+00,
    ];
    const P_LOW: f64 = 0.02425;

    let mut x: f64;
    if p < P_LOW {
        let q = (-2.0 * p.ln()).sqrt();
        x = (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
    } else if p <= 1.0 - P_LOW {
        let q = p - 0.5;
        let r = q * q;
        x = (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0);
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        x = -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
    }

    // One Halley refinement step.
    let e = normal_cdf(x) - p;
    let u = e * (2.0 * std::f64::consts::PI).sqrt() * (0.5 * x * x).exp();
    x -= u / (1.0 + 0.5 * x * u);
    x
}

/// Student-t CDF with `df` degrees of freedom.
pub fn t_cdf(t: f64, df: f64) -> f64 {
    if df <= 0.0 || t.is_nan() {
        return f64::NAN;
    }
    if t == f64::INFINITY {
        return 1.0;
    }
    if t == f64::NEG_INFINITY {
        return 0.0;
    }
    let x = df / (df + t * t);
    let p_two = beta_reg(df / 2.0, 0.5, x); // = 2 * P(T > |t|)
    if t >= 0.0 {
        1.0 - 0.5 * p_two
    } else {
        0.5 * p_two
    }
}

/// F distribution CDF with (d1, d2) degrees of freedom.
pub fn f_cdf(f: f64, d1: f64, d2: f64) -> f64 {
    if d1 <= 0.0 || d2 <= 0.0 || f.is_nan() {
        return f64::NAN;
    }
    if f <= 0.0 {
        return 0.0;
    }
    if f == f64::INFINITY {
        return 1.0;
    }
    let x = d1 * f / (d1 * f + d2);
    beta_reg(d1 / 2.0, d2 / 2.0, x)
}

/// Chi-square CDF with `df` degrees of freedom.
pub fn chi2_cdf(x: f64, df: f64) -> f64 {
    if df <= 0.0 || x.is_nan() {
        return f64::NAN;
    }
    if x <= 0.0 {
        return 0.0;
    }
    gamma_p(df / 2.0, x / 2.0)
}

/// Generic bracketed bisection for a monotone CDF.
/// Returns the quantile x such that cdf(x) ≈ p.
fn quantile_by_bisection<F: Fn(f64) -> f64>(p: f64, mut lo: f64, mut hi: f64, cdf: F) -> f64 {
    let cdf_lo = cdf(lo);
    if cdf_lo >= p {
        return lo;
    }
    let cdf_hi = cdf(hi);
    if cdf_hi <= p {
        // Expand upward.
        let mut up = hi;
        for _ in 0..200 {
            up *= 2.0;
            if cdf(up) >= p || up > 1e300 {
                hi = up;
                break;
            }
            lo = up;
        }
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if cdf(mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
        if (hi - lo).abs() < 1e-12 * hi.abs().max(1.0) {
            break;
        }
    }
    0.5 * (lo + hi)
}

/// Student-t quantile. `p` in (0, 1).
pub fn t_quantile(p: f64, df: f64) -> Result<f64> {
    if !(0.0..=1.0).contains(&p) {
        return Err(NumerisError::numerical_failure(
            "Could not compute a t quantile.",
            "The requested probability is outside (0, 1).",
            "This is an internal error; please report the command that triggered it.",
        ));
    }
    if df <= 0.0 {
        return Err(NumerisError::numerical_failure(
            "Could not compute a t quantile with non-positive degrees of freedom.",
            "The degrees of freedom must be positive.",
            "This is an internal error; please report the command that triggered it.",
        ));
    }
    if p == 0.5 {
        return Ok(0.0);
    }
    if p > 0.5 {
        return Ok(quantile_by_bisection(p, 0.0, 1.0, |x| t_cdf(x, df)));
    }
    Ok(-quantile_by_bisection(1.0 - p, 0.0, 1.0, |x| t_cdf(x, df)))
}

/// F-distribution quantile. `p` in (0, 1).
pub fn f_quantile(p: f64, d1: f64, d2: f64) -> Result<f64> {
    if !(0.0..=1.0).contains(&p) || d1 <= 0.0 || d2 <= 0.0 {
        return Err(NumerisError::numerical_failure(
            "Could not compute an F quantile.",
            "The requested probability is outside (0, 1) or the degrees of freedom are non-positive.",
            "This is an internal error; please report the command that triggered it.",
        ));
    }
    Ok(quantile_by_bisection(p, 1e-12, 1.0, |x| f_cdf(x, d1, d2)))
}

/// Chi-square quantile. `p` in (0, 1).
pub fn chi2_quantile(p: f64, df: f64) -> Result<f64> {
    if !(0.0..=1.0).contains(&p) || df <= 0.0 {
        return Err(NumerisError::numerical_failure(
            "Could not compute a chi-square quantile.",
            "The requested probability is outside (0, 1) or the degrees of freedom are non-positive.",
            "This is an internal error; please report the command that triggered it.",
        ));
    }
    Ok(quantile_by_bisection(p, 1e-12, 1.0, |x| chi2_cdf(x, df)))
}

/// Two-sided p-value for a t statistic.
pub fn t_two_sided_p(t: f64, df: f64) -> f64 {
    let p = t_cdf(t.abs(), df);
    (2.0 * (1.0 - p)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn ln_gamma_matches_known_values() {
        assert!(close(ln_gamma(1.0), 0.0, 1e-12));
        assert!(close(ln_gamma(0.5), 0.5723649429247001, 1e-12));
        assert!(close(ln_gamma(6.0), 120.0f64.ln(), 1e-12));
        assert!(close(ln_gamma(12.0), 39916800.0f64.ln(), 1e-11));
    }

    #[test]
    fn normal_cdf_reference_values() {
        assert!(close(normal_cdf(0.0), 0.5, 1e-15));
        assert!(close(normal_cdf(1.0), 0.8413447460685429, 1e-12));
        assert!(close(normal_cdf(-1.96), 0.024997895148220435, 1e-12));
        assert!(close(normal_cdf(1.959963984540054), 0.975, 1e-12));
        assert!(close(normal_cdf(5.0), 0.9999997133484281, 1e-12));
    }

    #[test]
    fn normal_quantile_round_trip() {
        for &x in &[-3.5, -1.0, 0.0, 0.5, 2.0, 3.4] {
            let q = normal_quantile(normal_cdf(x));
            assert!(close(q, x, 1e-9), "x={x} q={q}");
        }
    }

    #[test]
    fn t_quantile_reference_values() {
        // Values cross-checked against published tables.
        assert!(close(t_quantile(0.975, 1.0).unwrap(), 12.706204736, 1e-6));
        assert!(close(t_quantile(0.975, 2.0).unwrap(), 4.302652730, 1e-7));
        assert!(close(t_quantile(0.975, 5.0).unwrap(), 2.570581836, 1e-7));
        assert!(close(t_quantile(0.975, 10.0).unwrap(), 2.228138852, 1e-7));
        assert!(close(t_quantile(0.975, 30.0).unwrap(), 2.042272456, 1e-7));
        assert!(close(t_quantile(0.975, 100.0).unwrap(), 1.983971519, 1e-7));
        // Symmetry.
        let a = t_quantile(0.025, 10.0).unwrap();
        assert!(close(a, -2.228138852, 1e-7));
    }

    #[test]
    fn chi2_quantile_reference_values() {
        assert!(close(chi2_quantile(0.95, 1.0).unwrap(), 3.841458821, 1e-6));
        assert!(close(chi2_quantile(0.95, 2.0).unwrap(), 5.991464547, 1e-6));
        assert!(close(chi2_quantile(0.95, 10.0).unwrap(), 18.30703805, 1e-6));
    }

    #[test]
    fn f_quantile_consistent_with_t_squared() {
        // F(1, ν) at p equals t at (1+p)/2 squared.
        let f = f_quantile(0.95, 1.0, 10.0).unwrap();
        let t = t_quantile(0.975, 10.0).unwrap();
        assert!(close(f, t * t, 1e-7), "f={f}, t²={}", t * t);
        // And the CDF is consistent with the quantile.
        assert!(close(f_cdf(f, 1.0, 10.0), 0.95, 1e-10));
    }

    #[test]
    fn t_cdf_matches_quantile_inverse() {
        for &df in &[1.0, 2.0, 5.0, 10.0, 60.0] {
            let t = t_quantile(0.975, df).unwrap();
            let c = t_cdf(t, df);
            assert!(close(c, 0.975, 1e-10), "df={df} cdf={c}");
        }
    }

    #[test]
    fn gamma_p_reference() {
        // chi2_cdf(x, df) = P(df/2, x/2): chi2_{0.95,10} = 18.307038
        assert!(close(chi2_cdf(18.30703805, 10.0), 0.95, 1e-9));
        assert!(close(chi2_cdf(3.841458821, 1.0), 0.95, 1e-9));
    }

    #[test]
    fn two_sided_p_extremes() {
        assert!(t_two_sided_p(0.0, 10.0) > 0.999999999);
        assert!(t_two_sided_p(100.0, 10.0) < 1e-15);
    }
}
