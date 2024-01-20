# Classes of lifetimes in the game engine

## Static Lifetimes

Some Modules will likely have a lifetime from their initialization until the end of the program lifecycle.

## Singular Model lifetimes

If we have some submodule that is only present for some fraction of the programs lifecycle, how do we initialize it -> explicitly initialize it by giving it all the constructor variables.

# Control flow in the game engine:

```


setup() // setup all the resources needed for he system
loop{
   process_input()
   update_world()
   render()
}



```

# Models for module communication:

Static handles via UnsafeCell get passed around to every location that needs it.
=> This makes for a big net of objects with static lifetimes
=> kinda messy but easy to set up

All Communication goes through a global arena, that can be queried for different kinds of data. E.g. there is a renderer that will go through all rendergraph nodes that got put into the arena each frame.

Maybe for now we will just model all submodules as `Arc<dyn RenderNode>` or something.

# A problem we have:

We want to be able to define modules that interact with each other.
Some of these modules are allocated once at the start of the program and then never changed again.
Others might only live for a limited amount of time in the lifecycle of the application.
yet we want to give handles to these modules out to different places.
a type that is like an Arc in that it can give out weak references that cause the backing memory to not be deallocated yet.
