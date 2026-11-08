//! Where each part is, where a macro is used.

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

/// The path to `bevy_arbor_bt` where a macro is used: the crate itself, or
/// else the `bt` module of the `bevy_arbor` facade, however each is named.
pub(crate) fn bt() -> TokenStream {
    part(
        crate_name("bevy_arbor_bt").ok(),
        crate_name("bevy_arbor").ok(),
        "bevy_arbor_bt",
        "bt",
    )
}

/// The path to a part: the crate `name`, as `own` finds it, or else the module
/// `module` of the facade, as `facade` finds it.
///
/// `::name` in the part itself, which names itself so. Where neither is found,
/// as in a crate using them through another's re-export,
/// `::bevy_arbor::module`.
fn part(
    own: Option<FoundCrate>,
    facade: Option<FoundCrate>,
    name: &str,
    module: &str,
) -> TokenStream {
    let module = Ident::new(module, Span::call_site());
    match (own, facade) {
        (Some(FoundCrate::Itself), _) => {
            let name = Ident::new(name, Span::call_site());
            quote! { ::#name }
        }
        (Some(FoundCrate::Name(name)), _) => {
            let name = Ident::new(&name, Span::call_site());
            quote! { ::#name }
        }
        (None, Some(FoundCrate::Name(name))) => {
            let name = Ident::new(&name, Span::call_site());
            quote! { ::#name::#module }
        }
        (None, Some(FoundCrate::Itself) | None) => quote! { ::bevy_arbor::#module },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bt(own: Option<FoundCrate>, facade: Option<FoundCrate>) -> String {
        part(own, facade, "bevy_arbor_bt", "bt").to_string()
    }

    #[test]
    fn paths_go_through_the_part_or_the_facade_as_they_are_named() {
        let name = |name: &str| Some(FoundCrate::Name(name.to_owned()));
        assert_eq!(bt(name("bt"), None), ":: bt");
        assert_eq!(bt(name("bt"), name("ai")), ":: bt", "the part first");
        assert_eq!(bt(None, name("ai")), ":: ai :: bt");
        assert_eq!(bt(Some(FoundCrate::Itself), None), ":: bevy_arbor_bt");
        assert_eq!(bt(None, None), ":: bevy_arbor :: bt");
    }
}
