//! The printed form of a number, shared by `show`, `print` and `--emit`.

/// Numbers print the way JavaScript prints them: the shortest digits that
/// read back as the same number, in positional notation from `1e-6` up to
/// just below `1e21` (`42`, `3.14`, `0.000001`) and with an exponent outside
/// that range (`1e+21`, `1e-7`, `-2.5e+300`). Negative zero prints as `0`.
pub fn format_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if n == 0.0 {
        return "0".into();
    }
    // `{:e}` writes the shortest round-trip digits as `d.ddde<exp>`.
    let scientific = format!("{n:e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("`{:e}` always writes an exponent");
    let exponent: i32 = exponent.parse().expect("the exponent is an integer");
    if (-6..21).contains(&exponent) {
        format!("{n}")
    } else if exponent > 0 {
        format!("{mantissa}e+{exponent}")
    } else {
        scientific
    }
}

#[cfg(test)]
mod tests {
    use super::format_number;

    #[test]
    fn numbers_print_like_other_languages() {
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(2.25), "2.25");
        assert_eq!(format_number(-0.5), "-0.5");
        assert_eq!(format_number(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(format_number(f64::INFINITY), "Infinity");
        assert_eq!(format_number(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(format_number(f64::NAN), "NaN");
    }

    #[test]
    fn negative_zero_prints_as_zero() {
        assert_eq!(format_number(-0.0), "0");
        assert_eq!(format_number(0.0), "0");
    }

    #[test]
    fn exponents_beyond_the_javascript_thresholds() {
        assert_eq!(format_number(1e300), "1e+300");
        assert_eq!(format_number(-2.5e300), "-2.5e+300");
        assert_eq!(format_number(1e21), "1e+21");
        assert_eq!(format_number(1.5e21), "1.5e+21");
        assert_eq!(format_number(1e20), "100000000000000000000");
        assert_eq!(
            format_number(123456789012345680000.0),
            "123456789012345680000"
        );
        assert_eq!(format_number(1e-7), "1e-7");
        assert_eq!(format_number(-1.25e-7), "-1.25e-7");
        assert_eq!(format_number(1e-6), "0.000001");
        assert_eq!(format_number(f64::MAX), "1.7976931348623157e+308");
        assert_eq!(format_number(5e-324), "5e-324");
    }

    #[test]
    fn printed_numbers_read_back_unchanged() {
        for n in [1e300, 1.5e-7, 1e21, 0.1 + 0.2, 123.456, -9e-10, f64::MAX] {
            assert_eq!(format_number(n).parse::<f64>(), Ok(n));
        }
    }
}
