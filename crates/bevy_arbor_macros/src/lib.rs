//! The macros of every `bevy_arbor` part, each re-exported by its part.

mod path;
mod tree;

use proc_macro::TokenStream;

/// Builds a `Behavior` from the tree's shape, as `bevy_arbor_bt` documents.
///
/// ```ignore
/// behavior_tree! {
///     Forever => {
///         Select => {
///             when(has_target) => Sequence => {
///                 task(Chase),
///                 Wait(1.0),
///             },
///             when(calm) => @ roam(),
///             Timeout(5.0) => { act(look_around) },
///         }
///     }
/// }
/// ```
///
/// `@ expr` is a `Behavior` built elsewhere; `@[ expr ]`, in a list, is an
/// iterator of them (of `Branch`es in a `Select` or `Fallback`, and
/// `ScoredBranch`es in a `Scored`).
#[proc_macro]
pub fn behavior_tree(input: TokenStream) -> TokenStream {
    tree::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
