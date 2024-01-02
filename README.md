## Vert Game Engine

An experimental game engine in Rust. Work in Progress.

![vert](https://github.com/tadeohepperle/vert/assets/62739623/fa94f89b-ba90-40a9-940a-62df76558665)

This is not an engine meant to be used by anyone yet, it is mainly for myself to learn graphics programming with wgpu and to write a 3D trading card game in this engine. The structure is gonna change a lot. The aim of this is to create a game engine that is simple enough to understand back to front for one person. It is not a goal of this engine to be as general-purpose as possible.

The game engine provides a system where you can specify Modules that have other Modules as dependencies. It then takes care of initializing modules when all the dependencies of a module have been already initialized. Cycles and missing modules are detected and reported at startup. The composition and dependency analysis of the modules is done at startup and not at compile time to not have too many generics and macros that slow down compilation.

## Features

- [x] Module system, to dynamically compose dependency hierarchies.
- [x] UI Rectangles with rounded borders
- [x] UI and 3d Text rendering
- [x] Tonemapping
- [x] Bloom
- [ ] PBR materials
- [ ] lighting
- [ ] audio
- [ ] async systems
- [ ] printing Module dependency graph.
- [ ] Render Graph (currently all renderering is done sequentially)
