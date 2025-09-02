extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

use crate::command_handler::{generate_command_handler, generate_field_code, parse_command_attr};

mod command_handler;

/// CommandHandler convert a RESP array to a struct and register the command handler to the command
/// registry automatically.
///
/// # Example
/// ```rust
/// use rudis_macros::CommandHandler;
///
/// #[derive(CommandHandler)]
/// #[command("GET")]
/// struct GetCommand {
///    key: String,
/// }
/// ```
/// 
/// This will generate the following code:
/// 
/// ```rust,ignore
/// impl TryFrom<Vec<RespValue>> for GetCommand {
///     type Error = CommandError;
///     fn try_from(values: Vec<RespValue>) -> Result<Self, CommandError> {
///         if values.len() != 1usize {
///             return Err(CommandError::InvalidArgumentNumber(
///                 "GET".to_string(),
///                 1usize,
///             ));
///         }
///         let [key] = values
///             .try_into()
///             .map_err(|_| CommandError::InvalidArgumentNumber("GET".to_string(), 1usize))?;
///         Ok(Self {
///             key: crate::resp::FromRespValue::from_resp_value(key, "GET")?,
///         })
///     }
/// }
/// #[::ctor::ctor]
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
#[proc_macro_derive(CommandHandler, attributes(command))]
pub fn derive_command_handler(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let command_attr = parse_command_attr(&input);
    let command_name = match command_attr {
        Some(attr) => attr.command_name,
        None => {
            return syn::Error::new(struct_name.span(), "lack of #[command(\"...\")] attribute")
                .to_compile_error()
                .into();
        }
    };

    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => {
            return syn::Error::new(struct_name.span(), "CommandHandler can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };
    let field_count = match fields {
        Fields::Named(fields) => fields.named.len(),
        Fields::Unnamed(fields) => fields.unnamed.len(),
        Fields::Unit => 0,
    };
    // generate TryFrom<Vec<RespValue>> impl
    let (destructure_pattern, field_conversions) = generate_field_code(fields, &command_name);
    let try_from_impl = quote! {
        impl TryFrom<Vec<RespValue>> for #struct_name {
            type Error = CommandError;

            fn try_from(values: Vec<RespValue>) -> Result<Self, CommandError> {
                // check argument number
                if values.len() != #field_count {
                    return Err(CommandError::InvalidArgumentNumber(
                        #command_name.to_string(), #field_count
                    ));
                }

                // destructure the values
                let #destructure_pattern = values
                    .try_into()
                    .map_err(|_| CommandError::InvalidArgumentNumber(#command_name.to_string(), #field_count))?;

                Ok(Self {
                    #field_conversions
                })
            }
        }
    };

    // generate command handler and register it
    let handler_impl = generate_command_handler(struct_name, &command_name);

    TokenStream::from(quote! {
        #try_from_impl
        #handler_impl
    })
}
