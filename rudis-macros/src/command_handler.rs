use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Fields, Ident};

// CommandHandler's attribute structure
pub(crate) struct CommandAttr {
    pub(crate) command_name: String,
}

impl Parse for CommandAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // parse the command name as a string literal
        let command_name_lit: syn::LitStr = input.parse()?;
        let command_name = command_name_lit.value();

        Ok(CommandAttr { command_name })
    }
}

/// generate TryFrom<Vec<RespValue>> implementation for the struct
pub(crate) fn generate_try_from_impl(
    struct_name: &Ident,
    fields: &Fields,
    command_name: &str,
) -> proc_macro2::TokenStream {
    let field_count = match fields {
        Fields::Named(named_fields) => named_fields.named.len(),
        Fields::Unnamed(unnamed_fields) => unnamed_fields.unnamed.len(),
        Fields::Unit => 0,
    };
    let (destructure_pattern, field_conversions) = generate_field_code(fields);
    let (arg_check, destructure_code) = if field_count == 0 {
        (
            quote! {
                if !values.is_empty() {
                    return Err(crate::command::error::CommandError::InvalidArgumentNumber(
                        #command_name.to_string(),
                        #field_count,
                    ));
                }
            },
            quote! {},
        )
    } else {
        (
            quote! {
                if values.len() != #field_count {
                    return Err(crate::command::error::CommandError::InvalidArgumentNumber(
                        #command_name.to_string(),
                        #field_count,
                    ));
                }
            },
            quote! {
                let arr: [crate::resp::RespValue; #field_count] = values
                    .try_into()
                    .unwrap_or_else(|_| ::std::unreachable!());
                let #destructure_pattern = arr;
            },
        )
    };

    quote! {
        impl TryFrom<Vec<crate::resp::RespValue>> for #struct_name {
            type Error = crate::command::error::CommandError;

            fn try_from(values: Vec<crate::resp::RespValue>) -> Result<Self, Self::Error> {
                #arg_check
                #destructure_code

                Ok(Self {
                    #field_conversions
                })
            }
        }
    }
}

/// generate command handler registration code
pub(crate) fn generate_command_handler(
    struct_name: &Ident,
    command_name: &str,
) -> proc_macro2::TokenStream {
    let register_fn_name = syn::Ident::new(
        &format!("__register_{}_command", command_name.to_lowercase()),
        proc_macro2::Span::call_site(),
    );

    quote! {
        #[::ctor::ctor]
        fn #register_fn_name() {
            use ::std::sync::Arc;
            use ::std::future::Future;
            use ::std::pin::Pin;
            use ::std::boxed::Box;

            const COMMAND_NAME: &str = #command_name;

            fn handler_func(
                ctx: ::std::sync::Arc<crate::context::Context>,
                args: Vec<crate::resp::RespValue>
            ) -> crate::command::registry::CommandFuture {
                Box::pin(async move {
                    let cmd: #struct_name = args.try_into()?;
                    cmd.execute(ctx).await
                })
            }

            crate::command::registry::en_register_queue(COMMAND_NAME, handler_func);
        }
    }
}

/// generate field destructuring and conversion code
fn generate_field_code(
    fields: &Fields,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    match fields {
        Fields::Named(named_fields) => {
            let field_idents: Vec<&Ident> = named_fields
                .named
                .iter()
                .filter_map(|f| f.ident.as_ref())
                .collect();
            let destructure_pattern = quote! { [#(#field_idents),*] };
            // Convert each resp value to field using FromRespValue
            let field_conversions = field_idents.iter().map(|field_ident| {
                quote! {
                    #field_ident: crate::resp::FromRespValue::from_resp_value(#field_ident)?,
                }
            });

            (destructure_pattern, quote! { #(#field_conversions)* })
        }
        Fields::Unnamed(unnamed_fields) => {
            let field_count = unnamed_fields.unnamed.len();
            let field_idents: Vec<Ident> = (0..field_count)
                .map(|i| syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site()))
                .collect();
            let destructure_pattern = quote! { [#(#field_idents),*] };
            let field_conversions = field_idents.iter().map(|field_ident| {
                quote! {
                    crate::resp::FromRespValue::from_resp_value(#field_ident)?,
                }
            });

            (destructure_pattern, quote! { #(#field_conversions)* })
        }
        Fields::Unit => (quote! { [] }, quote! {}),
    }
}
