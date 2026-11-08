//! The code building a tree, from its shape.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Error, Ident, Result};

use super::decorator::decorator_args;
use super::length::length;
use super::node::Node;
use super::syntax::{Entry, Item, Prefix};

/// Writes the code building a tree, through `bevy_arbor_bt` at `bt`.
pub(super) struct Expand {
    bt: TokenStream,
}

impl Expand {
    pub(super) fn new(bt: TokenStream) -> Self {
        Self { bt }
    }

    /// The code building the tree whose root is `root`.
    pub(super) fn tree(&self, root: &Item) -> Result<TokenStream> {
        self.item(root)
    }

    /// The code building one node.
    fn item(&self, item: &Item) -> Result<TokenStream> {
        let bt = &self.bt;
        let (name, args, body) = match item {
            Item::Embed(node) => return Ok(quote! { #bt::behavior::Behavior::from(#node) }),
            Item::Named { name, args, body } => (name, args.as_slice(), body.as_deref()),
        };
        let label = name.to_string();
        let Some(node) = Node::named(&label) else {
            return Err(Error::new(name.span(), Node::unknown(&label)));
        };
        let behavior = quote! { #bt::behavior::Behavior };
        match (node, args, body) {
            (Node::Forever, [], Some(body)) => self.one(name, "forever", &[], body),
            (Node::Invert, [], Some(body)) => self.one(name, "invert", &[], body),
            (Node::Succeed, [], Some(body)) => self.one(name, "succeed", &[], body),
            (Node::Commit, [], Some(body)) => self.one(name, "commit", &[], body),
            (Node::Repeat, [times], Some(body)) => {
                self.one(name, "repeat", &[quote! { #times }], body)
            }
            (Node::Timeout, [time], Some(body)) => {
                self.one(name, "timeout", &[length(time)?], body)
            }
            (Node::State, [bundle], Some(body)) => {
                self.one(name, "state", &[quote! { #bundle }], body)
            }
            (Node::RepeatWhile, [_] | [_, _], Some(body)) => {
                let decorator = decorator_args(bt, name.span(), args)?;
                self.one(name, "repeat_while", &[decorator], body)
            }
            (Node::Sequence, [], Some(body)) => self.list(name, "sequence", &[], body),
            (Node::All, [], Some(body)) => self.list(name, "all", &[], body),
            (Node::Any, [], Some(body)) => self.list(name, "any", &[], body),
            (Node::Race, [], Some(body)) => self.list(name, "race", &[], body),
            (Node::Random, [roll], Some(body)) => {
                self.list(name, "random", &[quote! { #roll }], body)
            }
            (Node::Select, [], Some(body)) => {
                self.branches(name, "select", Branches::Decorated, body)
            }
            (Node::Fallback, [], Some(body)) => {
                self.branches(name, "fallback", Branches::Decorated, body)
            }
            (Node::Scored, [], Some(body)) => self.branches(name, "scored", Branches::Scored, body),
            (Node::Wait, [time], None) => {
                let time = length(time)?;
                Ok(quote! { #behavior::wait(#time) })
            }
            (Node::Socket, [socket], None) => Ok(quote! { #behavior::socket(#socket) }),
            (Node::Task, [bundle], None) => Ok(quote! { #behavior::task(#bundle) }),
            (Node::Task, [label, bundle], None) => {
                Ok(quote! { #behavior::task_named(#label, #bundle) })
            }
            (Node::Act, [system], None) => Ok(quote! { #behavior::act(#system) }),
            (Node::Act, [system, param], None) => {
                Ok(quote! { #behavior::act_with(#system, #param) })
            }
            _ => Err(Error::new(
                name.span(),
                format!("expected {}", node.usage()),
            )),
        }
    }

    /// A node over one child, built by `build` with `args` before it: several
    /// children run as a sequence.
    fn one(
        &self,
        name: &Ident,
        build: &str,
        args: &[TokenStream],
        body: &[Entry],
    ) -> Result<TokenStream> {
        if body.is_empty() {
            return Err(Error::new(name.span(), format!("`{name}` needs a child")));
        }
        let bt = &self.bt;
        let build = Ident::new(build, name.span());
        let children = self.children(body)?;
        Ok(quote! {
            #bt::behavior::Behavior::#build(#(#args,)* #bt::behavior::Behavior::group(#children))
        })
    }

    /// A node over a list of children, built by `build` with `args` before it.
    fn list(
        &self,
        name: &Ident,
        build: &str,
        args: &[TokenStream],
        body: &[Entry],
    ) -> Result<TokenStream> {
        let bt = &self.bt;
        let build = Ident::new(build, name.span());
        let children = self.children(body)?;
        Ok(quote! { #bt::behavior::Behavior::#build(#(#args,)* #children) })
    }

    /// A node over branches, built by `build`.
    fn branches(
        &self,
        name: &Ident,
        build: &str,
        kind: Branches,
        body: &[Entry],
    ) -> Result<TokenStream> {
        let (bt, out) = (&self.bt, out());
        let build = Ident::new(build, name.span());
        let ty = match kind {
            Branches::Decorated => quote! { #bt::behavior::Branch },
            Branches::Scored => quote! { #bt::behavior::ScoredBranch },
        };
        let pushes = body
            .iter()
            .map(|entry| match entry {
                Entry::Many(branches) => Ok(quote! { #out.extend(#branches); }),
                Entry::One { prefix, item } => {
                    let branch = self.branch(kind, prefix.as_ref(), self.item(item)?)?;
                    Ok(quote! { #out.push(#branch); })
                }
            })
            .collect::<Result<Vec<_>>>()?;
        if pushes.is_empty() {
            return Ok(quote! { #bt::behavior::Behavior::#build(::std::vec::Vec::<#ty>::new()) });
        }
        Ok(quote! {{
            let mut #out: ::std::vec::Vec<#ty> = ::std::vec::Vec::new();
            #(#pushes)*
            #bt::behavior::Behavior::#build(#out)
        }})
    }

    /// The code building a list of children.
    fn children(&self, body: &[Entry]) -> Result<TokenStream> {
        let (bt, out) = (&self.bt, out());
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
                    let node = self.item(item)?;
                    Ok(quote! { #out.push(#node); })
                }
            })
            .collect::<Result<Vec<_>>>()?;
        if pushes.is_empty() {
            return Ok(quote! { ::std::vec::Vec::<#bt::behavior::Behavior>::new() });
        }
        Ok(quote! {{
            let mut #out: ::std::vec::Vec<#bt::behavior::Behavior> = ::std::vec::Vec::new();
            #(#pushes)*
            #out
        }})
    }

    /// The code building one branch of `node`, as `kind` takes them.
    fn branch(
        &self,
        kind: Branches,
        prefix: Option<&Prefix>,
        node: TokenStream,
    ) -> Result<TokenStream> {
        let bt = &self.bt;
        let Some(prefix) = prefix else {
            return Ok(match kind {
                Branches::Decorated => quote! { #bt::behavior::Branch::always(#node) },
                Branches::Scored => quote! { #bt::behavior::ScoredBranch::last(#node) },
            });
        };
        let span = prefix.name.span();
        match (
            kind,
            prefix.name.to_string().as_str(),
            prefix.args.as_slice(),
        ) {
            (Branches::Decorated, "when", args) => {
                let decorator = decorator_args(bt, span, args)?;
                Ok(quote! { #bt::behavior::Branch::when(#decorator, #node) })
            }
            (Branches::Scored, "score", [system]) => {
                Ok(quote! { #bt::behavior::ScoredBranch::new(#system, #node) })
            }
            (Branches::Scored, "score", _) => Err(Error::new(span, "expected `score(system)`")),
            (Branches::Scored, ..) => Err(Error::new(span, "a `Scored` takes `score(system)`")),
            (Branches::Decorated, ..) => Err(Error::new(span, "`score` goes in a `Scored`")),
        }
    }
}

/// What a node over branches takes them as.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Branches {
    /// Decorated: `when(..) => node`, as a `Select` or `Fallback` takes them.
    Decorated,
    /// Scored: `score(..) => node`, as a `Scored` takes them.
    Scored,
}

/// The name of the list being built: invisible to what goes in it, so an
/// expression there can use the same name.
fn out() -> Ident {
    Ident::new("out", Span::mixed_site())
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use crate::tree::tests::{error, tree};

    #[test]
    fn every_node_expands() {
        let expanded = tree(quote! {
            Forever => {
                Select => {
                    when(decorator) => Sequence => { task("Named", A), task(B), Wait(1.0), },
                    when(!a && (b || c)) => @ part(),
                    when(within, 5.0) => Commit => { act(system) },
                    Timeout(2.0) => { act(system, 1) },
                    All => { @ part(), @[ parts ] },
                    Any => { Invert => { act(system) }, Succeed => { act(system) } },
                    Race => { act(a), act(b) },
                    RepeatWhile(decorator) => { act(a) },
                    RepeatWhile(within, 2.0) => { act(a) },
                    State(Hunting) => { act(a) },
                    Socket(Weapon),
                    Fallback => { when(a) => act(b), act(c) },
                    Scored => { score(a) => act(b), act(c) },
                    Random(roll) => { act(a) },
                    Repeat(3) => { act(a) },
                    @[ branches ],
                }
            }
        });
        assert!(expanded.is_ok(), "{:?}", expanded.err());
    }

    #[test]
    fn its_code_goes_through_the_path_it_is_given() {
        let code = tree(quote! { Select => { when(a) => act(b) } })
            .unwrap()
            .to_string();
        assert!(code.contains(":: arbor :: behavior :: Branch"), "{code}");
    }

    #[test]
    fn a_node_used_wrongly_says_how_it_goes() {
        assert_eq!(error(quote! { Wait }), "expected `Wait(length)`");
        assert_eq!(
            error(quote! { task }),
            "expected `task(bundle)` or `task(name, bundle)`"
        );
        assert_eq!(
            error(quote! { act(a, b, c) }),
            "expected `act(system)` or `act(system, param)`"
        );
        assert_eq!(error(quote! { Select }), "expected `Select => { .. }`");
        assert_eq!(
            error(quote! { Timeout => { act(a) } }),
            "expected `Timeout(length) => { .. }`"
        );
        assert_eq!(
            error(quote! { RepeatWhile(a, b, c) => { act(d) } }),
            "expected `RepeatWhile(decorator) => { .. }`"
        );
        assert_eq!(
            error(quote! { Wait(1.0) => { act(a) } }),
            "expected `Wait(length)`"
        );
    }

    #[test]
    fn a_task_is_named_after_its_bundle_or_by_name() {
        let code = tree(quote! { task(Chase) }).unwrap().to_string();
        assert!(code.contains("Behavior :: task (Chase)"), "{code}");
        let code = tree(quote! { task("Run", Chase) }).unwrap().to_string();
        assert!(
            code.contains("Behavior :: task_named (\"Run\" , Chase)"),
            "{code}"
        );
    }

    #[test]
    fn lengths_in_nodes_are_checked() {
        assert!(
            tree(quote! { Timeout(2) => { act(a) } })
                .unwrap()
                .to_string()
                .contains("from_nanos (2000000000u64)")
        );
        assert_eq!(
            error(quote! { Timeout(2e10) => { act(a) } }),
            "too long: a length is at most 18446744073 seconds"
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
        assert_eq!(
            error(quote! { Scored => { score(a, b) => act(c) } }),
            "expected `score(system)`"
        );
    }

    #[test]
    fn a_single_child_node_needs_one() {
        assert_eq!(error(quote! { Forever => {} }), "`Forever` needs a child");
    }

    #[test]
    fn empty_lists_and_branches_still_build() {
        assert!(tree(quote! { Sequence => {} }).is_ok());
        assert!(tree(quote! { Select => {} }).is_ok());
    }
}
