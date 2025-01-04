use crate::{generate::*, type_info::*, TypeInfo};
use std::{collections::HashSet, fmt};

/// Definition of a class member.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberDef {
    pub name: &'static str,
    pub r#type: TypeInfo,
    pub default: Option<&'static str>,
    pub doc: &'static str,
}

impl Import for MemberDef {
    fn import(&self) -> HashSet<ModuleRef> {
        self.r#type.import.clone()
    }
}

impl From<&MemberInfo> for MemberDef {
    fn from(info: &MemberInfo) -> Self {
        Self {
            name: info.name,
            r#type: (info.r#type)(),
            default: info.default,
            doc: info.doc,
        }
    }
}

impl fmt::Display for MemberDef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let indent = indent();
        if let Some(default) = self.default {
            writeln!(f, "{indent}{}: {} = {default}", self.name, self.r#type)?;
        } else {
            writeln!(f, "{indent}{}: {}", self.name, self.r#type)?;
        }
        if !self.doc.is_empty() {
            writeln!(f, r#"{indent}r""""#)?;
            for line in self.doc.lines() {
                writeln!(f, "{indent}{}", line)?;
            }
            writeln!(f, r#"{indent}""""#)?;
        }
        Ok(())
    }
}
