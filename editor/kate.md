# Kate
https://kate-editor.org/

## How to configure
Goto Settings > Configure Kate > LSP Client > User Server Settings
And paste in this config with your path to java_lsp executable
```json
{
    "servers": {
        "java": {
            "command": ["/path/to/executable/java_lsp"],
            "url": "",
            "rootIndicationFileNames": ["pom.xml", "gradlew"],
            "highlightingModeRegex": "^Java$"
        }
    }
}
```
