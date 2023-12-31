use std::ops::Range;

use super::buffer::ToRaw;

#[derive(Debug)]
pub struct ImmediateMeshRanges {
    pub index_range: Range<u32>,
    pub instance_range: Range<u32>,
}

impl ImmediateMeshRanges {
    pub fn index_range(&self) -> Range<u32> {
        self.index_range.clone()
    }
    pub fn instance_range(&self) -> Range<u32> {
        self.instance_range.clone()
    }
}

#[derive(Debug)]
pub struct ImmediateMeshQueue<V: Copy, I: ToRaw> {
    /// index and instance ranges into the other vecs.
    immediate_meshes: Vec<ImmediateMeshRanges>,
    // buffers for immediate geometry, cleared each frame:
    vertices: Vec<V>,
    indices: Vec<u32>,
    instances: Vec<I::Raw>,
}

impl<V: Copy, I: ToRaw> Default for ImmediateMeshQueue<V, I> {
    fn default() -> Self {
        Self {
            immediate_meshes: Default::default(),
            vertices: Default::default(),
            indices: Default::default(),
            instances: Default::default(),
        }
    }
}

impl<V: Copy, I: ToRaw> ImmediateMeshQueue<V, I> {
    pub fn add_mesh(&mut self, vertices: &[V], indices: &[u32], transforms: &[I]) {
        let v_count = self.vertices.len() as u32;
        let i_count = self.indices.len() as u32;
        let t_count = self.instances.len() as u32;
        self.vertices.extend(vertices.iter().copied());
        self.indices.extend(indices.iter().map(|e| *e + v_count));
        self.instances.extend(transforms.iter().map(|e| e.to_raw()));
        self.immediate_meshes.push(ImmediateMeshRanges {
            index_range: i_count..(i_count + indices.len() as u32),
            instance_range: t_count..(t_count + transforms.len() as u32),
        });
    }

    /// Note: does not clear immediate meshes, those should be swapped out instead.
    pub fn clear_and_take_meshes(&mut self, out: &mut Vec<ImmediateMeshRanges>) {
        self.vertices.clear();
        self.indices.clear();
        self.instances.clear();
        out.clear();
        std::mem::swap(out, &mut self.immediate_meshes);
    }

    pub fn vertices(&self) -> &[V] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn instances(&self) -> &[I::Raw] {
        &self.instances
    }
}
