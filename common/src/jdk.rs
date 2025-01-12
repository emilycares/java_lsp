use std::path::Path;

use dashmap::DashMap;
use parser::loader::SourceDestination;

pub fn load_classes(class_map: &DashMap<std::string::String, parser::dto::Class>) {
    let path = Path::new(".jdk.cfc");
    if path.exists() {
        if let Ok(classes) = parser::loader::load_class_folder("jdk") {
            for class in classes.classes {
                class_map.insert(class.class_path.clone(), class);
            }
        }
    } else {
        // nix run github:nix-community/nix-index#nix-locate -- jmods/java.base.jmod
        // ``` bash
        // mkdir jdk
        // cd jdk
        // # jmod is in the jdk bin dir
        // jmod extract openjdk-22.0.2_windows-x64_bin/jdk-22.0.2/jmods/java.base.jmod
        // cd ..
        // mvn dependency:unpack
        // ```
        let classes = parser::loader::load_classes("./jdk/classes/", SourceDestination::None);
        if let Err(e) = parser::loader::save_class_folder("jdk", &classes) {
            eprintln!("Failed to save .jdk.cfc because: {e}");
        };
        for class in classes.classes {
            class_map.insert(class.class_path.clone(), class);
        }
    }
}
