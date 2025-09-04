extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemStruct, parse_macro_input};

use crate::command_handler::{CommandAttr, generate_command_handler, generate_try_from_impl};

mod command_handler;

/// CommandHandler convert a RESP array to a struct and register the command handler to the command
/// registry automatically.
///
/// # Example
/// ```rust
/// use rudis_macros::register;
///
/// #[register("GET")]
/// struct GetCommand {
///    key: String,
/// }
/// ```
///
/// This will generate the following code:
///
/// ```rust,ignore
/// #[derive(Debug, PartialEq, Eq)]
/// struct GetCommand {
///     key: String,
/// }
/// impl TryFrom<Vec<crate::resp::RespValue>> for GetCommand {
///     type Error = crate::command::error::CommandError;
///     fn try_from(values: Vec<crate::resp::RespValue>) -> Result<Self, Self::Error> {
///         if values.len() != 1usize {
///             return Err(crate::command::error::CommandError::InvalidArgumentNumber(
///                 "GET".to_string(),
///                 1usize,
///             ));
///         }
///         let arr: [crate::resp::RespValue; 1usize] = values
///             .try_into()
///             .unwrap_or_else(|_| core::panicking::panic("internal error: entered unreachable code"));
///         let [key] = arr;
///         Ok(Self {
///             key: crate::resp::FromRespValue::from_resp_value(key, "GET")?,
///         })
///     }
/// }
/// #[allow(unused)]
/// fn __register_get_command() {
///     use ::std::boxed::Box;
///     use ::std::future::Future;
///     use ::std::pin::Pin;
///     use ::std::sync::Arc;
///     let handler = move |ctx: Arc<crate::context::Context>, args: Vec<crate::resp::RespValue>|
///           -> Pin<Box<dyn Future<Output = crate::command::registry::CommandResult> + Send>,
///     > {
///         let args_clone = args.clone();
///         Box::pin(async move {
///             let cmd: GetCommand = args_clone.try_into()?;
///             cmd.execute(ctx).await
///         })
///     };
///     crate::command::registry::en_register_queue("GET", handler);
/// }
/// ```
///
/// # Attributes
/// - `command`: The command name, usually be uppercase.
///
/// # Errors
/// - If the struct is not a struct, it will return a compile error.
/// - If the struct does not have the `command` attribute, it will return a compile error.
/// - If the number of fields in the struct does not match the number of arguments in the RESP array,
///  it will return a runtime error.
/// - If the conversion from `RespValue` to the field type fails, it will return a runtime error.
/// - The field types must implement the `FromRespValue` trait, otherwise it will return a compile error.
///
/// # Notes
/// - This macro requires the `ctor` crate to be included in the dependencies.
/// - This macro assumes that the command executor will implement the `CommandExecutor` trait with an
/// `execute` method that takes `self` and a `Context` and returns a `CommandResult`.
/// - This macro is designed to work with the existing command registration system in `rudis`.
#[proc_macro_attribute]
pub fn register(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as CommandAttr);
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = &input.ident;
    let command_name = &args.command_name;

    // generate TryFrom<Vec<RespValue>> impl
    let try_from_impl = generate_try_from_impl(struct_name, &input.fields, command_name);

    // generate command handler and register it
    let handler_impl = generate_command_handler(struct_name, &command_name);
    let sct = quote! {
        #[derive(Debug)]
        #input
    };

    TokenStream::from(quote! {
        #sct
        #try_from_impl
        #handler_impl
    })
}
