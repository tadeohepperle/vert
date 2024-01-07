use std::ops::Range;

use wgpu::VertexFormat;

use crate::{
    elements::{rect::Aabb, Color},
    modules::{arenas::Key, Attribute, VertexT},
};

use super::{
    board::{Board, CachedTextLayout, Div, DivContent, Text, TextContent},
    font_cache::RasterizedFont,
};

/// Warning: call only after layout has been performed on the billboard (for all rects and the text in them)
pub fn get_batches(board: &Board) -> BatchingResult {
    // fill a vec of sort primitives
    // todo! reuse allocated vec next frame!
    let mut sort_primitives: Vec<SortPrimitive> = vec![];
    for div in board.iter_divs() {
        sort_primitives.push(SortPrimitive::Rect { div });
        // add the div itself as a primitive.

        if let DivContent::Text(text) = &div.content {
            sort_primitives.push(SortPrimitive::Text { div, text });
            // add the text glyphs as primitives.
        }
    }

    // sort them and then do batching accordingly: each sequence of Rects is a Rect Batch, each sequence of texts of the same font is a text batch.
    sort_primitives.sort_by(|a, b| a.z_index().cmp(&b.z_index()));

    // create continous batches that refer to either a bunch of rect or glyph instances.
    let mut rects: Vec<RectRaw> = vec![];
    let mut glyphs: Vec<GlyphRaw> = vec![];
    let mut batches: Vec<BatchRegion> = vec![];

    if let Some(first) = sort_primitives.first() {
        let batch = match first {
            SortPrimitive::Rect { .. } => BatchRegion::Rect(0..0),
            SortPrimitive::Text { text: div_text, .. } => {
                BatchRegion::Text(0..0, div_text.text().font)
            }
        };
        batches.push(batch);
    }

    for prim in sort_primitives {
        // if the last batch is incompatible, set its end index correctly and start a new batch.
        let batch = batches.last_mut().unwrap();
        let incompatible = batch.batch_key() != prim.batch_key();
        if incompatible {
            // end the current batch:
            match batch {
                BatchRegion::Rect(r) => r.end = rects.len(),
                BatchRegion::Text(r, _) => r.end = glyphs.len(),
            }
            // create a new batch:
            let new_batch = match prim {
                SortPrimitive::Rect { .. } => BatchRegion::Rect(0..0),
                SortPrimitive::Text { text: div_text, .. } => {
                    BatchRegion::Text(0..0, div_text.text().font)
                }
            };
            batches.push(new_batch);
        }

        // add the rect / the glyphs to the buffers.
        match prim {
            SortPrimitive::Rect { div } => {
                let rect_raw = RectRaw {
                    pos: div.computed_aabb(),
                    color: div.style.color,
                };
                rects.push(rect_raw);
            }
            SortPrimitive::Text { div, text } => {
                // todo! this lookup is bad for performance! a simple pointer to the layout result would be better.
                let layout_res = text.get_cached_layout();
                for (pos, uv) in layout_res.glyph_pos_and_atlas_uv.iter().copied() {
                    glyphs.push(GlyphRaw {
                        pos,
                        color: text.text().color,
                        uv,
                    });
                }
            }
        }
    }

    // end the last batch:
    if !batches.is_empty() {
        let region = batches.last_mut().unwrap();
        match region {
            BatchRegion::Rect(r) => r.end = rects.len(),
            BatchRegion::Text(r, _) => r.end = glyphs.len(),
        }
    }

    BatchingResult {
        rects,
        glyphs,
        batches,
    }
}

#[derive(Debug, Clone, Copy)]
enum SortPrimitive<'a> {
    Rect { div: &'a Div },
    Text { div: &'a Div, text: &'a TextContent },
}

impl<'a> SortPrimitive<'a> {
    /// Returns the z index of this [`SortPrimitive`]. Adds 16 for text, to make batching work better.
    #[inline]
    fn z_index(&self) -> i32 {
        match self {
            SortPrimitive::Rect { div } => div.z_index,
            SortPrimitive::Text { div, .. } => div.z_index + 16,
        }
    }

    #[inline]
    fn batch_key(&self) -> u64 {
        match self {
            SortPrimitive::Rect { .. } => u64::MAX,
            SortPrimitive::Text { text: div_text, .. } => div_text.text().font.as_u64(),
        }
    }
}

#[derive(Debug)]
pub enum BatchRegion {
    Rect(Range<usize>),
    Text(Range<usize>, Key<RasterizedFont>),
}

impl BatchRegion {
    #[inline]
    fn batch_key(&self) -> u64 {
        match self {
            BatchRegion::Rect(_) => u64::MAX,
            BatchRegion::Text(_, font) => font.as_u64(),
        }
    }
}

#[derive(Debug)]
pub struct BatchingResult {
    pub rects: Vec<RectRaw>,
    pub glyphs: Vec<GlyphRaw>,
    pub batches: Vec<BatchRegion>,
}

impl BatchingResult {
    pub fn new() -> Self {
        BatchingResult {
            rects: vec![],
            glyphs: vec![],
            batches: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        let empty = self.batches.is_empty();
        if empty {
            assert!(self.rects.is_empty());
            assert!(self.glyphs.is_empty());
        }
        empty
    }

    pub fn combine(&mut self, mut other: BatchingResult) {
        if self.is_empty() {
            *self = other;
            return;
        }

        let rects_before: usize = self.rects.len();
        let glyphs_before: usize = self.glyphs.len();
        self.rects.append(&mut other.rects);
        self.glyphs.append(&mut other.glyphs);
        // adjust indices of batch regions, because they now point to later regions in the other two vectors.
        self.batches
            .extend(other.batches.into_iter().map(|e| match e {
                BatchRegion::Rect(r) => BatchRegion::Rect(Range {
                    start: r.start + rects_before,
                    end: r.end + rects_before,
                }),
                BatchRegion::Text(r, font) => BatchRegion::Text(
                    Range {
                        start: r.start + glyphs_before,
                        end: r.end + glyphs_before,
                    },
                    font,
                ),
            }))
    }
}

#[repr(C)]
#[derive(Debug, Clone, bytemuck::Pod, bytemuck::Zeroable, Copy)]
pub struct RectRaw {
    pos: Aabb,
    color: Color,
    // border_radius: [] // todo!() needle
}

impl VertexT for RectRaw {
    const ATTRIBUTES: &'static [Attribute] = &[
        Attribute::new("pos", VertexFormat::Float32x4),
        Attribute::new("color", VertexFormat::Float32x4),
    ];
}

#[repr(C)]
#[derive(Debug, Clone, bytemuck::Pod, bytemuck::Zeroable, Copy)]
pub struct GlyphRaw {
    pos: Aabb,
    color: Color,
    uv: Aabb,
}

impl VertexT for GlyphRaw {
    const ATTRIBUTES: &'static [Attribute] = &[
        Attribute::new("pos", VertexFormat::Float32x4),
        Attribute::new("color", VertexFormat::Float32x4),
        Attribute::new("uv", VertexFormat::Float32x4),
    ];
}
