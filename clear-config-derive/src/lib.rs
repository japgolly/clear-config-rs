use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Fields, parse_macro_input};

/// Derives the `ConfigParser` trait for enums or single-field unnamed structs.
///
/// # Supported Targets
///
/// 1. **Unit Enums**:
///    - Variant names are matched case-insensitively (e.g. `"dev"`, `"Dev"`, `"DEV"` match `Dev`).
///    - Variants can be renamed using `#[config(rename = "...")]`.
///
///    ```rust
///    #[derive(ConfigParser)]
///    enum Environment {
///        Dev,
///        Staging,
///        #[config(rename = "Prod")]
///        Production,
///    }
///    ```
///
/// 2. **Single-Field Unnamed Structs (Newtypes)**:
///    - Delegates parsing directly to the inner type via `<T>::parse_config(s).map(Self)`.
///    - Automatically adds `T: ConfigParser` trait bounds for generic parameters.
///
///    ```rust
///    #[derive(ConfigParser)]
///    struct Port(u16);
///
///    #[derive(ConfigParser)]
///    struct Wrapper<T>(T);
///    ```
///
/// Structs with named fields or more/fewer than 1 field are rejected.
#[proc_macro_derive(ConfigParser, attributes(config))]
pub fn derive_config_parser(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_config_parser_inner(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn derive_config_parser_inner(input: DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    match &input.data {
        Data::Enum(data_enum) => derive_config_parser_enum(&input, data_enum),
        Data::Struct(data_struct) => derive_config_parser_struct(&input, data_struct),
        Data::Union(_) => Err(syn::Error::new_spanned(
            &input.ident,
            "ConfigParser can only be derived for enums or single-field unnamed structs",
        )),
    }
}

fn derive_config_parser_enum(
    input: &DeriveInput,
    data_enum: &DataEnum,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let name = &input.ident;

    let mut match_arms = Vec::new();
    let mut variant_names = Vec::new();

    // Validate variants and build match arms
    for variant in &data_enum.variants {
        // Enforce unit variants (e.g., Development, not Development(String))
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                variant,
                "ConfigParser derive currently only supports unit enum variants (without fields)",
            ));
        }

        let v_ident = &variant.ident;
        let mut v_str = v_ident.to_string();

        for attr in &variant.attrs {
            if attr.path().is_ident("config") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        let value = meta.value()?;
                        let s: syn::LitStr = value.parse()?;
                        v_str = s.value();
                        Ok(())
                    } else {
                        Err(meta.error(
                            "unrecognized config attribute on enum variant (supported: `rename`)",
                        ))
                    }
                })?;
            }
        }

        variant_names.push(v_str.clone());

        // Support case-insensitive matching
        match_arms.push(quote! {
            s if s.eq_ignore_ascii_case(#v_str) => ::core::result::Result::Ok(#name::#v_ident),
        });
    }

    let expected_list = variant_names.join(", ");
    let type_name_str = name.to_string();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        const _: () = {
            use clear_config::{ConfigParser, ErrorMsg};

            impl #impl_generics ConfigParser for #name #ty_generics #where_clause {
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

    Ok(expanded)
}

fn derive_config_parser_struct(
    input: &DeriveInput,
    data_struct: &DataStruct,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    for attr in &input.attrs {
        if attr.path().is_ident("config") {
            return Err(syn::Error::new_spanned(
                attr,
                "unrecognized config attribute on struct",
            ));
        }
    }

    let name = &input.ident;

    let field = match &data_struct.fields {
        Fields::Named(_) => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "ConfigParser cannot be derived for structs with named fields",
            ));
        }
        Fields::Unnamed(fields) if fields.unnamed.len() != 1 => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                format!(
                    "ConfigParser on a struct requires exactly 1 field, found {}",
                    fields.unnamed.len()
                ),
            ));
        }
        Fields::Unit => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "ConfigParser on a struct requires exactly 1 field, found 0",
            ));
        }
        Fields::Unnamed(fields) => fields.unnamed.first().unwrap(),
    };

    for attr in &field.attrs {
        if attr.path().is_ident("config") {
            return Err(syn::Error::new_spanned(
                attr,
                "unrecognized config attribute on struct field",
            ));
        }
    }

    let inner_ty = &field.ty;

    let mut generics = input.generics.clone();
    for param in &mut generics.params {
        if let syn::GenericParam::Type(ref mut type_param) = *param {
            type_param
                .bounds
                .push(syn::parse_quote!(clear_config::ConfigParser));
        }
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        const _: () = {
            use clear_config::{ConfigParser, ErrorMsg};

            impl #impl_generics ConfigParser for #name #ty_generics #where_clause {
                fn parse_config(s: &str) -> ::core::result::Result<Self, ErrorMsg> {
                    <#inner_ty>::parse_config(s).map(Self)
                }
            }
        };
    };

    Ok(expanded)
}

/// Derives the `ConfigReader` trait for named structs.
///
/// Automatically generates code to read each field from a `ConfigContext`
/// using its uppercase field name (or an overridden key name).
///
/// # Struct Attributes
///
/// - `#[config(key_prefix = "...")]`: Prepends a prefix to all field keys within the struct.
///   Prefixes compose hierarchically on nested structs.
///
/// # Field Attributes
///
/// - `#[config(key = "...")]`: Overrides the default lookup key name (default is the field name in `UPPERCASE`).
/// - `#[config(default = "...")]`: Provides a fallback string value if the key is not set. Parsed via `ConfigParser`.
/// - `#[config(key_prefix = "...")]`: Adds a key prefix specifically for a nested struct field.
/// - `#[config(secret)]`: Marks the key as sensitive, masking its value in `report_used`.
///
/// # Field Types
///
/// - **Required fields (`T: ConfigParser`)**: Looked up via `ctx.need("KEY")` (or `ctx.get_or_parse` if `default` is specified).
/// - **Optional fields (`Option<T>`)**: Looked up via `ctx.get("KEY")`. Defaults to `None` if missing.
/// - **Nested configs (`T: ConfigReader`)**: Recursively loaded with `T::read(ctx)`, respecting nested prefixes.
///
/// # Example
///
/// ```rust
/// #[derive(ConfigReader)]
/// #[config(key_prefix = "APP_")]
/// struct AppConfig {
///     #[config(key = "DEBUG_MODE", default = "false")]
///     debug: bool,
///
///     #[config(secret)]
///     api_token: String,
///
///     log_level: Option<String>,
///
///     #[config(key_prefix = "DB_")]
///     database: DatabaseConfig,
/// }
/// ```
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
        let mut secret = false;

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
                    } else if meta.path.is_ident("secret") {
                        secret = true;
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized config attribute on field (supported: `key`, `default`, `key_prefix`, `secret`)"))
                    }
                });
                if let Err(e) = res {
                    return e.to_compile_error().into();
                }
            }
        }

        let raw_key = key.unwrap_or_else(|| field_ident.to_string().to_ascii_uppercase());
        let field_prefix = field_key_prefix.as_deref().unwrap_or("");
        let key_str = format!("{field_prefix}{raw_key}");
        let prefix_str = field_prefix;

        if secret {
            field_bindings.push(quote! {
                ctx.add_secret_key(#key_str);
            });
        }

        if let Some(inner_ty) = extract_option_inner(field_ty) {
            field_bindings.push(quote! {
                let #field_ident = ctx.get::<#inner_ty>(#key_str);
            });
        } else {
            let default_tokens = match default {
                Some(def_str) => quote! { ::core::option::Option::Some(#def_str) },
                None => quote! { ::core::option::Option::None },
            };
            field_bindings.push(quote! {
                let #field_ident = __Loader::<#field_ty>::new().read_field(ctx, #key_str, #prefix_str, #default_tokens);
            });
        }
    }

    let struct_prefix = struct_key_prefix.unwrap_or_default();
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
                fn read_field(
                    self,
                    ctx: &mut ConfigContext,
                    key: &str,
                    prefix: &str,
                    default: ::core::option::Option<&str>,
                ) -> ::core::option::Option<Self::Out>;
            }

            impl<T: ConfigReader> __LoadDef for __Loader<T> {
                type Out = T;
                fn read_field(
                    self,
                    ctx: &mut ConfigContext,
                    _key: &str,
                    prefix: &str,
                    _default: ::core::option::Option<&str>,
                ) -> ::core::option::Option<T> {
                    ctx.with_key_prefix(prefix, |ctx| T::read(ctx))
                }
            }

            trait __LoadParser {
                type Out;
                fn read_field(
                    self,
                    ctx: &mut ConfigContext,
                    key: &str,
                    prefix: &str,
                    default: ::core::option::Option<&str>,
                ) -> ::core::option::Option<Self::Out>;
            }

            impl<T: ConfigParser> __LoadParser for &__Loader<T> {
                type Out = T;
                fn read_field(
                    self,
                    ctx: &mut ConfigContext,
                    key: &str,
                    _prefix: &str,
                    default: ::core::option::Option<&str>,
                ) -> ::core::option::Option<T> {
                    match default {
                        ::core::option::Option::Some(def) => ctx.get_or_parse::<T>(key, def),
                        ::core::option::Option::None => ctx.need::<T>(key),
                    }
                }
            }

            impl #impl_generics ConfigReader for #name #ty_generics #where_clause {
                fn read(ctx: &mut ConfigContext) -> ::core::option::Option<Self> {
                    ctx.with_key_prefix(#struct_prefix, |ctx| {
                        #(#field_bindings)*

                        ::core::option::Option::Some(#name {
                            #(
                                #field_names: #field_names?,
                            )*
                        })
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
