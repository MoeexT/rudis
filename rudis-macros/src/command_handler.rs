use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Fields, Ident};

// CommandHandler's attribute structure
pub(crate) struct CommandAttr {
    pub(crate) command_name: String,
}

impl Parse for CommandAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(CommandAttr {
                command_name: String::new(),
            });
        }
        // parse the command name as a string literal
        let command_name_lit: syn::LitStr = input.parse()?;
        let command_name = command_name_lit.value();

        Ok(CommandAttr { command_name })
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
                args: crate::command::parser::Parser,
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

pub(crate) fn generate_from_parse_impl(
    struct_name: &Ident,
    fields: &Fields,
    command_name: &str,
) -> proc_macro2::TokenStream {
    let (parse_statement, field_conversions) = generate_fields_from_parser(fields);
    let required_args_count = calculate_required_args_count(fields);
    quote!(
        impl TryFrom<crate::command::parser::Parser> for #struct_name {
            type Error = crate::command::error::CommandError;

            fn try_from(mut parser: crate::command::parser::Parser) -> Result<Self, Self::Error> {
                if parser.len() < #required_args_count {
                    return Err(crate::command::error::CommandError::InvalidArgumentNumber(
                        #command_name.to_string(),
                        #required_args_count,
                    ));
                }
                #parse_statement
                Ok(
                    #field_conversions
                )
            }
        }
    )
}

fn generate_fields_from_parser(
    fields: &Fields,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    match fields {
        Fields::Named(named_fields) => {
            let mut parse_statements = Vec::new();
            let mut field_assignments = Vec::new();

            for field in &named_fields.named {
                let field_name = field.ident.as_ref().unwrap();
                let field_type = &field.ty;

                let parse_statement = if is_option_type(field_type) {
                    quote! {
                        let #field_name: #field_type = Some(parser.next()?);
                    }
                } else {
                    quote! {
                        let #field_name: #field_type = parser.next()?;
                    }
                };

                parse_statements.push(parse_statement);
                field_assignments.push(quote! { #field_name: #field_name });
            }

            (
                quote! { #(#parse_statements)* },
                quote! { Self { #(#field_assignments),* } },
            )
        }
        Fields::Unnamed(unnamed_fields) => {
            let mut parse_statements = Vec::new();
            let mut field_assignments = Vec::new();

            for (index, field) in unnamed_fields.unnamed.iter().enumerate() {
                let field_index = syn::Index::from(index);
                let field_type = &field.ty;

                let parse_statement = if is_option_type(field_type) {
                    quote! {
                        let #field_index: #field_type = parser.next_optional()?;
                    }
                } else {
                    quote! {
                        let #field_index: #field_type = parser.next()?;
                    }
                };

                parse_statements.push(parse_statement);
                field_assignments.push(quote! { #field_index });
            }

            (
                quote! { #(#parse_statements)* },
                quote! { Self(#(#field_assignments),*) },
            )
        }
        Fields::Unit => (quote! {}, quote! { Self }),
    }
}

fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

fn calculate_required_args_count(fields: &Fields) -> usize {
    match fields {
        Fields::Named(named_fields) => named_fields.named.iter()
            .filter(|field| !is_option_type(&field.ty))
            .count(),
        Fields::Unnamed(unnamed_fields) => unnamed_fields.unnamed.iter()
            .filter(|field| !is_option_type(&field.ty))
            .count(),
        Fields::Unit => 0,
    }
}
