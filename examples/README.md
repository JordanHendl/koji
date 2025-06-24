# Example Programs

These demos showcase different Koji features and can be executed with

```bash
cargo run --example <name>
```

Some programs require the `gpu_tests` feature to be enabled. Shaders usually
pull from `assets/shaders/` and rely on the uniform block provided by
`assets/shaders/timing.slang` when referencing `KOJI_time`.

## Available Examples

- **sample** – draw a single triangle and animate its color using `KOJI_time`
- **pbr_spheres** – grid of spheres with PBR shading
- **bindless_rendering** – textured triangle using bindless descriptors *(gpu_tests)*
- **skeletal_animation** – play an animated skeletal mesh *(gpu_tests)*
- **text2d** – draw 2D text using the text renderer *(gpu_tests)*
