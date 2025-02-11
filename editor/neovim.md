# How to configure java-lsp for neovim

``` lua
local configs = require("lspconfig.configs")
configs.java_lsp = {
   default_config = {
     cmd = { "/path/to/executable/java_lsp" },
     filetypes = { "java" },
     root_dir = function(fname)
       return require("lspconfig").util.find_git_ancestor(fname)
     end,
     autostart = true,
     settings = {},
   },
}
require("lspconfig").java_lsp.setup()
```

