# Running the Examples

Each subdirectory provides a small demo showcasing some feature of **Koji**. The
examples can be run with `cargo run --example <name>`.

Most demos compile shaders from the `assets/shaders/` directory and open an SDL2
window. Heavier integrations that were originally tests (such as bindless or
skeletal rendering) are behind the `gpu_tests` feature flag.

```
cargo run --example sample                        # run the triangle sample
cargo run --features gpu_tests --example text2d   # run an example requiring gpu_tests
```

## Available Examples

- **sample** – draw a single triangle
- **deferred_sample** – basic deferred rendering
- **shadow_sample** – cascaded shadow maps
- **pbr_spheres** – grid of spheres with PBR shading
- **custom_pass** – building a render pass from a YAML description
- **bindless_rendering** – textured triangle using bindless resources *(gpu_tests)*
- **bindless_lighting** – simple lighting with bindless buffers *(gpu_tests)*
- **pbr_renderer** – textured quad with the PBR material *(gpu_tests)*
- **skinned_mesh_render** – render a skinned glTF mesh *(gpu_tests)*
- **skeletal_renderer** – skeleton rendering and bone updates *(gpu_tests)*
- **skeletal_animation** – play an animated skeletal mesh *(gpu_tests)*
- **static_movement** – update vertices of a static mesh *(gpu_tests)*
- **text2d** – draw 2D text using the text renderer *(gpu_tests)*
