# Module instance configuration

`app` imports a search key encoder and a text exporter. Both use the same
`codec` source directory. Search selects hexadecimal UTF-8 bytes; export selects
plain UTF-8 text. Each reads its own `representation` at compile time.

```sh
target/loom run compiler/examples/module_configuration/app
target/loom test compiler/examples/module_configuration/app
```

The codec's default is `hex`. Omitting an explicit `representation = "hex"`
therefore reuses that effective instance. Choosing `utf8` creates another
instance: its `Encoded` type is distinct despite the identical declaration.
The compiler's native integration tests exercise rejection of cross-instance
values, normalized sharing, macros, generics, closures, edited caches and build
receipts. The Git dependency tests additionally cover locked offline builds,
stale configuration and explicit resolution.
