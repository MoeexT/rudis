extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr};

/// Register redis command with command name automatically.
/// Example:
/// 
/// ```
/// #[redis_command("GET")]
/// pub async fn get_command(ctx: Arc<Context>, args: Vec<RespValue>) -> CommandResult {
///     let cmd: GetCommand = args.try_into()?;
///     cmd.execute(ctx).await
/// }
/// ```
#[proc_macro_attribute]
pub fn redis_command(attr: TokenStream, item: TokenStream) -> TokenStream {
    if attr.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "Command name must be specified. Usage: #[redis_command(\"GET\")]",
        )
        .to_compile_error()
        .into();
    }

    let command_name = match syn::parse::<LitStr>(attr.clone()) {
        Ok(lit) => lit,
        Err(e) => return e.to_compile_error().into(),
    };

    let input_fn = match syn::parse::<ItemFn>(item.clone()) {
        Ok(func) => func,
        Err(e) => return e.to_compile_error().into(),
    };

    let cmd_name_str = command_name.value();
    let fn_name = &input_fn.sig.ident;

    let register_fn_name = syn::Ident::new(
        &format!("__register_command_{}", cmd_name_str.to_lowercase()),
        command_name.span(),
    );

    let expanded = quote! {
        #input_fn

        #[::ctor::ctor]
        fn #register_fn_name() {
            use ::std::sync::Arc;
            use ::std::future::Future;
            use ::std::pin::Pin;
            use ::std::boxed::Box;

            let handler = |ctx: Arc<crate::context::Context>,
                           args: Vec<crate::resp::RespValue>|
                -> Pin<Box<dyn Future<Output = Result<crate::resp::RespValue, crate::command::error::CommandError>> + Send>>
            {
                Box::pin(#fn_name(ctx, args))
            };

            crate::command::registry::en_register_queue(#cmd_name_str, handler);
        }
    };

    TokenStream::from(expanded)
}
