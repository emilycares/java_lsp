#![deny(clippy::redundant_clone)]
use std::path::Path;

use ast::{
    lexer::{PositionToken, Token},
    range::GetRange,
    types::{
        AstAnnotated, AstAnnotatedParameter, AstAnnotatedParameterKind, AstAnnotation,
        AstAnnotationField, AstAvailability, AstBlock, AstBlockEntry, AstClass, AstClassBlock,
        AstClassConstructor, AstClassMethod, AstClassVariable, AstEnumeration,
        AstExpressionIdentifier, AstExpressionKind, AstExpressionOperator,
        AstExpressionOrAnnotated, AstExpressionOrDefault, AstExpressionOrValue, AstForContent,
        AstIdentifier, AstIf, AstIfContent, AstImport, AstImportUnit, AstInterface,
        AstInterfaceConstant, AstInterfaceMethod, AstInterfaceMethodDefault, AstJType,
        AstJTypeKind, AstLambdaRhs, AstMethodHeader, AstMethodParameterFlags, AstMethodParameters,
        AstModule, AstModuleRequiresFlags, AstNewRhs, AstPackage, AstPoint, AstRecord,
        AstRecordEntries, AstSuperClass, AstSwitchCaseArrowContent, AstThing, AstThingAttributes,
        AstThrowsDeclaration, AstTopLevel, AstTypeParameters, AstValue, AstValueNuget,
        AstValuesWithAnnotated, AstVolatileTransient, AstWhileContent,
    },
};
use config::FormatterConfig;

pub mod google;
pub mod idea;

#[derive(Debug)]
pub enum FormatError {
    IO(std::io::Error),
    Spawn(std::io::Error),
    Diagnostic(Vec<FormatLineError>),
    NoFormatterSpecified,
    Ast(ast::error::AstError),
    Lexer(ast::lexer::LexerError),
}

#[derive(Debug)]
pub struct FormatLineError {
    pub line: u32,
    pub col: u32,
    pub message: String,
}
pub fn get_formatter_name(formatter: &FormatterConfig) -> String {
    match formatter {
        FormatterConfig::None => String::from("No formatter"),
        FormatterConfig::Internal => String::from("java_lsp format"),
        FormatterConfig::Google => String::from("Google java format"),
        FormatterConfig::Idea => String::from("Idea format"),
    }
}

pub fn format(
    formatter: &FormatterConfig,
    content: &[u8],
    path: &Path,
    project_dir: &Path,
) -> Result<Option<Vec<u8>>, FormatError> {
    match formatter {
        FormatterConfig::None => Err(FormatError::NoFormatterSpecified),
        FormatterConfig::Internal => internal(content),
        FormatterConfig::Google => google::google_java_format(content),
        FormatterConfig::Idea => idea::idea_java_format(path, project_dir),
    }
}

fn internal(content: &[u8]) -> Result<Option<Vec<u8>>, FormatError> {
    let mut formatter = Formatter::new(content)?;
    let tokens = ast::lexer::lex_v::<false>(content).map_err(FormatError::Lexer)?;
    let ast = ast::parse_file(&tokens).map_err(FormatError::Ast)?;

    for t in ast.top {
        match t {
            AstTopLevel::Package(p) => {
                write_package(p, &mut formatter)?;
            }
            AstTopLevel::Import(ast_import) => write_import(&ast_import, &mut formatter),
            AstTopLevel::Thing(ast_thing) => write_thing(&ast_thing, &mut formatter),
            AstTopLevel::Method(ast_class_method) => {
                write_class_method(&ast_class_method, &mut formatter)
            }
            AstTopLevel::Module(ast_module) => write_module(&ast_module, &mut formatter),
        }
    }

    Ok(Some(formatter.buf))
}

struct Formatter {
    pub with_comments: Vec<PositionToken>,
    pub index: usize,
    pub buf: Vec<u8>,
    pub indent: usize,
}

impl Formatter {
    pub fn new(content: &[u8]) -> Result<Self, FormatError> {
        let with_comments = ast::lexer::lex_v::<true>(content).map_err(FormatError::Lexer)?;
        Ok(Self {
            with_comments,
            index: 0,
            buf: Vec::new(),
            indent: 0,
        })
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.buf.extend_from_slice(b"    ");
        }
    }

    pub fn insert_comments(&mut self, up_to: AstPoint) {
        while let Some(t) = self.with_comments.get(self.index) {
            let pos = t.start_point();
            if pos >= up_to {
                break;
            }
            match &t.token {
                Token::LineComment(l) => {
                    self.buf.extend_from_slice(b"//");
                    self.buf.extend_from_slice(l);
                    self.write_indent();
                }
                Token::BlockComment(c, _) => {
                    self.buf.extend_from_slice(b"/*");
                    self.buf.extend_from_slice(c);
                    self.buf.extend_from_slice(b"*/");
                    self.insert_line_or_space();
                }
                _ => {}
            }
            self.index += 1;
        }
    }

    pub fn write_with_comments(&mut self, up_to: AstPoint, content: &[u8]) {
        self.insert_comments(up_to);
        self.buf.extend_from_slice(content);
    }

    pub fn write_identifier(&mut self, ident: &AstIdentifier) {
        self.write_with_comments(ident.range.start, ident.value.as_bytes());
        self.skip_to(ident.range.end);
    }

    pub fn write(&mut self, content: &[u8]) {
        while let Some(t) = self.with_comments.get(self.index) {
            match &t.token {
                Token::LineComment(l) => {
                    self.buf.extend_from_slice(b"//");
                    self.buf.extend_from_slice(l);
                    self.write_indent();
                    self.index += 1;
                }
                Token::BlockComment(c, _) => {
                    self.buf.extend_from_slice(b"/*");
                    self.buf.extend_from_slice(c);
                    self.buf.extend_from_slice(b"*/");
                    self.insert_line_or_space();
                    self.index += 1;
                }
                _ => {
                    self.index += 1;
                    break;
                }
            }
        }
        self.buf.extend_from_slice(content);
    }

    pub fn insert_line_or_space(&mut self) {
        let last_line = self
            .index
            .checked_sub(1)
            .and_then(|i| self.with_comments.get(i))
            .map(|t| t.line);
        let next_line = self.with_comments.get(self.index).map(|t| t.line);
        if matches!((last_line, next_line), (Some(a), Some(b)) if a == b) {
            self.buf.push(b' ');
        } else {
            self.buf.extend_from_slice(b"\n");
            self.write_indent();
        }
    }

    pub fn skip_to(&mut self, up_to: AstPoint) {
        loop {
            let pos = self.with_comments.get(self.index).map(|t| t.start_point());
            let Some(pos) = pos else { break };
            if pos >= up_to {
                break;
            }
            self.index += 1;
        }
    }
}

fn write_package(p: AstPackage, formatter: &mut Formatter) -> Result<(), FormatError> {
    for ann in &p.annotated {
        write_annotation(ann, formatter);
    }
    formatter.write(b"package ");
    formatter.write_identifier(&p.name);
    formatter.write(b";");
    formatter.buf.push(b'\n');
    Ok(())
}

fn write_annotation(ann: &AstAnnotated, formatter: &mut Formatter) {
    formatter.write(b"@");
    formatter.write_identifier(&ann.name);
    match &ann.parameters {
        AstAnnotatedParameterKind::None => {}
        AstAnnotatedParameterKind::Parameter(params) => {
            formatter.write(b"(");
            for (i, param) in params.iter().enumerate() {
                if i > 0 {
                    formatter.write(b", ");
                }
                write_annotation_parameter(param, formatter);
            }
            formatter.write(b")");
        }
        AstAnnotatedParameterKind::Array(values) => {
            formatter.write(b"(");
            write_values_with_annotated(values, formatter);
            formatter.write(b")");
        }
    }
    formatter.insert_line_or_space();
}

fn write_annotation_parameter(param: &AstAnnotatedParameter, formatter: &mut Formatter) {
    match param {
        AstAnnotatedParameter::Expression(expr) => {
            write_expression(expr, formatter);
        }
        AstAnnotatedParameter::NamedExpression {
            name, expression, ..
        } => {
            formatter.write_identifier(name);
            formatter.write(b" = ");
            write_expression(expression, formatter);
        }
        AstAnnotatedParameter::Annotated(ann) => {
            write_annotation(ann, formatter);
        }
        AstAnnotatedParameter::NamedArray { name, values, .. } => {
            formatter.write_identifier(name);
            formatter.write(b" = ");
            write_values_with_annotated(values, formatter);
        }
    }
}

fn write_values_with_annotated(values: &AstValuesWithAnnotated, formatter: &mut Formatter) {
    formatter.write(b"{");
    for (i, v) in values.values.iter().enumerate() {
        if i > 0 {
            formatter.write(b", ");
        }
        match v {
            AstExpressionOrAnnotated::Expression(expr) => write_expression(expr, formatter),
            AstExpressionOrAnnotated::Annotated(ann) => write_annotation(ann, formatter),
        }
    }
    formatter.write(b"}");
}

fn write_expression(expr: &[AstExpressionKind], formatter: &mut Formatter) {
    let mut is_large = false;
    let range = expr.get_range();
    if range.start.line != range.end.line {
        is_large = true;
    }
    if is_large {
        formatter.indent += 1;
    }
    for kind in expr {
        write_expression_kind(kind, formatter, is_large);
    }
    if is_large {
        formatter.indent -= 1;
    }
}

fn write_expression_kind(kind: &AstExpressionKind, formatter: &mut Formatter, is_large: bool) {
    match kind {
        AstExpressionKind::Base(base) => {
            if let Some(ident) = &base.ident {
                write_expression_identifier(ident, formatter);
            }
            if let Some(values) = &base.values {
                formatter.write(b"(");
                for (i, expr) in values.values.iter().enumerate() {
                    if i > 0 {
                        formatter.write(b", ");
                    }
                    write_expression(expr, formatter);
                }
                formatter.write(b")");
            }
            write_expression_operator(&base.operator, formatter, is_large);
        }
        AstExpressionKind::Casted(casted) => {
            formatter.write(b"(");
            write_jtype(&casted.cast, formatter);
            formatter.write(b")");
        }
        AstExpressionKind::JType(jtype_expr) => {
            write_jtype(&jtype_expr.cast, formatter);
        }
        AstExpressionKind::Array(values) => {
            formatter.write(b"{");
            for (i, expr) in values.values.iter().enumerate() {
                if i > 0 {
                    formatter.write(b", ");
                }
                write_expression(expr, formatter);
            }
            formatter.write(b"}");
        }
        AstExpressionKind::Generics(generics) => {
            formatter.write(b"<");
            for (i, jtype) in generics.jtypes.iter().enumerate() {
                if i > 0 {
                    formatter.write(b", ");
                }
                write_jtype(jtype, formatter);
            }
            formatter.write(b">");
        }
        AstExpressionKind::InstanceOf(instanceof) => {
            formatter.write(b"instanceof ");
            if instanceof.availability.contains(AstAvailability::Final) {
                formatter.write(b"final ");
            }
            for ann in &instanceof.annotated {
                write_annotation(ann, formatter);
            }
            write_jtype(&instanceof.jtype, formatter);
            if let Some(var) = &instanceof.variable {
                formatter.buf.push(b' ');
                formatter.write_identifier(var);
            }
        }
        AstExpressionKind::Lambda(lambda) => {
            let has_brace = lambda.parameters.values.len() > 1;
            if has_brace {
                formatter.write(b"(");
            }
            for (i, param) in lambda.parameters.values.iter().enumerate() {
                if i > 0 {
                    formatter.write(b", ");
                }
                if let Some(jtype) = &param.jtype {
                    write_jtype(jtype, formatter);
                    formatter.buf.push(b' ');
                }
                formatter.write_identifier(&param.name);
            }
            if has_brace {
                formatter.write(b")");
            }
            formatter.write(b" -> ");
            match &lambda.rhs {
                AstLambdaRhs::None => {}
                AstLambdaRhs::Expr(expr) => write_expression(expr, formatter),
                AstLambdaRhs::Block(block) => write_block(block, formatter),
            }
        }
        AstExpressionKind::NewClass(new_class) => {
            formatter.write(b"new ");
            write_jtype(&new_class.jtype, formatter);
            match new_class.rhs.as_ref() {
                AstNewRhs::None => {}
                AstNewRhs::Parameters(_range, exprs) => {
                    formatter.write(b"(");
                    for (i, expr) in exprs.iter().enumerate() {
                        if i > 0 {
                            formatter.write(b", ");
                        }
                        write_expression(expr, formatter);
                    }
                    formatter.write(b")");
                }
                AstNewRhs::ParametersAndBlock(_range, exprs, block) => {
                    formatter.write(b"(");
                    for (i, expr) in exprs.iter().enumerate() {
                        if i > 0 {
                            formatter.write(b", ");
                        }
                        write_expression(expr, formatter);
                    }
                    formatter.write(b")");
                    formatter.buf.push(b' ');
                    write_class_block_braced(block, formatter);
                }
                AstNewRhs::ArrayParameters(param_groups) => {
                    for group in param_groups {
                        formatter.write(b"[");
                        for (i, expr) in group.iter().enumerate() {
                            if i > 0 {
                                formatter.write(b", ");
                            }
                            write_expression(expr, formatter);
                        }
                        formatter.write(b"]");
                    }
                }
                AstNewRhs::Array(values) => {
                    formatter.write(b"{");
                    for (i, expr) in values.values.iter().enumerate() {
                        if i > 0 {
                            formatter.write(b", ");
                        }
                        write_expression(expr, formatter);
                    }
                    formatter.write(b"}");
                }
                AstNewRhs::Block(block) => {
                    formatter.buf.push(b' ');
                    write_class_block_braced(block, formatter);
                }
            }
        }
        AstExpressionKind::InlineSwitch(switch) => {
            formatter.write(b"switch ");
            formatter.write(b"(");
            write_expression(&switch.check, formatter);
            formatter.write(b")");
            formatter.buf.push(b' ');
            write_block(&switch.block, formatter);
        }
    }
}

fn write_jtype(jtype: &AstJType, formatter: &mut Formatter) {
    for ann in &jtype.annotated {
        write_annotation(ann, formatter);
    }
    match &jtype.value {
        AstJTypeKind::Void => {
            formatter.write_with_comments(jtype.range.start, b"void");
            formatter.skip_to(jtype.range.end);
        }
        AstJTypeKind::Byte => {
            formatter.write_with_comments(jtype.range.start, b"byte");
            formatter.skip_to(jtype.range.end);
        }
        AstJTypeKind::Char => {
            formatter.write_with_comments(jtype.range.start, b"char");
            formatter.skip_to(jtype.range.end);
        }
        AstJTypeKind::Double => {
            formatter.write_with_comments(jtype.range.start, b"double");
            formatter.skip_to(jtype.range.end);
        }
        AstJTypeKind::Float => {
            formatter.write_with_comments(jtype.range.start, b"float");
            formatter.skip_to(jtype.range.end);
        }
        AstJTypeKind::Int => {
            formatter.write_with_comments(jtype.range.start, b"int");
            formatter.skip_to(jtype.range.end);
        }
        AstJTypeKind::Long => {
            formatter.write_with_comments(jtype.range.start, b"long");
            formatter.skip_to(jtype.range.end);
        }
        AstJTypeKind::Short => {
            formatter.write_with_comments(jtype.range.start, b"short");
            formatter.skip_to(jtype.range.end);
        }
        AstJTypeKind::Boolean => {
            formatter.write_with_comments(jtype.range.start, b"boolean");
            formatter.skip_to(jtype.range.end);
        }
        AstJTypeKind::Wildcard => {
            formatter.write_with_comments(jtype.range.start, b"?");
            formatter.skip_to(jtype.range.end);
        }
        AstJTypeKind::Var => {
            formatter.write_with_comments(jtype.range.start, b"var");
            formatter.skip_to(jtype.range.end);
        }
        AstJTypeKind::Class(ident) => {
            formatter.write_identifier(ident);
        }
        AstJTypeKind::Array(inner) => {
            write_jtype(inner, formatter);
            formatter.write(b"[");
            formatter.write(b"]");
        }
        AstJTypeKind::Generic(ident, types) => {
            formatter.write_identifier(ident);
            formatter.write(b"<");
            for (i, t) in types.iter().enumerate() {
                if i > 0 {
                    formatter.write(b", ");
                }
                write_jtype(t, formatter);
            }
            formatter.write(b">");
        }
        AstJTypeKind::Access { base, inner } => {
            write_jtype(base, formatter);
            formatter.write(b".");
            write_jtype(inner, formatter);
        }
    }
}

fn write_availability(availability: &AstAvailability, formatter: &mut Formatter) {
    if availability.contains(AstAvailability::Public) {
        formatter.write(b"public ");
    }
    if availability.contains(AstAvailability::Protected) {
        formatter.write(b"protected ");
    }
    if availability.contains(AstAvailability::Private) {
        formatter.write(b"private ");
    }
    if availability.contains(AstAvailability::Abstract) {
        formatter.write(b"abstract ");
    }
    if availability.contains(AstAvailability::Static) {
        formatter.write(b"static ");
    }
    if availability.contains(AstAvailability::Final) {
        formatter.write(b"final ");
    }
    if availability.contains(AstAvailability::Synchronized) {
        formatter.write(b"synchronized ");
    }
    if availability.contains(AstAvailability::Native) {
        formatter.write(b"native ");
    }
}

fn write_expr_or_value(eov: &AstExpressionOrValue, formatter: &mut Formatter) {
    match eov {
        AstExpressionOrValue::None => {}
        AstExpressionOrValue::Expression(expr) => write_expression(expr, formatter),
        AstExpressionOrValue::Value(val) => write_value(val, formatter),
    }
}

fn write_block(block: &AstBlock, formatter: &mut Formatter) {
    formatter.write(b"{");
    formatter.buf.push(b'\n');
    formatter.indent += 1;
    for entry in &block.entries {
        write_block_entry(entry, formatter, true);
    }
    formatter.indent -= 1;
    formatter.write_indent();
    formatter.write(b"}");
}

fn write_block_entry(entry: &AstBlockEntry, formatter: &mut Formatter, around: bool) {
    match entry {
        AstBlockEntry::Semicolon(_) => {
            if around {
                formatter.write_indent();
            }
            formatter.write(b";");
            if around {
                formatter.buf.push(b'\n');
            }
        }
        AstBlockEntry::Return(ret) => {
            if around {
                formatter.write_indent();
            }
            formatter.write(b"return");
            if !matches!(ret.expression, AstExpressionOrValue::None) {
                formatter.buf.push(b' ');
                write_expr_or_value(&ret.expression, formatter);
            }
            if around {
                formatter.write(b";");
                formatter.buf.push(b'\n');
            }
        }
        AstBlockEntry::Yield(yield_) => {
            if around {
                formatter.write_indent();
            }
            formatter.write(b"yield");
            if !matches!(yield_.expression, AstExpressionOrValue::None) {
                formatter.buf.push(b' ');
                write_expr_or_value(&yield_.expression, formatter);
            }
            if around {
                formatter.write(b";");
                formatter.buf.push(b'\n');
            }
        }
        AstBlockEntry::Throw(throw) => {
            if around {
                formatter.write_indent();
            }
            formatter.write(b"throw ");
            write_expression(&throw.expression, formatter);
            if around {
                formatter.write(b";");
                formatter.buf.push(b'\n');
            }
        }
        AstBlockEntry::Break(br) => {
            if around {
                formatter.write_indent();
            }
            formatter.write(b"break");
            if let Some(label) = &br.label {
                formatter.buf.push(b' ');
                formatter.write_identifier(label);
            }
            if around {
                formatter.write(b";");
                formatter.buf.push(b'\n');
            }
        }
        AstBlockEntry::Continue(cont) => {
            if around {
                formatter.write_indent();
            }
            formatter.write(b"continue");
            if let Some(label) = &cont.label {
                formatter.buf.push(b' ');
                formatter.write_identifier(label);
            }
            if around {
                formatter.write(b";");
                formatter.buf.push(b'\n');
            }
        }
        AstBlockEntry::Assert(assert) => {
            if around {
                formatter.write_indent();
            }
            formatter.write(b"assert ");
            write_expression(&assert.expression, formatter);
            if around {
                formatter.write(b";");
                formatter.buf.push(b'\n');
            }
        }
        AstBlockEntry::Expression(expr) => {
            if around {
                formatter.write_indent();
            }
            write_expression(&expr.value, formatter);
            if around {
                formatter.write(b";");
                formatter.buf.push(b'\n');
            }
        }
        AstBlockEntry::Assign(assign) => {
            if around {
                formatter.write_indent();
            }
            write_expression(&assign.key, formatter);
            formatter.write(b" = ");
            write_expression(&assign.expression, formatter);
            if around {
                formatter.write(b";");
                formatter.buf.push(b'\n');
            }
        }
        AstBlockEntry::Variable(vars) if !vars.is_empty() => {
            if around {
                formatter.write_indent();
            }
            for ann in &vars[0].annotated {
                write_annotation(ann, formatter);
                formatter.write_indent();
            }
            if vars[0].fin {
                formatter.write(b"final ");
            }
            write_jtype(&vars[0].jtype, formatter);
            formatter.buf.push(b' ');
            formatter.write_identifier(&vars[0].name);
            if let Some(val) = &vars[0].value {
                formatter.write(b" = ");
                write_expression(val, formatter);
            }
            for var in &vars[1..] {
                formatter.write(b", ");
                formatter.write_identifier(&var.name);
                if let Some(val) = &var.value {
                    formatter.write(b" = ");
                    write_expression(val, formatter);
                }
            }
            if around {
                formatter.write(b";");
                formatter.buf.push(b'\n');
            }
        }
        AstBlockEntry::Variable(_) => {}
        AstBlockEntry::If(if_) => write_if_entry(if_, formatter),
        AstBlockEntry::While(while_) => {
            formatter.write_indent();
            if let Some(label) = &while_.label {
                formatter.write_identifier(label);
                formatter.write(b": ");
            }
            formatter.write(b"while ");
            formatter.write(b"(");
            write_expression(&while_.control, formatter);
            formatter.write(b") ");
            write_while_content(&while_.content, formatter);
        }
        AstBlockEntry::For(for_) => {
            formatter.write_indent();
            if let Some(label) = &for_.label {
                formatter.write_identifier(label);
                formatter.write(b": ");
            }
            formatter.write(b"for ");
            formatter.write(b"(");
            for (i, entry) in for_.vars.iter().enumerate() {
                if i > 0 {
                    formatter.write(b", ");
                }
                write_block_entry(entry, formatter, false);
            }
            formatter.write(b"; ");
            for (i, entry) in for_.check.iter().enumerate() {
                if i > 0 {
                    formatter.write(b", ");
                }
                write_block_entry(entry, formatter, false);
            }
            formatter.write(b"; ");
            for (i, entry) in for_.changes.iter().enumerate() {
                if i > 0 {
                    formatter.write(b", ");
                }
                write_block_entry(entry, formatter, false);
            }
            formatter.write(b") ");
            write_for_content(&for_.content, formatter);
        }
        AstBlockEntry::ForEnhanced(for_) => {
            formatter.write_indent();
            if let Some(label) = &for_.label {
                formatter.write_identifier(label);
                formatter.write(b": ");
            }
            formatter.write(b"for ");
            formatter.write(b"(");
            for var in &for_.var {
                for ann in &var.annotated {
                    write_annotation(ann, formatter);
                }
                if var.fin {
                    formatter.write(b"final ");
                }
                write_jtype(&var.jtype, formatter);
                formatter.buf.push(b' ');
                formatter.write_identifier(&var.name);
            }
            formatter.write(b" : ");
            write_expression(&for_.rhs, formatter);
            formatter.write(b") ");
            write_for_content(&for_.content, formatter);
        }
        AstBlockEntry::Switch(switch) => {
            formatter.write_indent();
            formatter.write(b"switch ");
            formatter.write(b"(");
            write_expression(&switch.check, formatter);
            formatter.write(b") ");
            write_block(&switch.block, formatter);
            formatter.buf.push(b'\n');
        }
        AstBlockEntry::SwitchCase(case) => {
            formatter.write_indent();
            formatter.write(b"case ");
            for (i, expr) in case.expressions.iter().enumerate() {
                if i > 0 {
                    formatter.write(b", ");
                }
                write_expression_or_default(expr, formatter);
            }
            formatter.write(b":");
            formatter.buf.push(b'\n');
        }
        AstBlockEntry::SwitchDefault(_) => {
            formatter.write_indent();
            formatter.write(b"default:");
            formatter.buf.push(b'\n');
        }
        AstBlockEntry::SwitchCaseArrowValues(arrow) => {
            formatter.write_indent();
            formatter.write(b"case ");
            for (i, val) in arrow.values.iter().enumerate() {
                if i > 0 {
                    formatter.write(b", ");
                }
                write_expression_or_default(val, formatter);
            }
            formatter.write(b" -> ");
            write_switch_arrow_content(&arrow.content, formatter);
        }
        AstBlockEntry::SwitchCaseArrowType(arrow) => {
            formatter.write_indent();
            formatter.write(b"case ");
            write_jtype(&arrow.var.jtype, formatter);
            formatter.buf.push(b' ');
            formatter.write_identifier(&arrow.var.name);
            formatter.write(b" -> ");
            write_switch_arrow_content(&arrow.content, formatter);
        }
        AstBlockEntry::SwitchCaseArrowDefault(arrow) => {
            formatter.write_indent();
            formatter.write(b"default -> ");
            write_switch_arrow_content(&arrow.content, formatter);
        }
        AstBlockEntry::TryCatch(try_catch) => {
            formatter.write_indent();
            formatter.write(b"try ");
            if let Some(resources) = &try_catch.resources_block {
                write_block_delimited(resources, b"(", b")", formatter);
                formatter.buf.push(b' ');
            }
            write_block(&try_catch.block, formatter);
            for case in &try_catch.cases {
                formatter.buf.extend_from_slice(b" catch ");
                formatter.write(b"(");
                if case.variable.fin {
                    formatter.write(b"final ");
                }
                for (i, jtype) in case.variable.jtypes.iter().enumerate() {
                    if i > 0 {
                        formatter.write(b" | ");
                    }
                    write_jtype(jtype, formatter);
                }
                formatter.buf.push(b' ');
                formatter.write_identifier(&case.variable.name);
                formatter.write(b") ");
                write_block(&case.block, formatter);
            }
            if let Some(finally) = &try_catch.finally_block {
                formatter.buf.extend_from_slice(b" finally ");
                write_block(finally, formatter);
            }
            formatter.buf.push(b'\n');
        }
        AstBlockEntry::SynchronizedBlock(sync) => {
            formatter.write_indent();
            formatter.write(b"synchronized ");
            formatter.write(b"(");
            write_expression(&sync.expression, formatter);
            formatter.write(b") ");
            write_block(&sync.block, formatter);
            formatter.buf.push(b'\n');
        }
        AstBlockEntry::InlineBlock(inline) => {
            formatter.write_indent();
            if let Some(label) = &inline.label {
                formatter.write_identifier(label);
                formatter.write(b": ");
            }
            write_block(&inline.block, formatter);
            formatter.buf.push(b'\n');
        }
        AstBlockEntry::Thing(thing) => {
            write_thing(thing, formatter);
        }
    }
}

fn write_if_entry(aif: &AstIf, formatter: &mut Formatter) {
    match aif {
        AstIf::If {
            control, content, ..
        } => {
            formatter.write_indent();
            formatter.write(b"if ");
            formatter.write(b"(");
            write_expression(control, formatter);
            formatter.write(b") ");
            write_if_content(content, formatter);
        }
        AstIf::ElseIf {
            control, content, ..
        } => {
            formatter.write_indent();
            formatter.buf.extend_from_slice(b"else if ");
            formatter.write(b"(");
            write_expression(control, formatter);
            formatter.write(b") ");
            write_if_content(content, formatter);
        }
        AstIf::Else { content, .. } => {
            formatter.write_indent();
            formatter.buf.extend_from_slice(b"else ");
            write_if_content(content, formatter);
        }
    }
}

fn write_if_content(content: &AstIfContent, formatter: &mut Formatter) {
    match content {
        AstIfContent::Block(block) => {
            write_block(block, formatter);
            formatter.buf.push(b'\n');
        }
        AstIfContent::BlockEntry(entry) => {
            formatter.buf.push(b'\n');
            formatter.indent += 1;
            write_block_entry(entry, formatter, true);
            formatter.indent -= 1;
        }
    }
}

fn write_while_content(content: &AstWhileContent, formatter: &mut Formatter) {
    match content {
        AstWhileContent::None => {
            formatter.write(b";");
            formatter.buf.push(b'\n');
        }
        AstWhileContent::Block(block) => {
            write_block(block, formatter);
            formatter.buf.push(b'\n');
        }
        AstWhileContent::BlockEntry(entry) => {
            formatter.buf.push(b'\n');
            formatter.indent += 1;
            write_block_entry(entry, formatter, true);
            formatter.indent -= 1;
        }
    }
}

fn write_for_content(content: &AstForContent, formatter: &mut Formatter) {
    match content {
        AstForContent::None => {
            formatter.write(b";");
            formatter.buf.push(b'\n');
        }
        AstForContent::Block(block) => {
            write_block(block, formatter);
            formatter.buf.push(b'\n');
        }
        AstForContent::BlockEntry(entry) => {
            formatter.buf.push(b'\n');
            formatter.indent += 1;
            write_block_entry(entry, formatter, true);
            formatter.indent -= 1;
        }
    }
}

fn write_expression_or_default(eod: &AstExpressionOrDefault, formatter: &mut Formatter) {
    match eod {
        AstExpressionOrDefault::Default => {
            formatter.write(b"default");
        }
        AstExpressionOrDefault::Expression(e) => write_expression(e, formatter),
    }
}

fn write_switch_arrow_content(content: &AstSwitchCaseArrowContent, formatter: &mut Formatter) {
    match content {
        AstSwitchCaseArrowContent::Block(block) => {
            write_block(block, formatter);
            formatter.buf.push(b'\n');
        }
        AstSwitchCaseArrowContent::Entry(entry) => {
            write_block_entry(entry, formatter, true);
        }
    }
}

fn write_block_delimited(block: &AstBlock, open: &[u8], close: &[u8], formatter: &mut Formatter) {
    formatter.write(open);
    formatter.buf.push(b'\n');
    formatter.indent += 1;
    for entry in &block.entries {
        write_block_entry(entry, formatter, true);
    }
    formatter.indent -= 1;
    formatter.write_indent();
    formatter.write(close);
}

fn write_thing(thing: &AstThing, formatter: &mut Formatter) {
    match thing {
        AstThing::Class(class) => write_class(class, formatter),
        AstThing::Record(record) => write_record(record, formatter),
        AstThing::Interface(iface) => write_interface(iface, formatter),
        AstThing::Enumeration(enumeration) => write_enumeration(enumeration, formatter),
        AstThing::Annotation(annotation) => write_annotation_type(annotation, formatter),
    }
}

fn write_class(class: &AstClass, formatter: &mut Formatter) {
    for ann in &class.annotated {
        formatter.write_indent();
        write_annotation(ann, formatter);
    }
    formatter.write_indent();
    write_availability(&class.availability, formatter);
    if class.attributes.contains(AstThingAttributes::Sealed) {
        formatter.write(b"sealed ");
    }
    formatter.write(b"class ");
    formatter.write_identifier(&class.name);
    if let Some(tp) = &class.type_parameters {
        write_type_parameters(tp, formatter);
    }
    if !class.superclass.is_empty() {
        formatter.write(b" extends ");
        for (i, sc) in class.superclass.iter().enumerate() {
            if i > 0 {
                formatter.write(b", ");
            }
            if let AstSuperClass::Name(ident) = sc {
                formatter.write_identifier(ident);
            }
        }
    }
    if !class.implements.is_empty() {
        formatter.write(b" implements ");
        for (i, jtype) in class.implements.iter().enumerate() {
            if i > 0 {
                formatter.write(b", ");
            }
            write_jtype(jtype, formatter);
        }
    }
    if !class.permits.is_empty() {
        formatter.write(b" permits ");
        for (i, jtype) in class.permits.iter().enumerate() {
            if i > 0 {
                formatter.write(b", ");
            }
            write_jtype(jtype, formatter);
        }
    }
    formatter.buf.push(b' ');
    write_class_block_braced(&class.block, formatter);
    formatter.buf.push(b'\n');
}

fn write_class_block_braced(block: &AstClassBlock, formatter: &mut Formatter) {
    formatter.write(b"{");
    formatter.buf.push(b'\n');
    formatter.indent += 1;
    write_class_block(block, formatter);
    formatter.indent -= 1;
    formatter.write_indent();
    formatter.write(b"}");
}

fn write_class_block(block: &AstClassBlock, formatter: &mut Formatter) {
    let mut members: Vec<(AstPoint, u8, usize)> = Vec::new();
    for (i, v) in block.variables.iter().enumerate() {
        members.push((v.range.start, 0, i));
    }
    for (i, m) in block.methods.iter().enumerate() {
        members.push((m.range.start, 1, i));
    }
    for (i, c) in block.constructors.iter().enumerate() {
        members.push((c.range.start, 2, i));
    }
    for (i, s) in block.static_blocks.iter().enumerate() {
        members.push((s.range.start, 3, i));
    }
    for (i, t) in block.inner.iter().enumerate() {
        members.push((t.get_range().start, 4, i));
    }
    for (i, b) in block.blocks.iter().enumerate() {
        members.push((b.range.start, 5, i));
    }
    members.sort_by(|(a, _, _), (b, _, _)| a.line.cmp(&b.line).then(a.col.cmp(&b.col)));
    for (_, type_id, idx) in &members {
        match type_id {
            0 => write_class_variable(&block.variables[*idx], formatter),
            1 => write_class_method(&block.methods[*idx], formatter),
            2 => write_class_constructor(&block.constructors[*idx], formatter),
            3 => {
                formatter.write_indent();
                formatter.write(b"static ");
                write_block(&block.static_blocks[*idx].block, formatter);
                formatter.buf.push(b'\n');
            }
            4 => write_thing(&block.inner[*idx], formatter),
            5 => {
                formatter.write_indent();
                write_block(&block.blocks[*idx], formatter);
                formatter.buf.push(b'\n');
            }
            _ => unreachable!(),
        }
    }
}

fn write_class_variable(v: &AstClassVariable, formatter: &mut Formatter) {
    for ann in &v.annotated {
        formatter.write_indent();
        write_annotation(ann, formatter);
    }
    formatter.write_indent();
    write_availability(&v.availability, formatter);
    if v.volatile_transient
        .contains(AstVolatileTransient::Volatile)
    {
        formatter.write(b"volatile ");
    }
    if v.volatile_transient
        .contains(AstVolatileTransient::Transient)
    {
        formatter.write(b"transient ");
    }
    write_jtype(&v.jtype, formatter);
    formatter.buf.push(b' ');
    formatter.write_identifier(&v.name);
    if let Some(expr) = &v.expression {
        formatter.write(b" = ");
        write_expression(expr, formatter);
    }
    formatter.write(b";");
    formatter.buf.push(b'\n');
}

fn write_class_method(method: &AstClassMethod, formatter: &mut Formatter) {
    write_method_header(&method.header, formatter);
    if let Some(block) = &method.block {
        formatter.buf.push(b' ');
        write_block(block, formatter);
        formatter.buf.push(b'\n');
    } else {
        formatter.write(b";");
        formatter.buf.push(b'\n');
    }
}

fn write_class_constructor(c: &AstClassConstructor, formatter: &mut Formatter) {
    for ann in &c.header.annotated {
        formatter.write_indent();
        write_annotation(ann, formatter);
    }
    formatter.write_indent();
    write_availability(&c.header.availability, formatter);
    if let Some(tp) = &c.header.type_parameters {
        write_type_parameters(tp, formatter);
        formatter.buf.push(b' ');
    }
    formatter.write_identifier(&c.header.name);
    write_method_parameters(&c.header.parameters, formatter);
    if let Some(throws) = &c.header.throws {
        write_throws(throws, formatter);
    }
    formatter.buf.push(b' ');
    write_block(&c.block, formatter);
    formatter.buf.push(b'\n');
}

fn write_method_header(header: &AstMethodHeader, formatter: &mut Formatter) {
    for ann in &header.annotated {
        formatter.write_indent();
        write_annotation(ann, formatter);
    }
    formatter.write_indent();
    write_availability(&header.availability, formatter);
    if let Some(tp) = &header.type_parameters {
        write_type_parameters(tp, formatter);
        formatter.buf.push(b' ');
    }
    write_jtype(&header.jtype, formatter);
    formatter.buf.push(b' ');
    formatter.write_identifier(&header.name);
    write_method_parameters(&header.parameters, formatter);
    if let Some(throws) = &header.throws {
        write_throws(throws, formatter);
    }
}

fn write_method_parameters(params: &AstMethodParameters, formatter: &mut Formatter) {
    const NEWLINE: usize = 3;
    formatter.write(b"(");
    if params.parameters.len() > NEWLINE {
        formatter.indent += 1;
    }
    for (i, param) in params.parameters.iter().enumerate() {
        if i > 0 {
            formatter.write(b",");
        }
        if params.parameters.len() > NEWLINE {
            formatter.buf.push(b'\n');
            formatter.write_indent();
        } else {
            if i > 0 {
                formatter.write(b" ");
            }
        }
        for ann in &param.annotated {
            write_annotation(ann, formatter);
        }
        if param.flags.contains(AstMethodParameterFlags::Fin) {
            formatter.write(b"final ");
        }
        write_jtype(&param.jtype, formatter);
        if param.flags.contains(AstMethodParameterFlags::Variatic) {
            formatter.write(b".");
            formatter.write(b".");
            formatter.write(b".");
        }
        formatter.buf.push(b' ');
        formatter.write_identifier(&param.name);
    }
    if params.parameters.len() > NEWLINE {
        formatter.indent -= 1;
        formatter.buf.push(b'\n');
        formatter.write_indent();
    }
    formatter.write(b")");
}

fn write_throws(throws: &AstThrowsDeclaration, formatter: &mut Formatter) {
    formatter.write(b" throws ");
    for (i, t) in throws.parameters.iter().enumerate() {
        if i > 0 {
            formatter.write(b", ");
        }
        write_jtype(t, formatter);
    }
}

fn write_type_parameters(tp: &AstTypeParameters, formatter: &mut Formatter) {
    formatter.write(b"<");
    for (i, param) in tp.parameters.iter().enumerate() {
        if i > 0 {
            formatter.write(b", ");
        }
        for ann in &param.annotated {
            write_annotation(ann, formatter);
        }
        formatter.write_identifier(&param.name);
        if let Some(superclasses) = &param.supperclass
            && !superclasses.is_empty()
        {
            formatter.write(b" extends ");
            for (j, sc) in superclasses.iter().enumerate() {
                if j > 0 {
                    formatter.write(b" & ");
                }
                if let AstSuperClass::Name(ident) = sc {
                    formatter.write_identifier(ident);
                }
            }
        }
        formatter.skip_to(param.range.end);
    }
    formatter.write(b">");
}

fn write_record(record: &AstRecord, formatter: &mut Formatter) {
    for ann in &record.annotated {
        formatter.write_indent();
        write_annotation(ann, formatter);
    }
    formatter.write_indent();
    write_availability(&record.availability, formatter);
    formatter.write(b"record ");
    formatter.write_identifier(&record.name);
    if let Some(tp) = &record.type_parameters {
        write_type_parameters(tp, formatter);
    }
    write_record_entries(&record.record_entries, formatter);
    if !record.implements.is_empty() {
        formatter.write(b" implements ");
        for (i, jtype) in record.implements.iter().enumerate() {
            if i > 0 {
                formatter.write(b", ");
            }
            write_jtype(jtype, formatter);
        }
    }
    formatter.buf.push(b' ');
    write_class_block_braced(&record.block, formatter);
    formatter.buf.push(b'\n');
}

fn write_record_entries(entries: &AstRecordEntries, formatter: &mut Formatter) {
    formatter.write(b"(");
    for (i, entry) in entries.entries.iter().enumerate() {
        if i > 0 {
            formatter.write(b", ");
        }
        for ann in &entry.annotated {
            write_annotation(ann, formatter);
        }
        write_jtype(&entry.jtype, formatter);
        if entry.variadic {
            formatter.write(b".");
            formatter.write(b".");
            formatter.write(b".");
        }
        formatter.buf.push(b' ');
        formatter.write_identifier(&entry.name);
    }
    formatter.write(b")");
}

fn write_interface(iface: &AstInterface, formatter: &mut Formatter) {
    for ann in &iface.annotated {
        formatter.write_indent();
        write_annotation(ann, formatter);
    }
    formatter.write_indent();
    write_availability(&iface.availability, formatter);
    if iface.attributes.contains(AstThingAttributes::Sealed) {
        formatter.write(b"sealed ");
    }
    formatter.write(b"interface ");
    formatter.write_identifier(&iface.name);
    if let Some(tp) = &iface.type_parameters {
        write_type_parameters(tp, formatter);
    }
    if let Some(extends) = &iface.extends {
        formatter.write(b" extends ");
        for (i, jtype) in extends.parameters.iter().enumerate() {
            if i > 0 {
                formatter.write(b", ");
            }
            write_jtype(jtype, formatter);
        }
    }
    if !iface.permits.is_empty() {
        formatter.write(b" permits ");
        for (i, jtype) in iface.permits.iter().enumerate() {
            if i > 0 {
                formatter.write(b", ");
            }
            write_jtype(jtype, formatter);
        }
    }
    formatter.buf.push(b' ');
    formatter.write(b"{");
    formatter.buf.push(b'\n');
    formatter.indent += 1;
    let mut members: Vec<(AstPoint, u8, usize)> = Vec::new();
    for (i, c) in iface.constants.iter().enumerate() {
        members.push((c.range.start, 0, i));
    }
    for (i, m) in iface.methods.iter().enumerate() {
        members.push((m.range.start, 1, i));
    }
    for (i, d) in iface.default_methods.iter().enumerate() {
        members.push((d.range.start, 2, i));
    }
    for (i, t) in iface.inner.iter().enumerate() {
        members.push((t.get_range().start, 3, i));
    }
    members.sort_by(|(a, _, _), (b, _, _)| a.line.cmp(&b.line).then(a.col.cmp(&b.col)));
    for (_, type_id, idx) in &members {
        match type_id {
            0 => write_interface_constant(&iface.constants[*idx], formatter),
            1 => write_interface_method(&iface.methods[*idx], formatter),
            2 => write_interface_default_method(&iface.default_methods[*idx], formatter),
            3 => write_thing(&iface.inner[*idx], formatter),
            _ => unreachable!(),
        }
    }
    formatter.indent -= 1;
    formatter.write_indent();
    formatter.write(b"}");
    formatter.buf.push(b'\n');
}

fn write_interface_constant(c: &AstInterfaceConstant, formatter: &mut Formatter) {
    for ann in &c.annotated {
        formatter.write_indent();
        write_annotation(ann, formatter);
    }
    formatter.write_indent();
    write_availability(&c.availability, formatter);
    write_jtype(&c.jtype, formatter);
    formatter.buf.push(b' ');
    formatter.write_identifier(&c.name);
    if let Some(expr) = &c.expression {
        formatter.write(b" = ");
        write_expression(expr, formatter);
    }
    formatter.write(b";");
    formatter.buf.push(b'\n');
}

fn write_interface_method(m: &AstInterfaceMethod, formatter: &mut Formatter) {
    for ann in &m.annotated {
        formatter.write_indent();
        write_annotation(ann, formatter);
    }
    write_method_header(&m.header, formatter);
    formatter.write(b";");
    formatter.buf.push(b'\n');
}

fn write_interface_default_method(m: &AstInterfaceMethodDefault, formatter: &mut Formatter) {
    for ann in &m.annotated {
        formatter.write_indent();
        write_annotation(ann, formatter);
    }
    write_method_header(&m.header, formatter);
    formatter.buf.push(b' ');
    write_block(&m.block, formatter);
    formatter.buf.push(b'\n');
}

fn write_enumeration(e: &AstEnumeration, formatter: &mut Formatter) {
    for ann in &e.annotated {
        formatter.write_indent();
        write_annotation(ann, formatter);
    }
    formatter.write_indent();
    write_availability(&e.availability, formatter);
    formatter.write(b"enum ");
    formatter.write_identifier(&e.name);
    if !e.implements.is_empty() {
        formatter.write(b" implements ");
        for (i, jtype) in e.implements.iter().enumerate() {
            if i > 0 {
                formatter.write(b", ");
            }
            write_jtype(jtype, formatter);
        }
    }
    formatter.buf.push(b' ');
    formatter.write(b"{");
    formatter.buf.push(b'\n');
    formatter.indent += 1;
    for (i, variant) in e.variants.iter().enumerate() {
        if i > 0 {
            formatter.write(b",");
            formatter.buf.push(b'\n');
        }
        formatter.write_indent();
        for ann in &variant.annotated {
            write_annotation(ann, formatter);
        }
        formatter.write_identifier(&variant.name);
        if !variant.parameters.is_empty() {
            formatter.write(b"(");
            for (j, param) in variant.parameters.iter().enumerate() {
                if j > 0 {
                    formatter.write(b", ");
                }
                write_expression(param, formatter);
            }
            formatter.write(b")");
        }
    }
    let has_members = !e.methods.is_empty()
        || !e.variables.is_empty()
        || !e.constructors.is_empty()
        || !e.static_blocks.is_empty()
        || !e.inner.is_empty();
    if has_members {
        formatter.write(b";");
        formatter.buf.push(b'\n');
        let block = AstClassBlock {
            variables: e.variables.clone(),
            methods: e.methods.clone(),
            constructors: e.constructors.clone(),
            static_blocks: e.static_blocks.clone(),
            inner: e.inner.clone(),
            blocks: vec![],
        };
        write_class_block(&block, formatter);
    } else if !e.variants.is_empty() {
        formatter.buf.push(b'\n');
    }
    formatter.indent -= 1;
    formatter.write_indent();
    formatter.write(b"}");
    formatter.buf.push(b'\n');
}

fn write_annotation_type(ann_type: &AstAnnotation, formatter: &mut Formatter) {
    for ann in &ann_type.annotated {
        formatter.write_indent();
        write_annotation(ann, formatter);
    }
    formatter.write_indent();
    write_availability(&ann_type.availability, formatter);
    formatter.write(b"@");
    formatter.write(b"interface ");
    formatter.write_identifier(&ann_type.name);
    formatter.buf.push(b' ');
    formatter.write(b"{");
    formatter.buf.push(b'\n');
    formatter.indent += 1;
    for field in &ann_type.fields {
        write_annotation_field(field, formatter);
    }
    for inner in &ann_type.inner {
        write_thing(inner, formatter);
    }
    formatter.indent -= 1;
    formatter.write_indent();
    formatter.write(b"}");
    formatter.buf.push(b'\n');
}

fn write_annotation_field(field: &AstAnnotationField, formatter: &mut Formatter) {
    for ann in &field.annotated {
        formatter.write_indent();
        write_annotation(ann, formatter);
    }
    formatter.write_indent();
    write_availability(&field.availability, formatter);
    write_jtype(&field.jtype, formatter);
    formatter.buf.push(b' ');
    formatter.write_identifier(&field.name);
    formatter.write(b"(");
    formatter.write(b")");
    if let Some(expr) = &field.expression {
        formatter.write(b" default ");
        write_expression(expr, formatter);
    }
    formatter.write(b";");
    formatter.buf.push(b'\n');
}

fn write_expression_identifier(ident: &AstExpressionIdentifier, formatter: &mut Formatter) {
    match ident {
        AstExpressionIdentifier::Identifier(i) => formatter.write_identifier(i),
        AstExpressionIdentifier::Nuget(n) => write_value_nuget(n, formatter),
        AstExpressionIdentifier::Value(v) => write_value(v, formatter),
        AstExpressionIdentifier::ArrayAccess { expr, .. } => {
            formatter.write(b"[");
            write_expression(expr, formatter);
            formatter.write(b"]");
        }

        AstExpressionIdentifier::EmptyArrayAccess(_) => {
            formatter.write(b"[]");
        }
    }
}

fn write_value(value: &AstValue, formatter: &mut Formatter) {
    match value {
        AstValue::Variable(i) => formatter.write_identifier(i),
        AstValue::Nuget(n) => write_value_nuget(n, formatter),
    }
}

fn write_value_nuget(nuget: &AstValueNuget, formatter: &mut Formatter) {
    match nuget {
        AstValueNuget::Int(i) => {
            formatter.write_with_comments(i.range.start, i.value.as_bytes());
            formatter.skip_to(i.range.end);
        }
        AstValueNuget::Long(l) => {
            formatter.write_with_comments(l.range.start, l.value.as_bytes());
            formatter.skip_to(l.range.end);
            formatter.write(b"L");
        }
        AstValueNuget::Double(d) => {
            formatter.write_with_comments(d.range.start, d.value.as_bytes());
            formatter.skip_to(d.range.end);
        }
        AstValueNuget::Float(f) => {
            formatter.write_with_comments(f.range.start, f.value.as_bytes());
            formatter.skip_to(f.range.end);
            formatter.write(b"f");
        }
        AstValueNuget::StringLiteral(s) => {
            formatter.write_with_comments(s.range.start, b"\"");
            formatter.buf.extend_from_slice(s.value.as_bytes());
            formatter.buf.extend_from_slice(b"\"");
            formatter.skip_to(s.range.end);
        }
        AstValueNuget::CharLiteral(c) => {
            formatter.write_with_comments(c.range.start, b"'");
            formatter.buf.extend_from_slice(c.value.as_bytes());
            formatter.buf.extend_from_slice(b"'");
            formatter.skip_to(c.range.end);
        }
        AstValueNuget::BooleanLiteral(b) => {
            formatter.write_with_comments(b.range.start, if b.value { b"true" } else { b"false" });
            formatter.skip_to(b.range.end);
        }
        AstValueNuget::HexLiteral(h) => {
            formatter.write_with_comments(h.range.start, b"0x");
            formatter.buf.extend_from_slice(h.value.as_bytes());
            formatter.skip_to(h.range.end);
        }
        AstValueNuget::BinaryLiteral(b) => {
            formatter.write_with_comments(b.range.start, b"0b");
            formatter.buf.extend_from_slice(b.value.as_bytes());
            formatter.skip_to(b.range.end);
        }
    }
}

fn write_expression_operator(
    op: &AstExpressionOperator,
    formatter: &mut Formatter,
    is_large: bool,
) {
    let (r, bytes): (_, &[u8]) = match op {
        AstExpressionOperator::None => return,
        AstExpressionOperator::Dot(r) => (r, b"."),
        AstExpressionOperator::Plus(r) => (r, b" + "),
        AstExpressionOperator::PlusPlus(r) => (r, b"++ "),
        AstExpressionOperator::Minus(r) => (r, b" - "),
        AstExpressionOperator::MinusMinus(r) => (r, b" -- "),
        AstExpressionOperator::Multiply(r) => (r, b" * "),
        AstExpressionOperator::Divide(r) => (r, b" / "),
        AstExpressionOperator::Modulo(r) => (r, b" % "),
        AstExpressionOperator::Equal(r) => (r, b" == "),
        AstExpressionOperator::NotEqual(r) => (r, b" != "),
        AstExpressionOperator::Le(r) => (r, b" <= "),
        AstExpressionOperator::Lt(r) => (r, b" < "),
        AstExpressionOperator::Ge(r) => (r, b" >= "),
        AstExpressionOperator::Gt(r) => (r, b" > "),
        AstExpressionOperator::ExclamationMark(r) => (r, b"!"),
        AstExpressionOperator::Ampersand(r) => (r, b" & "),
        AstExpressionOperator::AmpersandAmpersand(r) => (r, b" && "),
        AstExpressionOperator::VerticalBar(r) => (r, b" | "),
        AstExpressionOperator::VerticalBarVerticalBar(r) => (r, b" || "),
        AstExpressionOperator::QuestionMark(r) => (r, b" ? "),
        AstExpressionOperator::Colon(r) => (r, b" : "),
        AstExpressionOperator::ColonColon(r) => (r, b"::"),
        AstExpressionOperator::Assign(r) => (r, b" = "),
        AstExpressionOperator::Tilde(r) => (r, b"~"),
        AstExpressionOperator::Caret(r) => (r, b" ^ "),
    };
    if is_large
        && matches!(
            op,
            AstExpressionOperator::VerticalBarVerticalBar(_)
                | AstExpressionOperator::AmpersandAmpersand(_)
                | AstExpressionOperator::Plus(_)
                | AstExpressionOperator::Minus(_)
        )
    {
        formatter.buf.push(b'\n');
        formatter.write_indent();
    }
    formatter.write_with_comments(r.start, bytes);
    formatter.skip_to(r.end);
}

fn write_import(import: &AstImport, formatter: &mut Formatter) {
    formatter.write(b"import ");
    match &import.unit {
        AstImportUnit::Class(ident) => {
            formatter.write_identifier(ident);
        }
        AstImportUnit::StaticClass(ident) => {
            formatter.write(b"static ");
            formatter.write_identifier(ident);
        }
        AstImportUnit::StaticClassMethod(class, method) => {
            formatter.write(b"static ");
            formatter.write_identifier(class);
            formatter.buf.push(b'.');
            formatter.write_identifier(method);
        }
        AstImportUnit::Prefix(ident) => {
            formatter.write_identifier(ident);
            formatter.write(b".");
            formatter.write(b"*");
        }
        AstImportUnit::StaticPrefix(ident) => {
            formatter.write(b"static ");
            formatter.write_identifier(ident);
            formatter.write(b".");
            formatter.write(b"*");
        }
    }
    formatter.write(b";");
    formatter.buf.push(b'\n');
}

fn write_module(module: &AstModule, formatter: &mut Formatter) {
    for ann in &module.annotated {
        write_annotation(ann, formatter);
    }
    if module.open {
        formatter.write(b"open ");
    }
    formatter.write(b"module ");
    formatter.write_identifier(&module.name);
    formatter.buf.push(b' ');
    formatter.write(b"{");
    formatter.buf.push(b'\n');
    formatter.indent += 1;

    let mut entries: Vec<(AstPoint, u8, usize)> = Vec::new();
    for (i, e) in module.exports.iter().enumerate() {
        entries.push((e.range.start, 0, i));
    }
    for (i, o) in module.opens.iter().enumerate() {
        entries.push((o.range.start, 1, i));
    }
    for (i, u) in module.uses.iter().enumerate() {
        entries.push((u.range.start, 2, i));
    }
    for (i, p) in module.provides.iter().enumerate() {
        entries.push((p.range.start, 3, i));
    }
    for (i, r) in module.requires.iter().enumerate() {
        entries.push((r.range.start, 4, i));
    }
    entries.sort_by(|(a, _, _), (b, _, _)| a.line.cmp(&b.line).then(a.col.cmp(&b.col)));

    for (_, type_id, idx) in &entries {
        formatter.write_indent();
        match type_id {
            0 => {
                let e = &module.exports[*idx];
                formatter.write(b"exports ");
                formatter.write_identifier(&e.name);
                if !e.to.is_empty() {
                    formatter.write(b" to ");
                    for (i, t) in e.to.iter().enumerate() {
                        if i > 0 {
                            formatter.write(b", ");
                        }
                        formatter.write_identifier(t);
                    }
                }
                formatter.write(b";");
                formatter.buf.push(b'\n');
            }
            1 => {
                let o = &module.opens[*idx];
                formatter.write(b"opens ");
                formatter.write_identifier(&o.name);
                if !o.to.is_empty() {
                    formatter.write(b" to ");
                    for (i, t) in o.to.iter().enumerate() {
                        if i > 0 {
                            formatter.write(b", ");
                        }
                        formatter.write_identifier(t);
                    }
                }
                formatter.write(b";");
                formatter.buf.push(b'\n');
            }
            2 => {
                let u = &module.uses[*idx];
                formatter.write(b"uses ");
                formatter.write_identifier(&u.name);
                formatter.write(b";");
                formatter.buf.push(b'\n');
            }
            3 => {
                let p = &module.provides[*idx];
                formatter.write(b"provides ");
                formatter.write_identifier(&p.name);
                formatter.write(b" with ");
                for (i, w) in p.with.iter().enumerate() {
                    if i > 0 {
                        formatter.write(b", ");
                    }
                    formatter.write_identifier(w);
                }
                formatter.write(b";");
                formatter.buf.push(b'\n');
            }
            4 => {
                let r = &module.requires[*idx];
                formatter.write(b"requires ");
                if r.flags.contains(AstModuleRequiresFlags::Transitive) {
                    formatter.write(b"transitive ");
                }
                if r.flags.contains(AstModuleRequiresFlags::Static) {
                    formatter.write(b"static ");
                }
                formatter.write_identifier(&r.name);
                formatter.write(b";");
                formatter.buf.push(b'\n');
            }
            _ => unreachable!(),
        }
    }

    formatter.indent -= 1;
    formatter.write_indent();
    formatter.write(b"}");
    formatter.buf.push(b'\n');
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;

    #[test]
    fn package() {
        let content = b"// This is a cool file

        @Thing(Type.IMPORTANT)
        @Retention(RetentionPolicy.RUNTIME)
        package ch.emilycares;

        // Now the imports
        ";

        let o = internal(content).unwrap();
        let expected = expect![[r#"
            // This is a cool file
            @Thing(Type.IMPORTANT)
            @Retention(RetentionPolicy.RUNTIME)
            package ch.emilycares;
        "#]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn method() {
        let content = br#"package ch.emilycares;
        /**
         * hostile getter
         */
        @NonNull
        public static int isHostile(@NonNull Gremlin g) {
            if (g.name().equals("thorben")) {
                return false;
            }
            
            return true;
        }
        "#;

        let o = internal(content).unwrap();
        let expected = expect![[r#"
            package ch.emilycares;
            /**
                     * hostile getter
                     */
            @NonNull
            public static int isHostile(@NonNull Gremlin g) {
                if (g.name().equals("thorben")) {
                    return false;
                }
                return true;
            }
        "#]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn method_parameters() {
        let content = br#"
        package ch.emilycares;
        public class Application {

            private final Database db;
            private final Messages msgs;
            private final Telemetry tel;
            private final Obeservability obs;

            public Application(Database db, Messages msgs, Telemetry tel, Obeservability obs) {
                this.db = db;
                this.msgs = msgs;
                this.tel = tel;
                this.obs = obs;
            }
        }
        "#;

        let o = internal(content).unwrap();
        let expected = expect![[r#"
            package ch.emilycares;
            public class Application {
                private final Database db;
                private final Messages msgs;
                private final Telemetry tel;
                private final Obeservability obs;
                public Application(
                    Database db,
                    Messages msgs,
                    Telemetry tel,
                    Obeservability obs
                ) {
                    this.db = db;
                    this.msgs = msgs;
                    this.tel = tel;
                    this.obs = obs;
                }
            }
        "#]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }
    #[test]
    fn enum_base() {
        let content = br#"
        package ch.emilycares;
        public enum EType {
            A, B,
            C;
            
            public int aaa() {
               switch (this) {
                   case A:
                      return 1
                   case B:
                      return 2
                   case C: {
                      return 2
                      }
               }
            }
        }
        "#;

        let o = internal(content).unwrap();
        let expected = expect![[r#"
            package ch.emilycares;
            public enum EType {
                A,
                B,
                C;
                public int aaa() {
                    switch (this) {
                        case A:
                        return 1;
                        case B:
                        return 2;
                        case C:
                        {
                            return 2;
                        }
                    }
                }
            }
        "#]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn long_plus() {
        let content = br#"
        package ch.emilycares;
        public class Test {
            public int aaa() {
                 int b = 1;
                 return this.that + this.other + this.taetsch 
                 + this.boing + b;
            }
        }
        "#;

        let o = internal(content).unwrap();
        let expected = expect![[r#"
            package ch.emilycares;
            public class Test {
                public int aaa() {
                    int b = 1;
                    return this.that
                         + this.other
                         + this.taetsch
                         + this.boing
                         + b;
                }
            }
        "#]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn for_base() {
        let content = br#"
        package ch.emilycares;
        public class Test {
            public int aaa() {
                for (int i; i < 5; i++) {
                log.info("hihihaha")

                }

            }
        }
        "#;

        let o = internal(content).unwrap();
        let expected = expect![[r#"
            package ch.emilycares;
            public class Test {
                public int aaa() {
                    for (int i; i < 5; i++ ) {
                        log.info("hihihaha");
                    }
                }
            }
        "#]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }
}
