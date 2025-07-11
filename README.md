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

## Compute Pipelines

Custom compute pipelines can be added with `Renderer::register_compute_pipeline`.
After registering, schedule work with `Renderer::queue_compute`, specifying the
pipeline id and `[x, y, z]` workgroup counts. Queued tasks are dispatched at the
start of the next `present_frame` call.

## Sample Binaries

Example programs live under the `examples/` directory and can be run with
`cargo run --example <name>`. These require a Vulkan-capable GPU and a working
window system. Some of the demos are gated behind the `gpu_tests` feature flag.

Available examples:

- `sample` – simple triangle animated with `KOJI_time`
- `pbr_spheres` – grid of spheres demonstrating PBR shading
- `bindless_rendering` – textured triangle using bindless descriptors *(gpu_tests)*
- `skeletal_animation` – animated skeletal mesh *(gpu_tests)*
- `text2d` – draw 2D text *(gpu_tests)*

See [examples/README.md](examples/README.md) for more information.

## Contributing

Before submitting a pull request, run `cargo test` and ensure it completes successfully.
