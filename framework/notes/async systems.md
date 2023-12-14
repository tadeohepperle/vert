We want ot have a way to run async systems.

That is systems, that extract some Send + Sync information from the modules (sync point) then to their thing in the async tokio runtime and at a later frame, when it is determined that the system is done with its async thing, it can access the modules + state again and do something with it.

This allows, e.g. async asset loading over the network.

Imagine at the start of our program we want to make sure some texture is ready before using it.
Two options:

### Load it in the initialize function of our state.

- downside: halts program start, window does not even open yet...

### Kick off the loading in the background by spawning a tokio task and checking every frame if it is loaded.
