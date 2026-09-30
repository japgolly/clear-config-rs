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

#[proc_macro_derive(ConfigReader, attributes(config))]
pub fn derive_config_def(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let Data::Struct(data_struct) = &input.data else {
        return syn::Error::new_spanned(
            &input.ident,
            "ConfigReader can only be derived for structs",
        )
        .to_compile_error()
        .into();
    };

    let Fields::Named(fields_named) = &data_struct.fields else {
        return syn::Error::new_spanned(
            &input.ident,
            "ConfigReader can only be derived for structs with named fields",
        )
        .to_compile_error()
        .into();
    };

    // Parse struct-level attributes (e.g. #[config(key_prefix = "SERVER_")])
    let mut struct_key_prefix = None;
    for attr in &input.attrs {
        if attr.path().is_ident("config") {
            let res = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("key_prefix") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    struct_key_prefix = Some(s.value());
                    Ok(())
                } else {
                    Err(meta
                        .error("unrecognized config attribute on struct (supported: `key_prefix`)"))
                }
            });
            if let Err(e) = res {
                return e.to_compile_error().into();
            }
        }
    }

    let mut field_bindings = Vec::new();
    let mut field_names = Vec::new();

    for field in &fields_named.named {
        let field_ident = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;
        field_names.push(field_ident);

        let mut key = None;
        let mut default = None;
        let mut field_key_prefix = None;

        for attr in &field.attrs {
            if attr.path().is_ident("config") {
                let res = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("key") {
                        let value = meta.value()?;
                        let s: syn::LitStr = value.parse()?;
                        key = Some(s.value());
                        Ok(())
                    } else if meta.path.is_ident("default") {
                        let value = meta.value()?;
                        let s: syn::LitStr = value.parse()?;
                        default = Some(s.value());
                        Ok(())
                    } else if meta.path.is_ident("key_prefix") {
                        let value = meta.value()?;
                        let s: syn::LitStr = value.parse()?;
                        field_key_prefix = Some(s.value());
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized config attribute on field (supported: `key`, `default`, `key_prefix`)"))
                    }
                });
                if let Err(e) = res {
                    return e.to_compile_error().into();
                }
            }
        }

        let raw_key = key.unwrap_or_else(|| field_ident.to_string().to_ascii_uppercase());
        let prefix = field_key_prefix
            .as_deref()
            .or(struct_key_prefix.as_deref())
            .unwrap_or("");
        let key_str = format!("{prefix}{raw_key}");

        if let Some(inner_ty) = extract_option_inner(field_ty) {
            field_bindings.push(quote! {
                let #field_ident = ctx.get::<#inner_ty>(&::std::string::String::from(#key_str));
            });
        } else {
            let default_tokens = match default {
                Some(def_str) => quote! { ::core::option::Option::Some(#def_str) },
                None => quote! { ::core::option::Option::None },
            };
            field_bindings.push(quote! {
                let #field_ident = __Loader::<#field_ty>::new().read_field(ctx, #key_str, #default_tokens);
            });
        }
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        const _: () = {
            use clear_config::{ConfigContext, ConfigReader, ConfigParser};

            struct __Loader<T>(::core::marker::PhantomData<T>);

            impl<T> __Loader<T> {
                fn new() -> Self {
                    __Loader(::core::marker::PhantomData)
                }
            }

            trait __LoadDef {
                type Out;
                fn read_field(self, ctx: &mut ConfigContext, key: &str, default: ::core::option::Option<&str>) -> ::core::option::Option<Self::Out>;
            }

            impl<T: ConfigReader> __LoadDef for __Loader<T> {
                type Out = T;
                fn read_field(self, ctx: &mut ConfigContext, _key: &str, _default: ::core::option::Option<&str>) -> ::core::option::Option<T> {
                    T::read(ctx)
                }
            }

            trait __LoadParser {
                type Out;
                fn read_field(self, ctx: &mut ConfigContext, key: &str, default: ::core::option::Option<&str>) -> ::core::option::Option<Self::Out>;
            }

            impl<T: ConfigParser> __LoadParser for &__Loader<T> {
                type Out = T;
                fn read_field(self, ctx: &mut ConfigContext, key: &str, default: ::core::option::Option<&str>) -> ::core::option::Option<T> {
                    let key_str = ::std::string::String::from(key);
                    match default {
                        ::core::option::Option::Some(def) => ctx.get_or_parse::<T>(&key_str, def),
                        ::core::option::Option::None => ctx.need::<T>(&key_str),
                    }
                }
            }

            impl #impl_generics ConfigReader for #name #ty_generics #where_clause {
                fn read(ctx: &mut ConfigContext) -> ::core::option::Option<Self> {
                    #(#field_bindings)*

                    ::core::option::Option::Some(#name {
                        #(
                            #field_names: #field_names?,
                        )*
                    })
                }
            }
        };
    };

    TokenStream::from(expanded)
}

fn extract_option_inner(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        if type_path.qself.is_none() {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Option" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            return Some(inner_ty);
                        }
                    }
                }
            }
        }
    }
    None
}
