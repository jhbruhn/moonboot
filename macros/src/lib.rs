use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

mod attributes;

#[proc_macro_attribute]
#[proc_macro_error]
pub fn pre_jump_handler(args: TokenStream, item: TokenStream) -> TokenStream {
    attributes::pre_jump_handler::expand(args, item)
}

