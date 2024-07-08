use core::panic;
use std::path::Path;
use thiserror::Error;

use java_asm::jvms::{element::Const, read::JvmsClassReader};

#[derive(Error, Debug)]
pub enum ClassError {
    #[error("IO error")]
    IO(#[from] std::io::Error),
    #[error("unknown")]
    Unknown,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Class {
    pub name: String,
    pub methods: Vec<Method>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Method {
    pub name: String,
    pub attributes: Vec<Attribute>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Attribute {
    name: String,
}
pub fn load_fs<T: AsRef<Path>>(path: T) -> Result<Class, ClassError> {
    let bytes = std::fs::read(path)?;
    load(&bytes)
}

pub fn load(bytes: &[u8]) -> Result<Class, ClassError> {
    let compile_testing_class = JvmsClassReader::read_class_bytes(bytes).unwrap();

    let methods: Vec<_> = compile_testing_class
        .methods
        .iter()
        .map(|method| {
            let attributes = method
                .attributes
                .iter()
                .map(|attribute| {
                    let name =
                        lookup_string(&compile_testing_class, attribute.attribute_name_index);
                    Attribute { name }
                })
                .collect();

            let name = lookup_string(&compile_testing_class, method.name_index);
            Method { name, attributes }
        })
        .collect();

    let class_name = lookup_class(&compile_testing_class, compile_testing_class.this_class);
    let class_name = lookup_string(&compile_testing_class, class_name);
    let class_name = class_name.split("/").last().unwrap();
    Ok(Class {
        name: class_name.to_string(),
        methods,
    })
}

fn lookup_string(compile_testing_class: &java_asm::jvms::element::ClassFile, idx: u16) -> String {
    let name = compile_testing_class
        .constant_pool
        .get(idx as usize)
        .expect("The coompiler shoult ensure that the name can be looked up");
    let Const::Utf8 { bytes, length: _ } = &name.info else {
        panic!("A method name was not Utf8");
    };
    String::from_utf8_lossy(&bytes).to_string()
}
fn lookup_class(compile_testing_class: &java_asm::jvms::element::ClassFile, idx: u16) -> u16 {
    let name = compile_testing_class
        .constant_pool
        .get(idx as usize)
        .expect("The coompiler shoult ensure that the name can be looked up");
    let Const::Class { name_index } = &name.info else {
        panic!("A method name was not Utf8");
    };
    *name_index
}
