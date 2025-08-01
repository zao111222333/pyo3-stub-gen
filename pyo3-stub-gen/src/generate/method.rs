use crate::{generate::*, type_info::*, TypeInfo};
use std::{collections::HashSet, fmt};

pub use crate::type_info::MethodType;

/// Definition of a class method.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDef {
    pub name: &'static str,
    pub args: Vec<Arg>,
    pub r#return: TypeInfo,
    pub doc: &'static str,
    pub r#type: MethodType,
}

impl Import for MethodDef {
    fn import(&self) -> HashSet<ModuleRef> {
        let mut import = self.r#return.import.clone();
        for arg in &self.args {
            import.extend(arg.import().into_iter());
        }
        import
    }
}

impl From<&MethodInfo> for MethodDef {
    fn from(info: &MethodInfo) -> Self {
        Self {
            name: info.name,
            args: info.args.iter().map(Arg::from).collect(),
            r#return: (info.r#return)(),
            doc: info.doc,
            r#type: info.r#type,
        }
    }
}

impl fmt::Display for MethodDef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let indent = indent();
        let mut needs_comma = false;
        match self.r#type {
            MethodType::Static => {
                writeln!(f, "{indent}@staticmethod")?;
                write!(f, "{indent}def {}(", self.name)?;
            }
            MethodType::Class | MethodType::New => {
                if self.r#type == MethodType::Class {
                    // new is a classmethod without the decorator
                    writeln!(f, "{indent}@classmethod")?;
                }
                write!(f, "{indent}def {}(cls", self.name)?;
                needs_comma = true;
            }
            MethodType::Instance => {
                write!(f, "{indent}def {}(self", self.name)?;
                needs_comma = true;
            }
        }
        for arg in &self.args {
            if needs_comma {
                write!(f, ", ")?;
            }
            write!(f, "{arg}")?;
            needs_comma = true;
        }
        write!(f, ") -> {}:", self.r#return)?;

        let doc = self.doc;
        if !doc.is_empty() {
            writeln!(f)?;
            let double_indent = format!("{indent}{indent}");
            docstring::write_docstring(f, self.doc, &double_indent)?;
        } else {
            writeln!(f, " ...")?;
        }
        Ok(())
    }
}
