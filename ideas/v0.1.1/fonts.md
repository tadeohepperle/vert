For now we want to support font atlasses at different sizes.

The user wants to render some text, e.g. "Hello" with a certain size and a certain font and a certain rasterization size.

We don't want to rasterize all letters every frame.

It would be nice if the user can just let us know what fonts they need at what size upfront and then we can rasterize the fonts upfront.

When then a word is submitted to be rendered, we can just split it into the characters,
look up each character in the texture atlas for this font at this size,...

or we use one big texture atlas for everyone instead?? Yeah maybe.
