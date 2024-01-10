use std::ops::Range;

use fontdue::Font;
use glam::vec2;
use wgpu::VertexFormat;

use crate::{
    elements::{rect::Aabb, Color},
    modules::{arenas::Key, Attribute, VertexT},
};

use super::{
    board::{
        offset_dvec2, Board, BorderRadius, CachedTextLayout, Div, DivContent, Text, TextEntry,
    },
    font_cache::{FontSize, TextLayoutResult},
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
            SortPrimitive::Text { text, .. } => BatchRegion::Text(0..0, text.text.font),
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
                SortPrimitive::Text { text, .. } => BatchRegion::Text(0..0, text.text.font),
            };
            batches.push(new_batch);
        }

        // add the rect / the glyphs to the buffers.
        match prim {
            SortPrimitive::Rect { div } => {
                let rect_raw = RectRaw {
                    pos: div.computed_aabb(),
                    color: div.style.color,
                    border_radius: div.style.border_radius,
                    border_color: div.style.border_color,
                    border_thickness: div.style.border_thickness,
                    border_softness: div.style.border_softness,
                    _unused2: 0.0,
                    _unused3: 0.0,
                };
                rects.push(rect_raw);
            }
            SortPrimitive::Text { div, text } => {
                // todo! add text pos to glyphs
                let text_pos = text.c_pos.get();
                for (pos, uv) in text
                    .c_text_layout
                    .get()
                    .result
                    .glyph_pos_and_atlas_uv
                    .iter()
                    .copied()
                {
                    // dbg!(pos);
                    // dbg!(div_pos);
                    glyphs.push(GlyphRaw {
                        pos: pos + text_pos.as_vec2(),
                        color: text.text.color,
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
    Text { div: &'a Div, text: &'a TextEntry },
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
            SortPrimitive::Text { text, .. } => text.text.font.map(|e| e.as_u64()).unwrap_or(0),
        }
    }
}

#[derive(Debug)]
pub enum BatchRegion {
    Rect(Range<usize>),
    Text(Range<usize>, Option<Key<Font>>),
}

impl BatchRegion {
    #[inline]
    fn batch_key(&self) -> u64 {
        match self {
            BatchRegion::Rect(_) => u64::MAX,
            BatchRegion::Text(_, font) => font.map(|e| e.as_u64()).unwrap_or(0),
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
    border_radius: BorderRadius,
    border_color: Color,
    // these are bundled together into another 16 byte chunk.
    border_thickness: f32,
    border_softness: f32,
    _unused2: f32,
    _unused3: f32,
}

impl VertexT for RectRaw {
    const ATTRIBUTES: &'static [Attribute] = &[
        Attribute::new("pos", VertexFormat::Float32x4),
        Attribute::new("color", VertexFormat::Float32x4),
        Attribute::new("border_radius", VertexFormat::Float32x4),
        Attribute::new("border_color", VertexFormat::Float32x4),
        Attribute::new("others", VertexFormat::Float32x4),
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
