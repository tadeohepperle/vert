## How to render stuff.

There are different ways of how to render things:

- batched vs instanced
- going over all entities in the ECS World vs immediate mode queue up of draw calls...

There is probably a distinction we want to make between two things:

big complicated meshes that should be created once and not modified.
We do not want to send all the verts and indices to the GPU over and over.
Instancing also makes sense here.

Let's consider a few things we might want to draw:

### Gizmos

- simple lines and dots

Best rendered:

- immediate mode, build up gizmos vertex and index buffer every frame.

### Rectangles in UI space

- with border rounding
- with a color per vertex
- with texture: e.g. for images, text glyphs
- or without texture: e.g. for buttons

- for each rect: store min and max points (bounding box), rotation, zindex

Best rendered:

- no owned buffers.
- instead every frame, build up a new quad buffer and write it to the GPU
- maybe StagingBelt from wgpu useful.
- need to know about aspect ratio, but no camera projection, bind group at (0) for the aspect ratio.

- one render pass
- at start of render pass sort by y-index.
- regarding textures:
  - group by textures at start of render pass (textures need to have bind groups already)
  - for each group:
    - set the bindgroup at (1) for the texure, then render all rects.

NO TRANSFORM: just convey all information via vertices (e.g. px_pos, color, uv)

Considerations:
what about custom fragment shaders for cool effects? Maybe not necessary right now.

Look at comfy engine, to see how this can be done.

### Colored Meshes

- relatively static meshes colored by vertex colors.
- Static vertices and index buffer
  - instanced (`Vec<Transform>`)
  - single (`Transform`)

Best rendered:

- should own their buffers (vertex, index, instance (transform)), these are rarely updated
- stored in the ECS as components
- render pipeline should set bindgroup (0) for camera (view projection mat4) at the start.

but later we could also consider something like Godot Immediate Geometry:

https://docs.godotengine.org/en/stable/tutorials/3d/procedural_geometry/immediatemesh.html

### Fully textured meshes

see colored meshes, but additionally set bindgroup for the texture of each mesh?
Not super important right now, probably combine with colored mesh, to have a single pipeline
Probably

### 3D Sprites and UI:

have a:

- texture
- color

# Lighting considerations

- not made at all at the moment.
