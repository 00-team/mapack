use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, ToTokens};
use quote_into::quote_into;

#[derive(Debug, Clone)]
struct Field {
    ident: syn::Ident,
    ty: syn::Path,
    key: String,
    auto_encode: bool,
    auto_decode: bool,
}

impl syn::parse::Parse for Field {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut auto_encode = true;
        let mut auto_decode = true;

        let attrs = input.call(syn::Attribute::parse_outer)?;
        for attr in attrs {
            if let syn::Meta::Path(mp) = attr.meta {
                match mp.to_token_stream().to_string().as_str() {
                    "no_decode" => auto_decode = false,
                    "no_encode" => auto_encode = false,
                    _ => {}
                }
            }
        }

        let ident: syn::Ident = input.parse()?;
        input.parse::<syn::Token![:]>()?;
        let ty: syn::Path = input.parse()?;

        let key = ident.to_string();
        Ok(Self { ident, ty, key, auto_decode, auto_encode })
    }
}

#[derive(Debug)]
struct Layer {
    ident: syn::Ident,
    name: syn::Ident,
    fields: Vec<Field>,
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
        let fields = content.parse_terminated(Field::parse, syn::Token![,])?;
        layer.fields = fields.iter().cloned().collect();

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
                       #{for Field { ident, ty, .. } in fields.iter() {
                            quote_into!(s += pub #ident: #ty,);
                       }}
                    }

                    impl #ident {
                        pub const NAME: &str = #name_str;
                        pub const KEYS: [&str; #keys_len] = [
                            #{for Field { key, .. } in fields {
                                quote_into!(s += #key,)}
                            }
                        ];

                        pub fn new(coordinate: #ci::Coordinate) -> Self {
                            Self {
                                coordinate,
                                #{for Field { ident, ty, .. } in fields.iter() {
                                    quote_into!(s += #ident: #ty::default(),);
                                }}
                            }
                        }

                        #[allow(dead_code)]
                        pub fn decode_point(
                            zom: u8, tx: u32, ty: u32,
                            feature: &#ci::Feature, values: &[#ci::Value],
                        ) -> Result<Self, &'static str> {
                            #{point_decode(s, layer)}
                        }

                        pub fn decode_layer(
                            zom: u8, tx: u32, ty: u32,
                            layer: &#ci::Layer
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

                        #{for field in fields {
                            let Field {ty, key, ..} = field;
                            if field.auto_decode {
                                let ident = format_ident!("decode_{key}");
                                quote_into! {s +=
                                    fn #ident(v: &#ci::Value) -> Option<#ty> {
                                        #{point_auto_decode(s, field);}
                                    }
                                }
                            }
                            if field.auto_encode {
                                let ident = format_ident!("encode_{}", field.key);
                                quote_into! {s +=
                                    fn #ident(&self) -> #ci::Value {
                                        #{point_auto_encode(s, field);}
                                    }
                                }
                            }
                        }}
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

            #[allow(dead_code)]
            pub fn decode(zom: u8, tx: u32, ty: u32, pbf: Vec<u8>) -> #ci::protobuf::Result<Self> {
                let mut tile = Self::new();
                let vec_tile = <#ci::Tile as #ci::protobuf::Message>::parse_from_bytes(&pbf)?;
                if vec_tile.layers.is_empty() { return Ok(tile); }

                for layer in vec_tile.layers.iter() {#{
                    tile_decode(s, &tile.layers)
                }}

                Ok(tile)
            }

            #[allow(dead_code)]
            pub fn encode(&self) -> #ci::protobuf::Result<Vec<u8>> {
                let mut vec_tile = #ci::Tile::default();

                #{for layer in tile.layers.iter() {
                    quote_into!(s += 'a: {#{tile_encode(s, layer)}});
                }}

                #ci::protobuf::Message::write_to_bytes(&vec_tile)
            }
        }
    }

    s.into()
}

fn tile_encode(s: &mut TokenStream2, Layer { ident, name, fields }: &Layer) {
    let keys_len = fields.len();
    let ci = crate_ident();
    let name_str = name.to_string();

    quote_into! {s +=
        let mut values = Vec::<#ci::Value>::with_capacity(self.#name.len() * #keys_len);
        let mut features = Vec::<#ci::Feature>::with_capacity(self.#name.len());

        for point in self.#name.iter() {
            #{for Field { key, .. } in fields.iter() {
                let ptv = format_ident!("encode_{key}");
                let val = format_ident!("{key}_value");
                quote_into! {s +=
                    let #val = values.len() as u32;
                    values.push(point.#ptv());
                }
            }}

            features.push(#ci::Feature {
                tags: vec![#{for (idx, Field { key, .. }) in fields.iter().enumerate() {
                    let idx = idx as u32;
                    let val = format_ident!("{key}_value");
                    quote_into!(s += #idx, #val,);
                }}],
                geometry: point.coordinate.to_geometry().to_vec(),
                type_: Some(#ci::protobuf::EnumOrUnknown::new(#ci::GeomType::POINT)),
                ..Default::default()
            });
        }

        vec_tile.layers.push(#ci::Layer {
            name: Some(String::from(#name_str)),
            extent: Some(4096),
            version: Some(2),
            features,
            keys: #ident::KEYS.map(|k| k.to_string()).to_vec(),
            values,
            ..Default::default()
        });
    }
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

fn point_auto_encode(s: &mut TokenStream2, field: &Field) {
    let Field { ident, ty, .. } = field;
    let ci = crate_ident();
    let ty_str = ty.to_token_stream().to_string();
    match ty_str.as_str() {
        "bool" => quote_into!(s += #ci::Value::from_bool(self.#ident)),
        "u8" | "u16" | "u32" | "u64" => {
            quote_into!(s += #ci::Value::from_uint(self.#ident as u64))
        }
        "String" => {
            quote_into!(s += #ci::Value::from_string(self.#ident.clone()))
        }
        _ => quote_into! {s +=
            compile_error!(concat!("bad prop type for auto encoding: ", #ty_str));
        },
    }
}
fn point_auto_decode(s: &mut TokenStream2, field: &Field) {
    let ty = &field.ty;
    let ty_str = ty.to_token_stream().to_string();
    match ty_str.as_str() {
        "bool" => quote_into!(s += Some(v.bool_value())),
        "String" => quote_into! {s += Some(v.string_value().to_string())},
        "u8" | "u16" | "u32" | "u64" => {
            quote_into!(s += Some(v.uint_value() as #ty))
        }
        _ => quote_into! {s +=
            compile_error!(concat!("bad prop type for auto decoding: ", #ty_str));
        },
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
                #{for Field { ident, key, .. } in fields {
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
