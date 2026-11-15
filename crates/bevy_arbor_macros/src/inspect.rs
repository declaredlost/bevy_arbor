//! `#[derive(Inspect)]`: a type showing in a trace as its name, with fields.
//!
//! The fields are those it marks as its parameters or its children.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Index, Member, Result};

use crate::path;

/// What `#[inspect(..)]` takes.
const EXPECTED: &str = "expected `param`, `child` or `children`";

/// What a field marked `#[inspect(..)]` is.
enum Mark {
    /// A parameter, in brackets after the name: `#[inspect(param)]`.
    Param,
    /// A child, under it: `#[inspect(child)]`.
    Child,
    /// Children, each under it: `#[inspect(children)]`.
    Children,
}

/// The `Inspect` impl for `input`.
pub(crate) fn derive(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let bt = path::bt();
    let ident = &input.ident;
    let name = ident.to_string();
    let fields = match &input.data {
        Data::Struct(data) => marked(&data.fields)?,
        Data::Enum(_) => Vec::new(),
        Data::Union(_) => {
            return Err(Error::new_spanned(
                ident,
                "`Inspect` can't be derived for a union",
            ));
        }
    };
    let writes = fields.iter().map(|(member, mark)| match mark {
        Mark::Param => quote! { inspection.param(&self.#member); },
        Mark::Child => quote! { inspection.child(&self.#member); },
        Mark::Children => quote! {
            for child in &self.#member {
                inspection.child(child);
            }
        },
    });
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #bt::inspect::Inspect for #ident #ty_generics #where_clause {
            fn inspect(&self, inspection: &mut #bt::inspect::Inspection<'_>) {
                inspection.name(#name);
                #(#writes)*
            }
        }
    })
}

/// The fields marked `#[inspect(..)]`, in order, with what each is.
fn marked(fields: &Fields) -> Result<Vec<(Member, Mark)>> {
    let mut marked = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        for attr in field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("inspect"))
        {
            let mut mark = None;
            attr.parse_nested_meta(|meta| {
                mark = Some(if meta.path.is_ident("param") {
                    Mark::Param
                } else if meta.path.is_ident("child") {
                    Mark::Child
                } else if meta.path.is_ident("children") {
                    Mark::Children
                } else {
                    return Err(meta.error(EXPECTED));
                });
                Ok(())
            })?;
            let Some(mark) = mark else {
                return Err(Error::new_spanned(attr, EXPECTED));
            };
            let member = match &field.ident {
                Some(ident) => Member::Named(ident.clone()),
                None => Member::Unnamed(Index::from(index)),
            };
            marked.push((member, mark));
        }
    }
    Ok(marked)
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::derive;

    #[test]
    fn it_writes_the_name_and_the_marked_fields_in_order() {
        let code = derive(quote! {
            struct Alternate<T> {
                #[inspect(children)]
                children: [Behavior; 2],
                #[inspect(param)]
                next: usize,
                unseen: T,
            }
        })
        .unwrap()
        .to_string();
        assert!(code.contains("inspection . name (\"Alternate\")"), "{code}");
        assert!(code.contains("for child in & self . children"), "{code}");
        assert!(
            code.contains("inspection . param (& self . next)"),
            "{code}"
        );
        assert!(!code.contains("unseen"), "{code}");
        assert!(code.contains("impl < T >"), "{code}");
    }

    #[test]
    fn an_unknown_mark_says_what_there_is() {
        let error = derive(quote! {
            struct Bad {
                #[inspect(label)]
                field: u32,
            }
        })
        .unwrap_err()
        .to_string();
        assert_eq!(error, "expected `param`, `child` or `children`");
    }
}
