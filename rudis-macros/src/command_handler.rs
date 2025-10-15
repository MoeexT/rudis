use quote::quote;
use syn::punctuated::Punctuated;
use syn::{Field, Fields, Ident, Meta, Result};

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
) -> Result<proc_macro2::TokenStream> {
    let (parse_statement, field_conversions) = generate_fields_from_parser(fields)?;
    let required_args_count = calculate_required_args_count(fields);
    Ok(quote!(
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
    ))
}

fn generate_fields_from_parser(
    fields: &Fields,
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream)> {
    match fields {
        Fields::Named(named_fields) => {
            // classify fields: positional, pair(option), flag(bool)
            let mut positional_parsers = Vec::new(); // (ident, ty)
            let mut pair_fields = Vec::new(); // (ident, ty_inner, aliases)
            let mut flag_fields = Vec::new(); // (ident, flag_name)

            for field in &named_fields.named {
                let field_name = field.ident.as_ref().unwrap();
                let field_type = &field.ty;

                let (flag_opt, aliases) = parse_arg_attributes(&field)?;

                if let Some(flag_name) = flag_opt {
                    // flag field (must be bool)
                    flag_fields.push((field_name.clone(), flag_name));
                } else if is_option_type(field_type) {
                    // treat Option<T> as pair-style optional parameter
                    // extract inner type name for documentation purposes
                    pair_fields.push((field_name.clone(), quote! { #field_type }, aliases));
                } else {
                    // positional required argument
                    positional_parsers.push((field_name.clone(), quote! { #field_type }));
                }
            }

            // generate initialization for pair and flag fields
            let mut inits = Vec::new();
            for (ident, ty, _aliases) in &pair_fields {
                inits.push(quote! { let mut #ident: #ty = None; });
            }
            for (ident, _flag_name) in &flag_fields {
                inits.push(quote! { let mut #ident: bool = false; });
            }

            // generate positional parsing
            let mut positional_stmts = Vec::new();
            for (ident, ty) in &positional_parsers {
                positional_stmts.push(quote! {
                    let #ident: #ty = parser.next()?;
                });
            }

            // generate match arms for pair and flags
            let mut match_arms = Vec::new();

            // pair fields arms (one arm per alias)
            for (ident, _ty, aliases) in &pair_fields {
                if aliases.is_empty() {
                    // if no aliases provided, skip generating arms (user must provide aliases)
                    continue;
                }
                for alias in aliases {
                    let lit =
                        syn::LitStr::new(&alias.to_uppercase(), proc_macro2::Span::call_site());
                    // each alias consumes a value and fills the corresponding Option field
                    match_arms.push(quote! {
                        #lit => {
                            // read value as string (value conversion delegated to TryFrom<(String,String)> for inner type)
                            let v: String = parser.next()?;
                            let pair = (k.clone(), v);
                            #ident = Some(pair.try_into()?);
                        }
                    });
                }
            }

            // flag fields arms
            for (ident, flag_name) in &flag_fields {
                let lit =
                    syn::LitStr::new(&flag_name.to_uppercase(), proc_macro2::Span::call_site());
                match_arms.push(quote! {
                    #lit => {
                        #ident = true;
                    }
                });
            }

            // default unknown option arm
            match_arms.push(quote! {
                other => {
                    return Err(crate::command::error::CommandError::InvalidCommandFormat(
                        format!("Unknown option {}", other)
                    ))
                }
            });

            // build while-loop match
            let loop_block = if match_arms.len() > 0 {
                quote! {
                    while parser.has_next() {
                        let k: String = parser.next()?;
                        match k.to_uppercase().as_str() {
                            #(#match_arms),*
                        }
                    }
                }
            } else {
                quote! {}
            };

            // build final assignments in field order
            let mut field_assignments = Vec::new();
            for field in &named_fields.named {
                let name = field.ident.as_ref().unwrap();
                field_assignments.push(quote! { #name: #name });
            }

            let parse_statement = quote! {
                #(#inits)*
                #(#positional_stmts)*
                #loop_block
            };

            Ok((parse_statement, quote! { Self { #(#field_assignments),* } }))
        }
        Fields::Unnamed(unnamed_fields) => {
            // keep existing behavior for tuple structs
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

            Ok((
                quote! { #(#parse_statements)* },
                quote! { Self(#(#field_assignments),*) },
            ))
        }
        Fields::Unit => Ok((quote! {}, quote! { Self })),
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

fn parse_arg_attributes(
    field: &Field,
) -> Result<(Option<String> /*flag*/, Vec<String> /*aliases*/)> {
    let mut alias: Option<String> = None;
    let mut aliases: Vec<String> = Vec::new();

    for attr in &field.attrs {
        if attr.path().is_ident("arg") {
            if let Ok(meta_list) =
                attr.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)
            {
                for meta in meta_list {
                    match meta {
                        syn::Meta::NameValue(name_value) if name_value.path.is_ident("alias") => {
                            if let syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(lit_str),
                                ..
                            }) = &name_value.value
                            {
                                alias = Some(lit_str.value());
                            }
                        }
                        syn::Meta::NameValue(name_value) if name_value.path.is_ident("aliases") => {
                            if let syn::Expr::Array(syn::ExprArray { elems, .. }) =
                                &name_value.value
                            {
                                for elem in elems {
                                    if let syn::Expr::Lit(syn::ExprLit {
                                        lit: syn::Lit::Str(alias_str),
                                        ..
                                    }) = elem
                                    {
                                        aliases.push(alias_str.value());
                                    }
                                }
                            }
                        }
                        _ => {
                            return Err(syn::Error::new_spanned(
                                meta,
                                "Unsupported attribute in 'arg', only 'alias' and 'aliases' are supported",
                            ));
                        }
                    }
                }
            } else {
                alias = Some(field.ident.as_ref().unwrap().to_string());
            }
        }
    }

    Ok((alias, aliases))
}

fn calculate_required_args_count(fields: &Fields) -> usize {
    match fields {
        Fields::Named(named_fields) => named_fields
            .named
            .iter()
            .filter(|field| !is_option_type(&field.ty))
            .count(),
        Fields::Unnamed(unnamed_fields) => unnamed_fields
            .unnamed
            .iter()
            .filter(|field| !is_option_type(&field.ty))
            .count(),
        Fields::Unit => 0,
    }
}
