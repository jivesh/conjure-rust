// Copyright 2018 Palantir Technologies, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use proc_macro2::TokenStream;
use quote::quote;

use crate::context::Context;
use crate::types::AliasDefinition;

pub fn generate(ctx: &Context, def: &AliasDefinition) -> TokenStream {
    let name = ctx.type_name(def.type_name().name());
    let alias = ctx.rust_type(def.type_name(), def.alias());
    let result = ctx.result_ident(def.type_name());
    let docs = ctx.docs(def.docs());

    let mut type_attrs = vec![];
    let mut field_attrs = vec![];
    let mut derives = vec!["Debug", "Clone"];

    if ctx.is_copy(def.alias()) {
        derives.push("Copy");
    }

    if ctx.is_double(def.alias()) {
        derives.push("conjure_object::private::Educe");
        type_attrs.push(quote!(#[educe(PartialEq, Eq, PartialOrd, Ord, Hash)]));
        field_attrs.push(quote! {
            #[educe(
                PartialEq(trait = "conjure_object::private::DoubleOps"),
                PartialOrd(trait = "conjure_object::private::DoubleOps"),
                Ord(trait = "conjure_object::private::DoubleOps"),
                Hash(trait = "conjure_object::private::DoubleOps"),
            )]
        })
    } else {
        derives.push("PartialEq");
        derives.push("Eq");
        derives.push("PartialOrd");
        derives.push("Ord");
        derives.push("Hash");
    }

    if ctx.is_default(def.alias()) {
        derives.push("Default");
    }

    let derives = derives.iter().map(|s| s.parse::<TokenStream>().unwrap());
    // The derive attr has to be before the educe attr, so insert rather than push
    type_attrs.insert(0, quote!(#[derive(#(#derives),*)]));

    let display = if ctx.is_display(def.alias()) {
        quote! {
            impl std::fmt::Display for #name {
                fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    std::fmt::Display::fmt(&self.0, fmt)
                }
            }
        }
    } else {
        quote!()
    };

    let plain = if ctx.is_plain(def.alias()) {
        quote! {
            impl conjure_object::Plain for #name {
                fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    conjure_object::Plain::fmt(&self.0, fmt)
                }
            }

            impl conjure_object::FromPlain for #name {
                type Err = <#alias as conjure_object::FromPlain>::Err;

                #[inline]
                fn from_plain(s: &str) -> #result<#name, Self::Err> {
                    conjure_object::FromPlain::from_plain(s).map(#name)
                }
            }
        }
    } else {
        quote!()
    };

    quote! {
        use conjure_object::serde::{ser, de};

        #docs
        #(#type_attrs)*
        pub struct #name(#(#field_attrs)* pub #alias);

        #display

        #plain

        impl std::ops::Deref for #name {
            type Target = #alias;

            #[inline]
            fn deref(&self) -> &#alias {
                &self.0
            }
        }

        impl std::ops::DerefMut for #name {
            #[inline]
            fn deref_mut(&mut self) -> &mut #alias {
                &mut self.0
            }
        }

        impl ser::Serialize for #name {
            fn serialize<S>(&self, s: S) -> #result<S::Ok, S::Error>
            where
                S: ser::Serializer
            {
                self.0.serialize(s)
            }
        }

        impl<'de> de::Deserialize<'de> for #name {
            fn deserialize<D>(d: D) -> #result<#name, D::Error>
            where
                D: de::Deserializer<'de>
            {
                de::Deserialize::deserialize(d).map(#name)
            }
        }
    }
}
