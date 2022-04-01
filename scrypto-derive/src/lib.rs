mod ast;
mod auth;
mod blueprint;
mod import;
mod non_fungible_data;
mod utils;

use proc_macro::TokenStream;

/// Declares a blueprint.
///
/// The `blueprint!` macro is a convenient way to define a new blueprint. It takes
/// two arguments:
/// - A `struct` which defines the structure
/// - A `impl` which defines the implementation.
///
/// This macro will derive the dispatcher method responsible for handling invocation
/// according to Scrypto ABI.
///
/// # Example
/// ```ignore
/// use scrypto::prelude::*;
///
/// blueprint! {
///     struct Counter {
///         count: u32
///     }
///
///     impl Counter {
///         pub fn new() -> Component {
///             Self {
///                 count: 0
///             }.instantiate()
///         }
///
///         pub fn get_and_incr(&mut self) -> u32 {
///             let n = self.count;
///             self.count += 1;
///             n
///         }
///     }
/// }
/// ```
#[proc_macro]
pub fn blueprint(input: TokenStream) -> TokenStream {
    blueprint::handle_blueprint(proc_macro2::TokenStream::from(input))
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro]
pub fn auth(input: TokenStream) -> TokenStream {
    auth::handle_auth(proc_macro2::TokenStream::from(input))
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Imports a blueprint from its ABI.
///
/// This macro will generate stubs for accessing the blueprint according to
/// its ABI specification.
///
/// # Example
/// ```ignore
/// use scrypto::prelude::*;
///
/// import! {
/// r#"
/// {
///     "package_id": "01a405d3129b61e86c51c3168d553d2ffd7a3f0bd2f66b5a3e9876",
///     "blueprint_name": "GumballMachine",
///     "functions": [
///         {
///             "name": "new",
///             "inputs": [],
///             "output": {
///                 "type": "Custom",
///                 "name": "ComponentId"
///             }
///         }
///     ],
///     "methods": [
///         {
///             "name": "get_gumball",
///             "mutability": "Mutable",
///             "inputs": [
///                 {
///                     "type": "Custom",
///                     "name": "Bucket"
///                 }
///             ],
///             "output": {
///                 "type": "Custom",
///                 "name": "Bucket"
///             }
///         }
///     ]
/// }
/// "#
/// }
/// ```
#[proc_macro]
pub fn import(input: TokenStream) -> TokenStream {
    import::handle_import(proc_macro2::TokenStream::from(input))
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive code that describe a non-fungible data structure.
///
/// # Example
///
/// ```ignore
/// use scrypto::prelude::*;
///
/// #[derive(NonFungibleData)]
/// pub struct AwesomeNonFungible {
///     pub field_1: u32,
///     #[scrypto(mutable)]
///     pub field_2: String,
/// }
/// ```
#[proc_macro_derive(NonFungibleData, attributes(scrypto))]
pub fn non_fungible_data(input: TokenStream) -> TokenStream {
    non_fungible_data::handle_non_fungible_data(proc_macro2::TokenStream::from(input))
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
