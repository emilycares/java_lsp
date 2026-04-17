use std::str::Utf8Error;

use ast::{
    dto_extra::access_from_availability,
    lexer,
    types::{
        AstAnnotated, AstAnnotationField, AstClassConstructor, AstClassMethod, AstClassVariable,
        AstEnumerationVariant, AstExtends, AstFile, AstImports, AstInterfaceConstant,
        AstInterfaceMethod, AstInterfaceMethodDefault, AstJTypeKind, AstSuperClass, AstThing,
        AstTypeParameter, AstTypeParameters,
    },
};
use my_string::{
    MyString,
    smol_str::{SmolStr, SmolStrBuilder},
};

use dto::{
    Access, Class, ClassError, Field, ImportUnit, JType, Method, Parameter, SourceDestination,
    SuperClass,
};

#[derive(Debug)]
pub enum ParseJavaError {
    Utf8(Utf8Error),
    Class(ClassError),
    Io(std::io::Error),
    UnknownJType(String, String),
    UnknownWildcard(String),
    Ast(ast::error::AstError),
    Lexer(ast::lexer::LexerError),
}
pub fn load_java(bytes: &[u8], source: SourceDestination) -> Result<Class, ParseJavaError> {
    let tokens = lexer::lex(bytes).map_err(ParseJavaError::Lexer)?;
    let parsed = ast::parse_file(&tokens).map_err(ParseJavaError::Ast)?;
    Ok(load_java_tree(&parsed, source))
}

pub fn load_java_tree(ast: &AstFile, source: SourceDestination) -> Class {
    let mut methods: Vec<Method> = vec![];
    let mut fields: Vec<Field> = vec![];
    let class_path_base: MyString = ast
        .package
        .as_ref()
        .map_or_else(|| MyString::new(""), |p| (&p.name).into());
    let mut name = SmolStr::new("");
    let mut super_class = SuperClass::None;
    let mut super_interfaces = vec![];
    let imports: Vec<ImportUnit> = ast.imports.as_ref().map_or_else(Vec::new, |imports| {
        imports.imports.iter().map(Into::into).collect()
    });
    let mut access = Access::empty();
    if let Some(thing) = ast.things.first() {
        match thing {
            AstThing::Class(class) => {
                access = access_from_availability(&class.availability, Access::Public);
                load_deprecated(&mut access, &class.annotated);
                name.clone_from(&class.name.value);
                methods.extend(
                    class
                        .block
                        .constructors
                        .iter()
                        .map(|i| convert_class_constructor(i, class.type_parameters.as_ref())),
                );
                methods.extend(
                    class
                        .block
                        .methods
                        .iter()
                        .map(|i| convert_class_method(i, class.type_parameters.as_ref())),
                );
                fields.extend(class.block.variables.iter().map(convert_class_field));
                //TODO: Handle others
                super_class = match &class.superclass.first() {
                    None | Some(AstSuperClass::None) => SuperClass::None,
                    Some(AstSuperClass::Name(ast_identifier)) => {
                        SuperClass::Name(ast_identifier.into())
                    }
                };
            }
            AstThing::Record(record) => {
                access = access_from_availability(&record.availability, Access::Public);
                load_deprecated(&mut access, &record.annotated);
                methods.extend(
                    record
                        .block
                        .methods
                        .iter()
                        .map(|i| convert_class_method(i, record.type_parameters.as_ref())),
                );
                fields.extend(record.block.variables.iter().map(convert_class_field));
                // TODO entries
                super_class = match &record.superclass.first() {
                    None | Some(AstSuperClass::None) => SuperClass::None,
                    Some(AstSuperClass::Name(ast_identifier)) => {
                        SuperClass::Name(ast_identifier.into())
                    }
                };
            }
            AstThing::Enumeration(enumeration) => {
                access = access_from_availability(&enumeration.availability, Access::Public);
                load_deprecated(&mut access, &enumeration.annotated);
                name = (&enumeration.name).into();
                methods.extend(
                    enumeration
                        .methods
                        .iter()
                        .map(|i| convert_class_method(i, None)),
                );
                let jtype = JType::Class(enumeration.name.value.clone());
                fields.extend(
                    enumeration
                        .variants
                        .iter()
                        .map(|i| convert_enum_variant(i, &jtype)),
                );
                fields.extend(enumeration.variables.iter().map(convert_class_field));
            }
            AstThing::Interface(interface) => {
                access = access_from_availability(&interface.availability, Access::Public);
                load_deprecated(&mut access, &interface.annotated);
                name = (&interface.name).into();
                if let Some(ext) = &interface.extends {
                    super_interfaces.extend(fun_name(ext, &imports));
                }
                methods.extend(
                    interface
                        .methods
                        .iter()
                        .map(|i| convert_interface_method(i, interface.type_parameters.as_ref())),
                );
                methods.extend(interface.default_methods.iter().map(|i| {
                    convert_interface_default_method(i, interface.type_parameters.as_ref())
                }));
                fields.extend(interface.constants.iter().map(convert_interface_constant));
            }
            AstThing::Annotation(annotation) => {
                access = access_from_availability(&annotation.availability, Access::Public);
                load_deprecated(&mut access, &annotation.annotated);
                name = (&annotation.name).into();
                fields.extend(annotation.fields.iter().map(convert_annotation_field));
            }
        }
    }
    let mut class_path = SmolStrBuilder::new();
    class_path.push_str(&class_path_base);
    class_path.push('.');
    class_path.push_str(&name);
    let class_path = class_path.finish();

    Class {
        source,
        class_path,
        access,
        super_class,
        super_interfaces,
        imports: convert_imports(ast.imports.as_ref(), class_path_base),
        name,
        methods,
        fields,
    }
}

fn load_deprecated(access: &mut Access, annotated: &[AstAnnotated]) {
    if annotated.iter().any(|i| i.name.value == "Deprecated") {
        access.insert(Access::Deprecated);
    }
}

fn fun_name(ext: &AstExtends, imports: &[ImportUnit]) -> impl Iterator<Item = SuperClass> {
    ext.parameters.iter().filter_map(|i| {
        if let AstJTypeKind::Class(c) = &i.value {
            return imports
                .iter()
                .find_map(|i: &ImportUnit| i.get_imported_class_package(&c.value))
                .map_or_else(
                    || Some(SuperClass::Name(c.into())),
                    |class_path| Some(SuperClass::ClassPath(class_path)),
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

fn convert_class_method(
    m: &AstClassMethod,
    class_type_parameters: Option<&AstTypeParameters>,
) -> Method {
    let mut access = access_from_availability(&m.header.availability, Access::Public);
    load_deprecated(&mut access, &m.header.annotated);
    let type_parameters =
        merge_type_parameters(class_type_parameters, m.header.type_parameters.as_ref());
    let parameters = m
        .header
        .parameters
        .parameters
        .iter()
        .map(|p| Parameter {
            name: Some((&p.name).into()),
            jtype: check_type_parameters(&p.jtype, &type_parameters),
        })
        .collect();
    let throws = m
        .header
        .throws
        .as_ref()
        .map_or_else(Vec::new, |t| t.parameters.iter().map(Into::into).collect());
    Method {
        access,
        name: Some((&m.header.name).into()),
        parameters,
        throws,
        ret: check_type_parameters(&m.header.jtype, &type_parameters),
        source: None,
    }
}

fn merge_type_parameters(
    class_type_parameters: Option<&AstTypeParameters>,
    type_parameters: Option<&AstTypeParameters>,
) -> Vec<AstTypeParameter> {
    let mut out = Vec::new();
    if let Some(t) = class_type_parameters {
        out.extend(t.parameters.clone());
    }
    if let Some(t) = type_parameters {
        out.extend(t.parameters.clone());
    }
    out
}
fn convert_class_constructor(
    m: &AstClassConstructor,
    class_type_parameters: Option<&AstTypeParameters>,
) -> Method {
    let access = access_from_availability(&m.header.availability, Access::Public);
    let type_parameters =
        merge_type_parameters(class_type_parameters, m.header.type_parameters.as_ref());
    let parameters = m
        .header
        .parameters
        .parameters
        .iter()
        .map(|p| Parameter {
            name: Some((&p.name).into()),
            jtype: check_type_parameters(&p.jtype, &type_parameters),
        })
        .collect();
    let throws = m
        .header
        .throws
        .as_ref()
        .map_or_else(Vec::new, |t| t.parameters.iter().map(Into::into).collect());
    Method {
        access,
        name: None,
        parameters,
        throws,
        ret: JType::Void,
        source: None,
    }
}
fn convert_interface_method(
    m: &AstInterfaceMethod,
    interface_type_parameters: Option<&AstTypeParameters>,
) -> Method {
    let access = access_from_availability(&m.header.availability, Access::Public);
    let type_parameters =
        merge_type_parameters(interface_type_parameters, m.header.type_parameters.as_ref());
    let parameters = m
        .header
        .parameters
        .parameters
        .iter()
        .map(|p| Parameter {
            name: Some((&p.name).into()),
            jtype: check_type_parameters(&p.jtype, &type_parameters),
        })
        .collect();
    let throws = m.header.throws.as_ref().map_or_else(Vec::new, |t| {
        t.parameters
            .iter()
            .map(|i| check_type_parameters(i, &type_parameters))
            .collect()
    });
    Method {
        access,
        name: Some((&m.header.name).into()),
        parameters,
        throws,
        ret: check_type_parameters(&m.header.jtype, &type_parameters),
        source: None,
    }
}
fn convert_interface_default_method(
    m: &AstInterfaceMethodDefault,
    interface_type_parameters: Option<&AstTypeParameters>,
) -> Method {
    let access = access_from_availability(&m.header.availability, Access::Public);
    let type_parameters =
        merge_type_parameters(interface_type_parameters, m.header.type_parameters.as_ref());
    let parameters = m
        .header
        .parameters
        .parameters
        .iter()
        .map(|p| Parameter {
            name: Some((&p.name).into()),
            jtype: check_type_parameters(&p.jtype, &type_parameters),
        })
        .collect();
    let throws = m.header.throws.as_ref().map_or_else(Vec::new, |t| {
        t.parameters
            .iter()
            .map(|i| check_type_parameters(i, &type_parameters))
            .collect()
    });
    Method {
        access,
        name: Some((&m.header.name).into()),
        parameters,
        throws,
        ret: check_type_parameters(&m.header.jtype, &type_parameters),
        source: None,
    }
}

fn convert_interface_constant(c: &AstInterfaceConstant) -> Field {
    let mut access = access_from_availability(&c.availability, Access::Public);
    load_deprecated(&mut access, &c.annotated);
    Field {
        access,
        name: (&c.name).into(),
        jtype: (&c.jtype).into(),
        source: None,
    }
}
fn convert_annotation_field(c: &AstAnnotationField) -> Field {
    let mut access = access_from_availability(&c.availability, Access::Public);
    load_deprecated(&mut access, &c.annotated);
    Field {
        access,
        name: (&c.name).into(),
        jtype: (&c.jtype).into(),
        source: None,
    }
}
fn convert_class_field(c: &AstClassVariable) -> Field {
    let mut access = access_from_availability(&c.availability, Access::Public);
    load_deprecated(&mut access, &c.annotated);
    let jtype: JType = (&c.jtype).into();

    Field {
        access,
        jtype,
        name: c.name.value.clone(),
        source: None,
    }
}

fn convert_enum_variant(c: &AstEnumerationVariant, jtype: &JType) -> Field {
    Field {
        access: Access::Public,
        jtype: jtype.clone(),
        name: c.name.value.clone(),
        source: None,
    }
}

fn check_type_parameters(
    jtype: &ast::types::AstJType,
    type_parameters: &[AstTypeParameter],
) -> JType {
    let jtype: JType = jtype.into();

    if let JType::Class(ref p) = jtype
        && type_parameters.iter().any(|i| i.name.value == *p)
    {
        return JType::Parameter(p.to_owned());
    }
    if let JType::Generic(name, params) = jtype {
        let params = params
            .iter()
            .map(|i: &JType| {
                if let JType::Class(p) = i
                    && type_parameters.iter().any(|i| i.name.value == *p)
                {
                    return JType::Parameter(p.to_owned());
                }
                i.clone()
            })
            .collect();
        return JType::Generic(name, params);
    }

    jtype
}

#[cfg(test)]
pub mod tests {
    use dto::SourceDestination;

    use super::load_java;

    #[test]
    fn jtype_recognition() {
        let result = load_java(
            include_bytes!("../test/Types.java"),
            SourceDestination::None,
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn super_class() {
        let content = "
package a.test;
public class Test extends AThing { }
        ";
        let result = load_java(content.as_bytes(), SourceDestination::None);
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn generic_type_declare() {
        let content = "
package a.test;
public class Test {
  public static <T> int add(Collection<T> list, T item){}
}
        ";
        let result = load_java(content.as_bytes(), SourceDestination::None);
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn thrower() {
        let content = include_str!("../test/Thrower.java");
        let result = load_java(content.as_bytes(), SourceDestination::None);
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn interface_constants() {
        let result = load_java(
            include_bytes!("../test/Constants.java"),
            SourceDestination::None,
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn interface_base() {
        let result = load_java(
            include_bytes!("../test/InterfaceBase.java"),
            SourceDestination::None,
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn jenum() {
        let result = load_java(
            include_bytes!("../test/Variants.java"),
            SourceDestination::None,
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn jannotation() {
        let result = load_java(
            include_bytes!("../test/Annotation.java"),
            SourceDestination::None,
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
        let src = "
package a.test;

import jakarta.inject.Inject;
import jakarta.ws.rs.GET;

import jakarta.ws.rs.Path;

public class Test {
}
 ";
        let result = load_java(src.as_bytes(), SourceDestination::None);

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
