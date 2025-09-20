extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemStruct, parse_macro_input};

use crate::command_handler::{CommandAttr, generate_command_handler, generate_from_parse_impl};

mod command_handler;

/// CommandHandler convert a Frame to a struct and register the command handler to the command
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
/// impl TryFrom<crate::command::parser::Parser> for GetCommand {
///     type Error = crate::command::error::CommandError;
///     fn try_from(mut parse: crate::command::parser::Parser) -> Result<Self, Self::Error> {
///         let key = parse.next::<String>()?;
///         Ok(Self { key })
///     }
/// }
///
/// #[allow(unused)]
/// fn __register_get_command() {
///     {
///         use ::std::boxed::Box;
///         use ::std::future::Future;
///         use ::std::pin::Pin;
///         use ::std::sync::Arc;
///         const COMMAND_NAME: &str = "GET";
///         fn handler_func(
///             ctx: ::std::sync::Arc<crate::context::Context>,
///             args: crate::command::parser::Parser,
///         ) -> crate::command::registry::CommandFuture {
///             Box::pin(async move {
///                 let cmd: GetCommand = args.try_into()?;
///                 cmd.execute(ctx).await
///             })
///         }
///         crate::command::registry::en_register_queue(COMMAND_NAME, handler_func);
///     }
/// }
/// ```
///
/// # Attributes
/// - `command`: The command name, usually be uppercase.
///
/// # Errors
/// - If the struct is not a struct, it will return a compile error.
/// - If the struct does not have the `command` attribute, it will return a compile error.
/// - If the number of fields in the struct does not match the number of arguments in the Frame,
///  it will return a runtime error.
/// - If the conversion from `Frame` to the field type fails, it will return a runtime error.
/// - The field types must implement the `TryFrom<Frame>` trait, otherwise it will return a compile error.
///
/// # Notes
/// - This macro requires the `ctor` crate to be included in the dependencies.
/// - This macro assumes that the command executor will implement the `CommandExecutor` trait with an
/// `execute` method that takes `self` and a `Context` and returns a `CommandResult`.
/// - This macro is designed to work with the existing command registration system in `rudis`.
#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as CommandAttr);
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = &input.ident;
    let command_name = if &args.command_name == "" {
        &struct_name.to_string().to_uppercase()
    } else {
        &args.command_name
    };

    // generate TryFrom<Parser> impl
    let try_from_impl = generate_from_parse_impl(struct_name, &input.fields, &command_name);

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
