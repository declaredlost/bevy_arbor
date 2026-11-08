//! Lengths of time: `Wait(1.5)`, `Timeout(cooldown)`.

use std::time::Duration;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Expr, ExprLit, Lit, Result, UnOp};

/// What a length too long for `u64` nanoseconds gets.
const TOO_LONG: &str = "too long: a length is at most 18446744073 seconds";

/// The code building a `Duration`: a number is seconds, checked here, so a
/// negative one doesn't compile; anything else is a `Duration` already.
pub(super) fn length(expr: &Expr) -> Result<TokenStream> {
    let Some(seconds) = seconds(expr)? else {
        return Ok(quote! { #expr });
    };
    if seconds < 0.0 {
        return Err(Error::new_spanned(
            expr,
            "expected a length in seconds, 0 or more",
        ));
    }
    let nanos = Duration::try_from_secs_f64(seconds)
        .ok()
        .and_then(|length| u64::try_from(length.as_nanos()).ok())
        .ok_or_else(|| Error::new_spanned(expr, TOO_LONG))?;
    Ok(quote! { ::core::time::Duration::from_nanos(#nanos) })
}

/// `expr` as a number, if it's a number literal (negated or not).
fn seconds(expr: &Expr) -> Result<Option<f64>> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Float(lit),
            ..
        }) => lit.base10_parse().map(Some),
        Expr::Lit(ExprLit {
            lit: Lit::Int(lit), ..
        }) => lit.base10_parse().map(Some),
        Expr::Unary(unary) if matches!(unary.op, UnOp::Neg(_)) => {
            Ok(seconds(&unary.expr)?.map(|seconds| -seconds))
        }
        Expr::Paren(inner) => seconds(&inner.expr),
        Expr::Group(inner) => seconds(&inner.expr),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    fn code(expr: TokenStream) -> Result<String> {
        length(&syn::parse2(expr).unwrap()).map(|code| code.to_string())
    }

    #[test]
    fn a_number_of_seconds_is_checked_and_turned_into_a_duration() {
        assert!(
            code(quote! { 1.5 })
                .unwrap()
                .contains("from_nanos (1500000000u64)")
        );
        assert!(
            code(quote! { 2 })
                .unwrap()
                .contains("from_nanos (2000000000u64)")
        );
        assert!(
            code(quote! { (0.5) })
                .unwrap()
                .contains("from_nanos (500000000u64)")
        );
    }

    #[test]
    fn anything_else_is_taken_as_a_duration() {
        assert_eq!(code(quote! { cooldown }).unwrap(), "cooldown");
    }

    #[test]
    fn a_negative_length_does_not_compile() {
        assert_eq!(
            code(quote! { -1.0 }).unwrap_err().to_string(),
            "expected a length in seconds, 0 or more"
        );
    }

    #[test]
    fn a_length_too_long_says_so() {
        for too_long in [quote! { 1e300 }, quote! { 2e10 }] {
            assert_eq!(code(too_long).unwrap_err().to_string(), TOO_LONG);
        }
    }
}
