use proc_macro::TokenStream;

#[proc_macro]
pub fn mapack(code: TokenStream) -> TokenStream {
    println!("code: {code:?}");

    TokenStream::new()
}
