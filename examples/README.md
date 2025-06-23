# Running the Examples

Each subdirectory provides a small demo showcasing some feature of **Koji**. The
examples can be run with `cargo run --example <name>`.

Most demos compile shaders from the `assets/shaders/` directory and open a winit
window. Heavier integrations that were originally tests (such as bindless or
skeletal rendering) are behind the `gpu_tests` feature flag.

The `assets/shaders/timing.slang` file defines a uniform block providing frame
timing information. Any shader referencing the `KOJI_time` uniform will
automatically receive a timing buffer when its pipeline is built.
To access this uniform add:

```glsl
#include "timing.slang"
```

This makes a `KOJI_time` uniform available in set `0`, binding `0` without any
additional setup.

```
cargo run --example sample                        # run the triangle sample
cargo run --features gpu_tests --example text2d   # run an example requiring gpu_tests
```

## Available Examples

- **sample** – draw a single triangle and animate its color using `KOJI_time`
- **pbr_spheres** – grid of spheres with PBR shading
- **bindless_rendering** – textured triangle using bindless resources *(gpu_tests)*
- **skeletal_animation** – play an animated skeletal mesh *(gpu_tests)*
- **text2d** – draw 2D text using the text renderer *(gpu_tests)*
