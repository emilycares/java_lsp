use std::path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR};

use crate::SourceDestination;
use crate::dto::{
    Access, Class, ClassError, Field, ImportUnit, JType, Method, Parameter, SuperClass,
};
use classfile_parser::attribute_info::{AttributeInfo, CodeAttribute};
use classfile_parser::code_attribute::LocalVariableTableAttribute;
use classfile_parser::constant_info::ConstantInfo;
use classfile_parser::field_info::{FieldAccessFlags, FieldInfo};
use classfile_parser::method_info::MethodAccessFlags;
use classfile_parser::{ClassAccessFlags, ClassFile, class_parser};
use my_string::MyString;

pub fn load_class(
    bytes: &[u8],
    class_path: MyString,
    source: SourceDestination,
) -> Result<Class, ClassError> {
    let (_, c) = class_parser(bytes).map_err(|_| ClassError::ParseError)?;
    let code_attribute = parse_code_attribute(&c, &c.attributes);
    let mut used_classes = parse_used_classes(&c, code_attribute);

    let methods: Vec<_> = c
        .methods
        .iter()
        .filter_map(|method| {
            let code_attribute = parse_code_attribute(&c, &method.attributes);
            used_classes.extend(parse_used_classes(&c, code_attribute));

            let method = parse_method(&c, method);
            if let Some(ref m) = method {
                used_classes.extend(
                    m.parameters
                        .iter()
                        .filter(|i| {
                            matches!(
                                i.jtype,
                                JType::Class(_) | JType::Array(_) | JType::Generic(_, _)
                            )
                        })
                        .flat_map(|i| jtype_class_names(i.jtype.clone())),
                );
                used_classes.extend(jtype_class_names(m.ret.clone()));
            }
            method
        })
        //.filter(|f| !f.access.contains(&Access::Private))
        .collect();
    let fields: Vec<_> = c
        .fields
        .iter()
        .filter_map(|field| {
            let field = parse_field(&c, field);

            if let Some(ref f) = field {
                used_classes.extend(jtype_class_names(f.jtype.clone()));
            }

            field
        })
        //.filter(|f| !f.access.contains(&Access::Private))
        .collect();

    let name = lookup_class_name(&c, c.this_class.into()).ok_or(ClassError::UnknownClassName)?;
    let package = class_path
        .trim_end_matches(name.as_str())
        .trim_end_matches('.');
    let mut imports = vec![ImportUnit::Package(package.into())];
    imports.extend(
        used_classes
            .into_iter()
            .filter(|i| *i != class_path)
            .map(ImportUnit::Class),
    );

    let source = match source {
        SourceDestination::RelativeInFolder(e) => format!(
            "{}{}{}.java",
            e,
            MAIN_SEPARATOR,
            &class_path.replace('.', MAIN_SEPARATOR_STR)
        ),
        SourceDestination::Here(e) => e,
        SourceDestination::None => String::new(),
    };
    let super_interfaces: Vec<_> = c
        .interfaces
        .iter()
        .map(|index| {
            lookup_class_name(&c, *index as usize).map_or(SuperClass::None, SuperClass::Name)
        })
        .collect();
    let deprecated = c.attributes.iter().any(|attribute_info| {
        let Some(lookup_string) = lookup_string(&c, attribute_info.attribute_name_index) else {
            return false;
        };
        matches!(lookup_string.as_str(), "Deprecated")
    });

    Ok(Class {
        source,
        class_path,
        super_interfaces,
        super_class: match lookup_class_name(&c, c.super_class.into()) {
            Some(c) if c == "Object" => SuperClass::None,
            Some(c) => SuperClass::Name(c),
            None => SuperClass::None,
        },
        imports,
        access: parse_class_access(c.access_flags, deprecated),
        name,
        methods,
        fields,
    })
}

fn lookup_class_name(c: &ClassFile, index: usize) -> Option<MyString> {
    match c.const_pool.get(index.saturating_sub(1)) {
        Some(ConstantInfo::Class(class)) => lookup_string(c, class.name_index)
            .expect("Class to have name")
            .split('/')
            .next_back()
            .map(Into::into),
        _ => None,
    }
}

fn parse_field(c: &ClassFile, field: &FieldInfo) -> Option<Field> {
    Some(Field {
        access: parse_field_access(field),
        name: lookup_string(c, field.name_index)?,
        jtype: parse_field_descriptor(&lookup_string(c, field.descriptor_index)?),
        source: None,
    })
}

fn parse_method(
    c: &ClassFile,
    method: &classfile_parser::method_info::MethodInfo,
) -> Option<Method> {
    let lname = lookup_string(c, method.name_index)?;

    if lname.starts_with("lambda$") {
        return None;
    }
    let (params, ret) = parse_method_descriptor(&lookup_string(c, method.descriptor_index)?);

    let mut params = params.into_iter();
    let mut parameters: Vec<Parameter> = method
        .attributes
        .iter()
        .filter_map(|attribute_info| {
            match lookup_string(c, attribute_info.attribute_name_index)?.as_str() {
                "MethodParameters" => {
                    classfile_parser::attribute_info::method_parameters_attribute_parser(
                        &attribute_info.info,
                    )
                    .ok()
                }
                _ => None,
            }
        })
        .flat_map(|method_parameters| {
            method_parameters
                .1
                .parameters
                .into_iter()
                .filter_map(|pa| {
                    Some(Parameter {
                        name: lookup_string(c, pa.name_index).and_then(|s| {
                            if s.is_empty() {
                                return None;
                            }
                            Some(s)
                        }),
                        jtype: params.next()?,
                    })
                })
                .collect::<Vec<Parameter>>()
        })
        .collect();
    // Remaining method descriptor data as params
    {
        for jtype in params {
            parameters.push(Parameter { name: None, jtype });
        }
    }
    let throws: Vec<JType> = method
        .attributes
        .iter()
        .filter_map(|attribute_info| {
            match lookup_string(c, attribute_info.attribute_name_index)?.as_str() {
                "Exceptions" => classfile_parser::attribute_info::exceptions_attribute_parser(
                    &attribute_info.info,
                )
                .ok(),
                _ => None,
            }
        })
        .flat_map(|i| {
            let exception_table = i.1.exception_table;
            exception_table
                .into_iter()
                .filter_map(|ex| c.const_pool.get((ex - 1) as usize))
                .filter_map(|ex_class| match ex_class {
                    ConstantInfo::Class(ex_class) => lookup_string(c, ex_class.name_index),
                    _ => None,
                })
                .filter_map(|name| {
                    if let Some((_, name)) = name.rsplit_once('/') {
                        return Some(name.into());
                    }
                    None
                })
                .map(JType::Class)
        })
        .collect();
    let deprecated = method.attributes.iter().any(|attribute_info| {
        let Some(lookup_string) = lookup_string(c, attribute_info.attribute_name_index) else {
            return false;
        };
        matches!(lookup_string.as_str(), "Deprecated")
    });
    let name = if lname == "<init>" { None } else { Some(lname) };
    Some(Method {
        access: parse_method_access(method, deprecated),
        name,
        parameters,
        ret,
        throws,
        source: None,
    })
}

fn parse_used_classes(c: &ClassFile, code_attribute: Option<CodeAttribute>) -> Vec<MyString> {
    if let Some(code_attribute) = code_attribute {
        let local_variable_table_attributes: Vec<LocalVariableTableAttribute> = code_attribute
            .attributes
            .iter()
            .filter_map(|attribute_info| {
                match lookup_string(c, attribute_info.attribute_name_index)?.as_str() {
                    "LocalVariableTable" => {
                        classfile_parser::code_attribute::local_variable_table_parser(
                            &attribute_info.info,
                        )
                        .ok()
                    }
                    _ => None,
                }
            })
            .map(|a| a.1)
            .collect();
        let types: Vec<JType> = local_variable_table_attributes
            .iter()
            .flat_map(|i| &i.items)
            .filter_map(|i| lookup_string(c, i.descriptor_index))
            .map(|i| parse_field_descriptor(&i))
            .collect();
        return types
            .iter()
            .flat_map(|i| jtype_class_names(i.clone()))
            .collect();
    }
    vec![]
}

fn jtype_class_names(i: JType) -> Vec<MyString> {
    match i {
        JType::Class(class) => vec![class],
        JType::Array(jtype) => jtype_class_names(*jtype),
        JType::Generic(_class, jtypes) => jtypes.into_iter().flat_map(jtype_class_names).collect(),
        _ => vec![],
    }
}

fn parse_code_attribute(c: &ClassFile, attributes: &[AttributeInfo]) -> Option<CodeAttribute> {
    attributes
        .iter()
        .find_map(|attribute_info| {
            match lookup_string(c, attribute_info.attribute_name_index)?.as_str() {
                "Code" => {
                    classfile_parser::attribute_info::code_attribute_parser(&attribute_info.info)
                        .ok()
                }
                _ => None,
            }
        })
        .map(|i| i.1)
}

fn parse_class_access(flags: ClassAccessFlags, deprecated: bool) -> Access {
    let mut access = Access::empty();
    if deprecated {
        access.insert(Access::Deprecated);
    }
    if flags == ClassAccessFlags::PUBLIC {
        access.insert(Access::Public);
    }
    if flags == ClassAccessFlags::FINAL {
        access.insert(Access::Final);
    }
    if flags == ClassAccessFlags::SUPER {
        access.insert(Access::Super);
    }
    if flags == ClassAccessFlags::INTERFACE {
        access.insert(Access::Interface);
    }
    if flags == ClassAccessFlags::SYNTHETIC {
        access.insert(Access::Synthetic);
    }
    if flags == ClassAccessFlags::ANNOTATION {
        access.insert(Access::Annotation);
    }
    if flags == ClassAccessFlags::ENUM {
        access.insert(Access::Enum);
    }
    access
}

fn parse_method_access(
    method: &classfile_parser::method_info::MethodInfo,
    deprecated: bool,
) -> Access {
    let mut access = Access::empty();
    if deprecated {
        access.insert(Access::Deprecated);
    }
    if method.access_flags == MethodAccessFlags::PUBLIC {
        access.insert(Access::Public);
    }
    if method.access_flags == MethodAccessFlags::PRIVATE {
        access.insert(Access::Private);
    }
    if method.access_flags == MethodAccessFlags::PROTECTED {
        access.insert(Access::Protected);
    }
    if method.access_flags == MethodAccessFlags::STATIC {
        access.insert(Access::Static);
    }
    if method.access_flags == MethodAccessFlags::FINAL {
        access.insert(Access::Final);
    }
    if method.access_flags == MethodAccessFlags::ABSTRACT {
        access.insert(Access::Abstract);
    }
    if method.access_flags == MethodAccessFlags::SYNTHETIC {
        access.insert(Access::Synthetic);
    }
    access
}

fn parse_field_access(method: &FieldInfo) -> Access {
    let mut access = Access::empty();
    if method.access_flags == FieldAccessFlags::PUBLIC {
        access.insert(Access::Public);
    }
    if method.access_flags == FieldAccessFlags::PRIVATE {
        access.insert(Access::Private);
    }
    if method.access_flags == FieldAccessFlags::PROTECTED {
        access.insert(Access::Protected);
    }
    if method.access_flags == FieldAccessFlags::STATIC {
        access.insert(Access::Static);
    }
    if method.access_flags == FieldAccessFlags::FINAL {
        access.insert(Access::Final);
    }
    if method.access_flags == FieldAccessFlags::SYNTHETIC {
        access.insert(Access::Synthetic);
    }
    access
}

fn lookup_string(c: &ClassFile, index: u16) -> Option<MyString> {
    if index == 0 {
        return None;
    }
    let con = &c.const_pool[(index - 1) as usize];
    match con {
        ConstantInfo::Utf8(utf8) => Some(utf8.utf8_string.to_string()),
        ConstantInfo::Module(m) => lookup_string(c, m.name_index),
        ConstantInfo::Package(p) => lookup_string(c, p.name_index),
        _ => None,
    }
}

fn parse_method_descriptor(descriptor: &str) -> (Vec<JType>, JType) {
    let mut param_types = Vec::new();
    let mut chars = descriptor.chars();
    let current = chars.next();
    match current {
        Some('(') => {
            while let Some(c) = chars.next() {
                if c == ')' {
                    break;
                }
                param_types.push(parse_field_type(Some(c), &mut chars));
            }

            let return_type = parse_field_type(chars.next(), &mut chars);
            (param_types, return_type)
        }
        Some(_) => {
            let return_type = parse_field_type(current, &mut chars);
            (param_types, return_type)
        }
        _ => (vec![], JType::Void),
    }
}
fn parse_field_descriptor(descriptor: &str) -> JType {
    let mut chars = descriptor.chars();
    let current = chars.next();
    parse_field_type(current, &mut chars)
}

fn parse_field_type(c: Option<char>, chars: &mut std::str::Chars) -> JType {
    let Some(c) = c else {
        return JType::Void;
    };
    match c {
        'B' => JType::Byte,
        'C' => JType::Char,
        'D' => JType::Double,
        'F' => JType::Float,
        'I' => JType::Int,
        'J' => JType::Long,
        'S' => JType::Short,
        'Z' => JType::Boolean,
        'V' => JType::Void,
        'L' => {
            let mut class_name = String::new();
            for ch in chars.by_ref() {
                if ch == ';' {
                    break;
                }
                class_name.push(ch);
            }
            JType::Class(class_name.replace('/', "."))
        }
        '[' => JType::Array(Box::new(parse_field_type(chars.next(), chars))),
        _ => {
            //panic!("Unknown type: {}", c);
            JType::Void
        }
    }
}
#[derive(Debug)]
pub struct ModuleInfo {
    pub exports: Vec<MyString>,
}

pub fn load_module(bytes: &[u8]) -> Result<ModuleInfo, ClassError> {
    let (_, c) = class_parser(bytes).map_err(|_| ClassError::ParseError)?;
    for a in &c.attributes {
        if let Some(name) = lookup_string(&c, a.attribute_name_index)
            && name == "Module"
            && let Ok((_, module)) =
                classfile_parser::attribute_info::module_attribute_parser(&a.info)
        {
            let mut exports = Vec::new();
            for e in module.exports {
                if !e.exports_to_index.is_empty() {
                    continue;
                }
                let get = c.const_pool.get(e.exports_index as usize);
                if let Some(ConstantInfo::Utf8(p)) = get {
                    let package = p.utf8_string.to_string();
                    exports.push(package);
                }
            }
            return Ok(ModuleInfo { exports });
        }
    }
    Err(ClassError::NoModuleAttribute)
}

#[cfg(test)]
mod tests {
    use crate::{
        SourceDestination,
        class::{load_class, load_module},
    };

    #[cfg(not(windows))]
    #[test]
    fn relative_source() {
        let result = load_class(
            include_bytes!("../test/Everything.class"),
            "ch.emilycares.Everything".into(),
            SourceDestination::RelativeInFolder("/source".into()),
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn everything() {
        let result = load_class(
            include_bytes!("../test/Everything.class"),
            "ch.emilycares.Everything".into(),
            SourceDestination::None,
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }
    #[test]
    fn super_base() {
        let result = load_class(
            include_bytes!("../test/Super.class"),
            "ch.emilycares.Super".into(),
            SourceDestination::None,
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }
    #[test]
    fn thrower() {
        let result = load_class(
            include_bytes!("../test/Thrower.class"),
            "ch.emilycares.Thrower".into(),
            SourceDestination::Here("/path/to/source/Thrower.java".into()),
        );
        insta::assert_debug_snapshot!(result.unwrap());
    }
    #[test]
    fn super_interfaces() {
        let result = load_class(
            include_bytes!("../test/SuperInterface.class"),
            "ch.emilycares.SuperInterface".into(),
            SourceDestination::None,
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }
    #[test]
    fn variables() {
        let result = load_class(
            include_bytes!("../test/LocalVariableTable.class"),
            "ch.emilycares.LocalVariableTable".into(),
            SourceDestination::None,
        );

        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[test]
    fn module_java_desktop() {
        let result = load_module(include_bytes!("../test/module-info.class"));
        insta::assert_debug_snapshot!(result.unwrap());
    }
}
