use std::str::Utf8Error;

use ast::{
    lexer,
    types::{
        AstAnnotationField, AstClassMethod, AstClassVariable, AstExtends, AstFile, AstImports,
        AstInterfaceConstant, AstInterfaceMethod, AstInterfaceMethodDefault, AstJTypeKind,
        AstThing, AstTypeParameters,
    },
};

use crate::{
    dto::{self, Access, Field, ImportUnit, Method},
    loader::SourceDestination,
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
    load_java_tree(parsed, source)
}

pub fn load_java_tree(
    ast: AstFile,
    source: SourceDestination,
) -> Result<crate::dto::Class, ParseJavaError> {
    let thing = ast.thing;
    let mut methods: Vec<Method> = vec![];
    let mut fields: Vec<Field> = vec![];
    let class_path_base: String = ast.package.into();
    let name;
    let mut super_class = dto::SuperClass::None;
    let mut super_interfaces = vec![];
    let imports: Vec<ImportUnit> = ast.imports.imports.iter().map(|i| i.into()).collect();
    match thing {
        AstThing::Class(class) => {
            name = class.name.into();
            methods.extend(class.methods.iter().map(convert_class_method));
            fields.extend(class.variables.iter().map(convert_class_field));
            super_class = match class.superclass {
                ast::types::AstSuperClass::None => dto::SuperClass::None,
                ast::types::AstSuperClass::Name(ast_identifier) => {
                    dto::SuperClass::Name(ast_identifier.into())
                }
                ast::types::AstSuperClass::ClassPath(ast_identifier) => {
                    dto::SuperClass::ClassPath(ast_identifier.into())
                }
            };
        }
        AstThing::Enumeration(enumeration) => {
            name = enumeration.name.into();
            methods.extend(enumeration.methods.iter().map(convert_class_method));
            fields.extend(enumeration.variables.iter().map(convert_class_field));
        }
        AstThing::Interface(interface) => {
            name = interface.name.into();
            if let Some(ext) = interface.extends {
                super_interfaces.extend(fun_name(&ext, &imports));
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
            name = annotation.name.into();
            fields.extend(annotation.fields.iter().map(convert_annotation_field));
        }
    }
    let source = match source {
        SourceDestination::RelativeInFolder(e) => {
            format!("{}/{}/{}.java", e, &class_path_base.replace(".", "/"), name)
        }
        SourceDestination::Here(e) => e.replace("\\", "/"),
        SourceDestination::None => "".to_string(),
    };

    Ok(dto::Class {
        source,
        class_path: format!("{class_path_base}.{name}"),
        access: vec![],
        super_class,
        super_interfaces,
        imports: convert_imports(ast.imports, class_path_base),
        name,
        methods,
        fields,
    })
}

fn fun_name(ext: &AstExtends, imports: &[ImportUnit]) -> impl Iterator<Item = dto::SuperClass> {
    ext.parameters.iter().filter_map(|i| {
        if let AstJTypeKind::Class(c) = &i.value {
            return match imports
                .iter()
                .find_map(|i| i.get_imported_class_package(&c.value))
            {
                Some(class_path) => Some(dto::SuperClass::ClassPath(class_path)),
                None => Some(dto::SuperClass::Name(c.into())),
            };
        }
        None
    })
}

fn convert_imports(imports: AstImports, package: String) -> Vec<ImportUnit> {
    let mut out = vec![ImportUnit::Package(package)];
    out.extend(imports.imports.iter().map(|i| i.into()));
    out
}

fn convert_class_method(m: &AstClassMethod) -> Method {
    let mut access = vec![];
    if m.header.stat {
        access.push(dto::Access::Static);
    }
    let avaliability = Access::from(&m.header.avaliability, Access::Protected);
    access.push(avaliability);
    let parameters = m
        .header
        .parameters
        .parameters
        .iter()
        .map(|p| dto::Parameter {
            name: Some((&p.name).into()),
            jtype: check_type_parameters(&p.jtype, &m.header.type_parameters),
        })
        .collect();
    let mut throws = vec![];
    if let Some(t) = &m.header.throws {
        throws = t.parameters.iter().map(|i| i.into()).collect();
    }
    Method {
        access,
        name: (&m.header.name).into(),
        parameters,
        throws,
        ret: check_type_parameters(&m.header.jtype, &m.header.type_parameters),
        source: None,
    }
}
fn convert_interface_method(m: &AstInterfaceMethod) -> Method {
    let mut access = vec![];
    if m.header.stat {
        access.push(dto::Access::Static);
    }
    let avaliability = Access::from(&m.header.avaliability, Access::Public);
    access.push(avaliability);
    let parameters = m
        .header
        .parameters
        .parameters
        .iter()
        .map(|p| dto::Parameter {
            name: Some((&p.name).into()),
            jtype: check_type_parameters(&p.jtype, &m.header.type_parameters),
        })
        .collect();
    let mut throws = vec![];
    if let Some(t) = &m.header.throws {
        throws = t
            .parameters
            .iter()
            .map(|i| check_type_parameters(i, &m.header.type_parameters))
            .collect();
    }
    Method {
        access,
        name: (&m.header.name).into(),
        parameters,
        throws,
        ret: check_type_parameters(&m.header.jtype, &m.header.type_parameters),
        source: None,
    }
}
fn convert_interface_default_method(m: &AstInterfaceMethodDefault) -> Method {
    let mut access = vec![];
    if m.header.stat {
        access.push(dto::Access::Static);
    }
    let avaliability = Access::from(&m.header.avaliability, Access::Public);
    access.push(avaliability);
    let parameters = m
        .header
        .parameters
        .parameters
        .iter()
        .map(|p| dto::Parameter {
            name: Some((&p.name).into()),
            jtype: check_type_parameters(&p.jtype, &m.header.type_parameters),
        })
        .collect();
    let mut throws = vec![];
    if let Some(t) = &m.header.throws {
        throws = t
            .parameters
            .iter()
            .map(|i| check_type_parameters(i, &m.header.type_parameters))
            .collect();
    }
    Method {
        access,
        name: (&m.header.name).into(),
        parameters,
        throws,
        ret: check_type_parameters(&m.header.jtype, &m.header.type_parameters),
        source: None,
    }
}

fn convert_interface_constant(c: &AstInterfaceConstant) -> dto::Field {
    dto::Field {
        access: vec![Access::from(&c.avaliability, Access::Public)],
        name: (&c.name).into(),
        jtype: (&c.jtype).into(),
        source: None,
    }
}
fn convert_annotation_field(c: &AstAnnotationField) -> dto::Field {
    dto::Field {
        access: vec![],
        name: (&c.name).into(),
        jtype: (&c.jtype).into(),
        source: None,
    }
}
fn convert_class_field(c: &AstClassVariable) -> dto::Field {
    dto::Field {
        access: vec![Access::from(&c.avaliability, Access::Public)],
        name: (&c.name).into(),
        jtype: (&c.jtype).into(),
        source: None,
    }
}

fn check_type_parameters(
    jtype: &ast::types::AstJType,
    type_parameters: &Option<AstTypeParameters>,
) -> dto::JType {
    let jtype: dto::JType = jtype.into();
    let Some(type_parameters) = type_parameters else {
        return jtype;
    };

    if let dto::JType::Class(ref p) = jtype {
        if type_parameters.parameters.iter().any(|i| i.value == p) {
            return dto::JType::Parameter(p.to_owned());
        }
    }
    if let dto::JType::Generic(name, params) = jtype {
        let params = params
            .iter()
            .map(|i| {
                if let dto::JType::Class(p) = i {
                    if type_parameters.parameters.iter().any(|i| i.value == p) {
                        return dto::JType::Parameter(p.to_owned());
                    }
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
    use crate::loader::SourceDestination;

    use super::load_java;

    #[test]
    fn jtype_recognition() {
        let result = load_java(
            include_bytes!("../test/Types.java"),
            SourceDestination::Here("/path/to/source/Test.java".to_string()),
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn super_class() {
        let content = r#"
package a.test;
public class Test extends a { }
        "#;
        let result = load_java(
            content.as_bytes(),
            SourceDestination::Here("/path/to/source/Test.java".to_string()),
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
            SourceDestination::Here("/path/to/source/Test.java".to_string()),
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn thrower() {
        let content = include_str!("../test/Thrower.java");
        let result = load_java(
            content.as_bytes(),
            SourceDestination::Here("/path/to/source/Thrower.java".to_string()),
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn interface_constants() {
        let result = load_java(
            include_bytes!("../test/Constants.java"),
            SourceDestination::RelativeInFolder("/path/to/source".to_string()),
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn interface_base() {
        let result = load_java(
            include_bytes!("../test/InterfaceBase.java"),
            SourceDestination::RelativeInFolder("/path/to/source".to_string()),
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn jenum() {
        let result = load_java(
            include_bytes!("../test/Variants.java"),
            SourceDestination::RelativeInFolder("/path/to/source".to_string()),
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn jannotation() {
        let result = load_java(
            include_bytes!("../test/Annotation.java"),
            SourceDestination::RelativeInFolder("/path/to/source".to_string()),
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
            SourceDestination::RelativeInFolder("/path/to/source".to_string()),
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
