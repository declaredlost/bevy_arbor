use std::time::Duration;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{BinOp, Error, Expr, ExprLit, Ident, Lit, Result, UnOp};

use crate::parse::{Entry, Item, Prefix, Root};

/// The nodes there are, for saying so when it's none of them.
const NODES: &str = "Forever, Select, Fallback, Sequence, All, Any, Race, Scored, Random(roll), \
                     Invert, Succeed, Commit, While(guard), Timeout(length), Wait(length), \
                     task(name, bundle), act(system), or @ expr";

/// The code building the tree `input` describes.
pub(crate) fn tree(input: TokenStream) -> Result<TokenStream> {
    let Root(item) = syn::parse2(input)?;
    self::item(&item)
}

/// The code building one node.
fn item(item: &Item) -> Result<TokenStream> {
    let (name, args, body) = match item {
        Item::Embed(node) => return Ok(quote! { ::bevy_arbor::behaviour::Behaviour::from(#node) }),
        Item::Named { name, args, body } => (name, args.as_slice(), body.as_deref()),
    };
    let label = name.to_string();
    match (label.as_str(), args, body) {
        ("Forever", [], Some(body)) => wrap(name, "forever", &[], body),
        ("Invert", [], Some(body)) => wrap(name, "invert", &[], body),
        ("Succeed", [], Some(body)) => wrap(name, "succeed", &[], body),
        ("Commit", [], Some(body)) => wrap(name, "commit", &[], body),
        ("Timeout", [length], Some(body)) => wrap(name, "timeout", &[duration(length)?], body),
        ("While", [_] | [_, _], Some(body)) => {
            let guard = guard_args(name.span(), args)?;
            wrap(name, "repeat_while", &[guard], body)
        }
        ("Sequence", [], Some(body)) => list(name, "sequence", &[], body),
        ("All", [], Some(body)) => list(name, "all", &[], body),
        ("Any", [], Some(body)) => list(name, "any", &[], body),
        ("Race", [], Some(body)) => list(name, "race", &[], body),
        ("Random", [roll], Some(body)) => list(name, "random", &[quote! { #roll }], body),
        ("Select", [], Some(body)) => branches("select", body, false),
        ("Fallback", [], Some(body)) => branches("fallback", body, false),
        ("Scored", [], Some(body)) => branches("scored", body, true),
        ("Wait", [length], None) => {
            let length = duration(length)?;
            Ok(quote! { ::bevy_arbor::behaviour::Behaviour::wait(#length) })
        }
        ("task", [name, bundle], None) => {
            Ok(quote! { ::bevy_arbor::behaviour::Behaviour::task(#name, #bundle) })
        }
        ("act", [system], None) => Ok(quote! { ::bevy_arbor::behaviour::Behaviour::act(#system) }),
        ("act", [system, param], None) => {
            Ok(quote! { ::bevy_arbor::behaviour::Behaviour::act_with(#system, #param) })
        }
        (
            "Forever" | "Invert" | "Succeed" | "Commit" | "Sequence" | "All" | "Any" | "Race"
            | "Select" | "Fallback" | "Scored",
            ..,
        ) => Err(Error::new(
            name.span(),
            format!("expected `{label} => {{ .. }}`"),
        )),
        ("Timeout", ..) => Err(Error::new(
            name.span(),
            "expected `Timeout(length) => { .. }`",
        )),
        ("While", ..) => Err(Error::new(name.span(), "expected `While(guard) => { .. }`")),
        ("Random", ..) => Err(Error::new(name.span(), "expected `Random(roll) => { .. }`")),
        ("Wait", ..) => Err(Error::new(name.span(), "expected `Wait(length)`")),
        ("task", ..) => Err(Error::new(name.span(), "expected `task(name, bundle)`")),
        ("act", ..) => Err(Error::new(name.span(), "expected `act(system)`")),
        _ => Err(Error::new(
            name.span(),
            format!("no node called `{label}`: there's {NODES}"),
        )),
    }
}

/// The code building a guard: `!`, `&&`, `||` and brackets read as written.
fn guard(expr: &Expr) -> TokenStream {
    match expr {
        Expr::Paren(inner) => guard(&inner.expr),
        Expr::Group(inner) => guard(&inner.expr),
        Expr::Unary(unary) if matches!(unary.op, UnOp::Not(_)) => {
            let inner = guard(&unary.expr);
            quote! { ::std::ops::Not::not(#inner) }
        }
        Expr::Binary(binary) if matches!(binary.op, BinOp::And(_)) => {
            let (left, right) = (guard(&binary.left), guard(&binary.right));
            quote! { #left.and(#right) }
        }
        Expr::Binary(binary) if matches!(binary.op, BinOp::Or(_)) => {
            let (left, right) = (guard(&binary.left), guard(&binary.right));
            quote! { #left.or(#right) }
        }
        _ => quote! { ::bevy_arbor::behaviour::IntoGuard::into_guard(#expr) },
    }
}

/// The code building a `Duration` for `Wait` or `Timeout`: a number is seconds,
/// checked here, so a negative one doesn't compile; anything else is a
/// `Duration` already.
fn duration(expr: &Expr) -> Result<TokenStream> {
    let Some(seconds) = seconds(expr)? else {
        return Ok(quote! { #expr });
    };
    let nanos = Duration::try_from_secs_f64(seconds)
        .ok()
        .and_then(|length| u64::try_from(length.as_nanos()).ok())
        .ok_or_else(|| Error::new_spanned(expr, "expected a length in seconds, 0 or more"))?;
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

/// The code building a guard from `(guard)` or `(system, param)`.
fn guard_args(span: Span, args: &[Expr]) -> Result<TokenStream> {
    match args {
        [expr] => Ok(guard(expr)),
        [system, param] => Ok(quote! { ::bevy_arbor::behaviour::Guard::with(#system, #param) }),
        _ => Err(Error::new(span, "expected `(guard)` or `(system, param)`")),
    }
}

/// A node over one child: several children run as a sequence.
fn wrap(name: &Ident, build: &str, args: &[TokenStream], body: &[Entry]) -> Result<TokenStream> {
    if body.is_empty() {
        return Err(Error::new(name.span(), format!("`{name}` needs a child")));
    }
    let build = Ident::new(build, name.span());
    let children = children(body)?;
    Ok(quote! {
        ::bevy_arbor::behaviour::Behaviour::#build(#(#args,)* ::bevy_arbor::behaviour::Behaviour::group(#children))
    })
}

/// A node over a list of children.
fn list(name: &Ident, build: &str, args: &[TokenStream], body: &[Entry]) -> Result<TokenStream> {
    let build = Ident::new(build, name.span());
    let children = children(body)?;
    Ok(quote! { ::bevy_arbor::behaviour::Behaviour::#build(#(#args,)* #children) })
}

/// The name of the list being built: invisible to what goes in it, so an
/// expression there can use the same name.
fn out() -> Ident {
    Ident::new("out", Span::mixed_site())
}

/// The code building a list of children.
fn children(body: &[Entry]) -> Result<TokenStream> {
    let out = out();
    let pushes = body
        .iter()
        .map(|entry| match entry {
            Entry::Many(nodes) => Ok(quote! { #out.extend(#nodes); }),
            Entry::One {
                prefix: Some(prefix),
                ..
            } => Err(Error::new(
                prefix.name.span(),
                format!("`{}` goes on a branch", prefix.name),
            )),
            Entry::One { prefix: None, item } => {
                let node = self::item(item)?;
                Ok(quote! { #out.push(#node); })
            }
        })
        .collect::<Result<Vec<_>>>()?;
    if pushes.is_empty() {
        return Ok(quote! { ::std::vec::Vec::<::bevy_arbor::behaviour::Behaviour>::new() });
    }
    Ok(quote! {{
        let mut #out: ::std::vec::Vec<::bevy_arbor::behaviour::Behaviour> = ::std::vec::Vec::new();
        #(#pushes)*
        #out
    }})
}

/// The code building a node over branches: guarded, or if `scored`, scored.
fn branches(build: &str, body: &[Entry], scored: bool) -> Result<TokenStream> {
    let out = out();
    let build = Ident::new(build, Span::call_site());
    let kind = if scored {
        quote! { ::bevy_arbor::behaviour::ScoredBranch }
    } else {
        quote! { ::bevy_arbor::behaviour::Branch }
    };
    let pushes = body
        .iter()
        .map(|entry| match entry {
            Entry::Many(branches) => Ok(quote! { #out.extend(#branches); }),
            Entry::One { prefix, item } => {
                let node = self::item(item)?;
                let branch = branch(prefix.as_ref(), node, scored)?;
                Ok(quote! { #out.push(#branch); })
            }
        })
        .collect::<Result<Vec<_>>>()?;
    if pushes.is_empty() {
        return Ok(
            quote! { ::bevy_arbor::behaviour::Behaviour::#build(::std::vec::Vec::<#kind>::new()) },
        );
    }
    Ok(quote! {{
        let mut #out: ::std::vec::Vec<#kind> = ::std::vec::Vec::new();
        #(#pushes)*
        ::bevy_arbor::behaviour::Behaviour::#build(#out)
    }})
}

/// The code building one branch.
fn branch(prefix: Option<&Prefix>, node: TokenStream, scored: bool) -> Result<TokenStream> {
    let Some(prefix) = prefix else {
        return Ok(if scored {
            quote! { ::bevy_arbor::behaviour::ScoredBranch::last(#node) }
        } else {
            quote! { ::bevy_arbor::behaviour::Branch::always(#node) }
        });
    };
    let span = prefix.name.span();
    if prefix.name == "when" {
        if scored {
            return Err(Error::new(span, "a `Scored` takes `score(system)`"));
        }
        let guard = guard_args(span, &prefix.args)?;
        return Ok(quote! { ::bevy_arbor::behaviour::Branch::when(#guard, #node) });
    }
    if !scored {
        return Err(Error::new(span, "`score` goes in a `Scored`"));
    }
    match prefix.args.as_slice() {
        [system] => Ok(quote! { ::bevy_arbor::behaviour::ScoredBranch::new(#system, #node) }),
        _ => Err(Error::new(span, "expected `score(system)`")),
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    fn error(input: TokenStream) -> String {
        tree(input).unwrap_err().to_string()
    }

    #[test]
    fn every_node_expands() {
        let expanded = tree(quote! {
            Forever => {
                Select => {
                    when(guard) => Sequence => { task("A", A), Wait(1.0), },
                    when(!a && (b || c)) => @ part(),
                    when(within, 5.0) => Commit => { act(system) },
                    Timeout(2.0) => { act(system, 1) },
                    All => { @ part(), @[ parts ] },
                    Any => { Invert => { act(system) }, Succeed => { act(system) } },
                    Race => { act(a), act(b) },
                    While(guard) => { act(a) },
                    While(within, 2.0) => { act(a) },
                    Fallback => { when(a) => act(b), act(c) },
                    Scored => { score(a) => act(b), act(c) },
                    Random(roll) => { act(a) },
                    @[ branches ],
                }
            }
        });
        assert!(expanded.is_ok(), "{:?}", expanded.err());
    }

    #[test]
    fn guards_turn_into_calls() {
        let code = tree(quote! { Select => { when(!a && b) => act(c) } })
            .unwrap()
            .to_string();
        assert!(code.contains("Not :: not"), "{code}");
        assert!(code.contains(". and ("), "{code}");
    }

    #[test]
    fn an_unknown_node_says_what_there_is() {
        let message = error(quote! { Parallel => { act(a) } });
        assert!(message.contains("no node called `Parallel`"), "{message}");
        assert!(message.contains("Select"), "{message}");
    }

    #[test]
    fn a_node_used_wrongly_says_how_it_goes() {
        assert_eq!(error(quote! { Wait }), "expected `Wait(length)`");
        assert_eq!(error(quote! { task("A") }), "expected `task(name, bundle)`");
        assert_eq!(error(quote! { Select }), "expected `Select => { .. }`");
        assert_eq!(
            error(quote! { Timeout => { act(a) } }),
            "expected `Timeout(length) => { .. }`"
        );
    }

    #[test]
    fn prefixes_only_go_on_the_right_branches() {
        assert_eq!(
            error(quote! { Sequence => { when(a) => act(b) } }),
            "`when` goes on a branch"
        );
        assert_eq!(
            error(quote! { Select => { score(a) => act(b) } }),
            "`score` goes in a `Scored`"
        );
        assert_eq!(
            error(quote! { Scored => { when(a) => act(b) } }),
            "a `Scored` takes `score(system)`"
        );
    }

    #[test]
    fn a_single_child_node_needs_one() {
        assert_eq!(error(quote! { Forever => {} }), "`Forever` needs a child");
    }

    #[test]
    fn a_tree_has_one_root() {
        let message = error(quote! { act(a), act(b) });
        assert!(message.contains("one root"), "{message}");
    }

    #[test]
    fn a_list_needs_a_list_to_go_in() {
        let message = error(quote! { @[ parts ] });
        assert!(message.contains("list of children"), "{message}");
    }

    #[test]
    fn a_number_of_seconds_is_checked_and_turned_into_a_duration() {
        let code = tree(quote! { Wait(1.5) }).unwrap().to_string();
        assert!(code.contains("from_nanos (1500000000u64)"), "{code}");
        let code = tree(quote! { Timeout(2) => { act(a) } })
            .unwrap()
            .to_string();
        assert!(code.contains("from_nanos (2000000000u64)"), "{code}");
    }

    #[test]
    fn anything_else_is_taken_as_a_duration() {
        let code = tree(quote! { Wait(cooldown) }).unwrap().to_string();
        assert!(code.contains("wait (cooldown)"), "{code}");
    }

    #[test]
    fn a_negative_length_does_not_compile() {
        assert_eq!(
            error(quote! { Wait(-1.0) }),
            "expected a length in seconds, 0 or more"
        );
    }
}
