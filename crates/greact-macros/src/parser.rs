use syn::parse::{Parse, ParseStream};

// ---------------------------------------------------------------------------
// AST types
// ---------------------------------------------------------------------------

/// A single JSX node: element, expression, or text literal.
pub enum JsxNode {
    Element(JsxElement),
    Expr(syn::Expr),
    Text(syn::LitStr),
    For(JsxFor),
}

/// `<tag attr={expr}>children</tag>` or `<tag attr={expr} />`
pub struct JsxElement {
    pub tag: JsxTag,
    pub attrs: Vec<JsxAttr>,
    pub children: Vec<JsxNode>,
    pub self_closing: bool,
}

/// Tag kind – built-in (lowercase) or component (PascalCase).
pub enum JsxTag {
    BuiltIn(syn::Ident),
    Component(syn::Ident),
}

/// `name={value}`
pub struct JsxAttr {
    pub name: syn::Ident,
    pub value: syn::Expr,
}

/// `for pat in expr { ...jsx children... }` (must appear inside `{ ... }`).
pub struct JsxFor {
    pub pat: syn::Pat,
    pub iter: syn::Expr,
    pub body: Vec<JsxNode>,
}

// ---------------------------------------------------------------------------
// Top-level input: `view! { cx, <root> ... </root> }`
// ---------------------------------------------------------------------------

pub struct ViewMacroInput {
    pub cx: syn::Ident,
    pub root: JsxNode,
}

impl Parse for ViewMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let cx: syn::Ident = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let root: JsxNode = input.parse()?;
        Ok(ViewMacroInput { cx, root })
    }
}

// ---------------------------------------------------------------------------
// JsxNode parsing
// ---------------------------------------------------------------------------

impl Parse for JsxNode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(syn::Token![<]) {
            // Element: <tag ...>
            Ok(JsxNode::Element(input.parse()?))
        } else if input.peek(syn::token::Brace) {
            // Either dynamic expression: {expr}
            // or for-loop node: {for pat in iter { ... }}
            let content;
            syn::braced!(content in input);
            if content.peek(syn::Token![for]) {
                Ok(JsxNode::For(content.parse()?))
            } else {
                Ok(JsxNode::Expr(content.parse()?))
            }
        } else if input.peek(syn::LitStr) {
            // Static text: "string"
            Ok(JsxNode::Text(input.parse()?))
        } else {
            Err(input.error("expected <element>, {expr}, {for ...}, or \"string\""))
        }
    }
}

impl Parse for JsxFor {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<syn::Token![for]>()?;
        let pat: syn::Pat = input.call(syn::Pat::parse_single)?;
        input.parse::<syn::Token![in]>()?;
        let iter: syn::Expr = input.parse()?;

        let content;
        syn::braced!(content in input);
        let mut body = Vec::new();
        while !content.is_empty() {
            body.push(content.parse::<JsxNode>()?);
        }

        Ok(JsxFor { pat, iter, body })
    }
}

// ---------------------------------------------------------------------------
// JsxElement parsing
// ---------------------------------------------------------------------------

impl Parse for JsxElement {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // 1. '<'
        input.parse::<syn::Token![<]>()?;

        // 2. Tag name
        let tag_ident: syn::Ident = input.parse()?;
        let tag = if tag_ident
            .to_string()
            .chars()
            .next()
            .unwrap()
            .is_uppercase()
        {
            JsxTag::Component(tag_ident.clone())
        } else {
            JsxTag::BuiltIn(tag_ident.clone())
        };

        // 3. Attributes: name={expr} ...
        let mut attrs = Vec::new();
        while !input.peek(syn::Token![>]) && !input.peek(syn::Token![/]) {
            let name: syn::Ident = input.parse()?;
            input.parse::<syn::Token![=]>()?;
            let content;
            syn::braced!(content in input);
            let value: syn::Expr = content.parse()?;
            attrs.push(JsxAttr { name, value });
        }

        // 4. Self-closing '/>' or opening '>'
        if input.peek(syn::Token![/]) {
            input.parse::<syn::Token![/]>()?;
            input.parse::<syn::Token![>]>()?;
            return Ok(JsxElement {
                tag,
                attrs,
                children: vec![],
                self_closing: true,
            });
        }
        input.parse::<syn::Token![>]>()?;

        // 5. Children (recursive)
        let mut children = Vec::new();
        while !Self::peek_closing(input) {
            children.push(input.parse::<JsxNode>()?);
        }

        // 6. Closing tag: '</tag>'
        input.parse::<syn::Token![<]>()?;
        input.parse::<syn::Token![/]>()?;
        let closing: syn::Ident = input.parse()?;
        if closing != tag_ident {
            return Err(syn::Error::new(
                closing.span(),
                format!("expected </{}>, found </{}>", tag_ident, closing),
            ));
        }
        input.parse::<syn::Token![>]>()?;

        Ok(JsxElement {
            tag,
            attrs,
            children,
            self_closing: false,
        })
    }
}

impl JsxElement {
    /// Peek ahead to see if the next tokens are `</` (closing tag start).
    fn peek_closing(input: ParseStream) -> bool {
        let fork = input.fork();
        fork.parse::<syn::Token![<]>().is_ok() && fork.peek(syn::Token![/])
    }
}
