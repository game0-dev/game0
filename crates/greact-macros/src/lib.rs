mod parser;
mod codegen;

use proc_macro::TokenStream;

/// JSX-like view macro.
///
/// ```ignore
/// view! { cx,
///     <div style={Style::new().padding_all(10.0)}>
///         "Hello"
///         <button on_click={|| println!("clicked")}>
///             "OK"
///         </button>
///     </div>
/// }
/// ```
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    let parsed = syn::parse2::<parser::ViewMacroInput>(input).unwrap_or_else(|e| {
        panic!("view! parse error: {}", e)
    });
    let output = codegen::generate(&parsed);
    TokenStream::from(output)
}
