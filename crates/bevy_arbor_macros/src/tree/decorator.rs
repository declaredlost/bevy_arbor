//! Decorators: `when(has_target && !calm)`, `when(within, 5.0)`.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{BinOp, Error, Expr, Result, UnOp};

/// The code building a decorator from `(decorator)` or `(system, param)`,
/// through `bevy_arbor_bt` at `bt`.
pub(super) fn decorator_args(bt: &TokenStream, span: Span, args: &[Expr]) -> Result<TokenStream> {
    match args {
        [expr] => Ok(decorator(bt, expr)),
        [system, param] => Ok(quote! { #bt::decorator::Decorator::with(#system, #param) }),
        _ => Err(Error::new(
            span,
            "expected `(decorator)` or `(system, param)`",
        )),
    }
}

/// The code building a decorator: `!`, `&&`, `||` and brackets read as written.
fn decorator(bt: &TokenStream, expr: &Expr) -> TokenStream {
    match expr {
        Expr::Paren(inner) => decorator(bt, &inner.expr),
        Expr::Group(inner) => decorator(bt, &inner.expr),
        Expr::Unary(unary) if matches!(unary.op, UnOp::Not(_)) => {
            let inner = decorator(bt, &unary.expr);
            quote! { ::std::ops::Not::not(#inner) }
        }
        Expr::Binary(binary) if matches!(binary.op, BinOp::And(_)) => {
            let (left, right) = (decorator(bt, &binary.left), decorator(bt, &binary.right));
            quote! { #left.and(#right) }
        }
        Expr::Binary(binary) if matches!(binary.op, BinOp::Or(_)) => {
            let (left, right) = (decorator(bt, &binary.left), decorator(bt, &binary.right));
            quote! { #left.or(#right) }
        }
        _ => quote! { #bt::decorator::IntoDecorator::into_decorator(#expr) },
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use crate::tree::tests::{error, tree};

    #[test]
    fn decorators_turn_into_calls() {
        let code = tree(quote! { Select => { when(!a && b) => act(c) } })
            .unwrap()
            .to_string();
        assert!(code.contains("Not :: not"), "{code}");
        assert!(code.contains(". and ("), "{code}");
    }

    #[test]
    fn a_decorator_takes_a_decorator_or_a_system_and_its_param() {
        assert_eq!(
            error(quote! { Select => { when(a, b, c) => act(d) } }),
            "expected `(decorator)` or `(system, param)`"
        );
    }
}
