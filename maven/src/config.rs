/// Add maven flags to overwrite settings.xml
pub fn overwrite_settings_xml(command: &mut std::process::Command) -> &mut std::process::Command {
    if let Ok(p) = std::env::var("JAVA_LSP_MAVEN_SETTINGS_XML_PATH") {
        return command.args(["-s", &p]);
    }
    command
}
/// Add maven flags to overwrite settings.xml
pub fn overwrite_settings_xml_tokio(
    command: &mut tokio::process::Command,
) -> &mut tokio::process::Command {
    if let Ok(p) = std::env::var("JAVA_LSP_MAVEN_SETTINGS_XML_PATH") {
        return command.args(["-s", &p]);
    }
    command
}
