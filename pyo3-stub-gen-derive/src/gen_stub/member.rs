use super::{escape_return_type, parse_pyo3_attrs,extract_documents, Attr};

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{Error, Field, ImplItemFn, Result, Type};

#[derive(Debug)]
pub struct MemberInfo {
    name: String,
    r#type: Type,
    doc: String,
}

impl MemberInfo {
    pub fn is_candidate_item(item: &ImplItemFn) -> Result<bool> {
        let attrs = parse_pyo3_attrs(&item.attrs)?;
        Ok(attrs.iter().any(|attr| matches!(attr, Attr::Getter(_))))
    }

    pub fn is_candidate_field(field: &Field) -> Result<bool> {
        let Field { attrs, .. } = field;
        Ok(parse_pyo3_attrs(attrs)?
            .iter()
            .any(|attr| matches!(attr, Attr::Get)))
    }
}

impl TryFrom<ImplItemFn> for MemberInfo {
    type Error = Error;
    fn try_from(item: ImplItemFn) -> Result<Self> {
        assert!(Self::is_candidate_item(&item)?);
        let ImplItemFn { attrs, sig, .. } = &item;
        let _attrs = parse_pyo3_attrs(attrs)?;
        for attr in _attrs {
            if let Attr::Getter(name) = attr {
                let name = name.unwrap_or(sig.ident.to_string());
                return Ok(MemberInfo {
                    name: if let Some(name) = name.strip_prefix("get_") {
                        name.to_owned()
                    } else {
                        name
                    },
                    r#type: escape_return_type(&sig.output).expect("Getter must return a type"),
                    doc: extract_documents(attrs).join("\n"),
                });
            }
        }
        unreachable!("Not a getter: {:?}", item)
    }
}

impl TryFrom<Field> for MemberInfo {
    type Error = Error;
    fn try_from(field: Field) -> Result<Self> {
        let Field {
            ident, ty, attrs, ..
        } = field;
        let mut field_name = None;
        for attr in parse_pyo3_attrs(&attrs)? {
            if let Attr::Name(name) = attr {
                field_name = Some(name);
            }
        }
        Ok(Self {
            name: field_name.unwrap_or(ident.unwrap().to_string()),
            r#type: ty,
            doc: extract_documents(&attrs).join("\n"),
        })
    }
}

impl ToTokens for MemberInfo {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { name, r#type: ty, doc } = self;
        tokens.append_all(quote! {
            ::pyo3_stub_gen::type_info::MemberInfo {
                name: #name,
                r#type: <#ty as ::pyo3_stub_gen::PyStubType>::type_output,
                doc: #doc,
            }
        })
    }
}
