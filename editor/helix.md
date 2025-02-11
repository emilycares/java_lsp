# How to configure java-lsp for helix
Open the file ~/.config/helix/languages.toml

``` toml
[[language]]
name = "java"
language-servers = [ "java-lsp" ]

[language-server.java-lsp]
command = "/path/to/executable/java_lsp"
```
