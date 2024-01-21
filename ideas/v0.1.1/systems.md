# Systems

We do not really need systems, we could just write all of our logic in the main loop.
However it is pretty nice to offload some of the logic to separate functions that run every frame and that can decide when to stop running at any time, much like an Actor.

Systems in this sense are not like e.g. Bevys systems that are registered at the start and cannot really be removed or changed.

Instead systems should just provide an abstraction to do some work independently of the main update loop, much like Coroutines, that step forward a bit every frame.
Systems should be registered and deregistered at runtime.
They extract the information they need from the modules and the state,
run, and then decide to either continue next frame, stop, or launch another system.

One example for this could be a system that is spawned to play an animation for a dying character. it plays for 2 seconds, then launches another system to make the character animate away, then terminates and it removed from the running systems.

Systems should be able to express sequences of actions and execute them once spawned.

E.g. when an enemy dies, we can just call:

spawn(entity_death_system) which will take the next 5 seconds to do some stuff and in the end taking care of the cleanup of the enemy meshes, etc.

Or a system A that alternates between executing itself for 100 frames, then executing system B for 100 frames, which when finished, executes system A again in an infinite cycle.

Probably these systems need to be spawned in a tree-like structure:
Spawning a system from another system inserts a child node.

imagine we kick off system A by spawning it. It runs every frame.
We need to get back a handle to it, such that we could shut system A down later.
But what if system A ends and spawns system B? Then system B should take the spot of system A and our handle is able to take down B. This is good.

Imagine we obtain a handle to A, which then spawns 3 child systems C,D,E. Our handle should be able to take out all of them at once in order to stop them from running.

Also we probably want to have systems that can operate asynchronously in a background thread and not run every frame.

Imagine:
async System A: when started, fetches an image from the internet (takes 0.5s)
-> when done, runs System B with its output:
System B: hooks back into the modules, creating a texture on the GPU.

Idk, how can be get access to this image back???

Maybe systems are not such a good idea right now, lets handle rendering first.

But what is a nice interface to spawn a sprite that needs a texture?

We would like to do something like:

```rs
modules.spawn_sprite_3d(Vec::Zero, "https://th.com/img.png");
```

this will of course make the displaying of the sprite delayed, but it should register it in some asset server to make the delay only appear once.

Or should we never spawn a sprite like this and just always handle fetching textures first? -> Probably.
