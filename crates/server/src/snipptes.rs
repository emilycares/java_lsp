pub const IF: &str = "if (${1}) {\n    ${2}\n}";
pub const SWITCH: &str =
    "switch (${1}) {\n    case ${2}:\n        break;\n    default:\n        break;\n}";
pub const FUNCTION: &str = "public ${1} ${2}(${3}) {\n    ${4}\n}";
