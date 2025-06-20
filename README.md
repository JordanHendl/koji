# Koji

Koji is a small rendering crate built on top of [Dashi](https://github.com/JordanHendl/dashi).  
It provides helpers for building render pipelines, loading `glTF` assets and drawing text.
SPIR-V shaders are compiled at build time via `inline-spirv` and winit is used for windowing.


## Dependencies

Key dependencies include:

- **dashi** &ndash; GPU and Vulkan abstractions used by Koji
- **glam** &ndash; math types (vectors, matrices)
- **winit** &ndash; window/event handling
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

## Frame Timing

Shaders that reference the `KOJI_time` uniform automatically receive a timing
buffer when their pipeline is built. The helper file
`assets/shaders/timing.slang` defines the uniform block and can be included with

```glsl
#include "timing.slang"
```

No extra resource registration is necessary.

## Sample Binaries

Example programs live under the `examples/` directory and can be run with
`cargo run --example <name>`. These require a Vulkan-capable GPU and a working
window system. Some of the heavier demos are gated behind the `gpu_tests`
feature flag. See [examples/README.md](examples/README.md) for a description of
each example and exact commands.

## Contributing

Before submitting a pull request, run `cargo test` and ensure it completes successfully.
