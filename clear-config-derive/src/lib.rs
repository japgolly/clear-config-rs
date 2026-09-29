use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(ConfigParser)]
pub fn derive_config_parser(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // 1. Ensure it's an enum
    let Data::Enum(data_enum) = &input.data else {
        return syn::Error::new_spanned(&input.ident, "ConfigParser can only be derived for enums")
            .to_compile_error()
            .into();
    };

    let mut match_arms = Vec::new();
    let mut variant_names = Vec::new();

    // 2. Validate variants and build match arms
    for variant in &data_enum.variants {
        // Enforce unit variants (e.g., Development, not Development(String))
        if !matches!(variant.fields, Fields::Unit) {
            return syn::Error::new_spanned(
                variant,
                "ConfigParser derive currently only supports unit enum variants (without fields)",
            )
            .to_compile_error()
            .into();
        }

        let v_ident = &variant.ident;
        let v_str = v_ident.to_string();
        variant_names.push(v_str.clone());

        // Support case-insensitive matching
        match_arms.push(quote! {
            s if s.eq_ignore_ascii_case(#v_str) => ::core::result::Result::Ok(#name::#v_ident),
        });
    }

    let expected_list = variant_names.join(", ");
    let type_name_str = name.to_string();

    // 3. Generate the trait implementation
    let expanded = quote! {
        const _: () = {
            use clear_config::{ConfigParser, ErrorMsg};

            impl ConfigParser for #name {
                fn parse_config(s: &str) -> ::core::result::Result<Self, ErrorMsg> {
                    match s {
                        #(#match_arms)*
                        _ => ::core::result::Result::Err(ErrorMsg(format!(
                            "{s:?} is not a valid {} (expected one of: {})",
                            #type_name_str,
                            #expected_list
                        ))),
                    }
                }
            }
        };
    };

    TokenStream::from(expanded)
}
