use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::format_ident;
use quote_into::quote_into;

#[derive(Debug)]
struct Layer {
    ident: syn::Ident,
    name: syn::Ident,
    fields: Vec<(syn::Ident, syn::Path)>,
}

impl syn::parse::Parse for Layer {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse::<syn::Ident>()?;

        let mut layer = Self {
            ident: format_ident!("Point{}", to_camelcase(&name.to_string())),
            name,
            fields: Vec::new(),
        };

        input.parse::<syn::Token![:]>()?;

        let content;
        syn::braced!(content in input);

        loop {
            if content.is_empty() {
                break;
            }

            let ident: syn::Ident = content.parse()?;
            content.parse::<syn::Token![:]>()?;
            let ty: syn::Path = content.parse()?;

            layer.fields.push((ident, ty));

            if content.is_empty() {
                break;
            }
            content.parse::<syn::Token![,]>()?;
        }

        Ok(layer)
    }
}

#[derive(Debug)]
struct Tile {
    layers: Vec<Layer>,
}

impl syn::parse::Parse for Tile {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut layers = Vec::<Layer>::new();
        loop {
            if input.is_empty() {
                break;
            }

            let layer: Layer = input.parse()?;
            layers.push(layer);

            if input.is_empty() {
                break;
            }
            input.parse::<syn::Token![,]>()?;
        }

        Ok(Self { layers })
    }
}

#[proc_macro]
pub fn mapack(code: TokenStream) -> TokenStream {
    let tile = syn::parse_macro_input!(code as Tile);
    let mut s = TokenStream2::new();
    let ci = crate_ident();

    quote_into! {s +=
        #{
            for Layer { ident, fields, name } in tile.layers.iter() {
                let name_str = name.to_string();
                quote_into! {s +=
                    pub struct #ident {
                       pub coordinate: #ci::Coordinate,
                       #{
                            for (ident, ty) in fields.iter() {
                                quote_into!(s += pub #ident: #ty,);
                            }
                        }
                    }

                    impl #ident {
                        pub const NAME: &str = #name_str;
                    }
                }
            }
        }

        pub struct Tile {
            #{
                for Layer { ident, name, .. } in tile.layers.iter() {
                    quote_into!(s += pub #name: Vec<#ident>, );
                }
            }
        }
    }

    s.into()
}

fn to_camelcase(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for word in input.split('_') {
        let (h, r) = word.split_at(1);
        out.push_str(&h.to_uppercase());
        out.push_str(&r.to_lowercase());
    }

    out
}

fn crate_ident() -> syn::Ident {
    // let found_crate = crate_name("shah").unwrap();
    // let name = match &found_crate {
    //     FoundCrate::Itself => "shah",
    //     FoundCrate::Name(name) => name,
    // };

    syn::Ident::new("mapack", proc_macro2::Span::call_site())
}
