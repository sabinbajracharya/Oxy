// === STRESS: math:: stdlib functions ===

#[test]
fn test_pi_constant() {
    let p = math::PI;
    assert(p > 3.14 && p < 3.15);
}

#[test]
fn test_sqrt() {
    assert_eq(math::sqrt(16.0), 4.0);
    assert_eq(math::sqrt(0.0), 0.0);
}

#[test]
fn test_abs_int() { assert_eq(math::abs(-5), 5); }
#[test]
fn test_abs_float() { assert_eq(math::abs(-2.5), 2.5); }

#[test]
fn test_pow() { assert_eq(math::pow(2.0, 10.0), 1024.0); }

#[test]
fn test_sin_zero_stays_float() {
    // Now that math fns always return float, sin(0.0) is F64(0.0) — not
    // I64(0) like before. Verify the contract.
    let r = math::sin(0.0);
    assert(r > -0.001 && r < 0.001);
}
#[test]
fn test_sin_known_value() {
    // sin(0.5) ≈ 0.479425538604203
    let r = math::sin(0.5);
    assert(r > 0.47 && r < 0.49);
}

#[test]
fn test_floor() { assert_eq(math::floor(3.9), 3.0); }
#[test]
fn test_ceil() { assert_eq(math::ceil(3.1), 4.0); }
#[test]
fn test_round() { assert_eq(math::round(3.5), 4.0); }

#[test]
fn test_min_int() { assert_eq(math::min(3, 7), 3); }
#[test]
fn test_max_int() { assert_eq(math::max(3, 7), 7); }

#[test]
fn test_gcd() { assert_eq(math::gcd(48, 18), 6); }
#[test]
fn test_lcm() { assert_eq(math::lcm(12, 18), 36); }

#[test]
fn test_clamp_above() { assert_eq(math::clamp(15, 0, 10), 10); }
#[test]
fn test_clamp_below() { assert_eq(math::clamp(-5, 0, 10), 0); }
#[test]
fn test_clamp_within() { assert_eq(math::clamp(5, 0, 10), 5); }
#[test]
fn test_clamp_float() { assert_eq(math::clamp(0.5, 0.0, 1.0), 0.5); }
