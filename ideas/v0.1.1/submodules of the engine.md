We want to strike a good balance between decoupling and knowing what exists and what not.

Idea about Modules in the Engine:

What if modules can register themselves into the engine and when they do, we store a static reference to them somewhere that then all modules can query for. We can store that reference to have a safe connection to the other module.

Or does this sound like Spaghetti?

Imagine our Game State is one module, then we can query the engine for

> "Hey is the asset server module available? Yes? Okay I store a static reference to it, so I dont have to ask you again."

Most of this can happen in some initialization phase.
We really want to keep the engine pretty modular, right now we try to decouple the renderer a bit more.

If it is like this, we can maybe also get rid of static singletons, like are right now used for rendering a whole bunch...

References could also be given out at different access levels, maybe some references are only engine internal, while others are more global.

Lets see.
