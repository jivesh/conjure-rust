use proc_macro2::TokenStream;

// Copyright 2021 Palantir Technologies, Inc.
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
use crate::context::{Context, SetterBounds};
use crate::objects;
use crate::types::ObjectDefinition;
use quote::quote;

pub fn generate(ctx: &Context, def: &ObjectDefinition) -> TokenStream {
    let docs = ctx.docs(def.docs());
    let name = ctx.type_name(def.type_name().name());
    let default = ctx.default_ident(def.type_name());

    let mut type_attrs = vec![];
    let mut derives = vec!["Debug", "Clone"];

    if def.fields().iter().any(|v| ctx.has_double(v.type_())) {
        derives.push("conjure_object::private::Educe");
        type_attrs.push(quote!(#[educe(PartialEq, Eq, PartialOrd, Ord, Hash)]));
    } else {
        derives.push("PartialEq");
        derives.push("Eq");
        derives.push("PartialOrd");
        derives.push("Ord");
        derives.push("Hash");
    }

    if def.fields().iter().all(|v| ctx.is_copy(v.type_())) {
        derives.push("Copy");
    }

    let derives = derives.iter().map(|s| s.parse::<TokenStream>().unwrap());
    // The derive attr has to be before the educe attr, so insert rather than push
    type_attrs.insert(0, quote!(#[derive(#(#derives),*)]));

    let field_attrs = def.fields().iter().map(|s| {
        if ctx.is_double(s.type_()) {
            quote! {
                #[educe(
                    PartialEq(trait = "conjure_object::private::DoubleOps"),
                    PartialOrd(trait = "conjure_object::private::DoubleOps"),
                    Ord(trait = "conjure_object::private::DoubleOps"),
                    Hash(trait = "conjure_object::private::DoubleOps"),
                )]
            }
        } else {
            quote!()
        }
    });
    let fields = &objects::fields(ctx, def);
    let boxed_types = &def
        .fields()
        .iter()
        .map(|s| ctx.boxed_rust_type(def.type_name(), s.type_()))
        .collect::<Vec<_>>();

    let constructor = if fields.len() < 4 {
        generate_constructor(ctx, def)
    } else {
        quote!()
    };

    let accessors = def.fields().iter().map(|s| {
        let docs = ctx.docs(s.docs());
        let deprecated = ctx.deprecated(s.deprecated());
        let name = ctx.field_name(s.field_name());
        let ret_type = ctx.borrowed_rust_type(def.type_name(), s.type_());
        let borrow = ctx.borrow_rust_type(quote!(self.#name), s.type_());

        quote!(
            #docs
            #deprecated
            #[inline]
            pub fn #name(&self) -> #ret_type {
                #borrow
            }
        )
    });

    let builder_method = if fields.iter().any(|f| f == "builder") {
        quote!(builder_)
    } else {
        quote!(builder)
    };

    let builder_type = if ctx.staged_builders() {
        let stage = objects::stage_name(ctx, def, 0);
        quote!(#stage)
    } else {
        objects::builder_type(ctx, def)
    };

    quote! {
        #docs
        #(#type_attrs)*
        pub struct #name {
            #(
                #field_attrs
                #fields: #boxed_types,
            )*
        }

        impl #name {
            #constructor

            /// Returns a new builder.
            #[inline]
            pub fn #builder_method() -> #builder_type {
                #default::default()
            }

            #(#accessors)*
        }
    }
}

fn generate_constructor(ctx: &Context, def: &ObjectDefinition) -> TokenStream {
    let some = ctx.some_ident(def.type_name());
    let name = ctx.type_name(def.type_name().name());
    let mut param_it = vec![quote!(T), quote!(U), quote!(V)].into_iter();

    let mut parameters = vec![];
    let mut arguments = vec![];
    let mut where_clauses = vec![];
    let mut assignments = vec![];

    for field in def.fields() {
        let (field_type, optional) = match ctx.option_inner_type(field.type_()) {
            Some(field_type) => (field_type, true),
            None => (field.type_(), false),
        };
        let arg_name = ctx.field_name(field.field_name());
        match ctx.setter_bounds(def.type_name(), field_type, quote!(#arg_name)) {
            SetterBounds::Simple {
                argument_type,
                mut assign_rhs,
            } => {
                arguments.push(quote!(#arg_name: #argument_type));
                if optional {
                    assign_rhs = quote!(#some(#assign_rhs));
                }
                assignments.push(quote!(#arg_name: #assign_rhs));
            }
            SetterBounds::Generic {
                argument_bound,
                mut assign_rhs,
            } => {
                let param = param_it.next().unwrap();
                parameters.push(param.clone());
                arguments.push(quote!(#arg_name: #param));
                where_clauses.push(quote!(#param: #argument_bound));
                if optional {
                    assign_rhs = quote!(#some(#assign_rhs));
                }
                assignments.push(quote!(#arg_name: #assign_rhs));
            }
            SetterBounds::Collection { argument_bound, .. } => {
                let param = param_it.next().unwrap();
                parameters.push(param.clone());
                arguments.push(quote!(#arg_name: #param));
                where_clauses.push(quote!(#param: #argument_bound));
                let mut assign_rhs = quote!(#arg_name.into_iter().collect());
                if optional {
                    assign_rhs = quote!(#some(#assign_rhs));
                }
                assignments.push(quote!(#arg_name: #assign_rhs));
            }
        }
    }

    let parameters = if parameters.is_empty() {
        quote!()
    } else {
        quote!(<#(#parameters,)*>)
    };

    let where_clauses = if where_clauses.is_empty() {
        quote!()
    } else {
        quote!(where #(#where_clauses,)*)
    };

    let new_ = if def.fields().iter().any(|f| **f.field_name() == "new") {
        quote!(new_)
    } else {
        quote!(new)
    };

    quote! {
        /// Constructs a new instance of the type.
        #[inline]
        pub fn #new_ #parameters(#(#arguments,)*) -> #name
        #where_clauses
        {
            #name {
                #(#assignments),*
            }
        }
    }
}
