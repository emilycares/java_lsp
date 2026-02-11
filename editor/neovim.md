# neovim

``` lua
vim.lsp.config.java_lsp = {
    cmd = { "/path/to/executable/java_lsp" },
    root_markers = { "pom.xml", "gradlew" },
    filetypes = { "java" },
}
vim.lsp.enable("java_lsp")
```

