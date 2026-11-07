//! The macros of every `bevy_arbor` part, each re-exported by its part.

mod expand;
mod parse;

use proc_macro::TokenStream;

/// Builds a `Behaviour` from the tree's shape, as `bevy_arbor_bt` documents.
///
/// ```ignore
/// behaviour_tree! {
///     Forever => {
///         Select => {
///             when(has_target) => Sequence => {
///                 task("Chase", Chase),
///                 Wait(1.0),
///             },
///             when(calm) => @ roam(),
///             Timeout(5.0) => { act(look_around) },
///         }
///     }
/// }
/// ```
///
/// `@ expr` is a `Behaviour` built elsewhere; `@[ expr ]`, in a list, is an
/// iterator of them (of `Branch`es in a `Select` or `Fallback`, and
/// `ScoredBranch`es in a `Scored`).
#[proc_macro]
pub fn behaviour_tree(input: TokenStream) -> TokenStream {
    expand::tree(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
