mod loam_syntax;

#[macro_use]
extern crate quote;

extern crate proc_macro;
use loam_syntax::{compile_new_ascent_to_ascent, LoamProgram};

/// Wrapper around the `ascent!` macro.
///
/// TODO: add more documentation
#[proc_macro]
pub fn loam(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let new_prog: LoamProgram = syn::parse_macro_input!(input as LoamProgram);
    match compile_new_ascent_to_ascent(new_prog) {
        Ok(token_stream) => token_stream.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
