use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Result, Type, parse_macro_input};

/// Derive macro that generates Default implementation
/// for configuration structs with inline default values.
///
/// The macro intelligently handles String fields by automatically
/// converting string literals with `.into()`. All other types
/// use the default value as-is, relying on Rust's type inference.
///
/// # Example
/// ```
/// use stardrift_macros::ConfigDefaults;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(ConfigDefaults, Serialize, Deserialize)]
/// #[serde(default)]
/// pub struct PhysicsConfig {
///     #[default(0.001)]
///     pub gravitational_constant: f64,
///     
///     #[default(100)]  // Type inference handles this
///     pub body_count: usize,
///     
///     #[default(None)]
///     pub optional_field: Option<u64>,
///     
///     #[default("simulation")]  // Automatically converts to String
///     pub name: String,
/// }
///
/// let config = PhysicsConfig::default();
/// assert_eq!(config.gravitational_constant, 0.001);
/// assert_eq!(config.body_count, 100);
/// assert_eq!(config.optional_field, None);
/// assert_eq!(config.name, "simulation");
/// ```
///
/// # Requirements
///
/// Every field in the struct must have a `#[default(...)]` attribute specifying
/// its default value. The value can be any valid Rust expression.
///
/// # Errors
///
/// The macro will produce a compile error if:
/// - Applied to anything other than a struct with named fields
/// - Any field is missing a `#[default(...)]` attribute
/// - The default attribute syntax is invalid
#[proc_macro_derive(ConfigDefaults, attributes(default))]
pub fn config_defaults(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate_default_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate the Default implementation for the struct
fn generate_default_impl(input: DeriveInput) -> Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Validate that this is a struct with named fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            Fields::Unnamed(_) => {
                return Err(Error::new_spanned(
                    &input,
                    "ConfigDefaults only supports structs with named fields",
                ));
            }
            Fields::Unit => {
                return Err(Error::new_spanned(
                    &input,
                    "ConfigDefaults cannot be derived for unit structs",
                ));
            }
        },
        Data::Enum(_) => {
            return Err(Error::new_spanned(
                &input,
                "ConfigDefaults can only be derived for structs, not enums",
            ));
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                &input,
                "ConfigDefaults can only be derived for structs, not unions",
            ));
        }
    };

    // Process each field and extract its default value
    let field_defaults = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_type = &field.ty;
            let default_value = extract_default_value(field)?;

            // Only String fields need special handling for &str -> String conversion
            // All other types (including numerics) work with Rust's type inference
            let field_init = if is_string_type(field_type) {
                quote! { #field_name: ::std::convert::Into::into(#default_value) }
            } else {
                quote! { #field_name: #default_value }
            };

            Ok(field_init)
        })
        .collect::<Result<Vec<_>>>()?;

    // Generate the Default implementation
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics Default for #name #ty_generics #where_clause {
            fn default() -> Self {
                Self {
                    #(#field_defaults),*
                }
            }
        }
    })
}

/// Check if the type is String
fn is_string_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "String";
        }
    }
    false
}

/// Extract the default value from a field's #[default(...)] attribute
fn extract_default_value(field: &syn::Field) -> Result<proc_macro2::TokenStream> {
    let field_name = field
        .ident
        .as_ref()
        .map(|i| i.to_string())
        .unwrap_or_else(|| "unnamed field".to_string());

    // Look for #[default(...)] attribute
    for attr in &field.attrs {
        if attr.path().is_ident("default") {
            // Parse the tokens inside the parentheses
            let tokens: proc_macro2::TokenStream = attr.parse_args().map_err(|e| {
                Error::new_spanned(
                    attr,
                    format!(
                        "Failed to parse default attribute for field '{}': {}",
                        field_name, e
                    ),
                )
            })?;

            // Validate that we got something
            if tokens.is_empty() {
                return Err(Error::new_spanned(
                    attr,
                    format!(
                        "Field '{}' has an empty #[default()] attribute. Please provide a default value.",
                        field_name
                    ),
                ));
            }

            return Ok(tokens);
        }
    }

    // No #[default(...)] attribute found
    Err(Error::new_spanned(
        field,
        format!(
            "Field '{}' must have a #[default(...)] attribute specifying its default value",
            field_name
        ),
    ))
}
