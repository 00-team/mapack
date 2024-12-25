use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::format_ident;
use quote_into::quote_into;

#[derive(Debug)]
struct Layer {
    ident: syn::Ident,
    name: syn::Ident,
    fields: Vec<(syn::Ident, syn::Path, String)>,
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

            let key = ident.to_string();
            layer.fields.push((ident, ty, key));

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
            for layer in tile.layers.iter() {
                let Layer { ident, fields, name } = layer;
                let name_str = name.to_string();
                let keys_len = fields.len();
                quote_into! {s +=
                    pub struct #ident {
                       pub coordinate: #ci::Coordinate,
                       #{for (ident, ty, _) in fields.iter() {
                            quote_into!(s += pub #ident: #ty,);
                       }}
                    }

                    impl #ident {
                        pub const NAME: &str = #name_str;
                        pub const KEYS: [&str; #keys_len] = [
                            #{for (_, _, key) in fields {quote_into!(s += #key,)}}
                        ];

                        pub fn new(coordinate: #ci::Coordinate) -> Self {
                            Self {
                                coordinate,
                                #{for (ident, ty, _) in fields.iter() {
                                    quote_into!(s += #ident: #ty::default(),);
                                }}
                            }
                        }

                        #[allow(dead_code)]
                        pub fn decode_point(
                            zom: u8, tx: u32, ty: u32,
                            feature: &#ci::vector_tile::tile::Feature,
                            values: &[#ci::vector_tile::tile::Value],
                        ) -> Result<Self, &'static str> {
                            #{point_decode(s, layer)}
                        }

                        pub fn decode_layer(
                            zom: u8, tx: u32, ty: u32,
                            layer: &#ci::vector_tile::tile::Layer
                        ) -> #ci::protobuf::Result<Vec<Self>> {
                            let mut points = Vec::<Self>::with_capacity(layer.features.len());

                            for feature in layer.features.iter() {
                                match Self::decode_point(zom, tx, ty, feature, &layer.values) {
                                    Ok(v) => points.push(v),
                                    Err(e) => {
                                        println!("found an invalid marker: {e:?}")
                                    }
                                }
                            }

                            Ok(points)
                        }
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

        impl Tile {
            pub fn new() -> Self {
                Self {#{
                    for Layer { name, .. } in tile.layers.iter() {
                        quote_into!(s += #name: Vec::new(), );
                    }
                }}
            }

            pub fn decode(zom: u8, tx: u32, ty: u32, pbf: Vec<u8>) -> #ci::protobuf::Result<Self> {
                let mut tile = Self::new();
                let vec_tile = <#ci::vector_tile::Tile as #ci::protobuf::Message>::parse_from_bytes(&pbf)?;
                if vec_tile.layers.is_empty() { return Ok(tile); }

                for layer in vec_tile.layers.iter() {#{
                    tile_decode(s, &tile.layers)
                }}



                Ok(tile)
            }
        }
    }

    println!("{s}");
    s.into()
}

fn tile_decode(s: &mut TokenStream2, layers: &[Layer]) {
    quote_into! {s +=
        if layer.version() != 2 { continue }

        match layer.name() {
            #{for Layer { name, ident, .. } in layers {
                let name_str = name.to_string();
                quote_into! {s += #name_str => {
                    tile.#name = #ident::decode_layer(zom, tx, ty, layer)?;
                }}
            }}
            _ => {}
        }

        continue;
    }
}

fn point_decode(s: &mut TokenStream2, Layer { fields, .. }: &Layer) {
    let ci = crate_ident();

    quote_into! {s +=
        if feature.geometry.len() != 3 {
            return Err("bad geometry");
        }

        let tags = &feature.tags;
        // if tags.is_empty() {
        //     return Err("no tags");
        // }
        if tags.len() % 2 != 0 {
            return Err("bad tags length");
        }

        let geometry: [u32; 3] = feature.geometry.clone().try_into().unwrap();
        let mut point = Self::new(#ci::Coordinate::from_geometry(zom, tx, ty, geometry));

        let mut tags_iter = tags.iter();
        loop {
            let Some(k) = tags_iter.next() else { break };
            let Some(v) = tags_iter.next() else { break };
            let k = *k as usize;
            let v = *v as usize;
            if k >= Self::KEYS.len() || v >= values.len() {
                return Err("invalid tags");
            }
            let v = &values[v];

            match Self::KEYS[k] {
                #{for (ident, _, key) in fields {
                    let pfv = format_ident!("decode_{key}");
                    quote_into! {s += #key => {
                        if let Some(value) = Self::#pfv(v) {
                            point.#ident = value;
                        } else {
                            return Err(concat!("could not decode ", #key, "s value"));
                        }
                    }}
                }}
                _ => unreachable!()
            }
        }

        Ok(point)
    }
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
