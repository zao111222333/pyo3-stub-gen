use super::{RenamingRule, Signature};
use proc_macro2::TokenTree;
use quote::ToTokens;
use syn::{
    parse::ParseStream, Attribute, Expr, ExprLit, Ident, Lit, Meta, MetaList, Result, Token, Type,
};

pub fn extract_documents(attrs: &[Attribute]) -> Vec<String> {
    let mut docs = Vec::new();
    for attr in attrs {
        // `#[doc = "..."]` case
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(syn::MetaNameValue {
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(doc), ..
                    }),
                ..
            }) = &attr.meta
            {
                let doc = doc.value();
                // Remove head space
                //
                // ```
                // /// This is special document!
                //    ^ This space is trimmed here
                // ```
                docs.push(if !doc.is_empty() && doc.starts_with(' ') {
                    doc[1..].to_string()
                } else {
                    doc
                });
            }
        }
    }
    docs
}

/// `#[pyo3(...)]` style attributes appear in `#[pyclass]` and `#[pymethods]` proc-macros
///
/// As the reference of PyO3 says:
///
/// https://docs.rs/pyo3/latest/pyo3/attr.pyclass.html
/// > All of these parameters can either be passed directly on the `#[pyclass(...)]` annotation,
/// > or as one or more accompanying `#[pyo3(...)]` annotations,
///
/// `#[pyclass(name = "MyClass", module = "MyModule")]` will be decomposed into
/// `#[pyclass]` + `#[pyo3(name = "MyClass")]` + `#[pyo3(module = "MyModule")]`,
/// i.e. two `Attr`s will be created for this case.
///
#[derive(Debug, Clone, PartialEq)]
#[expect(clippy::enum_variant_names)]
pub enum Attr {
    // Attributes appears in `#[pyo3(...)]` form or its equivalence
    Name(String),
    Get,
    GetAll,
    Set,
    SetAll,
    Module(String),
    Constructor(Signature),
    Signature(Signature),
    RenameAll(RenamingRule),
    Extends(Type),

    // Attributes appears in components within `#[pymethods]`
    // <https://docs.rs/pyo3/latest/pyo3/attr.pymethods.html>
    New,
    Getter(Option<String>),
    Setter(Option<String>),
    StaticMethod,
    ClassMethod,
    ClassAttr,
}

pub fn parse_pyo3_attrs(attrs: &[Attribute]) -> Result<Vec<Attr>> {
    let mut out = Vec::new();
    for attr in attrs {
        let mut new = parse_pyo3_attr(attr)?;
        out.append(&mut new);
    }
    Ok(out)
}

pub fn parse_pyo3_attr(attr: &Attribute) -> Result<Vec<Attr>> {
    let mut pyo3_attrs = Vec::new();
    let path = attr.path();
    let is_full_path_pyo3_attr = path.segments.len() == 2
        && path
            .segments
            .first()
            .is_some_and(|seg| seg.ident.eq("pyo3"))
        && path.segments.last().is_some_and(|seg| {
            seg.ident.eq("pyclass") || seg.ident.eq("pymethods") || seg.ident.eq("pyfunction")
        });
    if path.is_ident("pyclass")
        || path.is_ident("pymethods")
        || path.is_ident("pyfunction")
        || path.is_ident("pyo3")
        || is_full_path_pyo3_attr
    {
        // Inner tokens of `#[pyo3(...)]` may not be nested meta
        // which can be parsed by `Attribute::parse_nested_meta`
        // due to the case of `#[pyo3(signature = (...))]`.
        // https://pyo3.rs/v0.19.1/function/signature
        if let Meta::List(MetaList { tokens, .. }) = &attr.meta {
            use TokenTree::*;
            let tokens: Vec<TokenTree> = tokens.clone().into_iter().collect();
            // Since `(...)` part with `signature` becomes `TokenTree::Group`,
            // we can split entire stream by `,` first, and then pattern match to each cases.
            for tt in tokens.split(|tt| {
                if let Punct(p) = tt {
                    p.as_char() == ','
                } else {
                    false
                }
            }) {
                match tt {
                    [Ident(ident)] => {
                        if ident == "get" {
                            pyo3_attrs.push(Attr::Get);
                        }
                        if ident == "get_all" {
                            pyo3_attrs.push(Attr::GetAll);
                        }
                        if ident == "set" {
                            pyo3_attrs.push(Attr::Set);
                        }
                        if ident == "set_all" {
                            pyo3_attrs.push(Attr::SetAll);
                        }
                    }
                    [Ident(ident), Punct(_), Literal(lit)] => {
                        if ident == "name" {
                            pyo3_attrs
                                .push(Attr::Name(lit.to_string().trim_matches('"').to_string()));
                        }
                        if ident == "module" {
                            pyo3_attrs
                                .push(Attr::Module(lit.to_string().trim_matches('"').to_string()));
                        }
                        if ident == "rename_all" {
                            let name = lit.to_string().trim_matches('"').to_string();
                            if let Some(renaming_rule) = RenamingRule::try_new(&name) {
                                pyo3_attrs.push(Attr::RenameAll(renaming_rule));
                            }
                        }
                    }
                    [Ident(ident), Punct(_), Group(group)] => {
                        if ident == "signature" {
                            pyo3_attrs.push(Attr::Signature(syn::parse2(group.to_token_stream())?));
                        } else if ident == "constructor" {
                            pyo3_attrs
                                .push(Attr::Constructor(syn::parse2(group.to_token_stream())?));
                        }
                    }
                    [Ident(ident), Punct(_), Ident(ident2)] => {
                        if ident == "extends" {
                            pyo3_attrs.push(Attr::Extends(syn::parse2(ident2.to_token_stream())?));
                        }
                    }
                    _ => {}
                }
            }
        }
    } else if path.is_ident("new") {
        pyo3_attrs.push(Attr::New);
    } else if path.is_ident("staticmethod") {
        pyo3_attrs.push(Attr::StaticMethod);
    } else if path.is_ident("classmethod") {
        pyo3_attrs.push(Attr::ClassMethod);
    } else if path.is_ident("classattr") {
        pyo3_attrs.push(Attr::ClassAttr);
    } else if path.is_ident("getter") {
        if let Ok(inner) = attr.parse_args::<Ident>() {
            pyo3_attrs.push(Attr::Getter(Some(inner.to_string())));
        } else {
            pyo3_attrs.push(Attr::Getter(None));
        }
    } else if path.is_ident("setter") {
        if let Ok(inner) = attr.parse_args::<Ident>() {
            pyo3_attrs.push(Attr::Setter(Some(inner.to_string())));
        } else {
            pyo3_attrs.push(Attr::Setter(None));
        }
    }

    Ok(pyo3_attrs)
}

#[derive(Debug, Clone, PartialEq)]
pub enum StubGenAttr {
    /// Default value for getter
    Default(Expr),
    /// Skip a function in #[pymethods]
    Skip,
}

pub fn prune_attrs(attrs: &mut Vec<Attribute>) {
    attrs.retain(|attr| !attr.path().is_ident("gen_stub"));
}

pub fn parse_gen_stub_default(attrs: &[Attribute]) -> Result<Option<Expr>> {
    for attr in parse_gen_stub_attrs(attrs)? {
        if let StubGenAttr::Default(default) = attr {
            return Ok(Some(default));
        }
    }
    Ok(None)
}
pub fn parse_gen_stub_skip(attrs: &[Attribute]) -> Result<bool> {
    let skip = parse_gen_stub_attrs(attrs)?
        .iter()
        .any(|attr| matches!(attr, StubGenAttr::Skip));
    Ok(skip)
}
fn parse_gen_stub_attrs(attrs: &[Attribute]) -> Result<Vec<StubGenAttr>> {
    let mut out = Vec::new();
    for attr in attrs {
        let mut new = parse_gen_stub_attr(attr)?;
        out.append(&mut new);
    }
    Ok(out)
}

fn parse_gen_stub_attr(attr: &Attribute) -> Result<Vec<StubGenAttr>> {
    let mut gen_stub_attrs = Vec::new();
    let path = attr.path();
    if path.is_ident("gen_stub") {
        attr.parse_args_with(|input: ParseStream| {
            while !input.is_empty() {
                let ident: Ident = input.parse()?;
                #[allow(clippy::collapsible_else_if)]
                if input.peek(Token![=]) {
                    input.parse::<Token![=]>()?;
                    if ident == "default" {
                        gen_stub_attrs.push(StubGenAttr::Default(input.parse()?));
                    } else {
                        return Err(syn::Error::new(
                            ident.span(),
                            format!("Unsupport keyword `{ident}`, valid is `default=xxx`"),
                        ));
                    }
                } else {
                    if ident == "skip" {
                        gen_stub_attrs.push(StubGenAttr::Skip);
                    } else {
                        return Err(syn::Error::new(
                            ident.span(),
                            format!("Unsupport keyword `{ident}`, valid is `skip`"),
                        ));
                    }
                }
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                } else {
                    break;
                }
            }
            Ok(())
        })?;
    }
    Ok(gen_stub_attrs)
}

#[cfg(test)]
mod test {
    use super::*;
    use syn::{parse_str, Fields, ItemStruct};

    #[test]
    fn test_parse_pyo3_attr() -> Result<()> {
        let item: ItemStruct = parse_str(
            r#"
            #[pyclass(mapping, module = "my_module", name = "Placeholder")]
            #[pyo3(rename_all = "SCREAMING_SNAKE_CASE")]
            pub struct PyPlaceholder {
                #[pyo3(get)]
                pub name: String,
            }
            "#,
        )?;
        // `#[pyclass]` part
        let attrs = parse_pyo3_attrs(&item.attrs)?;
        assert_eq!(
            attrs,
            vec![
                Attr::Module("my_module".to_string()),
                Attr::Name("Placeholder".to_string()),
                Attr::RenameAll(RenamingRule::ScreamingSnakeCase),
            ]
        );

        // `#[pyo3(get)]` part
        if let Fields::Named(fields) = item.fields {
            let attrs = parse_pyo3_attr(&fields.named[0].attrs[0])?;
            assert_eq!(attrs, vec![Attr::Get]);
        } else {
            unreachable!()
        }
        Ok(())
    }

    #[test]
    fn test_parse_pyo3_attr_full_path() -> Result<()> {
        let item: ItemStruct = parse_str(
            r#"
            #[pyo3::pyclass(mapping, module = "my_module", name = "Placeholder")]
            pub struct PyPlaceholder {
                #[pyo3(get)]
                pub name: String,
            }
            "#,
        )?;
        // `#[pyclass]` part
        let attrs = parse_pyo3_attr(&item.attrs[0])?;
        assert_eq!(
            attrs,
            vec![
                Attr::Module("my_module".to_string()),
                Attr::Name("Placeholder".to_string())
            ]
        );

        // `#[pyo3(get)]` part
        if let Fields::Named(fields) = item.fields {
            let attrs = parse_pyo3_attr(&fields.named[0].attrs[0])?;
            assert_eq!(attrs, vec![Attr::Get]);
        } else {
            unreachable!()
        }
        Ok(())
    }
    #[test]
    fn test_parse_gen_stub_attr() -> Result<()> {
        let item: ItemStruct = parse_str(
            r#"
            pub struct PyPlaceholder {
                #[gen_stub(default = String::from("foo"), skip)]
                pub field0: String,
                #[gen_stub(skip)]
                pub field1: String,
                #[gen_stub(default = 1+2)]
                pub field2: usize,
            }
            "#,
        )?;
        let fields: Vec<_> = item.fields.into_iter().collect();
        let field0_attrs = parse_gen_stub_attrs(&fields[0].attrs)?;
        if let StubGenAttr::Default(expr) = &field0_attrs[0] {
            assert_eq!(
                expr.to_token_stream().to_string(),
                "String :: from (\"foo\")"
            );
        } else {
            panic!("attr shoubd be Default");
        };
        assert_eq!(&StubGenAttr::Skip, &field0_attrs[1]);
        let field1_attrs = parse_gen_stub_attrs(&fields[1].attrs)?;
        assert_eq!(vec![StubGenAttr::Skip], field1_attrs);
        let field2_attrs = parse_gen_stub_attrs(&fields[2].attrs)?;
        if let StubGenAttr::Default(expr) = &field2_attrs[0] {
            assert_eq!(expr.to_token_stream().to_string(), "1 + 2");
        } else {
            panic!("attr shoubd be Default");
        };
        Ok(())
    }
}
