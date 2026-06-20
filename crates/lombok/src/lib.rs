#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::enum_glob_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
use ast::types::{
    AstAvailability, AstBlock, AstClass, AstClassConstructor, AstClassMethod, AstClassVariable,
    AstConstructorHeader, AstFile, AstIdentifier, AstImportUnit, AstJType, AstJTypeKind,
    AstMethodHeader, AstMethodParameter, AstMethodParameterFlags, AstMethodParameters, AstRange,
    AstThing, AstTopLevel,
};
use bitflags::bitflags;
use my_string::smol_str::{SmolStr, format_smolstr};

bitflags! {
   #[derive(Clone, Eq, PartialEq, Debug, Default)]
   pub struct Features: u8 {
     const AllArgsConstructor       = 0b0000_0001;
     const NoArgsConstructor        = 0b0000_0010;
     const Getter                   = 0b0000_0100;
     const Setter                   = 0b0000_1000;
     const ToString                 = 0b0001_0000;
     const EqualsAndHashCode        = 0b0010_0000;
     const Value                    = 0b0100_0000;
   }
}

#[must_use]
pub fn preprocessor(ast: AstFile) -> AstFile {
    let mut features = Features::empty();

    for t in &ast.top {
        if let AstTopLevel::Import(im) = t {
            match &im.unit {
                AstImportUnit::Prefix(p) if p.value == "lombok" => {
                    features = Features::all();
                }
                AstImportUnit::Class(c) if c.value == "lombok.Data" => {
                    features |= Features::Getter
                        | Features::Setter
                        | Features::NoArgsConstructor
                        | Features::ToString
                        | Features::EqualsAndHashCode
                        | Features::Value;
                }
                AstImportUnit::Class(c) if c.value == "lombok.AllArgsConstructor" => {
                    features |= Features::AllArgsConstructor;
                }
                AstImportUnit::Class(c) if c.value == "lombok.NoArgsConstructor" => {
                    features |= Features::NoArgsConstructor;
                }
                AstImportUnit::Class(c) if c.value == "lombok.Getter" => {
                    features |= Features::Getter;
                }
                AstImportUnit::Class(c) if c.value == "lombok.Setter" => {
                    features |= Features::Setter;
                }
                AstImportUnit::Class(c) if c.value == "lombok.ToString" => {
                    features |= Features::ToString;
                }
                AstImportUnit::Class(c) if c.value == "lombok.EqualsAndHashCode" => {
                    features |= Features::EqualsAndHashCode;
                }
                AstImportUnit::Class(c) if c.value == "lombok.Value" => {
                    features |= Features::Value;
                }
                _ => (),
            }
        }
    }

    if features.is_empty() {
        return ast;
    }
    let mut ast = ast;

    for t in &mut ast.top {
        if let AstTopLevel::Thing(ast_thing) = t {
            thing(ast_thing, &features);
        }
    }

    ast
}

fn thing(ast_thing: &mut AstThing, features: &Features) {
    match ast_thing {
        AstThing::Class(ast_class) => {
            for an in &ast_class.annotated {
                if an.name.value.as_str() == "AllArgsConstructor"
                    && features.intersects(Features::AllArgsConstructor)
                {
                    let value = all_args_constructor(ast_class);
                    ast_class.block.constructors.push(value);
                }

                if an.name.value.as_str() == "NoArgsConstructor"
                    && features.intersects(Features::NoArgsConstructor)
                {
                    let value = no_args_constructor(ast_class);
                    ast_class.block.constructors.push(value);
                }

                if an.name.value.as_str() == "Getter" && features.intersects(Features::Getter) {
                    for f in &ast_class.block.variables {
                        ast_class.block.methods.push(getter_class_variable(f));
                    }
                }

                if an.name.value.as_str() == "Setter" && features.intersects(Features::Setter) {
                    for f in &ast_class.block.variables {
                        ast_class.block.methods.push(getter_class_variable(f));
                    }
                }

                if an.name.value.as_str() == "ToString" && features.intersects(Features::ToString) {
                    ast_class.block.methods.push(to_string_class());
                }

                if an.name.value.as_str() == "EqualsAndHashCode"
                    && features.intersects(Features::EqualsAndHashCode)
                {
                    ast_class.block.methods.push(equals_class());
                    ast_class.block.methods.push(hashcode_class());
                }

                if an.name.value.as_str() == "Data"
                    && features.contains(
                        Features::Getter
                            | Features::Setter
                            | Features::NoArgsConstructor
                            | Features::ToString
                            | Features::EqualsAndHashCode
                            | Features::Value,
                    )
                {
                    let value = no_args_constructor(ast_class);
                    ast_class.block.constructors.push(value);
                    for f in &ast_class.block.variables {
                        ast_class.block.methods.push(getter_class_variable(f));
                        ast_class.block.methods.push(setter_class_variable(f));
                    }

                    ast_class.block.methods.push(equals_class());
                    ast_class.block.methods.push(hashcode_class());
                    ast_class.block.methods.push(to_string_class());
                }

                if features.intersects(Features::Getter) {
                    for f in &ast_class.block.variables {
                        for an in &f.annotated {
                            if an.name.value.as_str() == "Getter" {
                                ast_class.block.methods.push(getter_class_variable(f));
                            }
                        }
                    }
                }

                if features.intersects(Features::Setter) {
                    for f in &ast_class.block.variables {
                        for an in &f.annotated {
                            if an.name.value.as_str() == "Setter" {
                                ast_class.block.methods.push(setter_class_variable(f));
                            }
                        }
                    }
                }
            }
        }
        AstThing::Interface(_)
        | AstThing::Enumeration(_)
        | AstThing::Record(_)
        | AstThing::Annotation(_) => (),
    }
}

fn hashcode_class() -> AstClassMethod {
    AstClassMethod {
        range: AstRange::default(),
        header: AstMethodHeader {
            range: AstRange::default(),
            availability: AstAvailability::empty(),
            name: AstIdentifier {
                range: AstRange::default(),
                value: SmolStr::new_inline("hashCode"),
            },
            jtype: AstJType {
                annotated: Vec::new(),
                range: AstRange::default(),
                value: AstJTypeKind::Int,
            },
            parameters: AstMethodParameters {
                range: AstRange::default(),
                parameters: Vec::new(),
            },
            throws: None,
            type_parameters: None,
            annotated: Vec::new(),
        },
        block: None,
    }
}

fn equals_class() -> AstClassMethod {
    AstClassMethod {
        range: AstRange::default(),
        header: AstMethodHeader {
            range: AstRange::default(),
            availability: AstAvailability::empty(),
            name: AstIdentifier {
                range: AstRange::default(),
                value: SmolStr::new_inline("equals"),
            },
            jtype: AstJType {
                annotated: Vec::new(),
                range: AstRange::default(),
                value: AstJTypeKind::Boolean,
            },
            parameters: AstMethodParameters {
                range: AstRange::default(),
                parameters: vec![AstMethodParameter {
                    range: AstRange::default(),
                    annotated: Vec::new(),
                    jtype: AstJType {
                        annotated: Vec::new(),
                        range: AstRange::default(),
                        value: AstJTypeKind::Class(AstIdentifier {
                            range: AstRange::default(),
                            value: SmolStr::new_inline("java.lang.Object"),
                        }),
                    },
                    name: AstIdentifier {
                        range: AstRange::default(),
                        value: SmolStr::new_inline("other"),
                    },
                    flags: AstMethodParameterFlags::empty(),
                }],
            },
            throws: None,
            type_parameters: None,
            annotated: Vec::new(),
        },
        block: None,
    }
}

fn to_string_class() -> AstClassMethod {
    AstClassMethod {
        range: AstRange::default(),
        header: AstMethodHeader {
            range: AstRange::default(),
            availability: AstAvailability::empty(),
            name: AstIdentifier {
                range: AstRange::default(),
                value: SmolStr::new_inline("toString"),
            },
            jtype: AstJType {
                annotated: Vec::new(),
                range: AstRange::default(),
                value: AstJTypeKind::Class(AstIdentifier {
                    range: AstRange::default(),
                    value: SmolStr::new_inline("java.lang.String"),
                }),
            },
            parameters: AstMethodParameters {
                range: AstRange::default(),
                parameters: Vec::new(),
            },
            throws: None,
            type_parameters: None,
            annotated: Vec::new(),
        },
        block: None,
    }
}

fn getter_class_variable(f: &AstClassVariable) -> AstClassMethod {
    let name = my_string::capitalize_first(&f.name.value);
    AstClassMethod {
        range: AstRange::default(),
        header: AstMethodHeader {
            range: AstRange::default(),
            availability: AstAvailability::empty(),
            name: AstIdentifier {
                range: AstRange::default(),
                value: format_smolstr!("get{}", name),
            },
            jtype: f.jtype.clone(),
            parameters: AstMethodParameters {
                range: AstRange::default(),
                parameters: Vec::new(),
            },
            throws: None,
            type_parameters: None,
            annotated: Vec::new(),
        },
        block: None,
    }
}

fn setter_class_variable(f: &AstClassVariable) -> AstClassMethod {
    let name = my_string::capitalize_first(&f.name.value);
    AstClassMethod {
        range: AstRange::default(),
        header: AstMethodHeader {
            range: AstRange::default(),
            availability: AstAvailability::empty(),
            name: AstIdentifier {
                range: AstRange::default(),
                value: format_smolstr!("set{}", name),
            },
            jtype: AstJType {
                annotated: Vec::new(),
                range: AstRange::default(),
                value: AstJTypeKind::Void,
            },
            parameters: AstMethodParameters {
                range: AstRange::default(),
                parameters: vec![AstMethodParameter {
                    range: AstRange::default(),
                    annotated: Vec::new(),
                    jtype: f.jtype.clone(),
                    name: f.name.clone(),
                    flags: AstMethodParameterFlags::empty(),
                }],
            },
            throws: None,
            type_parameters: None,
            annotated: Vec::new(),
        },
        block: None,
    }
}

fn all_args_constructor(ast_class: &AstClass) -> AstClassConstructor {
    let mut parameters = Vec::new();
    for f in &ast_class.block.variables {
        parameters.push(AstMethodParameter {
            range: AstRange::default(),
            annotated: Vec::new(),
            jtype: f.jtype.clone(),
            name: f.name.clone(),
            flags: AstMethodParameterFlags::empty(),
        });
    }

    AstClassConstructor {
        range: AstRange::default(),
        header: AstConstructorHeader {
            range: AstRange::default(),
            availability: AstAvailability::empty(),
            name: ast_class.name.clone(),
            parameters: AstMethodParameters {
                range: AstRange::default(),
                parameters,
            },
            throws: None,
            type_parameters: None,
            annotated: Vec::new(),
        },
        block: AstBlock {
            range: AstRange::default(),
            entries: Vec::new(),
        },
    }
}

fn no_args_constructor(ast_class: &AstClass) -> AstClassConstructor {
    AstClassConstructor {
        range: AstRange::default(),
        header: AstConstructorHeader {
            range: AstRange::default(),
            availability: AstAvailability::empty(),
            name: ast_class.name.clone(),
            parameters: AstMethodParameters {
                range: AstRange::default(),
                parameters: Vec::new(),
            },
            throws: None,
            type_parameters: None,
            annotated: Vec::new(),
        },
        block: AstBlock {
            range: AstRange::default(),
            entries: Vec::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;

    #[test]
    fn data() {
        let content = br"package com.mycompany.app.dto;
import lombok.Data;
@Data
public class MyData {
	private String project;
}";
        let tokens = ast::lexer::lex(content).unwrap();
        let file = ast::parse_file(&tokens).unwrap();
        let out = preprocessor(file);
        let expected = expect![[r#"
            AstFile {
                top: [
                    Package(
                        AstPackage {
                            range: AstRange {
                                start: AstPoint { 0:0 },
                                end: AstPoint { 0:30 },
                            },
                            annotated: [],
                            name: AstIdentifier {
                                range: AstRange {
                                    start: AstPoint { 0:8 },
                                    end: AstPoint { 0:29 },
                                },
                                value: "com.mycompany.app.dto",
                            },
                        },
                    ),
                    Import(
                        AstImport {
                            range: AstRange {
                                start: AstPoint { 1:0 },
                                end: AstPoint { 1:19 },
                            },
                            unit: Class(
                                AstIdentifier {
                                    range: AstRange {
                                        start: AstPoint { 1:7 },
                                        end: AstPoint { 1:18 },
                                    },
                                    value: "lombok.Data",
                                },
                            ),
                        },
                    ),
                    Thing(
                        Class(
                            AstClass {
                                range: AstRange {
                                    start: AstPoint { 2:0 },
                                    end: AstPoint { 5:1 },
                                },
                                availability: AstAvailability(
                                    Public,
                                ),
                                attributes: AstThingAttributes(
                                    0x0,
                                ),
                                annotated: [
                                    AstAnnotated {
                                        range: AstRange {
                                            start: AstPoint { 2:0 },
                                            end: AstPoint { 2:5 },
                                        },
                                        name: AstIdentifier {
                                            range: AstRange {
                                                start: AstPoint { 2:1 },
                                                end: AstPoint { 2:5 },
                                            },
                                            value: "Data",
                                        },
                                        parameters: None,
                                    },
                                ],
                                name: AstIdentifier {
                                    range: AstRange {
                                        start: AstPoint { 3:13 },
                                        end: AstPoint { 3:19 },
                                    },
                                    value: "MyData",
                                },
                                type_parameters: None,
                                superclass: [],
                                implements: [],
                                permits: [],
                                block: AstClassBlock {
                                    variables: [
                                        AstClassVariable {
                                            range: AstRange {
                                                start: AstPoint { 4:1 },
                                                end: AstPoint { 4:23 },
                                            },
                                            availability: AstAvailability(
                                                Private,
                                            ),
                                            annotated: [],
                                            name: AstIdentifier {
                                                range: AstRange {
                                                    start: AstPoint { 4:16 },
                                                    end: AstPoint { 4:23 },
                                                },
                                                value: "project",
                                            },
                                            jtype: AstJType {
                                                annotated: [],
                                                range: AstRange {
                                                    start: AstPoint { 4:9 },
                                                    end: AstPoint { 4:15 },
                                                },
                                                value: Class(
                                                    AstIdentifier {
                                                        range: AstRange {
                                                            start: AstPoint { 4:9 },
                                                            end: AstPoint { 4:15 },
                                                        },
                                                        value: "String",
                                                    },
                                                ),
                                            },
                                            expression: None,
                                            volatile_transient: AstVolatileTransient(
                                                0x0,
                                            ),
                                        },
                                    ],
                                    methods: [
                                        AstClassMethod {
                                            range: AstRange {
                                                start: AstPoint { 0:0 },
                                                end: AstPoint { 0:0 },
                                            },
                                            header: AstMethodHeader {
                                                range: AstRange {
                                                    start: AstPoint { 0:0 },
                                                    end: AstPoint { 0:0 },
                                                },
                                                availability: AstAvailability(
                                                    0x0,
                                                ),
                                                name: AstIdentifier {
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    value: "getProject",
                                                },
                                                jtype: AstJType {
                                                    annotated: [],
                                                    range: AstRange {
                                                        start: AstPoint { 4:9 },
                                                        end: AstPoint { 4:15 },
                                                    },
                                                    value: Class(
                                                        AstIdentifier {
                                                            range: AstRange {
                                                                start: AstPoint { 4:9 },
                                                                end: AstPoint { 4:15 },
                                                            },
                                                            value: "String",
                                                        },
                                                    ),
                                                },
                                                parameters: AstMethodParameters {
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    parameters: [],
                                                },
                                                throws: None,
                                                type_parameters: None,
                                                annotated: [],
                                            },
                                            block: None,
                                        },
                                        AstClassMethod {
                                            range: AstRange {
                                                start: AstPoint { 0:0 },
                                                end: AstPoint { 0:0 },
                                            },
                                            header: AstMethodHeader {
                                                range: AstRange {
                                                    start: AstPoint { 0:0 },
                                                    end: AstPoint { 0:0 },
                                                },
                                                availability: AstAvailability(
                                                    0x0,
                                                ),
                                                name: AstIdentifier {
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    value: "setProject",
                                                },
                                                jtype: AstJType {
                                                    annotated: [],
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    value: Void,
                                                },
                                                parameters: AstMethodParameters {
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    parameters: [
                                                        AstMethodParameter {
                                                            range: AstRange {
                                                                start: AstPoint { 0:0 },
                                                                end: AstPoint { 0:0 },
                                                            },
                                                            annotated: [],
                                                            jtype: AstJType {
                                                                annotated: [],
                                                                range: AstRange {
                                                                    start: AstPoint { 4:9 },
                                                                    end: AstPoint { 4:15 },
                                                                },
                                                                value: Class(
                                                                    AstIdentifier {
                                                                        range: AstRange {
                                                                            start: AstPoint { 4:9 },
                                                                            end: AstPoint { 4:15 },
                                                                        },
                                                                        value: "String",
                                                                    },
                                                                ),
                                                            },
                                                            name: AstIdentifier {
                                                                range: AstRange {
                                                                    start: AstPoint { 4:16 },
                                                                    end: AstPoint { 4:23 },
                                                                },
                                                                value: "project",
                                                            },
                                                            flags: AstMethodParameterFlags(
                                                                0x0,
                                                            ),
                                                        },
                                                    ],
                                                },
                                                throws: None,
                                                type_parameters: None,
                                                annotated: [],
                                            },
                                            block: None,
                                        },
                                        AstClassMethod {
                                            range: AstRange {
                                                start: AstPoint { 0:0 },
                                                end: AstPoint { 0:0 },
                                            },
                                            header: AstMethodHeader {
                                                range: AstRange {
                                                    start: AstPoint { 0:0 },
                                                    end: AstPoint { 0:0 },
                                                },
                                                availability: AstAvailability(
                                                    0x0,
                                                ),
                                                name: AstIdentifier {
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    value: "equals",
                                                },
                                                jtype: AstJType {
                                                    annotated: [],
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    value: Boolean,
                                                },
                                                parameters: AstMethodParameters {
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    parameters: [
                                                        AstMethodParameter {
                                                            range: AstRange {
                                                                start: AstPoint { 0:0 },
                                                                end: AstPoint { 0:0 },
                                                            },
                                                            annotated: [],
                                                            jtype: AstJType {
                                                                annotated: [],
                                                                range: AstRange {
                                                                    start: AstPoint { 0:0 },
                                                                    end: AstPoint { 0:0 },
                                                                },
                                                                value: Class(
                                                                    AstIdentifier {
                                                                        range: AstRange {
                                                                            start: AstPoint { 0:0 },
                                                                            end: AstPoint { 0:0 },
                                                                        },
                                                                        value: "java.lang.Object",
                                                                    },
                                                                ),
                                                            },
                                                            name: AstIdentifier {
                                                                range: AstRange {
                                                                    start: AstPoint { 0:0 },
                                                                    end: AstPoint { 0:0 },
                                                                },
                                                                value: "other",
                                                            },
                                                            flags: AstMethodParameterFlags(
                                                                0x0,
                                                            ),
                                                        },
                                                    ],
                                                },
                                                throws: None,
                                                type_parameters: None,
                                                annotated: [],
                                            },
                                            block: None,
                                        },
                                        AstClassMethod {
                                            range: AstRange {
                                                start: AstPoint { 0:0 },
                                                end: AstPoint { 0:0 },
                                            },
                                            header: AstMethodHeader {
                                                range: AstRange {
                                                    start: AstPoint { 0:0 },
                                                    end: AstPoint { 0:0 },
                                                },
                                                availability: AstAvailability(
                                                    0x0,
                                                ),
                                                name: AstIdentifier {
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    value: "hashCode",
                                                },
                                                jtype: AstJType {
                                                    annotated: [],
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    value: Int,
                                                },
                                                parameters: AstMethodParameters {
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    parameters: [],
                                                },
                                                throws: None,
                                                type_parameters: None,
                                                annotated: [],
                                            },
                                            block: None,
                                        },
                                        AstClassMethod {
                                            range: AstRange {
                                                start: AstPoint { 0:0 },
                                                end: AstPoint { 0:0 },
                                            },
                                            header: AstMethodHeader {
                                                range: AstRange {
                                                    start: AstPoint { 0:0 },
                                                    end: AstPoint { 0:0 },
                                                },
                                                availability: AstAvailability(
                                                    0x0,
                                                ),
                                                name: AstIdentifier {
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    value: "toString",
                                                },
                                                jtype: AstJType {
                                                    annotated: [],
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    value: Class(
                                                        AstIdentifier {
                                                            range: AstRange {
                                                                start: AstPoint { 0:0 },
                                                                end: AstPoint { 0:0 },
                                                            },
                                                            value: "java.lang.String",
                                                        },
                                                    ),
                                                },
                                                parameters: AstMethodParameters {
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    parameters: [],
                                                },
                                                throws: None,
                                                type_parameters: None,
                                                annotated: [],
                                            },
                                            block: None,
                                        },
                                    ],
                                    constructors: [
                                        AstClassConstructor {
                                            range: AstRange {
                                                start: AstPoint { 0:0 },
                                                end: AstPoint { 0:0 },
                                            },
                                            header: AstConstructorHeader {
                                                range: AstRange {
                                                    start: AstPoint { 0:0 },
                                                    end: AstPoint { 0:0 },
                                                },
                                                availability: AstAvailability(
                                                    0x0,
                                                ),
                                                name: AstIdentifier {
                                                    range: AstRange {
                                                        start: AstPoint { 3:13 },
                                                        end: AstPoint { 3:19 },
                                                    },
                                                    value: "MyData",
                                                },
                                                parameters: AstMethodParameters {
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    parameters: [],
                                                },
                                                throws: None,
                                                type_parameters: None,
                                                annotated: [],
                                            },
                                            block: AstBlock {
                                                range: AstRange {
                                                    start: AstPoint { 0:0 },
                                                    end: AstPoint { 0:0 },
                                                },
                                                entries: [],
                                            },
                                        },
                                    ],
                                    static_blocks: [],
                                    inner: [],
                                    blocks: [],
                                },
                            },
                        ),
                    ),
                ],
            }
        "#]];
        expected.assert_debug_eq(&out);
    }

    #[test]
    fn constructor() {
        let content = br"package com.mycompany.app.dto;
import lombok.AllArgsConstructor;


@AllArgsConstructor
public class AllArgs {
	private String project;
}";
        let tokens = ast::lexer::lex(content).unwrap();
        let file = ast::parse_file(&tokens).unwrap();
        let out = preprocessor(file);
        let expected = expect![[r#"
            AstFile {
                top: [
                    Package(
                        AstPackage {
                            range: AstRange {
                                start: AstPoint { 0:0 },
                                end: AstPoint { 0:30 },
                            },
                            annotated: [],
                            name: AstIdentifier {
                                range: AstRange {
                                    start: AstPoint { 0:8 },
                                    end: AstPoint { 0:29 },
                                },
                                value: "com.mycompany.app.dto",
                            },
                        },
                    ),
                    Import(
                        AstImport {
                            range: AstRange {
                                start: AstPoint { 1:0 },
                                end: AstPoint { 1:33 },
                            },
                            unit: Class(
                                AstIdentifier {
                                    range: AstRange {
                                        start: AstPoint { 1:7 },
                                        end: AstPoint { 1:32 },
                                    },
                                    value: "lombok.AllArgsConstructor",
                                },
                            ),
                        },
                    ),
                    Thing(
                        Class(
                            AstClass {
                                range: AstRange {
                                    start: AstPoint { 4:0 },
                                    end: AstPoint { 7:1 },
                                },
                                availability: AstAvailability(
                                    Public,
                                ),
                                attributes: AstThingAttributes(
                                    0x0,
                                ),
                                annotated: [
                                    AstAnnotated {
                                        range: AstRange {
                                            start: AstPoint { 4:0 },
                                            end: AstPoint { 4:19 },
                                        },
                                        name: AstIdentifier {
                                            range: AstRange {
                                                start: AstPoint { 4:1 },
                                                end: AstPoint { 4:19 },
                                            },
                                            value: "AllArgsConstructor",
                                        },
                                        parameters: None,
                                    },
                                ],
                                name: AstIdentifier {
                                    range: AstRange {
                                        start: AstPoint { 5:13 },
                                        end: AstPoint { 5:20 },
                                    },
                                    value: "AllArgs",
                                },
                                type_parameters: None,
                                superclass: [],
                                implements: [],
                                permits: [],
                                block: AstClassBlock {
                                    variables: [
                                        AstClassVariable {
                                            range: AstRange {
                                                start: AstPoint { 6:1 },
                                                end: AstPoint { 6:23 },
                                            },
                                            availability: AstAvailability(
                                                Private,
                                            ),
                                            annotated: [],
                                            name: AstIdentifier {
                                                range: AstRange {
                                                    start: AstPoint { 6:16 },
                                                    end: AstPoint { 6:23 },
                                                },
                                                value: "project",
                                            },
                                            jtype: AstJType {
                                                annotated: [],
                                                range: AstRange {
                                                    start: AstPoint { 6:9 },
                                                    end: AstPoint { 6:15 },
                                                },
                                                value: Class(
                                                    AstIdentifier {
                                                        range: AstRange {
                                                            start: AstPoint { 6:9 },
                                                            end: AstPoint { 6:15 },
                                                        },
                                                        value: "String",
                                                    },
                                                ),
                                            },
                                            expression: None,
                                            volatile_transient: AstVolatileTransient(
                                                0x0,
                                            ),
                                        },
                                    ],
                                    methods: [],
                                    constructors: [
                                        AstClassConstructor {
                                            range: AstRange {
                                                start: AstPoint { 0:0 },
                                                end: AstPoint { 0:0 },
                                            },
                                            header: AstConstructorHeader {
                                                range: AstRange {
                                                    start: AstPoint { 0:0 },
                                                    end: AstPoint { 0:0 },
                                                },
                                                availability: AstAvailability(
                                                    0x0,
                                                ),
                                                name: AstIdentifier {
                                                    range: AstRange {
                                                        start: AstPoint { 5:13 },
                                                        end: AstPoint { 5:20 },
                                                    },
                                                    value: "AllArgs",
                                                },
                                                parameters: AstMethodParameters {
                                                    range: AstRange {
                                                        start: AstPoint { 0:0 },
                                                        end: AstPoint { 0:0 },
                                                    },
                                                    parameters: [
                                                        AstMethodParameter {
                                                            range: AstRange {
                                                                start: AstPoint { 0:0 },
                                                                end: AstPoint { 0:0 },
                                                            },
                                                            annotated: [],
                                                            jtype: AstJType {
                                                                annotated: [],
                                                                range: AstRange {
                                                                    start: AstPoint { 6:9 },
                                                                    end: AstPoint { 6:15 },
                                                                },
                                                                value: Class(
                                                                    AstIdentifier {
                                                                        range: AstRange {
                                                                            start: AstPoint { 6:9 },
                                                                            end: AstPoint { 6:15 },
                                                                        },
                                                                        value: "String",
                                                                    },
                                                                ),
                                                            },
                                                            name: AstIdentifier {
                                                                range: AstRange {
                                                                    start: AstPoint { 6:16 },
                                                                    end: AstPoint { 6:23 },
                                                                },
                                                                value: "project",
                                                            },
                                                            flags: AstMethodParameterFlags(
                                                                0x0,
                                                            ),
                                                        },
                                                    ],
                                                },
                                                throws: None,
                                                type_parameters: None,
                                                annotated: [],
                                            },
                                            block: AstBlock {
                                                range: AstRange {
                                                    start: AstPoint { 0:0 },
                                                    end: AstPoint { 0:0 },
                                                },
                                                entries: [],
                                            },
                                        },
                                    ],
                                    static_blocks: [],
                                    inner: [],
                                    blocks: [],
                                },
                            },
                        ),
                    ),
                ],
            }
        "#]];
        expected.assert_debug_eq(&out);
    }
}
