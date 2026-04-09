use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use document::Document;
use dto::{Class, ImportUnit};
use local_variable::VarFlags;
use lsp_extra::to_lsp_range;
use lsp_types::{InlayHint, InlayHintKind, InlayHintLabel};
use my_string::MyString;
use variables::VariableContext;

use crate::hover::jtype_hover_display;

pub fn get_inlay_hint(
    document: &Document,
    class: &Class,
    imports: &[ImportUnit],
    class_map: Arc<Mutex<HashMap<MyString, Class>>>,
) -> Option<Vec<InlayHint>> {
    let vars = match variables::get_vars(
        &document.ast,
        &VariableContext {
            point: None,
            imports,
            class,
            class_map,
        },
    ) {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("Could not get vars: {e:?}");
            None
        }
    }?;
    Some(
        vars.iter()
            .filter(|i| i.flags.intersects(VarFlags::Computed))
            .filter_map(|i| {
                Some(InlayHint {
                    position: to_lsp_range(&i.range).ok()?.start,
                    label: InlayHintLabel::String(jtype_hover_display(&i.jtype)),
                    kind: Some(InlayHintKind::TYPE),
                    text_edits: None,
                    tooltip: None,
                    padding_left: None,
                    padding_right: None,
                    data: None,
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    use document::Document;
    use dto::{Access, JType, Method, SourceDestination};

    use super::*;

    #[test]
    fn base() {
        let cont = r#"
package ch.emilycares;
public class Test {
    public String hello() {
        var a = "";

    }
}
        "#;
        let document = Document::setup(cont, PathBuf::from_str("/Test.java").unwrap()).unwrap();
        let class = parser::java::load_java_tree(&document.ast, SourceDestination::None);
        let imports = imports::imports(&document.ast);
        let out = get_inlay_hint(&document, &class, &imports, get_class_map()).unwrap();
        insta::assert_debug_snapshot!(out);
    }
    fn get_class_map() -> Arc<Mutex<HashMap<MyString, Class>>> {
        let mut class_map: HashMap<MyString, Class> = HashMap::new();

        class_map.insert(
            "java.lang.String".into(),
            Class {
                source: SourceDestination::Here("String".into()),
                access: Access::Public,
                name: "String".into(),
                methods: vec![Method {
                    access: Access::Public,
                    name: Some("length".into()),
                    ret: JType::Int,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        Arc::new(Mutex::new(class_map))
    }
}
