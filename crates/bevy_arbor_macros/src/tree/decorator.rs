//! Decorators: `when(has_target && !calm)`, `when(has_target, cooldown(8.0))`.

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{BinOp, Error, Expr, Result, UnOp};

/// The code building a decorator from its parts, through `bevy_arbor_bt` at
/// `bt`: the one part, or a tuple of them.
///
/// Each part is a condition, put together with `!`, `&&` and `||`, or a
/// decorator acting as its branch starts and ends. In a tuple, each is made a
/// decorator on its own, so one that isn't is pointed at.
pub(super) fn decorator_args(bt: &TokenStream, span: Span, args: &[Expr]) -> Result<TokenStream> {
    match args {
        [] => Err(Error::new(span, "expected `(decorator, ..)`")),
        [one] => Ok(part(bt, one)),
        _ => {
            let parts = args
                .iter()
                .map(|arg| {
                    let part = part(bt, arg);
                    quote_spanned! { arg.span() =>
                        #bt::decorator::IntoDecorator::into_decorator(#part)
                    }
                })
                .collect();
            Ok(tuple(parts))
        }
    }
}

/// `parts` as a tuple, the last of eight another tuple of the rest past it.
fn tuple(mut parts: Vec<TokenStream>) -> TokenStream {
    if parts.len() > 8 {
        let rest = tuple(parts.split_off(7));
        parts.push(rest);
    }
    quote! { (#(#parts,)*) }
}

/// The code building one part: a condition, or what's there as it is.
///
/// A condition's `!`, `&&` and `||` are read as written.
fn part(bt: &TokenStream, expr: &Expr) -> TokenStream {
    if logic(expr) {
        condition(bt, expr)
    } else {
        quote! { #expr }
    }
}

/// Whether `expr` puts conditions together.
fn logic(expr: &Expr) -> bool {
    match strip(expr) {
        Expr::Unary(unary) => matches!(unary.op, UnOp::Not(_)),
        Expr::Binary(binary) => matches!(binary.op, BinOp::And(_) | BinOp::Or(_)),
        _ => false,
    }
}

/// The code building a condition: `!`, `&&`, `||` and brackets read as
/// written.
///
/// A run of one of `&&` or `||` is one `all` or `any` of everything in it.
fn condition(bt: &TokenStream, expr: &Expr) -> TokenStream {
    let condition_mod = quote! { #bt::condition };
    match strip(expr) {
        Expr::Unary(unary) if matches!(unary.op, UnOp::Not(_)) => {
            let inner = condition(bt, &unary.expr);
            quote! { #condition_mod::unless(#inner) }
        }
        Expr::Binary(binary) if matches!(binary.op, BinOp::And(_)) => {
            let parts = run(bt, expr, |op| matches!(op, BinOp::And(_)));
            joined(&quote! { #condition_mod::all }, parts)
        }
        Expr::Binary(binary) if matches!(binary.op, BinOp::Or(_)) => {
            let parts = run(bt, expr, |op| matches!(op, BinOp::Or(_)));
            joined(&quote! { #condition_mod::any }, parts)
        }
        _ => quote! { #expr },
    }
}

/// `expr` without the brackets around it.
fn strip(expr: &Expr) -> &Expr {
    match expr {
        Expr::Paren(inner) => strip(&inner.expr),
        Expr::Group(inner) => strip(&inner.expr),
        _ => expr,
    }
}

/// `parts` given to `join`, `all` or `any`.
///
/// Past eight, the tuple's last is another of the rest.
fn joined(join: &TokenStream, mut parts: Vec<TokenStream>) -> TokenStream {
    if parts.len() > 8 {
        let rest = joined(join, parts.split_off(7));
        parts.push(rest);
    }
    quote! { #join((#(#parts,)*)) }
}

/// The conditions joined by one operator, `same`, in `expr`, in order.
///
/// `a && b && c` is `a`, `b` and `c`.
fn run(bt: &TokenStream, expr: &Expr, same: fn(&BinOp) -> bool) -> Vec<TokenStream> {
    match strip(expr) {
        Expr::Binary(binary) if same(&binary.op) => {
            let mut parts = run(bt, &binary.left, same);
            parts.extend(run(bt, &binary.right, same));
            parts
        }
        _ => vec![condition(bt, expr)],
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
        assert!(code.contains("condition :: unless"), "{code}");
        assert!(code.contains("condition :: all"), "{code}");
    }

    #[test]
    fn a_run_of_one_operator_is_one_call() {
        let code = tree(quote! { Select => { when(a && (b && c) && d) => act(e) } })
            .unwrap()
            .to_string();
        assert_eq!(code.matches("condition :: all").count(), 1, "{code}");
        assert!(code.contains("all ((a , b , c , d ,))"), "{code}");
    }

    #[test]
    fn a_run_past_eight_nests() {
        let code = tree(quote! {
            Select => { when(a || b || c || d || e || f || g || h || i) => act(j) }
        })
        .unwrap()
        .to_string();
        assert_eq!(code.matches("condition :: any").count(), 2, "{code}");
        assert!(code.contains("any ((h , i ,))"), "{code}");
    }

    #[test]
    fn parts_after_commas_are_all_needed() {
        let code = tree(quote! { Select => { when(a || b, c, d) => act(e) } })
            .unwrap()
            .to_string();
        assert!(code.contains("condition :: any"), "{code}");
        assert_eq!(code.matches("into_decorator").count(), 3, "{code}");
    }

    #[test]
    fn a_decorator_takes_at_least_one_part() {
        assert_eq!(
            error(quote! { Select => { when() => act(d) } }),
            "expected `(decorator, ..)`"
        );
    }
}
