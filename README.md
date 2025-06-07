# Koji

Koji is a small rendering crate built on top of [Dashi](https://github.com/JordanHendl/dashi).  
It provides helpers for building render pipelines, loading `glTF` assets and drawing text.
SPIR-V shaders are compiled at build time via `inline-spirv` and SDL2 is used for windowing.

## Dependencies

Key dependencies include:

- **dashi** &ndash; GPU and Vulkan abstractions used by Koji
- **glam** &ndash; math types (vectors, matrices)
- **sdl2** &ndash; window/event handling
- **rhai** &ndash; optional scripting support
- **serde**, **serde_yaml**, **indexmap** &ndash; data serialization
- **gltf**, **rusttype** &ndash; asset loading and fonts

See `Cargo.toml` for the full list.

## Building

```bash
cargo build
```

## Running Tests

```bash
cargo test
```

## Sample Binaries

Three examples live under the `examples/` directory and can be run with:

```bash
cargo run --example sample           # basic triangle sample
cargo run --example deferred_sample  # deferred rendering example
cargo run --example shadow_sample    # cascaded shadow map demo
```

These demos compile included GLSL files in `assets/shaders/` and open an SDL2 window.
