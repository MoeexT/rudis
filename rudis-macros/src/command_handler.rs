use quote::quote;
use syn::{DeriveInput, Ident, Fields};
use syn::parse::{Parse, ParseStream};

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

/// parse the #[command("...")] attribute
pub(crate) fn parse_command_attr(input: &DeriveInput) -> Option<CommandAttr> {
    for attr in &input.attrs {
        if attr.path().is_ident("command") {
            return match attr.parse_args::<CommandAttr>() {
                Ok(attr) => Some(attr),
                Err(_) => None,
            };
        }
    }
    None
}

/// generate field destructuring and conversion code
pub(crate) fn generate_field_code(fields: &Fields, command_name: &str) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    match fields {
        Fields::Named(named_fields) => {
            let field_idents: Vec<&Ident> = named_fields.named.iter()
                .filter_map(|f| f.ident.as_ref())
                .collect();
            
            let destructure_pattern = quote! { [#(#field_idents),*] };
            
            let field_conversions = field_idents.iter().map(|field_ident| {
                quote! {
                    #field_ident: crate::resp::FromRespValue::from_resp_value(#field_ident, #command_name)?,
                }
            });
            
            (destructure_pattern, quote! { #(#field_conversions)* })
        }
        Fields::Unnamed(unnamed_fields) => {
            let field_count = unnamed_fields.unnamed.len();
            let field_indices: Vec<usize> = (0..field_count).collect();
            let field_idents: Vec<Ident> = field_indices.iter()
                .map(|i| syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site()))
                .collect();
            
            let destructure_pattern = quote! { [#(#field_idents),*] };
            
            let field_conversions = field_idents.iter().map(|field_ident| {
                quote! {
                    crate::resp::FromRespValue::from_resp_value(#field_ident, #command_name)?,
                }
            });
            
            (destructure_pattern, quote! { #(#field_conversions)* })
        }
        Fields::Unit => {
            (quote! { [] }, quote! {})
        }
    }
}

/// generate command handler registration code
pub(crate) fn generate_command_handler(struct_name: &Ident, command_name: &str) -> proc_macro2::TokenStream {
    let register_fn_name = syn::Ident::new(
        &format!("__register_{}_command", command_name.to_lowercase()),
        proc_macro2::Span::call_site()
    );
    
    quote! {
        #[::ctor::ctor]
        fn #register_fn_name() {
            use ::std::sync::Arc;
            use ::std::future::Future;
            use ::std::pin::Pin;
            use ::std::boxed::Box;
            
            let handler = move |ctx: Arc<crate::context::Context>, args: Vec<crate::resp::RespValue>|
                -> Pin<Box<dyn Future<Output = crate::command::registry::CommandResult> + Send>>
            {
                let args_clone = args.clone();
                Box::pin(async move {
                    let cmd: #struct_name = args_clone.try_into()?;
                    cmd.execute(ctx).await
                })
            };
            
            crate::command::registry::en_register_queue(#command_name, handler);
        }
    }
}
