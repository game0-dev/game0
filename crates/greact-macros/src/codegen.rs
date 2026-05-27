use proc_macro2::TokenStream;
use quote::quote;

use crate::parser::*;

/// Generate the Rust code for the entire `view!` macro invocation.
pub fn generate(input: &ViewMacroInput) -> TokenStream {
    let cx = &input.cx;
    match &input.root {
        JsxNode::Element(elem) => gen_root_element(elem, cx),
        _ => panic!("view! root must be an element (e.g. <div>...</div>)"),
    }
}

/// Root-level element: produces `cx.build(Tag, |__b| { ... })` which returns
/// `NodeId`.
fn gen_root_element(elem: &JsxElement, cx: &syn::Ident) -> TokenStream {
    match &elem.tag {
        JsxTag::BuiltIn(tag) => {
            let tag_variant = to_element_tag(tag);
            let attr_calls = gen_attrs(&elem.attrs);
            let child_stmts = gen_children(&elem.children, cx);
            quote! {
                #cx.build(greact::ElementTag::#tag_variant, |__b| {
                    #(#attr_calls)*
                    #(#child_stmts)*
                })
            }
        }
        JsxTag::Component(name) => gen_root_component(name, &elem.attrs, cx),
    }
}

fn gen_root_component(
    name: &syn::Ident,
    attrs: &[JsxAttr],
    cx: &syn::Ident,
) -> TokenStream {
    let fn_name = syn::Ident::new(&to_snake_case(&name.to_string()), name.span());
    if attrs.is_empty() {
        quote! { #fn_name(#cx) }
    } else {
        let props_type = syn::Ident::new(&format!("{}Props", name), name.span());
        let field_inits = gen_props_fields(attrs);
        quote! {
            #fn_name(#cx, #props_type {
                #(#field_inits)*
                ..Default::default()
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Child-level code generation
// ---------------------------------------------------------------------------

fn gen_child_node(node: &JsxNode, cx: &syn::Ident) -> TokenStream {
    match node {
        JsxNode::Element(elem) => gen_child_element(elem, cx),
        JsxNode::Text(lit) => quote! { __b.child_text(#lit); },
        JsxNode::Expr(expr) => gen_child_expr(expr),
        JsxNode::For(for_node) => gen_child_for(for_node, cx),
    }
}

fn gen_child_expr(expr: &syn::Expr) -> TokenStream {
    match expr {
        syn::Expr::Closure(_) => quote! { __b.child_text_dyn(#expr); },
        _ => quote! { __b.child_text(#expr); },
    }
}

fn gen_child_for(for_node: &JsxFor, cx: &syn::Ident) -> TokenStream {
    let pat = &for_node.pat;
    let iter = &for_node.iter;
    let body = gen_children(&for_node.body, cx);
    quote! {
        for #pat in #iter {
            #(#body)*
        }
    }
}

fn gen_child_element(elem: &JsxElement, cx: &syn::Ident) -> TokenStream {
    match &elem.tag {
        JsxTag::BuiltIn(tag) => {
            let tag_variant = to_element_tag(tag);
            let attr_calls = gen_attrs(&elem.attrs);
            let child_stmts = gen_children(&elem.children, cx);
            quote! {
                __b.child(greact::ElementTag::#tag_variant, |__b| {
                    #(#attr_calls)*
                    #(#child_stmts)*
                });
            }
        }
        JsxTag::Component(name) => gen_child_component(name, &elem.attrs, cx),
    }
}

fn gen_child_component(
    name: &syn::Ident,
    attrs: &[JsxAttr],
    cx: &syn::Ident,
) -> TokenStream {
    let fn_name = syn::Ident::new(&to_snake_case(&name.to_string()), name.span());
    if attrs.is_empty() {
        quote! {
            __b.child_component(|__cx| #fn_name(__cx));
        }
    } else {
        let props_type = syn::Ident::new(&format!("{}Props", name), name.span());
        let field_inits = gen_props_fields(attrs);
        quote! {
            __b.child_component(|__cx| #fn_name(__cx, #props_type {
                #(#field_inits)*
                ..Default::default()
            }));
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn gen_attrs(attrs: &[JsxAttr]) -> Vec<TokenStream> {
    attrs
        .iter()
        .map(|a| {
            let name = &a.name;
            let value = &a.value;
            quote! { __b.#name(#value); }
        })
        .collect()
}

fn gen_children(children: &[JsxNode], cx: &syn::Ident) -> Vec<TokenStream> {
    children.iter().map(|c| gen_child_node(c, cx)).collect()
}

fn gen_props_fields(attrs: &[JsxAttr]) -> Vec<TokenStream> {
    attrs
        .iter()
        .map(|a| {
            let field = &a.name;
            let value = &a.value;
            quote! { #field: (#value).into(), }
        })
        .collect()
}

/// `"div"` → `Div`, `"button"` → `Button`
fn to_element_tag(ident: &syn::Ident) -> syn::Ident {
    let s = ident.to_string();
    let cap = format!("{}{}", s[..1].to_uppercase(), &s[1..]);
    syn::Ident::new(&cap, ident.span())
}

/// PascalCase → snake_case: `"AppSidebar"` → `"app_sidebar"`
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_lowercase().next().unwrap());
    }
    result
}
