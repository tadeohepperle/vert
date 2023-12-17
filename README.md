## Vert Game Engine

An experimental game engine framework, abusing type erased arenas and a form of trait reflection.

![vert](https://github.com/tadeohepperle/vert/assets/62739623/fa94f89b-ba90-40a9-940a-62df76558665)

This is a work in progress, mainly for myself to learn graphics programming with wgpu. The structure is gonna change a lot. 
The aim of this is to create a game engine that is simple enough to understand back to front for one person. It is not a goal of this engine to be as general-purpose as possible.

## Features ()

- [x] Type Erased arenas to store components of any type.
- [x] Trait reflection: Iterate over all components that implement some trait.
- [x] UI Rectangles with rounded borders
- [x] UI and 3d Text rendering
- [x] Tonemapping
- [x] Bloom
- [ ] PBR materials
- [ ] lighting
- [ ] audio
- [ ] async systems
- [ ] multithreading of systems
