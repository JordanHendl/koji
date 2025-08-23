# Rendering Workflow

This guide outlines the typical Vulkan workflow used by Koji and how to drive it
from Koji's API. It covers initialization, resource creation, drawing, and
cleanup when building your own renderer on top of Koji.

## Core Koji Components

- **gpu::Context** – low level access to Vulkan (re-exported from Dashi).
- **Renderer** – orchestrates frame submission and presentation.
- **RenderGraph** or **Canvas** – describe render targets and pass ordering.
- **TextureManager** / `texture_manager` helpers – load textures with mip levels.

## Context and Device Initialization

1. Use `gpu::DeviceSelector` to choose a physical device and create a
   `gpu::Context`.
2. Enable validation layers when available.
3. Load function pointers through the entry and instance to avoid using
   uninitialized symbols.
4. Pass the `Context` into `Renderer::with_graph` or `Renderer::with_canvas`
   when constructing your renderer.

```rust
let device = gpu::DeviceSelector::new()?.select(gpu::DeviceFilter::default())?;
let mut ctx = gpu::Context::new(&gpu::ContextInfo { device })?;
let canvas = CanvasBuilder::new()
    .extent([800, 600])
    .color_attachment("color", Format::RGBA8)
    .build(&mut ctx)?;
let graph = RenderGraphBuilder::new()
    .add_canvas(&canvas)
    .build();
let mut renderer = Renderer::with_graph(800, 600, &mut ctx, graph)?;
// or load directly from configuration:
// let yaml = std::fs::read_to_string("graph_basic.yaml")?;
// let mut renderer = Renderer::with_graph_from_yaml(800, 600, &mut ctx, &yaml)?;
```

## Texture Creation

1. Allocate an image with the desired format and usage flags.
2. Specify the correct number of `mipLevels` when creating the image.
3. Upload the base level data and generate remaining mip levels with blits or
   compute shaders.
4. Transition each mip level to the layout expected by the pipeline.
5. With Koji, prefer using `texture_manager::load_from_file` or
   `TextureManager::register_image` to handle mip generation and resource
   bindings.

## Render Pass and Pipeline Setup

1. Define a render pass describing color and depth–stencil attachments. Koji
   provides `RenderPassBuilder` and `RenderGraph` nodes to do this declaratively.
2. Create a pipeline layout with descriptor set and push constant ranges.
3. Build a graphics pipeline that references shader stages and the render pass
   via `PipelineBuilder`.
4. Configure dynamic state (viewport, scissor, etc.) so they can be changed per
   frame.
5. Set depth–stencil state to match the attachments’ formats and desired compare
   operations.

## Frame Submission, Synchronization, and Presentation

1. Register meshes or text with the `Renderer` and call `present_frame` each
   loop iteration.
2. `present_frame` acquires the next swapchain image, records the render graph
   into command buffers, and submits work, signaling semaphores and optional
   fences.
3. Present the swapchain image once rendering is complete.

```rust
event_loop.run(move |event, _, control_flow| {
    match event {
        Event::MainEventsCleared => renderer.present_frame(&mut ctx).unwrap(),
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
            *control_flow = ControlFlow::Exit;
        }
        _ => {}
    }
});
```

## Resource Cleanup

Destroy resources in reverse order of creation after waiting for the device to
idle:

1. Semaphores and fences
2. Swapchains
3. Surfaces
4. Logical device
5. Vulkan instance

`Renderer::destroy` will release its own resources, but you are responsible for
calling `ctx.destroy()` afterwards.

## Developer Checklist

- [ ] Run `cargo test` and ensure all tests pass.
- [ ] Run with validation layers enabled and confirm **0** validation errors.

