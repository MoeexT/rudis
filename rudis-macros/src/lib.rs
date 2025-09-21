extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, LitStr, parse_macro_input};

use crate::command_handler::{generate_command_handler, generate_from_parse_impl};

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
#[proc_macro_derive(Command, attributes(command, arg))]
pub fn command(input: TokenStream) -> TokenStream {
    let input_ast = parse_macro_input!(input as DeriveInput);
    let struct_name = &input_ast.ident;

    let mut command_name: Option<String> = None;
    for attr in &input_ast.attrs {
        if attr.path().is_ident("command") {
            if let Ok(lit) = attr.parse_args::<LitStr>() {
                command_name = Some(lit.value());
            }
        }
    }
    let command_name = command_name.unwrap_or_else(|| struct_name.to_string().to_uppercase());

    let fields = match &input_ast.data {
        Data::Struct(ds) => &ds.fields,
        _ => {
            return syn::Error::new_spanned(
                input_ast,
                "Command derive can only be applied to structs",
            )
            .to_compile_error()
            .into();
        }
    };

    let try_from_impl = generate_from_parse_impl(struct_name, fields, &command_name);
    let handler_impl = generate_command_handler(struct_name, &command_name);

    let expanded = quote! {
        #try_from_impl
        #handler_impl
    };

    TokenStream::from(expanded)
}
