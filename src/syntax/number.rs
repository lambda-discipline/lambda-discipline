//! The printed form of a `Float`, shared by `show`, `print` and `--emit`.
//! An `Int` prints as its decimal digits.

/// Floats print with the shortest digits that read back as the same number,
/// in positional notation from `1e-6` up to just below `1e21` (`3.14`,
/// `0.000001`) and with an exponent outside that range (`1e+21`, `1e-7`,
/// `-2.5e+300`), as JavaScript prints them. A whole float keeps a `.0`
/// (`42.0`) so that it reads back as a `Float`, not an `Int`. Negative zero
/// keeps its sign (`-0.0`), as in Haskell, OCaml and Python, since
/// `(fdiv 1.0 -0.0)` is `-Infinity`.
pub fn format_float(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if n == 0.0 {
        return if n.is_sign_negative() { "-0.0" } else { "0.0" }.into();
    }
    // `{:e}` writes the shortest round-trip digits as `d.ddde<exp>`.
    let scientific = format!("{n:e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("`{:e}` always writes an exponent");
    let exponent: i32 = exponent.parse().expect("the exponent is an integer");
    if (-6..21).contains(&exponent) {
        let positional = format!("{n}");
        if positional.contains('.') {
            positional
        } else {
            positional + ".0"
        }
    } else if exponent > 0 {
        format!("{mantissa}e+{exponent}")
    } else {
        scientific
    }
}

#[cfg(test)]
mod tests {
    use super::format_float;

    #[test]
    fn floats_print_like_other_languages() {
        assert_eq!(format_float(42.0), "42.0");
        assert_eq!(format_float(-3.0), "-3.0");
        assert_eq!(format_float(2.25), "2.25");
        assert_eq!(format_float(-0.5), "-0.5");
        assert_eq!(format_float(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(format_float(f64::INFINITY), "Infinity");
        assert_eq!(format_float(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(format_float(f64::NAN), "NaN");
    }

    #[test]
    fn negative_zero_prints_as_zero() {
        assert_eq!(format_float(-0.0), "-0.0");
        assert_eq!(format_float(0.0), "0.0");
    }

    #[test]
    fn exponents_beyond_the_javascript_thresholds() {
        assert_eq!(format_float(1e300), "1e+300");
        assert_eq!(format_float(-2.5e300), "-2.5e+300");
        assert_eq!(format_float(1e21), "1e+21");
        assert_eq!(format_float(1.5e21), "1.5e+21");
        assert_eq!(format_float(1e20), "100000000000000000000.0");
        assert_eq!(
            format_float(123456789012345680000.0),
            "123456789012345680000.0"
        );
        assert_eq!(format_float(1e-7), "1e-7");
        assert_eq!(format_float(-1.25e-7), "-1.25e-7");
        assert_eq!(format_float(1e-6), "0.000001");
        assert_eq!(format_float(f64::MAX), "1.7976931348623157e+308");
        assert_eq!(format_float(5e-324), "5e-324");
    }

    #[test]
    fn printed_numbers_read_back_unchanged() {
        for n in [1e300, 1.5e-7, 1e21, 0.1 + 0.2, 123.456, -9e-10, f64::MAX] {
            assert_eq!(format_float(n).parse::<f64>(), Ok(n));
        }
    }
}
