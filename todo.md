- Images in UI (Separate Batches with batch key being related to the image)
- Put all shaders into one big file to enable more code sharing?
- Add a slider widget to the UI (just to check that it is capable enough), probably needs Rect in Response
- Add Pbr Materials, Directional Lights, Point Lights, Shadow Maps?
- Submodules that get inserted at Runtime.
- remove msaa again and render ui on top of post processing.
- currently there are multiple ways to render text: unify them (e.g. instant geometry text vs. ui boards)

### Make module system independent of the rest of the code

- the user should have the choice between using modules (static handles + automatic initialization)
- or if they rather want to own a struct e.g. a graphicscontext.
- then they need to take care of the initialization and lifetime of it themselves.


| -       | clonable | always has value |
| ------- | -------- | ---------------- |
| Own     | no       | yes              |
| Ref     | yes      | no               |
| Eternal | yes      | yes              |
