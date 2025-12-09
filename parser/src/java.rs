use std::str::Utf8Error;

use ast::{
    lexer,
    types::{
        AstAnnotationField, AstClassMethod, AstClassVariable, AstExtends, AstFile, AstImports,
        AstInterfaceConstant, AstInterfaceMethod, AstInterfaceMethodDefault, AstJTypeKind,
        AstSuperClass, AstThing, AstTypeParameters,
    },
};
use my_string::MyString;

use crate::{
    SourceDestination,
    dto::{self, Access, Field, ImportUnit, Method},
};

#[derive(Debug)]
pub enum ParseJavaError {
    Utf8(Utf8Error),
    Class(dto::ClassError),
    Io(std::io::Error),
    UnknownJType(String, String),
    UnknownWildcard(String),
    Ast(ast::error::AstError),
    Lexer(ast::lexer::LexerError),
}
pub fn load_java(
    bytes: &[u8],
    source: SourceDestination,
) -> Result<crate::dto::Class, ParseJavaError> {
    let str = str::from_utf8(bytes).map_err(ParseJavaError::Utf8)?;
    let tokens = lexer::lex(str).map_err(ParseJavaError::Lexer)?;
    let parsed = ast::parse_file(&tokens).map_err(ParseJavaError::Ast)?;
    load_java_tree(&parsed, source)
}

pub fn load_java_tree(
    ast: &AstFile,
    source: SourceDestination,
) -> Result<crate::dto::Class, ParseJavaError> {
    let mut methods: Vec<Method> = vec![];
    let mut fields: Vec<Field> = vec![];
    let class_path_base: MyString = ast
        .package
        .as_ref()
        .map_or_else(MyString::new, |p| (&p.name).into());
    let mut name: MyString = String::new();
    let mut super_class = dto::SuperClass::None;
    let mut super_interfaces = vec![];
    let imports: Vec<ImportUnit> = ast.imports.as_ref().map_or_else(Vec::new, |imports| {
        imports.imports.iter().map(Into::into).collect()
    });
    if let Some(thing) = ast.things.first() {
        match thing {
            AstThing::Class(class) => {
                name.clone_from(&class.name.value);
                methods.extend(class.block.methods.iter().map(convert_class_method));
                fields.extend(class.block.variables.iter().map(convert_class_field));
                //TDOO: Handle others
                super_class = match &class.superclass.first() {
                    None | Some(AstSuperClass::None) => dto::SuperClass::None,
                    Some(AstSuperClass::Name(ast_identifier)) => {
                        dto::SuperClass::Name(ast_identifier.into())
                    }
                };
            }
            AstThing::Record(_) => todo!(),
            AstThing::Enumeration(enumeration) => {
                name = (&enumeration.name).into();
                methods.extend(enumeration.methods.iter().map(convert_class_method));
                fields.extend(enumeration.variables.iter().map(convert_class_field));
            }
            AstThing::Interface(interface) => {
                name = (&interface.name).into();
                if let Some(ext) = &interface.extends {
                    super_interfaces.extend(fun_name(ext, &imports));
                }
                methods.extend(interface.methods.iter().map(convert_interface_method));
                methods.extend(
                    interface
                        .default_methods
                        .iter()
                        .map(convert_interface_default_method),
                );
                fields.extend(interface.constants.iter().map(convert_interface_constant));
            }
            AstThing::Annotation(annotation) => {
                name = (&annotation.name).into();
                fields.extend(annotation.fields.iter().map(convert_annotation_field));
            }
        }
    }
    let source = match source {
        SourceDestination::RelativeInFolder(e) => {
            format!("{}/{}/{}.java", e, &class_path_base.replace('.', "/"), name)
        }
        SourceDestination::Here(e) => e.replace('\\', "/"),
        SourceDestination::None => String::new(),
    };
    let mut class_path = String::new();
    class_path.push_str(&class_path_base);
    class_path.push('.');
    class_path.push_str(&name);

    Ok(dto::Class {
        source,
        class_path,
        access: Access::empty(),
        super_class,
        super_interfaces,
        imports: convert_imports(ast.imports.as_ref(), class_path_base),
        name,
        methods,
        fields,
    })
}

fn fun_name(ext: &AstExtends, imports: &[ImportUnit]) -> impl Iterator<Item = dto::SuperClass> {
    ext.parameters.iter().filter_map(|i| {
        if let AstJTypeKind::Class(c) = &i.value {
            return imports
                .iter()
                .find_map(|i| i.get_imported_class_package(&c.value))
                .map_or_else(
                    || Some(dto::SuperClass::Name(c.into())),
                    |class_path| Some(dto::SuperClass::ClassPath(class_path)),
                );
        }
        None
    })
}

fn convert_imports(imports: Option<&AstImports>, package: MyString) -> Vec<ImportUnit> {
    let mut out = vec![ImportUnit::Package(package)];
    if let Some(imports) = imports {
        out.extend(imports.imports.iter().map(Into::into));
    }
    out
}

fn convert_class_method(m: &AstClassMethod) -> Method {
    let access = Access::from(&m.header.avaliability, Access::Public);
    let parameters = m
        .header
        .parameters
        .parameters
        .iter()
        .map(|p| dto::Parameter {
            name: Some((&p.name).into()),
            jtype: check_type_parameters(&p.jtype, m.header.type_parameters.as_ref()),
        })
        .collect();
    let throws = m
        .header
        .throws
        .as_ref()
        .map_or_else(Vec::new, |t| t.parameters.iter().map(Into::into).collect());
    Method {
        access,
        name: (&m.header.name).into(),
        parameters,
        throws,
        ret: check_type_parameters(&m.header.jtype, m.header.type_parameters.as_ref()),
        source: None,
    }
}
fn convert_interface_method(m: &AstInterfaceMethod) -> Method {
    let access = Access::from(&m.header.avaliability, Access::Public);
    let parameters = m
        .header
        .parameters
        .parameters
        .iter()
        .map(|p| dto::Parameter {
            name: Some((&p.name).into()),
            jtype: check_type_parameters(&p.jtype, m.header.type_parameters.as_ref()),
        })
        .collect();
    let throws = m.header.throws.as_ref().map_or_else(Vec::new, |t| {
        t.parameters
            .iter()
            .map(|i| check_type_parameters(i, m.header.type_parameters.as_ref()))
            .collect()
    });
    Method {
        access,
        name: (&m.header.name).into(),
        parameters,
        throws,
        ret: check_type_parameters(&m.header.jtype, m.header.type_parameters.as_ref()),
        source: None,
    }
}
fn convert_interface_default_method(m: &AstInterfaceMethodDefault) -> Method {
    let access = Access::from(&m.header.avaliability, Access::Public);
    let parameters = m
        .header
        .parameters
        .parameters
        .iter()
        .map(|p| dto::Parameter {
            name: Some((&p.name).into()),
            jtype: check_type_parameters(&p.jtype, m.header.type_parameters.as_ref()),
        })
        .collect();
    let throws = m.header.throws.as_ref().map_or_else(Vec::new, |t| {
        t.parameters
            .iter()
            .map(|i| check_type_parameters(i, m.header.type_parameters.as_ref()))
            .collect()
    });
    Method {
        access,
        name: (&m.header.name).into(),
        parameters,
        throws,
        ret: check_type_parameters(&m.header.jtype, m.header.type_parameters.as_ref()),
        source: None,
    }
}

fn convert_interface_constant(c: &AstInterfaceConstant) -> dto::Field {
    dto::Field {
        access: Access::from(&c.avaliability, Access::Public),
        name: (&c.name).into(),
        jtype: (&c.jtype).into(),
        source: None,
    }
}
fn convert_annotation_field(c: &AstAnnotationField) -> dto::Field {
    dto::Field {
        access: Access::empty(),
        name: (&c.name).into(),
        jtype: (&c.jtype).into(),
        source: None,
    }
}
fn convert_class_field(c: &AstClassVariable) -> dto::Field {
    let access = Access::from(&c.avaliability, Access::Public);
    let jtype: dto::JType = (&c.jtype).into();

    dto::Field {
        access,
        jtype,
        name: c.name.value.clone(),
        source: None,
    }
}

fn check_type_parameters(
    jtype: &ast::types::AstJType,
    type_parameters: Option<&AstTypeParameters>,
) -> dto::JType {
    let jtype: dto::JType = jtype.into();
    let Some(type_parameters) = type_parameters else {
        return jtype;
    };

    if let dto::JType::Class(ref p) = jtype
        && type_parameters
            .parameters
            .iter()
            .any(|i| i.name.value == *p)
    {
        return dto::JType::Parameter(p.to_owned());
    }
    if let dto::JType::Generic(name, params) = jtype {
        let params = params
            .iter()
            .map(|i| {
                if let dto::JType::Class(p) = i
                    && type_parameters
                        .parameters
                        .iter()
                        .any(|i| i.name.value == *p)
                {
                    return dto::JType::Parameter(p.to_owned());
                }
                i.clone()
            })
            .collect();
        return dto::JType::Generic(name, params);
    }

    jtype
}

#[cfg(test)]
pub mod tests {
    use crate::SourceDestination;

    use super::load_java;

    #[test]
    fn jtype_recognition() {
        let result = load_java(
            include_bytes!("../test/Types.java"),
            SourceDestination::Here("/path/to/source/Test.java".into()),
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn super_class() {
        let content = r#"
package a.test;
public class Test extends AThing { }
        "#;
        let result = load_java(
            content.as_bytes(),
            SourceDestination::Here("/path/to/source/Test.java".into()),
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn generic_type_declare() {
        let content = r#"
package a.test;
public class Test {
  public static <T> int add(Collection<T> list, T item){}
}
        "#;
        let result = load_java(
            content.as_bytes(),
            SourceDestination::Here("/path/to/source/Test.java".into()),
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn thrower() {
        let content = include_str!("../test/Thrower.java");
        let result = load_java(
            content.as_bytes(),
            SourceDestination::Here("/path/to/source/Thrower.java".into()),
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn interface_constants() {
        let result = load_java(
            include_bytes!("../test/Constants.java"),
            SourceDestination::RelativeInFolder("/path/to/source".into()),
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn interface_base() {
        let result = load_java(
            include_bytes!("../test/InterfaceBase.java"),
            SourceDestination::RelativeInFolder("/path/to/source".into()),
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn jenum() {
        let result = load_java(
            include_bytes!("../test/Variants.java"),
            SourceDestination::RelativeInFolder("/path/to/source".into()),
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn jannotation() {
        let result = load_java(
            include_bytes!("../test/Annotation.java"),
            SourceDestination::RelativeInFolder("/path/to/source".into()),
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn everything() {
        let result = load_java(
            include_bytes!("../test/Everything.java"),
            SourceDestination::None,
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn int() {
        let src = r#"
package a.test;

import jakarta.inject.Inject;
import jakarta.ws.rs.GET;

import jakarta.ws.rs.Path;

public class Test {
}
 "#;
        let result = load_java(
            src.as_bytes(),
            SourceDestination::RelativeInFolder("/path/to/source".into()),
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }
    #[test]
    fn super_interfaces() {
        let result = load_java(
            include_bytes!("../test/SuperInterface.java"),
            SourceDestination::None,
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }
}
