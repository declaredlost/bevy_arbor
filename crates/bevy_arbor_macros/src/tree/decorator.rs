//! Decorators: `when(has_target && !calm)`, `when(has_target, cooldown(8.0))`.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{BinOp, Error, Expr, Result, UnOp};

/// The code building a decorator from its parts, through `bevy_arbor_bt` at
/// `bt`: each a condition, put together with `!`, `&&` and `||`, or a
/// decorator acting as its branch starts and ends.
pub(super) fn decorator_args(bt: &TokenStream, span: Span, args: &[Expr]) -> Result<TokenStream> {
    let Some((first, rest)) = args.split_first() else {
        return Err(Error::new(span, "expected `(decorator, ..)`"));
    };
    let first = part(bt, first);
    let rest = rest.iter().map(|arg| part(bt, arg));
    Ok(quote! { #bt::decorator::Decorator::new(#first)#(.and(#rest))* })
}

/// The code building one part: a condition with `!`, `&&` and `||` read as
/// written, or what's there as it is.
fn part(bt: &TokenStream, expr: &Expr) -> TokenStream {
    if logic(expr) {
        condition(bt, expr)
    } else {
        quote! { #expr }
    }
}

/// Whether `expr` puts conditions together.
fn logic(expr: &Expr) -> bool {
    match expr {
        Expr::Paren(inner) => logic(&inner.expr),
        Expr::Group(inner) => logic(&inner.expr),
        Expr::Unary(unary) => matches!(unary.op, UnOp::Not(_)),
        Expr::Binary(binary) => matches!(binary.op, BinOp::And(_) | BinOp::Or(_)),
        _ => false,
    }
}

/// The code building a condition: `!`, `&&`, `||` and brackets read as
/// written.
fn condition(bt: &TokenStream, expr: &Expr) -> TokenStream {
    let into = quote! { #bt::decorator::IntoCondition };
    match expr {
        Expr::Paren(inner) => condition(bt, &inner.expr),
        Expr::Group(inner) => condition(bt, &inner.expr),
        Expr::Unary(unary) if matches!(unary.op, UnOp::Not(_)) => {
            let inner = condition(bt, &unary.expr);
            quote! { #bt::decorator::not(#inner) }
        }
        Expr::Binary(binary) if matches!(binary.op, BinOp::And(_)) => {
            let (left, right) = (condition(bt, &binary.left), condition(bt, &binary.right));
            quote! { #into::and(#left, #right) }
        }
        Expr::Binary(binary) if matches!(binary.op, BinOp::Or(_)) => {
            let (left, right) = (condition(bt, &binary.left), condition(bt, &binary.right));
            quote! { #into::or(#left, #right) }
        }
        _ => quote! { #expr },
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use crate::tree::tests::{error, tree};

    #[test]
    fn conditions_turn_into_calls() {
        let code = tree(quote! { Select => { when(!a && b) => act(c) } })
            .unwrap()
            .to_string();
        assert!(code.contains("decorator :: not"), "{code}");
        assert!(code.contains("IntoCondition :: and"), "{code}");
    }

    #[test]
    fn parts_after_commas_are_all_needed() {
        let code = tree(quote! { Select => { when(a || b, c, d) => act(e) } })
            .unwrap()
            .to_string();
        assert!(code.contains("IntoCondition :: or"), "{code}");
        assert_eq!(code.matches(". and (").count(), 2, "{code}");
    }

    #[test]
    fn a_decorator_takes_at_least_one_part() {
        assert_eq!(
            error(quote! { Select => { when() => act(d) } }),
            "expected `(decorator, ..)`"
        );
    }
}
