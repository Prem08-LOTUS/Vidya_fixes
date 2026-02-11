pub fn cross_check(a_nm: f64, b_nm: f64, tol_nm: f64) -> Result<f64, &'static str> {
    if (a_nm - b_nm).abs() > tol_nm {
        Err("Sensor disagreement")
    } else {
        Ok((a_nm + b_nm) * 0.5)
    }
}
