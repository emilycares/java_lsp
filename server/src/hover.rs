use ast::types::{
    AstBlock, AstBlockEntry, AstExpression, AstExpressionIdentifier, AstFile, AstJType,
    AstJTypeKind, AstPoint, AstRange, AstRecursiveExpression, AstThing,
};
use call_chain::{self, CallItem};
use document::Document;
use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};
use parser::dto::{self, ImportUnit};
use smol_str::{SmolStr, ToSmolStr};
use tyres::TyresError;
use variables::LocalVariable;

use crate::codeaction::to_lsp_range;

#[allow(dead_code)]
#[derive(Debug)]
pub enum HoverError {
    Tyres(TyresError),
    CallChainEmpty,
    ParseError(parser::java::ParseJavaError),
    ValidatedItemDoesNotExists,
    LocalVariableNotFound { name: SmolStr },
    Unimlemented,
    NoClass(SmolStr),
    ArgumentNotFound,
}

pub fn base(
    document: &Document,
    point: &AstPoint,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<SmolStr, parser::dto::Class>,
) -> Result<Hover, HoverError> {
    let ast = &document.ast;
    match class_action(ast, point, lo_va, imports, class_map) {
        Ok((class, range)) => {
            return Ok(class_to_hover(class, range));
        }
        Err(ClassActionError::NotFound) => {}
        Err(e) => eprintln!("class action hover error: {e:?}"),
    };
    let Some(class) = class_map.get(&document.class_path) else {
        return Err(HoverError::NoClass(document.class_path.to_smolstr()));
    };

    let call_chain = call_chain::get_call_chain(ast, point);

    call_chain_hover(
        document,
        call_chain,
        point,
        lo_va,
        imports,
        class.value(),
        class_map,
    )
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum ClassActionError {
    /// No class for actions found
    NotFound,
    /// Under the cursor there was no text
    CouldNotGetNode,
    /// In the type resolution error
    Tyres {
        tyres_error: tyres::TyresError,
    },
    VariableFound,
}
struct FoundClass {
    name: String,
    range: AstRange,
}
fn get_class(ast: &AstFile, point: &AstPoint) -> Option<FoundClass> {
    let mut out = None;

    'thing: {
        match &ast.thing {
            AstThing::Class(ast_class) => {
                for v in &ast_class.block.variables {
                    if !v.range.is_in_range(point) {
                        continue;
                    }

                    if let Some(o) = get_class_jtype(&v.jtype, point) {
                        out = Some(o);
                        break 'thing;
                    }
                    if let Some(ex) = &v.expression {
                        if let Some(o) = get_class_expression(ex, point) {
                            out = Some(o);
                            break 'thing;
                        }
                    }
                }
                for m in &ast_class.block.methods {
                    if !m.range.is_in_range(point) {
                        continue;
                    }
                    for ano in &m.annotated {
                        if !ano.range.is_in_range(point) {
                            continue;
                        }

                        if let Some(c) = get_class_identifier(&ano.name, point) {
                            return Some(c);
                        }
                    }

                    if let Some(o) = get_class_jtype(&m.header.jtype, point) {
                        return Some(o);
                    }
                    if m.header.parameters.range.is_in_range(point) {
                        for p in &m.header.parameters.parameters {
                            if let Some(o) = get_class_jtype(&p.jtype, point) {
                                return Some(o);
                            }
                        }
                    }

                    if let Some(b) = get_class_block(&m.block, point) {
                        return Some(b);
                    }
                }
            }
            AstThing::Interface(_ast_interface) => (),
            AstThing::Enumeration(_ast_enumeration) => (),
            AstThing::Annotation(_ast_annotation) => (),
        }
    }
    out
}

fn get_class_block(block: &AstBlock, point: &AstPoint) -> Option<FoundClass> {
    if !block.range.is_in_range(point) {
        return None;
    }
    for entry in &block.entries {
        match entry {
            AstBlockEntry::Return(_ast_block_return) => todo!(),
            AstBlockEntry::Variable(_ast_block_variable) => todo!(),
            AstBlockEntry::Expression(_ast_block_expression) => todo!(),
            AstBlockEntry::Assign(_ast_block_assign) => todo!(),
            AstBlockEntry::If(_ast_if) => todo!(),
            AstBlockEntry::While(_ast_while) => todo!(),
            AstBlockEntry::For(_ast_for) => todo!(),
            AstBlockEntry::ForEnhanced(_ast_for_enhanced) => todo!(),
            AstBlockEntry::Break(_ast_block_break) => todo!(),
            AstBlockEntry::Continue(_ast_block_continue) => todo!(),
            AstBlockEntry::Switch(_ast_switch) => todo!(),
            AstBlockEntry::SwitchCase(_ast_switch_case) => todo!(),
            AstBlockEntry::SwitchDefault(_ast_switch_default) => todo!(),
            AstBlockEntry::TryCatch(_ast_try_catch) => todo!(),
            AstBlockEntry::Throw(_ast_throw) => todo!(),
            AstBlockEntry::SwitchCaseArrow(_ast_switch_case_arrow) => todo!(),
            AstBlockEntry::Yield(_ast_block_yield) => todo!(),
        }
    }
    None
}

fn get_class_expression(ex: &AstExpression, point: &AstPoint) -> Option<FoundClass> {
    match &ex {
        AstExpression::Casted(ast_casted_expression) => {
            if let Some(o) = get_class_jtype(&ast_casted_expression.cast, point) {
                return Some(o);
            }
            if let Some(o) =
                get_class_recursive_expression(&ast_casted_expression.expression, point)
            {
                return Some(o);
            }
            None
        }
        AstExpression::Recursive(ast_recursive_expression) => {
            get_class_recursive_expression(ast_recursive_expression, point)
        }
        AstExpression::Lambda(ast_lambda) => todo!(),
        AstExpression::InlineSwitch(ast_switch) => todo!(),
        AstExpression::NewClass(ast_new_class) => todo!(),
    }
}

fn get_class_recursive_expression(
    expression: &AstRecursiveExpression,
    point: &AstPoint,
) -> Option<FoundClass> {
    let mut expression = expression;
    loop {
        if let Some(ident) = &expression.ident {
            if let Some(i) = get_class_expression_identifier(ident, point) {
                return Some(i);
            }
        }

        if let Some(next) = &expression.next {
            expression = next;
        } else {
            break;
        }
    }
    None
}

fn get_class_jtype(jtype: &AstJType, point: &AstPoint) -> Option<FoundClass> {
    if !jtype.range.is_in_range(point) {
        return None;
    }
    match &jtype.value {
        AstJTypeKind::Void => None,
        AstJTypeKind::Byte => None,
        AstJTypeKind::Char => None,
        AstJTypeKind::Double => None,
        AstJTypeKind::Float => None,
        AstJTypeKind::Int => None,
        AstJTypeKind::Long => None,
        AstJTypeKind::Short => None,
        AstJTypeKind::Boolean => None,
        AstJTypeKind::Wildcard => None,
        AstJTypeKind::Var => None,
        AstJTypeKind::Parameter(ast_identifier) | AstJTypeKind::Class(ast_identifier) => {
            if !ast_identifier.range.is_in_range(point) {
                return None;
            }
            Some(FoundClass {
                name: ast_identifier.value.to_string(),
                range: ast_identifier.range,
            })
        }
        AstJTypeKind::Array(ast_jtype) => get_class_jtype(&ast_jtype, point),
        AstJTypeKind::Generic(ast_identifier, ast_jtypes) => {
            if let Some(value) = get_class_identifier(ast_identifier, point) {
                return Some(value);
            }
            for jt in ast_jtypes {
                if let Some(j) = get_class_jtype(jt, point) {
                    return Some(j);
                }
            }
            None
        }
    }
}

fn get_class_identifier(
    ast_identifier: &ast::types::AstIdentifier,
    point: &AstPoint,
) -> Option<FoundClass> {
    if ast_identifier.range.is_in_range(point) {
        return Some(FoundClass {
            name: ast_identifier.value.to_string(),
            range: ast_identifier.range,
        });
    }
    None
}
fn get_class_expression_identifier(
    ast_identifier: &AstExpressionIdentifier,
    point: &AstPoint,
) -> Option<FoundClass> {
    match ast_identifier {
        AstExpressionIdentifier::Identifier(ast_identifier) => {
            get_class_identifier(ast_identifier, point)
        }
        AstExpressionIdentifier::Nuget(_ast_value_nuget) => None,
        AstExpressionIdentifier::Value(_ast_value) => None,
        AstExpressionIdentifier::ArrayAccess(_ast_value) => None,
    }
}

pub fn class_action(
    ast: &AstFile,
    point: &AstPoint,
    _lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class_map: &dashmap::DashMap<SmolStr, parser::dto::Class>,
) -> Result<(dto::Class, Range), ClassActionError> {
    if let Some(class) = get_class(ast, point) {
        return match tyres::resolve(&class.name, imports, class_map) {
            Ok(resolve_state) => Ok((resolve_state.class, to_lsp_range(&class.range))),
            Err(tyres_error) => Err(ClassActionError::Tyres { tyres_error }),
        };
    }
    Err(ClassActionError::NotFound)
}

pub fn call_chain_hover(
    document: &Document,
    call_chain: Vec<CallItem>,
    point: &AstPoint,
    lo_va: &[LocalVariable],
    imports: &[ImportUnit],
    class: &dto::Class,
    class_map: &dashmap::DashMap<SmolStr, parser::dto::Class>,
) -> Result<Hover, HoverError> {
    let (item, relevat) = call_chain::validate(&call_chain, point);
    let Some(el) = call_chain.get(item) else {
        return Err(HoverError::ValidatedItemDoesNotExists);
    };
    let resolve_state =
        match tyres::resolve_call_chain_to_point(&relevat, lo_va, imports, class, class_map, point)
        {
            Ok(c) => Ok(c),
            Err(e) => Err(HoverError::Tyres(e)),
        }?;
    match el {
        CallItem::MethodCall { name, range } => {
            let methods: Vec<dto::Method> = resolve_state
                .class
                .methods
                .into_iter()
                .filter(|m| m.name == *name)
                .collect();
            Ok(methods_to_hover(&methods, to_lsp_range(range)))
        }
        CallItem::FieldAccess { name, range } => {
            let Some(method) = resolve_state.class.fields.iter().find(|m| m.name == *name) else {
                return Err(HoverError::LocalVariableNotFound { name: name.clone() });
            };
            Ok(field_to_hover(method, to_lsp_range(range)))
        }
        CallItem::Variable { name, range } => {
            let Some(var) = lo_va.iter().find(|v| v.name == *name) else {
                return Err(HoverError::LocalVariableNotFound { name: name.clone() });
            };
            Ok(variables_to_hover(vec![var], to_lsp_range(range)))
        }
        CallItem::Class { name: _, range } => {
            Ok(class_to_hover(resolve_state.class, to_lsp_range(range)))
        }
        CallItem::ClassOrVariable { name, range } => {
            let vars: Vec<_> = lo_va.iter().filter(|v| v.name == *name).collect();
            if vars.is_empty() {
                return Ok(class_to_hover(resolve_state.class, to_lsp_range(range)));
            }
            match parser::load_java(document.as_bytes(), parser::loader::SourceDestination::None) {
                Err(e) => Err(HoverError::ParseError(e)),
                Ok(local_class) => {
                    let vars = vars
                        .iter()
                        .filter(|v| !v.is_fun)
                        .map(|v| format!("{} {}", v.jtype, v.name))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let fields = local_class
                        .fields
                        .iter()
                        .filter(|m| m.name == *name)
                        .map(format_field)
                        .collect::<Vec<_>>()
                        .join("\n");
                    let methods = local_class
                        .methods
                        .iter()
                        .filter(|m| m.name == *name)
                        .map(format_method)
                        .collect::<Vec<_>>()
                        .join("\n");
                    Ok(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!("{}\n{}\n{}", vars, fields, methods),
                        }),
                        range: Some(to_lsp_range(range)),
                    })
                }
            }
        }
        CallItem::ArgumentList {
            prev: _,
            active_param,
            filled_params,
            range: _,
        } => {
            if let Some(active_param) = active_param
                && let Some(current_param) = filled_params.get(*active_param)
            {
                return call_chain_hover(
                    document,
                    current_param.clone(),
                    point,
                    lo_va,
                    imports,
                    &resolve_state.class,
                    class_map,
                );
            }
            Err(HoverError::ArgumentNotFound)
        }
        CallItem::This { range: _ } => Err(HoverError::Unimlemented),
    }
}

fn format_field(f: &dto::Field) -> String {
    format!("{} {}", f.jtype, f.name)
}

fn format_method(m: &dto::Method) -> String {
    let parameters = m
        .parameters
        .iter()
        .map(|p| match p.name.as_deref() {
            Some(name) => format!("{} {}", p.jtype, name),
            None => format!("{}", p.jtype),
        })
        .collect::<Vec<_>>()
        .join(", ");
    if m.throws.is_empty() {
        return format!("{} {}({})", m.ret, m.name, parameters);
    }
    let throws = m
        .throws
        .iter()
        .map(|p| format!("{}", p))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{} {}({}) throws {}", m.ret, m.name, parameters, throws)
}

fn variables_to_hover(vars: Vec<&LocalVariable>, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: vars
                .iter()
                .map(format_varible_hover)
                .collect::<Vec<_>>()
                .join("\n"),
        }),
        range: Some(range),
    }
}

fn format_varible_hover(var: &&LocalVariable) -> String {
    if var.is_fun {
        return format!("{} {}()", var.jtype, var.name);
    }
    format!("{} {}", var.jtype, var.name)
}

fn field_to_hover(f: &dto::Field, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("{} {}", f.jtype, f.name),
        }),
        range: Some(range),
    }
}

fn methods_to_hover(methods: &[dto::Method], range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: methods
                .iter()
                .map(format_method)
                .collect::<Vec<_>>()
                .join("\n"),
        }),
        range: Some(range),
    }
}

fn class_to_hover(class: dto::Class, range: Range) -> Hover {
    let methods: Vec<_> = class.methods.iter().map(format_method).collect();
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("# {}\n\nmethods:\n{}", class.name, methods.join("\n")),
        }),
        range: Some(range),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ast::types::AstPoint;
    use dashmap::DashMap;
    use document::Document;
    use parser::dto;
    use smol_str::SmolStr;

    use crate::hover::{call_chain_hover, class_action};

    #[test]
    fn class_action_base() {
        let content = "
package ch.emilycares;
public class Test {
    public String hello() {
        return;
    }
}
";
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();
        let ast = &doc.ast;

        let out = class_action(ast, &AstPoint::new(3, 14), &[], &[], &string_class_map());
        assert!(out.is_ok());
    }

    #[test]
    fn class_action_marker_annotation() {
        let content = "
package ch.emilycares;
public class Test {
    @String
    public void hello() {
        return;
    }
}
";
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();
        let ast = &doc.ast;

        let out = class_action(ast, &AstPoint::new(3, 9), &[], &[], &string_class_map());
        dbg!(&out);
        assert!(out.is_ok());
    }

    #[test]
    fn method_hover() {
        let class = dto::Class {
            access: vec![dto::Access::Public],
            name: "Test".into(),
            ..Default::default()
        };
        let content = "
package ch.emilycares;
public class Test {
    public void hello() {
    String other = \"asd\";
    String local = other.length().toString();
    }
}
";
        let doc = Document::setup(content, PathBuf::new(), "".into()).unwrap();
        let point = AstPoint::new(5, 29);
        let vars = variables::get_vars(&doc.ast, &point).unwrap();

        let chain = call_chain::get_call_chain(&doc.ast, &point);
        let out = call_chain_hover(&doc, chain, &point, &vars, &[], &class, &string_class_map());
        assert!(out.is_ok());
    }

    fn string_class_map() -> DashMap<SmolStr, dto::Class> {
        let class_map: DashMap<SmolStr, dto::Class> = DashMap::new();
        class_map.insert(
            "java.lang.String".into(),
            dto::Class {
                access: vec![dto::Access::Public],
                name: "String".into(),
                methods: vec![dto::Method {
                    access: vec![dto::Access::Public],
                    name: "length".into(),
                    ret: dto::JType::Int,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        class_map
    }
}
