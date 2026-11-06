use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Error, Expr, Ident, Result, Token, braced, bracketed, parenthesized};

/// What the macro takes: one node.
pub(crate) struct Root(pub(crate) Item);

/// A node in the tree's shape.
pub(crate) enum Item {
    /// `@ expr`: a node built elsewhere.
    Embed(Expr),
    /// `Name(args) => { .. }`, either part being optional.
    Named {
        name: Ident,
        args: Vec<Expr>,
        body: Option<Vec<Entry>>,
    },
}

/// One of a list of children or branches.
pub(crate) enum Entry {
    /// `@[ expr ]`: an iterator of them.
    Many(Expr),
    /// `when(..) => item`, `score(..) => item`, or just `item`.
    One { prefix: Option<Prefix>, item: Item },
}

/// What can come before a branch.
pub(crate) struct Prefix {
    /// `when` or `score`.
    pub(crate) name: Ident,
    pub(crate) args: Vec<Expr>,
}

/// Whether the next tokens are `@[`.
fn many_next(input: ParseStream) -> bool {
    input.peek(Token![@]) && input.peek2(syn::token::Bracket)
}

/// A parenthesized list of expressions.
fn args(input: ParseStream) -> Result<Vec<Expr>> {
    let inside;
    parenthesized!(inside in input);
    Ok(Punctuated::<Expr, Token![,]>::parse_terminated(&inside)?
        .into_iter()
        .collect())
}

impl Parse for Root {
    fn parse(input: ParseStream) -> Result<Self> {
        if many_next(input) {
            return Err(input.error("`@[ .. ]` goes in a list of children"));
        }
        if input.is_empty() {
            return Err(Error::new(Span::call_site(), "a tree needs a node"));
        }
        let item = input.parse()?;
        input.parse::<Option<Token![,]>>()?;
        if !input.is_empty() {
            return Err(input.error("a tree has one root: put these in a `Sequence` or `Select`"));
        }
        Ok(Self(item))
    }
}

impl Parse for Item {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.parse::<Option<Token![@]>>()?.is_some() {
            return Ok(Self::Embed(input.parse()?));
        }
        let name: Ident = input.parse()?;
        let args = if input.peek(syn::token::Paren) {
            args(input)?
        } else {
            Vec::new()
        };
        let mut body = None;
        if input.parse::<Option<Token![=>]>>()?.is_some() {
            let inside;
            braced!(inside in input);
            body = Some(
                Punctuated::<Entry, Token![,]>::parse_terminated(&inside)?
                    .into_iter()
                    .collect(),
            );
        }
        Ok(Self::Named { name, args, body })
    }
}

impl Parse for Entry {
    fn parse(input: ParseStream) -> Result<Self> {
        if many_next(input) {
            input.parse::<Token![@]>()?;
            let inside;
            bracketed!(inside in input);
            return Ok(Self::Many(inside.parse()?));
        }
        // `when(` and `score(` start a branch: no node's called that.
        let fork = input.fork();
        let prefixed = fork
            .parse::<Ident>()
            .is_ok_and(|name| (name == "when" || name == "score") && fork.peek(syn::token::Paren));
        let mut prefix = None;
        if prefixed {
            let name = input.parse()?;
            let args = args(input)?;
            input.parse::<Token![=>]>()?;
            prefix = Some(Prefix { name, args });
        }
        Ok(Self::One {
            prefix,
            item: input.parse()?,
        })
    }
}
