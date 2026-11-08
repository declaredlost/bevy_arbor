//! `behavior_tree!`: a tree's shape, read and turned into the code building
//! it.
//!
//! [`syntax`] reads the shape, and [`expand`](mod@expand) writes the code: each
//! [`node`], with its [`decorator`]s and [`length`]s.

mod decorator;
mod expand;
mod length;
mod node;
mod syntax;

use proc_macro2::TokenStream;
use syn::Result;

use self::expand::Expand;
use self::syntax::Root;

/// The code building the tree `input` describes.
pub(crate) fn expand(input: TokenStream) -> Result<TokenStream> {
    let Root(root) = syn::parse2(input)?;
    Expand::new(crate::path::bt()).tree(&root)
}

#[cfg(test)]
pub(crate) mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::Result;

    use super::expand::Expand;
    use super::syntax::Root;

    /// What `input` expands to, through `::arbor`.
    pub(crate) fn tree(input: TokenStream) -> Result<TokenStream> {
        let Root(root) = syn::parse2(input)?;
        Expand::new(quote! { ::arbor }).tree(&root)
    }

    /// The error `input` gives.
    pub(crate) fn error(input: TokenStream) -> String {
        tree(input).unwrap_err().to_string()
    }
}
