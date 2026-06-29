#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(clippy::redundant_clone)]
#![deny(clippy::enum_glob_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::too_many_lines)]
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
#[must_use]
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
    space: &str,
) -> Result<Option<Vec<u8>>, FormatError> {
    match formatter {
        FormatterConfig::None => Err(FormatError::NoFormatterSpecified),
        FormatterConfig::Internal => internal(content, space),
        FormatterConfig::Google => google::google_java_format(content),
        FormatterConfig::Idea => idea::idea_java_format(path, project_dir),
    }
}

fn internal(content: &[u8], space: &str) -> Result<Option<Vec<u8>>, FormatError> {
    let mut f = Formatter::new(content, space)?;
    let tokens = ast::lexer::lex_v::<false>(content).map_err(FormatError::Lexer)?;
    let ast = ast::parse_file(&tokens).map_err(FormatError::Ast)?;

    let mut top = ast.top.iter().peekable();

    while let Some(t) = top.next() {
        match t {
            AstTopLevel::Package(p) => {
                write_package(p, &mut f);
            }
            AstTopLevel::Import(ast_import) => write_import(ast_import, &mut f),
            AstTopLevel::Thing(ast_thing) => write_thing(ast_thing, &mut f),
            AstTopLevel::Method(ast_class_method) => {
                write_class_method(ast_class_method, &mut f);
            }
            AstTopLevel::Module(ast_module) => write_module(ast_module, &mut f),
        }
        if let Some(next) = top.peek() {
            f.insert_new_lines(t.get_range().end.line, next.get_range().start.line);
        }
    }

    Ok(Some(f.buf))
}

struct Formatter {
    pub with_comments: Vec<PositionToken>,
    pub index: usize,
    pub buf: Vec<u8>,
    pub indent: usize,
    pub space: String,
}

impl Formatter {
    pub fn new(content: &[u8], space: &str) -> Result<Self, FormatError> {
        let with_comments = ast::lexer::lex_v::<true>(content).map_err(FormatError::Lexer)?;
        Ok(Self {
            with_comments,
            index: 0,
            buf: Vec::new(),
            indent: 0,
            space: space.to_string(),
        })
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.buf.extend_from_slice(self.space.as_bytes());
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
                    if self.insert_line_or_space() {
                        self.write_indent();
                    }
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
                    if self.insert_line_or_space() {
                        self.write_indent();
                    }
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

    pub fn insert_line_or_space(&mut self) -> bool {
        let last_line = self
            .index
            .checked_sub(1)
            .and_then(|i| self.with_comments.get(i))
            .map(|t| t.line);
        let next_line = self.with_comments.get(self.index + 1).map(|t| t.line);
        if matches!((last_line, next_line), (Some(a), Some(b)) if a == b) {
            self.buf.push(b' ');
            false
        } else {
            self.new_line();
            true
        }
    }

    pub fn skip_to(&mut self, up_to: AstPoint) {
        loop {
            let pos = self
                .with_comments
                .get(self.index)
                .map(PositionToken::start_point);
            let Some(pos) = pos else { break };
            if pos >= up_to {
                break;
            }
            self.index += 1;
        }
    }

    pub fn insert_new_lines(&mut self, end_line: usize, next_line: usize) {
        let lns = next_line.saturating_sub(end_line) > 1;
        if lns {
            self.new_line();
        }
    }

    #[inline]
    fn new_line(&mut self) {
        self.buf.push(b'\n');
    }
}

fn write_package(p: &AstPackage, f: &mut Formatter) {
    for ann in &p.annotated {
        write_annotation(ann, f);
    }
    f.write(b"package ");
    f.write_identifier(&p.name);
    f.write(b";");
    f.new_line();
}

fn write_annotation(ann: &AstAnnotated, f: &mut Formatter) {
    f.write(b"@");
    f.write_identifier(&ann.name);
    match &ann.parameters {
        AstAnnotatedParameterKind::None => {}
        AstAnnotatedParameterKind::Parameter(params) => {
            f.write(b"(");
            for (i, param) in params.iter().enumerate() {
                if i > 0 {
                    f.write(b", ");
                }
                write_annotation_parameter(param, f);
            }
            f.write(b")");
        }
        AstAnnotatedParameterKind::Array(values) => {
            f.write(b"(");
            write_values_with_annotated(values, f);
            f.write(b")");
        }
    }
    f.insert_line_or_space();
}

fn write_annotation_parameter(param: &AstAnnotatedParameter, f: &mut Formatter) {
    match param {
        AstAnnotatedParameter::Expression(expr) => {
            write_expression(expr, f);
        }
        AstAnnotatedParameter::NamedExpression {
            name, expression, ..
        } => {
            f.write_identifier(name);
            f.write(b" = ");
            write_expression(expression, f);
        }
        AstAnnotatedParameter::Annotated(ann) => {
            write_annotation(ann, f);
        }
        AstAnnotatedParameter::NamedArray { name, values, .. } => {
            f.write_identifier(name);
            f.write(b" = ");
            write_values_with_annotated(values, f);
        }
        AstAnnotatedParameter::NamedAnnotated {
            name, annotated, ..
        } => {
            f.write_identifier(name);
            f.write(b" = ");
            write_annotation(annotated, f);
        }
    }
}

fn write_values_with_annotated(values: &AstValuesWithAnnotated, f: &mut Formatter) {
    f.write(b"{");
    for (i, v) in values.values.iter().enumerate() {
        if i > 0 {
            f.write(b", ");
        }
        match v {
            AstExpressionOrAnnotated::Expression(expr) => write_expression(expr, f),
            AstExpressionOrAnnotated::Annotated(ann) => write_annotation(ann, f),
        }
    }
    f.write(b"}");
}

fn write_expression(expr: &[AstExpressionKind], f: &mut Formatter) {
    let mut is_large = false;
    let range = expr.get_range();
    if range.start.line != range.end.line {
        let ops = expr
            .iter()
            .filter_map(|i| {
                if let AstExpressionKind::Base(b) = i {
                    return Some(b);
                }
                None
            })
            .filter(|i| {
                matches!(
                    i.operator,
                    AstExpressionOperator::VerticalBarVerticalBar(_)
                        | AstExpressionOperator::AmpersandAmpersand(_)
                        | AstExpressionOperator::Plus(_)
                        | AstExpressionOperator::Minus(_)
                        | AstExpressionOperator::Dot(_)
                )
            })
            .count();
        is_large = ops != 1;
    }

    let dot = !expr
        .iter()
        .filter_map(|i| {
            if let AstExpressionKind::Base(b) = i {
                return Some(b);
            }
            None
        })
        .any(|i| {
            matches!(
                i.operator,
                AstExpressionOperator::VerticalBarVerticalBar(_)
                    | AstExpressionOperator::AmpersandAmpersand(_)
                    | AstExpressionOperator::Plus(_)
                    | AstExpressionOperator::Minus(_)
            )
        });

    if is_large {
        f.indent += 1;
    }

    for (nth, k) in expr.iter().enumerate() {
        write_expression_kind(k, f, is_large, dot, nth);
    }
    if is_large {
        f.indent -= 1;
    }
}

fn write_expression_kind(
    kind: &AstExpressionKind,
    f: &mut Formatter,
    is_large: bool,
    dot: bool,
    nth: usize,
) {
    match kind {
        AstExpressionKind::Base(base) => {
            if let Some(ident) = &base.ident {
                write_expression_identifier(ident, f);
            }
            if let Some(values) = &base.values {
                let mut nl = values.range.start.line != values.range.end.line;
                if nl && values.values.len() == 1 {
                    let f = values.values.iter().flatten().next();
                    if matches!(f, Some(AstExpressionKind::Lambda(_))) {
                        nl = false;
                    }
                }
                f.write(b"(");
                for (i, expr) in values.values.iter().enumerate() {
                    if i > 0 {
                        f.write(b",");
                    }
                    if nl {
                        f.new_line();
                        f.write_indent();
                    } else if i > 0 {
                        f.write(b" ");
                    }
                    write_expression(expr, f);
                }
                if nl {
                    f.new_line();
                    f.write_indent();
                }
                f.write(b")");
            }
            write_expression_operator(&base.operator, f, is_large, dot, nth);
        }
        AstExpressionKind::JType(jtype_expr) => {
            write_jtype(&jtype_expr.jtype, f);
        }
        AstExpressionKind::Array(values) => {
            f.write(b"{");
            for (i, expr) in values.values.iter().enumerate() {
                if i > 0 {
                    f.write(b", ");
                }
                write_expression(expr, f);
            }
            f.write(b"}");
        }
        AstExpressionKind::Generics(generics) => {
            f.write(b"<");
            for (i, jtype) in generics.jtypes.iter().enumerate() {
                if i > 0 {
                    f.write(b", ");
                }
                write_jtype(jtype, f);
            }
            f.write(b">");
        }
        AstExpressionKind::InstanceOf(instanceof) => {
            f.write(b" instanceof ");
            if instanceof.availability.contains(AstAvailability::Final) {
                f.write(b"final ");
            }
            for ann in &instanceof.annotated {
                write_annotation(ann, f);
            }
            write_jtype(&instanceof.jtype, f);
            if let Some(var) = &instanceof.variable {
                f.buf.push(b' ');
                f.write_identifier(var);
            }
        }
        AstExpressionKind::Lambda(lambda) => {
            let has_brace = lambda.parameters.values.len() != 1;
            if has_brace {
                f.write(b"(");
            }
            for (i, param) in lambda.parameters.values.iter().enumerate() {
                if i > 0 {
                    f.write(b", ");
                }
                if let Some(jtype) = &param.jtype {
                    write_jtype(jtype, f);
                    f.buf.push(b' ');
                }
                f.write_identifier(&param.name);
            }
            if has_brace {
                f.write(b")");
            }
            f.write(b" -> ");
            match &lambda.rhs {
                AstLambdaRhs::None => {}
                AstLambdaRhs::Expr(expr) => write_expression(expr, f),
                AstLambdaRhs::Block(block) => write_block(block, f),
            }
        }
        AstExpressionKind::NewClass(new_class) => {
            f.write(b"new ");
            write_jtype(&new_class.jtype, f);
            match new_class.rhs.as_ref() {
                AstNewRhs::None => {}
                AstNewRhs::Parameters(_range, exprs) => {
                    f.write(b"(");
                    for (i, expr) in exprs.iter().enumerate() {
                        if i > 0 {
                            f.write(b", ");
                        }
                        write_expression(expr, f);
                    }
                    f.write(b")");
                }
                AstNewRhs::ParametersAndBlock(_range, exprs, block) => {
                    f.write(b"(");
                    for (i, expr) in exprs.iter().enumerate() {
                        if i > 0 {
                            f.write(b", ");
                        }
                        write_expression(expr, f);
                    }
                    f.write(b")");
                    f.buf.push(b' ');
                    write_class_block_braced(block, f);
                }
                AstNewRhs::ArrayParameters(param_groups) => {
                    for group in param_groups {
                        f.write(b"[");
                        for (i, expr) in group.iter().enumerate() {
                            if i > 0 {
                                f.write(b", ");
                            }
                            write_expression(expr, f);
                        }
                        f.write(b"]");
                    }
                }
                AstNewRhs::Array(values) => {
                    f.write(b"{");
                    for (i, expr) in values.values.iter().enumerate() {
                        if i > 0 {
                            f.write(b", ");
                        }
                        write_expression(expr, f);
                    }
                    f.write(b"}");
                }
                AstNewRhs::Block(block) => {
                    f.buf.push(b' ');
                    write_class_block_braced(block, f);
                }
            }
        }
        AstExpressionKind::InlineSwitch(switch) => {
            f.write(b"switch ");
            f.write(b"(");
            write_expression(&switch.check, f);
            f.write(b")");
            f.buf.push(b' ');
            write_block(&switch.block, f);
        }
    }
}

fn write_jtype(jtype: &AstJType, f: &mut Formatter) {
    for ann in &jtype.annotated {
        write_annotation(ann, f);
    }
    match &jtype.value {
        AstJTypeKind::Void => {
            f.write_with_comments(jtype.range.start, b"void");
            f.skip_to(jtype.range.end);
        }
        AstJTypeKind::Byte => {
            f.write_with_comments(jtype.range.start, b"byte");
            f.skip_to(jtype.range.end);
        }
        AstJTypeKind::Char => {
            f.write_with_comments(jtype.range.start, b"char");
            f.skip_to(jtype.range.end);
        }
        AstJTypeKind::Double => {
            f.write_with_comments(jtype.range.start, b"double");
            f.skip_to(jtype.range.end);
        }
        AstJTypeKind::Float => {
            f.write_with_comments(jtype.range.start, b"float");
            f.skip_to(jtype.range.end);
        }
        AstJTypeKind::Int => {
            f.write_with_comments(jtype.range.start, b"int");
            f.skip_to(jtype.range.end);
        }
        AstJTypeKind::Long => {
            f.write_with_comments(jtype.range.start, b"long");
            f.skip_to(jtype.range.end);
        }
        AstJTypeKind::Short => {
            f.write_with_comments(jtype.range.start, b"short");
            f.skip_to(jtype.range.end);
        }
        AstJTypeKind::Boolean => {
            f.write_with_comments(jtype.range.start, b"boolean");
            f.skip_to(jtype.range.end);
        }
        AstJTypeKind::Wildcard => {
            f.write_with_comments(jtype.range.start, b"?");
            f.skip_to(jtype.range.end);
        }
        AstJTypeKind::Var => {
            f.write_with_comments(jtype.range.start, b"var");
            f.skip_to(jtype.range.end);
        }
        AstJTypeKind::Class(ident) | AstJTypeKind::ClassOrPackage(ident) => {
            f.write_identifier(ident);
        }
        AstJTypeKind::Array(inner) => {
            write_jtype(inner, f);
            f.write(b"[");
            f.write(b"]");
        }
        AstJTypeKind::Generic(ident, types) => {
            f.write_identifier(ident);
            f.write(b"<");
            for (i, t) in types.iter().enumerate() {
                if i > 0 {
                    f.write(b", ");
                }
                write_jtype(t, f);
            }
            f.write(b">");
        }
        AstJTypeKind::Access { base, inner } => {
            write_jtype(base, f);
            f.write(b".");
            write_jtype(inner, f);
        }
    }
}

fn write_availability(availability: &AstAvailability, f: &mut Formatter) {
    if availability.contains(AstAvailability::Public) {
        f.write(b"public ");
    }
    if availability.contains(AstAvailability::Protected) {
        f.write(b"protected ");
    }
    if availability.contains(AstAvailability::Private) {
        f.write(b"private ");
    }
    if availability.contains(AstAvailability::Abstract) {
        f.write(b"abstract ");
    }
    if availability.contains(AstAvailability::Static) {
        f.write(b"static ");
    }
    if availability.contains(AstAvailability::Final) {
        f.write(b"final ");
    }
    if availability.contains(AstAvailability::Synchronized) {
        f.write(b"synchronized ");
    }
    if availability.contains(AstAvailability::Native) {
        f.write(b"native ");
    }
}

fn write_expr_or_value(eov: &AstExpressionOrValue, formatter: &mut Formatter) {
    match eov {
        AstExpressionOrValue::None => {}
        AstExpressionOrValue::Expression(expr) => write_expression(expr, formatter),
        AstExpressionOrValue::Value(val) => write_value(val, formatter),
    }
}

fn write_block(block: &AstBlock, f: &mut Formatter) {
    f.write(b"{");
    f.new_line();
    f.indent += 1;
    let base_indent = f.indent;
    let mut entries = block.entries.iter().peekable();
    let contains_case = block.entries.iter().any(|i| {
        matches!(
            i,
            AstBlockEntry::SwitchCase(_)
                | AstBlockEntry::SwitchCaseArrowType(_)
                | AstBlockEntry::SwitchCaseArrowValues(_)
                | AstBlockEntry::SwitchCaseArrowDefault(_)
        )
    });
    while let Some(entry) = entries.next() {
        let next_if = matches!(
            entries.peek(),
            Some(AstBlockEntry::If(AstIf::Else { .. } | AstIf::ElseIf { .. }))
        );
        if contains_case {
            if matches!(
                entry,
                AstBlockEntry::SwitchCase(_)
                    | AstBlockEntry::SwitchCaseArrowType(_)
                    | AstBlockEntry::SwitchCaseArrowValues(_)
                    | AstBlockEntry::SwitchCaseArrowDefault(_)
            ) {
                f.indent = base_indent;
            } else {
                f.indent = base_indent + 1;
            }
        }
        write_block_entry(entry, f, true, next_if);
        if let Some(next) = entries.peek() {
            f.insert_new_lines(entry.get_range().end.line, next.get_range().start.line);
        }
    }
    f.indent -= 1;
    f.write_indent();
    f.write(b"}");
}

fn write_block_entry(entry: &AstBlockEntry, f: &mut Formatter, around: bool, next_if: bool) {
    match entry {
        AstBlockEntry::Semicolon(_) => {
            if around {
                f.write_indent();
            }
            f.write(b";");
            if around {
                f.new_line();
            }
        }
        AstBlockEntry::Return(ret) => {
            if around {
                f.write_indent();
            }
            f.write(b"return");
            if !matches!(ret.expression, AstExpressionOrValue::None) {
                f.buf.push(b' ');
                write_expr_or_value(&ret.expression, f);
            }
            if around {
                f.write(b";");
                f.new_line();
            }
        }
        AstBlockEntry::Yield(yield_) => {
            if around {
                f.write_indent();
            }
            f.write(b"yield");
            if !matches!(yield_.expression, AstExpressionOrValue::None) {
                f.buf.push(b' ');
                write_expr_or_value(&yield_.expression, f);
            }
            if around {
                f.write(b";");
                f.new_line();
            }
        }
        AstBlockEntry::Throw(throw) => {
            if around {
                f.write_indent();
            }
            f.write(b"throw ");
            write_expression(&throw.expression, f);
            if around {
                f.write(b";");
                f.new_line();
            }
        }
        AstBlockEntry::Break(br) => {
            if around {
                f.write_indent();
            }
            f.write(b"break");
            if let Some(label) = &br.label {
                f.buf.push(b' ');
                f.write_identifier(label);
            }
            if around {
                f.write(b";");
                f.new_line();
            }
        }
        AstBlockEntry::Continue(cont) => {
            if around {
                f.write_indent();
            }
            f.write(b"continue");
            if let Some(label) = &cont.label {
                f.buf.push(b' ');
                f.write_identifier(label);
            }
            if around {
                f.write(b";");
                f.new_line();
            }
        }
        AstBlockEntry::Assert(assert) => {
            if around {
                f.write_indent();
            }
            f.write(b"assert ");
            write_expression(&assert.expression, f);
            if around {
                f.write(b";");
                f.new_line();
            }
        }
        AstBlockEntry::Expression(expr) => {
            if around {
                f.write_indent();
            }
            write_expression(&expr.value, f);
            if around {
                f.write(b";");
                f.new_line();
            }
        }
        AstBlockEntry::Assign(assign) => {
            if around {
                f.write_indent();
            }
            write_expression(&assign.key, f);
            f.write(b" = ");
            write_expression(&assign.expression, f);
            if around {
                f.write(b";");
                f.new_line();
            }
        }
        AstBlockEntry::Variable(vars) if !vars.is_empty() => {
            if around {
                f.write_indent();
            }
            for ann in &vars[0].annotated {
                write_annotation(ann, f);
                f.write_indent();
            }
            if vars[0].fin {
                f.write(b"final ");
            }
            write_jtype(&vars[0].jtype, f);
            f.buf.push(b' ');
            f.write_identifier(&vars[0].name);
            if let Some(val) = &vars[0].value {
                f.write(b" = ");
                write_expression(val, f);
            }
            for var in &vars[1..] {
                f.write(b", ");
                f.write_identifier(&var.name);
                if let Some(val) = &var.value {
                    f.write(b" = ");
                    write_expression(val, f);
                }
            }
            if around {
                f.write(b";");
                f.new_line();
            }
        }
        AstBlockEntry::Variable(_) => {}
        AstBlockEntry::If(if_) => write_if_entry(if_, f, next_if),
        AstBlockEntry::While(while_) => {
            f.write_indent();
            if let Some(label) = &while_.label {
                f.write_identifier(label);
                f.write(b": ");
            }
            f.write(b"while ");
            f.write(b"(");
            write_expression(&while_.control, f);
            f.write(b") ");
            write_while_content(&while_.content, f);
        }
        AstBlockEntry::For(for_) => {
            f.write_indent();
            if let Some(label) = &for_.label {
                f.write_identifier(label);
                f.write(b": ");
            }
            f.write(b"for ");
            f.write(b"(");
            for (i, entry) in for_.vars.iter().enumerate() {
                if i > 0 {
                    f.write(b", ");
                }
                write_block_entry(entry, f, false, false);
            }
            f.write(b"; ");
            for (i, entry) in for_.check.iter().enumerate() {
                if i > 0 {
                    f.write(b", ");
                }
                write_block_entry(entry, f, false, false);
            }
            f.write(b"; ");
            for (i, entry) in for_.changes.iter().enumerate() {
                if i > 0 {
                    f.write(b", ");
                }
                write_block_entry(entry, f, false, false);
            }
            f.write(b") ");
            write_for_content(&for_.content, f);
        }
        AstBlockEntry::ForEnhanced(for_) => {
            f.write_indent();
            if let Some(label) = &for_.label {
                f.write_identifier(label);
                f.write(b": ");
            }
            f.write(b"for ");
            f.write(b"(");
            for var in &for_.var {
                for ann in &var.annotated {
                    write_annotation(ann, f);
                }
                if var.fin {
                    f.write(b"final ");
                }
                write_jtype(&var.jtype, f);
                f.buf.push(b' ');
                f.write_identifier(&var.name);
            }
            f.write(b" : ");
            write_expression(&for_.rhs, f);
            f.write(b") ");
            write_for_content(&for_.content, f);
        }
        AstBlockEntry::Switch(switch) => {
            f.write_indent();
            f.write(b"switch ");
            f.write(b"(");
            write_expression(&switch.check, f);
            f.write(b") ");
            write_block(&switch.block, f);
            f.new_line();
        }
        AstBlockEntry::SwitchCase(case) => {
            f.write_indent();
            f.write(b"case ");
            for (i, expr) in case.expressions.iter().enumerate() {
                if i > 0 {
                    f.write(b", ");
                }
                write_expression_or_default(expr, f);
            }
            f.write(b":");
            f.new_line();
        }
        AstBlockEntry::SwitchDefault(_) => {
            f.write_indent();
            f.write(b"default:");
            f.new_line();
        }
        AstBlockEntry::SwitchCaseArrowValues(arrow) => {
            f.write_indent();
            f.write(b"case ");
            for (i, val) in arrow.values.iter().enumerate() {
                if i > 0 {
                    f.write(b", ");
                }
                write_expression_or_default(val, f);
            }
            f.write(b" -> ");
            write_switch_arrow_content(&arrow.content, f);
        }
        AstBlockEntry::SwitchCaseArrowType(arrow) => {
            f.write_indent();
            f.write(b"case ");
            write_jtype(&arrow.var.jtype, f);
            f.buf.push(b' ');
            f.write_identifier(&arrow.var.name);
            f.write(b" -> ");
            write_switch_arrow_content(&arrow.content, f);
        }
        AstBlockEntry::SwitchCaseArrowDefault(arrow) => {
            f.write_indent();
            f.write(b"default -> ");
            write_switch_arrow_content(&arrow.content, f);
        }
        AstBlockEntry::TryCatch(try_catch) => {
            f.write_indent();
            f.write(b"try ");
            if let Some(resources) = &try_catch.resources_block {
                write_block_delimited(resources, b"(", b")", f);
                f.buf.push(b' ');
            }
            write_block(&try_catch.block, f);
            for case in &try_catch.cases {
                f.buf.extend_from_slice(b" catch ");
                f.write(b"(");
                if case.variable.fin {
                    f.write(b"final ");
                }
                for (i, jtype) in case.variable.jtypes.iter().enumerate() {
                    if i > 0 {
                        f.write(b" | ");
                    }
                    write_jtype(jtype, f);
                }
                f.buf.push(b' ');
                f.write_identifier(&case.variable.name);
                f.write(b") ");
                write_block(&case.block, f);
            }
            if let Some(finally) = &try_catch.finally_block {
                f.buf.extend_from_slice(b" finally ");
                write_block(finally, f);
            }
            f.new_line();
        }
        AstBlockEntry::SynchronizedBlock(sync) => {
            f.write_indent();
            f.write(b"synchronized ");
            f.write(b"(");
            write_expression(&sync.expression, f);
            f.write(b") ");
            write_block(&sync.block, f);
            f.new_line();
        }
        AstBlockEntry::InlineBlock(inline) => {
            f.write_indent();
            if let Some(label) = &inline.label {
                f.write_identifier(label);
                f.write(b": ");
            }
            write_block(&inline.block, f);
            f.new_line();
        }
        AstBlockEntry::Thing(thing) => {
            write_thing(thing, f);
        }
    }
}

fn write_if_entry(aif: &AstIf, f: &mut Formatter, next_if: bool) {
    match aif {
        AstIf::If {
            control, content, ..
        } => {
            f.write_indent();
            f.write(b"if ");
            f.write(b"(");
            write_expression(control, f);
            f.write(b") ");
            write_if_content(content, f, next_if);
        }
        AstIf::ElseIf {
            control, content, ..
        } => {
            f.buf.extend_from_slice(b" else if ");
            f.write(b"(");
            write_expression(control, f);
            f.write(b") ");
            write_if_content(content, f, next_if);
        }
        AstIf::Else { content, .. } => {
            f.buf.extend_from_slice(b" else ");
            write_if_content(content, f, next_if);
        }
    }
}

fn write_if_content(content: &AstIfContent, f: &mut Formatter, next_if: bool) {
    match content {
        AstIfContent::Block(block) => {
            write_block(block, f);
            if !next_if {
                f.new_line();
            }
        }
        AstIfContent::BlockEntry(entry) => {
            f.new_line();
            f.indent += 1;
            write_block_entry(entry, f, true, false);
            f.indent -= 1;
        }
    }
}

fn write_while_content(content: &AstWhileContent, f: &mut Formatter) {
    match content {
        AstWhileContent::None => {
            f.write(b";");
            f.new_line();
        }
        AstWhileContent::Block(block) => {
            write_block(block, f);
            f.new_line();
        }
        AstWhileContent::BlockEntry(entry) => {
            f.new_line();
            f.indent += 1;
            write_block_entry(entry, f, true, false);
            f.indent -= 1;
        }
    }
}

fn write_for_content(content: &AstForContent, f: &mut Formatter) {
    match content {
        AstForContent::None => {
            f.write(b";");
            f.new_line();
        }
        AstForContent::Block(block) => {
            write_block(block, f);
            f.new_line();
        }
        AstForContent::BlockEntry(entry) => {
            f.new_line();
            f.indent += 1;
            write_block_entry(entry, f, true, false);
            f.indent -= 1;
        }
    }
}

fn write_expression_or_default(eod: &AstExpressionOrDefault, f: &mut Formatter) {
    match eod {
        AstExpressionOrDefault::Default => {
            f.write(b"default");
        }
        AstExpressionOrDefault::Expression(e) => write_expression(e, f),
    }
}

fn write_switch_arrow_content(content: &AstSwitchCaseArrowContent, f: &mut Formatter) {
    match content {
        AstSwitchCaseArrowContent::Block(block) => {
            write_block(block, f);
            f.new_line();
        }
        AstSwitchCaseArrowContent::Entry(entry) => {
            write_block_entry(entry, f, true, false);
        }
    }
}

fn write_block_delimited(block: &AstBlock, open: &[u8], close: &[u8], f: &mut Formatter) {
    f.write(open);
    f.new_line();
    f.indent += 1;
    for entry in &block.entries {
        write_block_entry(entry, f, true, false);
    }
    f.indent -= 1;
    f.write_indent();
    f.write(close);
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

fn write_class(class: &AstClass, f: &mut Formatter) {
    for ann in &class.annotated {
        f.write_indent();
        write_annotation(ann, f);
    }
    f.write_indent();
    write_availability(&class.availability, f);
    if class.attributes.contains(AstThingAttributes::Sealed) {
        f.write(b"sealed ");
    }
    f.write(b"class ");
    f.write_identifier(&class.name);
    if let Some(tp) = &class.type_parameters {
        write_type_parameters(tp, f);
    }
    if !class.superclass.is_empty() {
        f.write(b" extends ");
        for (i, sc) in class.superclass.iter().enumerate() {
            if i > 0 {
                f.write(b", ");
            }
            if let AstSuperClass::Name(ident) = sc {
                f.write_identifier(ident);
            }
        }
    }
    if !class.implements.is_empty() {
        f.write(b" implements ");
        for (i, jtype) in class.implements.iter().enumerate() {
            if i > 0 {
                f.write(b", ");
            }
            write_jtype(jtype, f);
        }
    }
    if !class.permits.is_empty() {
        f.write(b" permits ");
        for (i, jtype) in class.permits.iter().enumerate() {
            if i > 0 {
                f.write(b", ");
            }
            write_jtype(jtype, f);
        }
    }
    f.buf.push(b' ');
    write_class_block_braced(&class.block, f);
    f.new_line();
}

fn write_class_block_braced(block: &AstClassBlock, f: &mut Formatter) {
    f.write(b"{");
    f.new_line();
    f.indent += 1;
    write_class_block(block, f);
    f.indent -= 1;
    f.write_indent();
    f.write(b"}");
}

fn write_class_block(block: &AstClassBlock, f: &mut Formatter) {
    let mut members = Vec::new();
    for (i, v) in block.variables.iter().enumerate() {
        members.push((v.range, 0, i));
    }
    for (i, m) in block.methods.iter().enumerate() {
        members.push((m.range, 1, i));
    }
    for (i, c) in block.constructors.iter().enumerate() {
        members.push((c.range, 2, i));
    }
    for (i, s) in block.static_blocks.iter().enumerate() {
        members.push((s.range, 3, i));
    }
    for (i, t) in block.inner.iter().enumerate() {
        members.push((t.get_range(), 4, i));
    }
    for (i, b) in block.blocks.iter().enumerate() {
        members.push((b.range, 5, i));
    }
    members.sort_by(|(a, _, _), (b, _, _)| {
        a.start
            .line
            .cmp(&b.start.line)
            .then(a.start.col.cmp(&b.start.col))
    });
    if let Some(first) = members.first() {
        f.insert_new_lines(block.range.start.line, first.0.start.line);
    }
    let mut members = members.iter().peekable();
    while let Some((range, type_id, idx)) = members.next() {
        match type_id {
            0 => write_class_variable(&block.variables[*idx], f),
            1 => write_class_method(&block.methods[*idx], f),
            2 => write_class_constructor(&block.constructors[*idx], f),
            3 => {
                f.write_indent();
                f.write(b"static ");
                write_block(&block.static_blocks[*idx].block, f);
                f.new_line();
            }
            4 => write_thing(&block.inner[*idx], f),
            5 => {
                f.write_indent();
                write_block(&block.blocks[*idx], f);
                f.new_line();
            }
            _ => unreachable!(),
        }
        if let Some(next) = members.peek() {
            f.insert_new_lines(range.end.line, next.0.start.line);
        }
    }
}

fn write_class_variable(v: &AstClassVariable, f: &mut Formatter) {
    for ann in &v.annotated {
        f.write_indent();
        write_annotation(ann, f);
    }
    f.write_indent();
    write_availability(&v.availability, f);
    if v.volatile_transient
        .contains(AstVolatileTransient::Volatile)
    {
        f.write(b"volatile ");
    }
    if v.volatile_transient
        .contains(AstVolatileTransient::Transient)
    {
        f.write(b"transient ");
    }
    write_jtype(&v.jtype, f);
    f.buf.push(b' ');
    f.write_identifier(&v.name);
    if let Some(expr) = &v.expression {
        f.write(b" = ");
        write_expression(expr, f);
    }
    f.write(b";");
    f.new_line();
}

fn write_class_method(method: &AstClassMethod, f: &mut Formatter) {
    write_method_header(&method.header, f);
    if let Some(block) = &method.block {
        f.buf.push(b' ');
        write_block(block, f);
    } else {
        f.write(b";");
    }
    f.new_line();
}

fn write_class_constructor(c: &AstClassConstructor, f: &mut Formatter) {
    for ann in &c.header.annotated {
        f.write_indent();
        write_annotation(ann, f);
    }
    f.write_indent();
    write_availability(&c.header.availability, f);
    if let Some(tp) = &c.header.type_parameters {
        write_type_parameters(tp, f);
        f.buf.push(b' ');
    }
    f.write_identifier(&c.header.name);
    write_method_parameters(&c.header.parameters, f);
    if let Some(throws) = &c.header.throws {
        write_throws(throws, f);
    }
    f.buf.push(b' ');
    write_block(&c.block, f);
    f.new_line();
}

fn write_method_header(header: &AstMethodHeader, f: &mut Formatter) {
    for ann in &header.annotated {
        f.write_indent();
        write_annotation(ann, f);
    }
    f.write_indent();
    write_availability(&header.availability, f);
    if let Some(tp) = &header.type_parameters {
        write_type_parameters(tp, f);
        f.buf.push(b' ');
    }
    write_jtype(&header.jtype, f);
    f.buf.push(b' ');
    f.write_identifier(&header.name);
    write_method_parameters(&header.parameters, f);
    if let Some(throws) = &header.throws {
        write_throws(throws, f);
    }
}

fn write_method_parameters(params: &AstMethodParameters, f: &mut Formatter) {
    let nl = params.range.start.line != params.range.end.line;
    f.write(b"(");
    if nl {
        f.indent += 1;
    }
    for (i, param) in params.parameters.iter().enumerate() {
        if i > 0 {
            f.write(b",");
        }
        if nl {
            f.new_line();
            f.write_indent();
        } else if i > 0 {
            f.write(b" ");
        }
        for ann in &param.annotated {
            write_annotation(ann, f);
        }
        if param.flags.contains(AstMethodParameterFlags::Fin) {
            f.write(b"final ");
        }
        write_jtype(&param.jtype, f);
        if param.flags.contains(AstMethodParameterFlags::Variatic) {
            f.write(b".");
            f.write(b".");
            f.write(b".");
        }
        f.buf.push(b' ');
        f.write_identifier(&param.name);
    }
    if nl {
        f.indent -= 1;
        f.new_line();
        f.write_indent();
    }
    f.write(b")");
}

fn write_throws(throws: &AstThrowsDeclaration, f: &mut Formatter) {
    f.write(b" throws ");
    for (i, t) in throws.parameters.iter().enumerate() {
        if i > 0 {
            f.write(b", ");
        }
        write_jtype(t, f);
    }
}

fn write_type_parameters(tp: &AstTypeParameters, f: &mut Formatter) {
    f.write(b"<");
    for (i, param) in tp.parameters.iter().enumerate() {
        if i > 0 {
            f.write(b", ");
        }
        for ann in &param.annotated {
            write_annotation(ann, f);
        }
        f.write_identifier(&param.name);
        if let Some(superclasses) = &param.supperclass
            && !superclasses.is_empty()
        {
            f.write(b" extends ");
            for (j, sc) in superclasses.iter().enumerate() {
                if j > 0 {
                    f.write(b" & ");
                }
                if let AstSuperClass::Name(ident) = sc {
                    f.write_identifier(ident);
                }
            }
        }
        f.skip_to(param.range.end);
    }
    f.write(b">");
}

fn write_record(record: &AstRecord, f: &mut Formatter) {
    for ann in &record.annotated {
        f.write_indent();
        write_annotation(ann, f);
    }
    f.write_indent();
    write_availability(&record.availability, f);
    f.write(b"record ");
    f.write_identifier(&record.name);
    if let Some(tp) = &record.type_parameters {
        write_type_parameters(tp, f);
    }
    write_record_entries(&record.record_entries, f);
    if !record.implements.is_empty() {
        f.write(b" implements ");
        for (i, jtype) in record.implements.iter().enumerate() {
            if i > 0 {
                f.write(b", ");
            }
            write_jtype(jtype, f);
        }
    }
    f.buf.push(b' ');
    write_class_block_braced(&record.block, f);
    f.new_line();
}

fn write_record_entries(entries: &AstRecordEntries, f: &mut Formatter) {
    f.write(b"(");
    for (i, entry) in entries.entries.iter().enumerate() {
        if i > 0 {
            f.write(b", ");
        }
        for ann in &entry.annotated {
            write_annotation(ann, f);
        }
        write_jtype(&entry.jtype, f);
        if entry.variadic {
            f.write(b".");
            f.write(b".");
            f.write(b".");
        }
        f.buf.push(b' ');
        f.write_identifier(&entry.name);
    }
    f.write(b")");
}

fn write_interface(iface: &AstInterface, f: &mut Formatter) {
    for ann in &iface.annotated {
        f.write_indent();
        write_annotation(ann, f);
    }
    f.write_indent();
    write_availability(&iface.availability, f);
    if iface.attributes.contains(AstThingAttributes::Sealed) {
        f.write(b"sealed ");
    }
    f.write(b"interface ");
    f.write_identifier(&iface.name);
    if let Some(tp) = &iface.type_parameters {
        write_type_parameters(tp, f);
    }
    if let Some(extends) = &iface.extends {
        f.write(b" extends ");
        for (i, jtype) in extends.parameters.iter().enumerate() {
            if i > 0 {
                f.write(b", ");
            }
            write_jtype(jtype, f);
        }
    }
    if !iface.permits.is_empty() {
        f.write(b" permits ");
        for (i, jtype) in iface.permits.iter().enumerate() {
            if i > 0 {
                f.write(b", ");
            }
            write_jtype(jtype, f);
        }
    }
    f.buf.push(b' ');
    f.write(b"{");
    f.new_line();
    f.indent += 1;
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
            0 => write_interface_constant(&iface.constants[*idx], f),
            1 => write_interface_method(&iface.methods[*idx], f),
            2 => write_interface_default_method(&iface.default_methods[*idx], f),
            3 => write_thing(&iface.inner[*idx], f),
            _ => unreachable!(),
        }
    }
    f.indent -= 1;
    f.write_indent();
    f.write(b"}");
    f.new_line();
}

fn write_interface_constant(c: &AstInterfaceConstant, f: &mut Formatter) {
    for ann in &c.annotated {
        f.write_indent();
        write_annotation(ann, f);
    }
    f.write_indent();
    write_availability(&c.availability, f);
    write_jtype(&c.jtype, f);
    f.buf.push(b' ');
    f.write_identifier(&c.name);
    if let Some(expr) = &c.expression {
        f.write(b" = ");
        write_expression(expr, f);
    }
    f.write(b";");
    f.new_line();
}

fn write_interface_method(m: &AstInterfaceMethod, f: &mut Formatter) {
    for ann in &m.annotated {
        f.write_indent();
        write_annotation(ann, f);
    }
    write_method_header(&m.header, f);
    f.write(b";");
    f.new_line();
}

fn write_interface_default_method(m: &AstInterfaceMethodDefault, f: &mut Formatter) {
    for ann in &m.annotated {
        f.write_indent();
        write_annotation(ann, f);
    }
    write_method_header(&m.header, f);
    f.buf.push(b' ');
    write_block(&m.block, f);
    f.new_line();
}

fn write_enumeration(e: &AstEnumeration, f: &mut Formatter) {
    for ann in &e.annotated {
        f.write_indent();
        write_annotation(ann, f);
    }
    f.write_indent();
    write_availability(&e.availability, f);
    f.write(b"enum ");
    f.write_identifier(&e.name);
    if !e.implements.is_empty() {
        f.write(b" implements ");
        for (i, jtype) in e.implements.iter().enumerate() {
            if i > 0 {
                f.write(b", ");
            }
            write_jtype(jtype, f);
        }
    }
    f.buf.push(b' ');
    f.write(b"{");
    f.new_line();
    f.indent += 1;
    for (i, variant) in e.variants.iter().enumerate() {
        if i > 0 {
            f.write(b",");
            f.new_line();
        }
        f.write_indent();
        for ann in &variant.annotated {
            write_annotation(ann, f);
        }
        f.write_identifier(&variant.name);
        if !variant.parameters.is_empty() {
            f.write(b"(");
            for (j, param) in variant.parameters.iter().enumerate() {
                if j > 0 {
                    f.write(b", ");
                }
                write_expression(param, f);
            }
            f.write(b")");
        }
    }
    let has_members = !e.methods.is_empty()
        || !e.variables.is_empty()
        || !e.constructors.is_empty()
        || !e.static_blocks.is_empty()
        || !e.inner.is_empty();
    if has_members {
        f.write(b";");
        f.new_line();
        let block = AstClassBlock {
            range: e.range,
            variables: e.variables.clone(),
            methods: e.methods.clone(),
            constructors: e.constructors.clone(),
            static_blocks: e.static_blocks.clone(),
            inner: e.inner.clone(),
            blocks: vec![],
        };
        write_class_block(&block, f);
    } else if !e.variants.is_empty() {
        f.new_line();
    }
    f.indent -= 1;
    f.write_indent();
    f.write(b"}");
    f.new_line();
}

fn write_annotation_type(ann_type: &AstAnnotation, f: &mut Formatter) {
    for ann in &ann_type.annotated {
        f.write_indent();
        write_annotation(ann, f);
    }
    f.write_indent();
    write_availability(&ann_type.availability, f);
    f.write(b"@");
    f.write(b"interface ");
    f.write_identifier(&ann_type.name);
    f.buf.push(b' ');
    f.write(b"{");
    f.new_line();
    f.indent += 1;
    for field in &ann_type.fields {
        write_annotation_field(field, f);
    }
    for inner in &ann_type.inner {
        write_thing(inner, f);
    }
    f.indent -= 1;
    f.write_indent();
    f.write(b"}");
    f.new_line();
}

fn write_annotation_field(field: &AstAnnotationField, f: &mut Formatter) {
    for ann in &field.annotated {
        f.write_indent();
        write_annotation(ann, f);
    }
    f.write_indent();
    write_availability(&field.availability, f);
    write_jtype(&field.jtype, f);
    f.buf.push(b' ');
    f.write_identifier(&field.name);
    f.write(b"(");
    f.write(b")");
    if let Some(expr) = &field.expression {
        f.write(b" default ");
        write_expression(expr, f);
    }
    f.write(b";");
    f.new_line();
}

fn write_expression_identifier(ident: &AstExpressionIdentifier, f: &mut Formatter) {
    match ident {
        AstExpressionIdentifier::Identifier(i) => f.write_identifier(i),
        AstExpressionIdentifier::Nuget(n) => write_value_nuget(n, f),
        AstExpressionIdentifier::Value(v) => write_value(v, f),
        AstExpressionIdentifier::ArrayAccess { expr, .. } => {
            f.write(b"[");
            write_expression(expr, f);
            f.write(b"]");
        }

        AstExpressionIdentifier::EmptyArrayAccess(_) => {
            f.write(b"[]");
        }
    }
}

fn write_value(value: &AstValue, f: &mut Formatter) {
    match value {
        AstValue::Variable(i) => f.write_identifier(i),
        AstValue::Nuget(n) => write_value_nuget(n, f),
    }
}

fn write_value_nuget(nuget: &AstValueNuget, f: &mut Formatter) {
    match nuget {
        AstValueNuget::Int(i) => {
            f.write_with_comments(i.range.start, i.value.as_bytes());
            f.skip_to(i.range.end);
        }
        AstValueNuget::Long(l) => {
            f.write_with_comments(l.range.start, l.value.as_bytes());
            f.skip_to(l.range.end);
            f.write(b"L");
        }
        AstValueNuget::Double(d) => {
            f.write_with_comments(d.range.start, d.value.as_bytes());
            f.skip_to(d.range.end);
        }
        AstValueNuget::Float(fl) => {
            f.write_with_comments(fl.range.start, fl.value.as_bytes());
            f.skip_to(fl.range.end);
            f.write(b"f");
        }
        AstValueNuget::StringLiteral {
            value,
            multi_line: false,
        } => {
            f.write_with_comments(value.range.start, b"\"");
            f.buf.extend_from_slice(value.value.as_bytes());
            f.buf.extend_from_slice(b"\"");
            f.skip_to(value.range.end);
        }
        AstValueNuget::StringLiteral {
            value,
            multi_line: true,
        } => {
            f.write_with_comments(value.range.start, b"\"\"\"");
            f.buf.extend_from_slice(value.value.as_bytes());
            f.buf.extend_from_slice(b"\"\"\"");
            f.skip_to(value.range.end);
        }
        AstValueNuget::CharLiteral(c) => {
            f.write_with_comments(c.range.start, b"'");
            f.buf.extend_from_slice(c.value.as_bytes());
            f.buf.extend_from_slice(b"'");
            f.skip_to(c.range.end);
        }
        AstValueNuget::BooleanLiteral(b) => {
            f.write_with_comments(b.range.start, if b.value { b"true" } else { b"false" });
            f.skip_to(b.range.end);
        }
        AstValueNuget::HexLiteral(h) => {
            f.write_with_comments(h.range.start, b"0x");
            f.buf.extend_from_slice(h.value.as_bytes());
            f.skip_to(h.range.end);
        }
        AstValueNuget::BinaryLiteral(b) => {
            f.write_with_comments(b.range.start, b"0b");
            f.buf.extend_from_slice(b.value.as_bytes());
            f.skip_to(b.range.end);
        }
    }
}

fn write_expression_operator(
    op: &AstExpressionOperator,
    f: &mut Formatter,
    is_large: bool,
    dot: bool,
    nth: usize,
) {
    let (r, bytes): (_, &[u8]) = match op {
        AstExpressionOperator::None => return,
        AstExpressionOperator::Dot(r) => (r, b"."),
        AstExpressionOperator::Plus(r) => (r, b" + "),
        AstExpressionOperator::PlusPlus(r) => (r, b"++"),
        AstExpressionOperator::PlusEqual(r) => (r, b" += "),
        AstExpressionOperator::Minus(r) => (r, b" - "),
        AstExpressionOperator::MinusEqual(r) => (r, b" -= "),
        AstExpressionOperator::MinusMinus(r) => (r, b"--"),
        AstExpressionOperator::Multiply(r) => (r, b" * "),
        AstExpressionOperator::MultiplyEqual(r) => (r, b" *= "),
        AstExpressionOperator::Divide(r) => (r, b" / "),
        AstExpressionOperator::DivideEqual(r) => (r, b" /= "),
        AstExpressionOperator::Modulo(r) => (r, b" % "),
        AstExpressionOperator::ModuloEqual(r) => (r, b" %= "),
        AstExpressionOperator::Equal(r) => (r, b" == "),
        AstExpressionOperator::NotEqual(r) => (r, b" != "),
        AstExpressionOperator::Le(r) => (r, b" <= "),
        AstExpressionOperator::Lt(r) => (r, b" < "),
        AstExpressionOperator::LtLt(r) => (r, b" << "),
        AstExpressionOperator::Ge(r) => (r, b" >= "),
        AstExpressionOperator::Gt(r) => (r, b" > "),
        AstExpressionOperator::GtGt(r) => (r, b" >> "),
        AstExpressionOperator::GtGtGt(r) => (r, b" >>> "),
        AstExpressionOperator::ExclamationMark(r) => (r, b"!"),
        AstExpressionOperator::Ampersand(r) => (r, b" & "),
        AstExpressionOperator::AmpersandAmpersand(r) => (r, b" && "),
        AstExpressionOperator::VerticalBar(r) => (r, b" | "),
        AstExpressionOperator::VerticalBarEqual(r) => (r, b" |= "),
        AstExpressionOperator::VerticalBarVerticalBar(r) => (r, b" || "),
        AstExpressionOperator::QuestionMark(r) => (r, b" ? "),
        AstExpressionOperator::Colon(r) => (r, b" : "),
        AstExpressionOperator::ColonColon(r) => (r, b"::"),
        AstExpressionOperator::Assign(r) => (r, b" = "),
        AstExpressionOperator::Tilde(r) => (r, b"~"),
        AstExpressionOperator::Caret(r) => (r, b" ^ "),
    };
    if nth > 2
        && is_large
        && (matches!(
            op,
            AstExpressionOperator::VerticalBarVerticalBar(_)
                | AstExpressionOperator::AmpersandAmpersand(_)
                | AstExpressionOperator::Plus(_)
                | AstExpressionOperator::Minus(_)
        ) || (dot && matches!(op, AstExpressionOperator::Dot(_))))
    {
        f.new_line();
        f.write_indent();
    }
    f.write_with_comments(r.start, bytes);
    f.skip_to(r.end);
}

fn write_import(import: &AstImport, f: &mut Formatter) {
    f.write(b"import ");
    match &import.unit {
        AstImportUnit::Class(ident) => {
            f.write_identifier(ident);
        }
        AstImportUnit::StaticClass(ident) => {
            f.write(b"static ");
            f.write_identifier(ident);
        }
        AstImportUnit::StaticClassMethod(class, method) => {
            f.write(b"static ");
            f.write_identifier(class);
            f.buf.push(b'.');
            f.write_identifier(method);
        }
        AstImportUnit::Prefix(ident) => {
            f.write_identifier(ident);
            f.write(b".");
            f.write(b"*");
        }
        AstImportUnit::StaticPrefix(ident) => {
            f.write(b"static ");
            f.write_identifier(ident);
            f.write(b".");
            f.write(b"*");
        }
    }
    f.write(b";");
    f.new_line();
}

fn write_module(module: &AstModule, f: &mut Formatter) {
    for ann in &module.annotated {
        write_annotation(ann, f);
    }
    if module.open {
        f.write(b"open ");
    }
    f.write(b"module ");
    f.write_identifier(&module.name);
    f.buf.push(b' ');
    f.write(b"{");
    f.new_line();
    f.indent += 1;

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
        f.write_indent();
        match type_id {
            0 => {
                let e = &module.exports[*idx];
                f.write(b"exports ");
                f.write_identifier(&e.name);
                if !e.to.is_empty() {
                    f.write(b" to ");
                    for (i, t) in e.to.iter().enumerate() {
                        if i > 0 {
                            f.write(b", ");
                        }
                        f.write_identifier(t);
                    }
                }
                f.write(b";");
                f.new_line();
            }
            1 => {
                let o = &module.opens[*idx];
                f.write(b"opens ");
                f.write_identifier(&o.name);
                if !o.to.is_empty() {
                    f.write(b" to ");
                    for (i, t) in o.to.iter().enumerate() {
                        if i > 0 {
                            f.write(b", ");
                        }
                        f.write_identifier(t);
                    }
                }
                f.write(b";");
                f.new_line();
            }
            2 => {
                let u = &module.uses[*idx];
                f.write(b"uses ");
                f.write_identifier(&u.name);
                f.write(b";");
                f.new_line();
            }
            3 => {
                let p = &module.provides[*idx];
                f.write(b"provides ");
                f.write_identifier(&p.name);
                f.write(b" with ");
                for (i, w) in p.with.iter().enumerate() {
                    if i > 0 {
                        f.write(b", ");
                    }
                    f.write_identifier(w);
                }
                f.write(b";");
                f.new_line();
            }
            4 => {
                let r = &module.requires[*idx];
                f.write(b"requires ");
                if r.flags.contains(AstModuleRequiresFlags::Transitive) {
                    f.write(b"transitive ");
                }
                if r.flags.contains(AstModuleRequiresFlags::Static) {
                    f.write(b"static ");
                }
                f.write_identifier(&r.name);
                f.write(b";");
                f.new_line();
            }
            _ => unreachable!(),
        }
    }

    f.indent -= 1;
    f.write_indent();
    f.write(b"}");
    f.new_line();
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;

    const SPACE: &str = "    ";

    #[test]
    fn package() {
        let content = b"// This is a cool file

        @Thing(Type.IMPORTANT)
        @Retention(RetentionPolicy.RUNTIME)
        package ch.emilycares;

        // Now the imports
        ";

        let o = internal(content, SPACE).unwrap();
        let expected = expect![[r"
            // This is a cool file
            @Thing(Type.IMPORTANT)
            @Retention(RetentionPolicy.RUNTIME)
            package ch.emilycares;
        "]];
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
        public class Test {
            /**
             * hostile getter
             */
            public static int isHostile() {
                return true;
            }
        }
        "#;

        let o = internal(content, SPACE).unwrap();
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
            public class Test {

                /**
                         * hostile getter
                         */
                public static int isHostile() {
                    return true;
                }
            }
        "#]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn method_parameters() {
        let content = br"
        package ch.emilycares;
        public class Application {

            private final Database db;
            private final Messages msgs;
            private final Telemetry tel;
            private final Obeservability obs;

            public Application(Database db, Messages msgs, Telemetry tel, 
            Obeservability obs) {
                this.db = db;
                this.msgs = msgs;
                this.tel = tel;
                this.obs = obs;
            }
        }
        ";

        let o = internal(content, SPACE).unwrap();
        let expected = expect![[r"
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
        "]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }
    #[test]
    fn enum_base() {
        let content = br"
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
        ";

        let o = internal(content, SPACE).unwrap();
        let expected = expect![[r"
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
        "]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn long_plus() {
        let content = br"
        package ch.emilycares;
        public class Test {
            public int aaa() {
                 int b = 1;
                 return (this.that) + this.other + this.taetsch 
                 + this.boing + b;
            }
        }
        ";

        let o = internal(content, SPACE).unwrap();
        let expected = expect![[r"
            package ch.emilycares;
            public class Test {
                public int aaa() {
                    int b = 1;
                    return (this.that) + this.other
                         + this.taetsch
                         + this.boing
                         + b;
                }
            }
        "]];
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

        let o = internal(content, SPACE).unwrap();
        let expected = expect![[r#"
            package ch.emilycares;
            public class Test {
                public int aaa() {
                    for (int i; i < 5; i++) {
                        log.info("hihihaha");
                    }
                }
            }
        "#]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn multi_line_string() {
        let content = br#"
        package ch.emilycares;
        public class Test {
            public int aaa() {
               var description = """
               HIHI HAHA
               HIHI HAHA
               """;

            }
        }
        "#;

        let o = internal(content, SPACE).unwrap();
        let expected = expect![[r#"
            package ch.emilycares;
            public class Test {
                public int aaa() {
                    var description = """
                           HIHI HAHA
                           HIHI HAHA
                           """;
                }
            }
        "#]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn stream() {
        let content = br"
        package ch.emilycares;

        public class Test {
            public int aaa() {
                return intem
                .stream() .map(a -> a + 1) .filter(a > 5) .toList();

            }
        }
        ";

        let o = internal(content, SPACE).unwrap();
        let expected = expect![[r"
            package ch.emilycares;

            public class Test {
                public int aaa() {
                    return intem.stream()
                        .map(a -> a + 1)
                        .filter(a > 5)
                        .toList();
                }
            }
        "]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn annotated() {
        let content = br#"
@Path("/api/v1/thing")
@Consumes(MediaType.APPLICATION_JSON, MediaType.APPLICATION_JSON)
@Table(uniqueConstraints = @UniqueConstraint(columnNames = {"otherUuid", "thing_id"}))
public class ThingResource {

    @Inject
    ObjectMapper mapper;

    @POST
    @Path("/thing")
    @WithTransaction
    public Uni<Response> createThing(CreateThingRequest request) {
    Campaign c = new Campaign();
    c.slug = request.slug;
    c.displayName = request.displayName;
    c.style = request.style;
    c.active = true;
    return c
      .persist()
      .map(entity -> Response.status(201).entity(entity).build());
  }
  }
        "#;

        let o = internal(content, SPACE).unwrap();
        let expected = expect![[r#"
            @Path("/api/v1/thing")
            @Consumes(MediaType.APPLICATION_JSON, MediaType.APPLICATION_JSON)
            @Table(uniqueConstraints = @UniqueConstraint(columnNames = {"otherUuid", "thing_id"}) )
            public class ThingResource {

                @Inject
                ObjectMapper mapper;

                @POST
                @Path("/thing")
                @WithTransaction
                public Uni<Response> createThing(CreateThingRequest request) {
                    Campaign c = new Campaign();
                    c.slug = request.slug;
                    c.displayName = request.displayName;
                    c.style = request.style;
                    c.active = true;
                    return c.persist()
                        .map(entity -> Response.status(201).entity(entity).build());
                }
            }
        "#]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn if_else() {
        let content = br"
        package ch.emilycares;

        public class Test {
            public int aaa() {
            if (true) {
            return 1;
            } else if (false) {
            return 2;

            } else {
            return 2;
            }

            }
        }
        ";

        let o = internal(content, SPACE).unwrap();
        let expected = expect![[r"
            package ch.emilycares;

            public class Test {
                public int aaa() {
                    if (true) {
                        return 1;
                    } else if (false) {
                        return 2;
                    } else {
                        return 2;
                    }
                }
            }
        "]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn lambda_newline() {
        let content = br"
        package ch.emilycares;

        public class Test {
            public int aaa() {
                Suppliers.momoize(() -> {
                   // some processing
                   return true;
                });

                Thread.ofVirtual().start(() -> {
                            // do something
                        });
            }
        }
        ";

        let o = internal(content, SPACE).unwrap();
        let expected = expect![[r"
            package ch.emilycares;

            public class Test {
                public int aaa() {
                    Suppliers.momoize(() -> {
                            // some processing
                            return true;
                        });

                    Thread.ofVirtual()
                        .start(() -> {
                            // do something
                            });
                }
            }
        "]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn dyn_space_after_name() {
        let content = br#"
package ch.emilycares;

public class Test {
    private String a      = "a";
    private String aa     = "aa";
    private String aaa    = "aaa";
    private String aaaa   = "aaaa";
    private String aaaaa  = "aaaaa";
    private String aaaaaa = "aaaaaa";
}
        "#;

        let o = internal(content, SPACE).unwrap();
        let expected = expect![[r#"
            package ch.emilycares;

            public class Test {
                private String a = "a";
                private String aa = "aa";
                private String aaa = "aaa";
                private String aaaa = "aaaa";
                private String aaaaa = "aaaaa";
                private String aaaaaa = "aaaaaa";
            }
        "#]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn operators() {
        let content = br"
        package ch.emilycares;

        public class Test {
            public int aaa() {
                t |= avc();
                t |= a().b().c();
                t |= a().b()
                .c();
                t += 1;
                t -= 1;
                t *= 1;
                t /= 1;
                t %= 1;
                a & b;
                a | b;
                ~a;
                a << 2;
                a >> 1;
                a >>> 1;
                a instanceof String;
                a instanceof final String;
                if (a || b) { }
                a = b ? 1 : 2;
                a--;
                a++;
            }
        }
        ";

        let o = internal(content, SPACE).unwrap();
        let expected = expect![[r"
            package ch.emilycares;

            public class Test {
                public int aaa() {
                    t |= avc();
                    t |= a().b().c();
                    t |= a()
                        .b()
                        .c();
                    t += 1;
                    t -= 1;
                    t *= 1;
                    t /= 1;
                    t %= 1;
                    a & b;
                    a | b;
                    ~a;
                    a << 2;
                    a >> 1;
                    a >>> 1;
                    a instanceof String;
                    a instanceof final String;
                    if (a || b) {
                    }
                    a = b ? 1 : 2;
                    a--;
                    a++;
                }
            }
        "]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn nl_arguments() {
        let content = br"
        package ch.emilycares;

        public class Test {
            public int aaa() {
                other(
                    1 + 2,
                    1
                );
            }
        }
        ";

        let o = internal(content, SPACE).unwrap();
        let expected = expect![[r"
            package ch.emilycares;

            public class Test {
                public int aaa() {
                    other(
                        1 + 2,
                        1
                        );
                }
            }
        "]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }

    #[test]
    fn interface_indent() {
        let content = br"
        public interface Test {
            void a();

            /** hehehhehehe */
            void b();
        }
        ";

        let o = internal(content, SPACE).unwrap();
        let expected = expect![[r"
            public interface Test {
                void a();
                /** hehehhehehe */
                void b();
            }
        "]];
        expected.assert_eq(str::from_utf8(&o.unwrap_or_default()).unwrap());
    }
}
