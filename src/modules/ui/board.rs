use std::{
    borrow::{Borrow, Cow},
    cell::{Cell, RefCell, UnsafeCell},
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
    iter::Map,
};

use crate::{
    elements::{rect::Aabb, Color, Rect},
    modules::{arenas::Key, input::PressState, Input},
    prelude::{glam::Vec2, winit::event::MouseButton},
    utils::YoloCell,
};
use egui::ahash::HashSet;
use etagere::euclid::default;
use fontdue::layout::Layout;
use glam::{dvec2, vec2, DVec2};
use smallvec::{smallvec, SmallVec};

use super::{
    batching::{get_batches, BatchingResult},
    font_cache::{FontCache, RasterizedFont, TextLayoutResult},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParentDivId {
    /// you cannot set this manually, to ensure only DivIds that belong to a Div with DivContent::Children.
    _priv: DivId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DivId(pub u64);

impl From<u64> for DivId {
    fn from(value: u64) -> Self {
        DivId(value)
    }
}

impl DivId {
    const TOP_LEVEL: DivId = DivId(u64::MAX);
}

/// A Board represents a screen, that contains UI-elements.
/// The Board could just represent the window screen directly, or be somewhere in the 3d space.
/// If a Board is in 3d space in the world, we just need to render it differently
/// and pass in the mouse pos via raycasting.
pub struct Board {
    last_frame: u64,
    phase: BoardPhase,
    input: BoardInput,
    top_level_size: DVec2,
    top_level_children: Vec<DivId>,
    divs: HashMap<DivId, Div>,
}

/// The Board alternates between two phases:
/// - in AddDivs you can add elements to the Board.
/// - in Render you cannot change the Board, but you can extract batches to render from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardPhase {
    /// In this phase elements can be added
    AddDivs,
    /// Static, extract batches from the divs and their texts in this phase.
    Rendering,
}

impl Board {
    #[inline]
    pub fn phase(&self) -> BoardPhase {
        self.phase
    }
}

impl Board {
    /// call to transition from  BoardPhase::Rendering -> BoardPhase::AddDivs.
    pub fn start_frame(&mut self, input: BoardInput) {
        assert_eq!(self.phase, BoardPhase::Rendering);
        self.input = input;
        self.phase = BoardPhase::AddDivs;
        self.top_level_children.clear();
    }

    pub fn iter_divs(&self) -> impl Iterator<Item = &Div> {
        self.divs.values()
    }

    pub fn new(board_size: DVec2) -> Self {
        println!("new Board created");
        let last_frame = 0;

        Board {
            last_frame,
            input: BoardInput::default(),
            divs: HashMap::new(),
            phase: BoardPhase::Rendering,
            top_level_size: board_size,
            top_level_children: vec![],
        }
    }

    pub fn add_non_text_div(
        &mut self,
        props: LayoutProps,
        style: DivStyle,
        id: DivId,
        parent: Option<ParentDivId>,
    ) -> (ParentDivId, Option<Comm>) {
        let comm = self._add_div(props, style, id, parent);
        (ParentDivId { _priv: id }, comm)
    }

    pub fn add_text_div(
        &mut self,
        props: LayoutProps,
        style: DivStyle,
        text: Text,
        id: DivId,
        parent: Option<ParentDivId>,
    ) -> Option<Comm> {
        let comm = self._add_div(props, style, id, parent);
        comm
    }

    fn _add_div(
        &mut self,
        props: LayoutProps,
        style: DivStyle,
        id: DivId,
        parent: Option<ParentDivId>,
    ) -> Option<Comm> {
        // go into the parent and register the child:
        let parent_z_index = if let Some(parent) = parent {
            let parent = self.divs.get_mut(&parent._priv).expect("Invalid Parent...");
            match &mut parent.content {
                DivContent::Text { .. } => panic!("Invalid Parent... Text Div cannnot be parent"),
                DivContent::Children(children) => children.push(id),
            };
            parent.z_index
        } else {
            self.top_level_children.push(id);
            0
        };

        // insert child entry. z_index is always 1 more than parent to render on top.
        let z_index = parent_z_index + 1 + style.z_bias * 1024;
        let rect: Option<Rect>;
        match self.divs.entry(id) {
            Entry::Occupied(mut e) => {
                let div = e.get_mut();

                if div.last_frame == self.last_frame {
                    panic!("Div with id {id:?} inserted twice in one frame!");
                }

                div.props = props;
                div.z_index = parent_z_index + 1;
                div.last_frame = self.last_frame;
                div.style = style;
                div.i_id.set(usize::MAX);
                div.content = DivContent::Children(vec![]);

                // return the Rect. (must be set, because the node was already inserted at a previous frame.)
                let size = div.computed_size.get();
                let pos = div.computed_pos.get();
                rect = Some(Rect {
                    min_x: pos.x as f32,
                    min_y: pos.y as f32,
                    width: size.x as f32,
                    height: size.y as f32,
                });
            }
            Entry::Vacant(vacant) => {
                vacant.insert(Div {
                    id,
                    z_index,
                    props,
                    last_frame: self.last_frame,
                    style,
                    i_id: Cell::new(usize::MAX),
                    computed_size: Cell::new(DVec2::ZERO),
                    computed_pos: Cell::new(DVec2::ZERO),
                    content: DivContent::Children(vec![]),
                });
                rect = None;
            }
        };

        // build up the response
        let comm = if let Some(rect) = rect {
            let mut comm = Comm {
                rect,
                hovered: false,
                clicked: false,
            };
            if let Some(cursor_pos) = self.input.cursor_pos {
                if rect.contains(cursor_pos) {
                    comm.hovered = true;
                    if self.input.left_mouse_button == PressState::JustPressed {
                        comm.clicked = true;
                    }
                }
            }
            Some(comm)
        } else {
            None
        };
        comm
    }

    /// call to transition from  BoardPhase::AddDivs -> BoardPhase::LayoutDone
    pub fn end_frame(&mut self, font_cache: &FontCache) {
        assert_eq!(self.phase, BoardPhase::AddDivs);
        self.phase = BoardPhase::Rendering;
        // /////////////////////////////////////////////////////////////////////////////
        // Remove Nodes that have not been added/updated this frame
        // /////////////////////////////////////////////////////////////////////////////

        self.divs.retain(|_, v| v.last_frame == self.last_frame);

        // /////////////////////////////////////////////////////////////////////////////
        // Perform Layout
        // /////////////////////////////////////////////////////////////////////////////

        // Do text layout on all divs that have text in them:

        todo!();

        // for div in self.divs.values() {
        //     if let Some(text) = &div.text {
        //         if self.text_cache.get(text).is_none() {
        //             println!("performed text layout for {text:?}");
        //             let layout_settings = text.layout_settings(div);
        //             let result = font_cache.perform_text_layout(
        //                 &text.string,
        //                 None,
        //                 &layout_settings,
        //                 text.font,
        //             );
        //             self.text_cache.insert(text.clone(), result);
        //         }
        //     }
        // }
    }

    // determine the Rect of each div on this board.
    fn perform_layout(&mut self, font_cache: &FontCache) {
        // todo!() insert divs again later! or chang this.
        let divs = std::mem::take(&mut self.divs);

        // let mut divs: Vec<&Div> = vec![];
        // for (i, d) in self.divs.values().enumerate() {
        //     d.i_id.set(i);
        //     divs.push(d);
        // }

        // go divs down, to compute the sizes:

        // calculates and sets the sizes of the given div and all of its children recursively.
        fn set_sizes(div: &Div, divs: &HashMap<DivId, Div>, mut parent_max_size: DVec2) -> DVec2 {
            let w = div.props.width.px_value(parent_max_size.x);
            let h = div.props.width.px_value(parent_max_size.y);
            // None values indicate, that the size value is not known yet.
            match (w, h) {
                (Some(x), Some(y)) => {
                    let own_size = dvec2(x, y);
                    div.computed_size.set(own_size);

                    match div.text_or_child_iter(divs) {
                        TextOrChildIter::Text(content) => {
                            todo!()
                        }
                        TextOrChildIter::Children(children) => {
                            for child in children {
                                set_sizes(child, divs, own_size);
                            }
                        }
                    }
                    return own_size;
                }
                (Some(x), None) => {
                    // x is fixed, for y height, sum up the heights of all children.
                    let max_size = dvec2(x, parent_max_size.y);
                    let mut children_height = 0.0;

                    match div.text_or_child_iter(divs) {
                        TextOrChildIter::Text(content) => todo!(),
                        TextOrChildIter::Children(children) => {
                            for child in children {
                                let child_size = set_sizes(child, divs, max_size);
                                children_height += child_size.y;
                            }
                        }
                    }

                    let own_size = dvec2(x, children_height);
                    div.computed_size.set(own_size);
                    return own_size;
                }
                (None, Some(y)) => {
                    // y is fixed, for x height, sum up the widths of all children.
                    let max_size = dvec2(parent_max_size.x, y);
                    let mut children_width = 0.0;

                    match div.text_or_child_iter(divs) {
                        TextOrChildIter::Text(content) => todo!(),
                        TextOrChildIter::Children(children) => {
                            for child in children {
                                let child_size = set_sizes(child, divs, max_size);
                                children_width += child_size.x;
                            }
                        }
                    }

                    let own_size = dvec2(children_width, y);
                    div.computed_size.set(own_size);
                    return own_size;
                }
                (None, None) => {
                    // nothing is fixed, sum up the widths and heights of all children.
                    let mut children_size = DVec2::ZERO;

                    match div.text_or_child_iter(divs) {
                        TextOrChildIter::Text(content) => todo!(),
                        TextOrChildIter::Children(children) => {
                            for child in children {
                                let child_size = set_sizes(child, divs, parent_max_size);
                                children_size += child_size;
                            }
                        }
                    }

                    let own_size = children_size;
                    div.computed_size.set(own_size);
                    return own_size;
                }
            }

            // // combinations of fixed sizes:
            // (Size::Px(x), Size::Px(y)) => {
            //     let own_size = vec2(x, y);
            //     div.computed_size.set(own_size);
            //     for child in div.children(&divs) {
            //         set_sizes(child, divs, own_size);
            //     }
            //     return own_size;
            // }
            // (Size::Px(x), Size::FractionOfParent(fry)) => {
            //     let own_size = vec2(x, fry * parent_max_size.y);
            //     div.computed_size.set(own_size);
            //     for child in div.children(&divs) {
            //         set_sizes(child, divs, own_size);
            //     }
            //     return own_size;
            // }
            // (Size::FractionOfParent(frx), Size::Px(y)) => {
            //     let own_size = vec2(frx * parent_max_size.x, y);
            //     div.computed_size.set(own_size);
            //     for child in div.children(&divs) {
            //         set_sizes(child, divs, own_size);
            //     }
            //     return own_size;
            // }
            // (Size::FractionOfParent(frx), Size::FractionOfParent(fry)) => {
            //     let own_size = vec2(frx * parent_max_size.x, fry * parent_max_size.y);
            //     div.computed_size.set(own_size);
            //     for child in div.children(&divs) {
            //         set_sizes(child, divs, own_size);
            //     }
            //     return own_size;
            // }
            // // now it gets more interesting:
            // (Size::Px(x), Size::HugContent) => {
            //     let max_size = vec2(x, parent_max_size.y);
            //     let mut children_height: f32 = 0.0;
            //     for child in div.children(&divs) {
            //         let child_size = set_sizes(child, divs, max_size);
            //         children_height += child_size.y;
            //     }
            //     let own_size = vec2(x, children_height);
            //     div.computed_size.set(own_size);
            //     return own_size;
            // }
            // (Size::HugContent, Size::Px(_)) => todo!(),
            // (Size::FractionOfParent(_), Size::HugContent) => todo!(),
            // (Size::HugContent, Size::FractionOfParent(_)) => todo!(),
            // (Size::HugContent, Size::HugContent) => todo!(),

            // if text, calculate layout and cache it.
            // let size: Option<Vec2> = match &div.content {
            //     DivContent::Text(text) => {
            //         if let Some(cached) = text_cache.get(&text.text_and_font) {
            //             Some(cached.total_rect)
            //         } else {
            //         }
            //     }
            //     DivContent::Children(_) => todo!(),
            // };

            // set the size
        }

        for id in self.top_level_children.iter() {
            let top_div = self.divs.get(id).unwrap();
            set_sizes(top_div, &divs, self.top_level_size);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BoardInput {
    pub left_mouse_button: PressState,
    pub right_mouse_button: PressState,
    pub scroll: f32,
    pub cursor_pos: Option<Vec2>,
    pub cursor_delta: Vec2,
}

impl BoardInput {
    pub fn from_input_module(input: &Input) -> Self {
        let left_mouse_button = input.mouse_buttons().press_state(MouseButton::Left);
        let right_mouse_button = input.mouse_buttons().press_state(MouseButton::Left);

        BoardInput {
            left_mouse_button,
            right_mouse_button,
            scroll: input.scroll().unwrap_or(0.0),
            cursor_pos: Some(input.cursor_pos()),
            cursor_delta: input.cursor_delta(),
        }
    }
}

pub struct Comm {
    rect: Rect,
    // Some, if the mouse is hovering, clicking or releasing?
    hovered: bool,
    clicked: bool,
}

#[derive(Debug)]
pub struct Div {
    id: DivId,
    pub(crate) content: DivContent,
    pub(crate) props: LayoutProps,
    pub(crate) style: DivStyle,
    // last_frame and i_id are reset every frame.
    last_frame: u64,
    i_id: Cell<usize>,
    // calculated as parent.z_index + 1, important for sorting in batching.
    pub(crate) z_index: i32,
    // upon insertion, this is just a zero Rect.
    pub(crate) computed_size: Cell<DVec2>,
    pub(crate) computed_pos: Cell<DVec2>,
}

impl Div {
    #[inline(always)]
    pub fn computed_aabb(&self) -> Aabb {
        let size = self.computed_size.get();
        let pos = self.computed_pos.get();

        Aabb {
            min_x: pos.x as f32,
            min_y: pos.y as f32,
            max_x: (pos.x + size.x) as f32,
            max_y: (pos.y + size.y) as f32,
        }
    }

    fn text_or_child_iter<'a>(&'a self, divs: &'a HashMap<DivId, Div>) -> TextOrChildIter<'a> {
        match &self.content {
            DivContent::Text(t) => TextOrChildIter::Text(t),
            DivContent::Children(children) => TextOrChildIter::Children(DivChildIter {
                i: 0,
                children_ids: children,
                divs,
            }),
        }
    }
}

enum TextOrChildIter<'a> {
    Text(&'a TextContent),
    Children(DivChildIter<'a>),
}

// pub struct DivChildIter<'a> {
//     i: usize,
//     div: &'a Div,
//     divs: &'a HashMap<DivId, Div>,
// }

// impl<'a> Iterator for DivChildIter<'a> {
//     type Item = &'a Div;

//     fn next(&mut self) -> Option<Self::Item> {
//         match &self.div.content {
//             DivContent::Text(_) => None,
//             DivContent::Children(children) => {
//                 let child_id = children.get(self.i)?;
//                 let child = self.divs.get(child_id).unwrap();
//                 self.i += 1;
//                 Some(child)
//             }
//         }
//     }
// }

pub struct DivChildIter<'a> {
    i: usize,
    children_ids: &'a [DivId],
    divs: &'a HashMap<DivId, Div>,
}

impl<'a> Iterator for DivChildIter<'a> {
    type Item = &'a Div;

    fn next(&mut self) -> Option<Self::Item> {
        let child_id = self.children_ids.get(self.i)?;
        let child = self.divs.get(child_id).unwrap();
        self.i += 1;
        Some(child)
    }
}

#[derive(Debug)]
pub struct DivStyle {
    pub color: Color,
    /// Note: z_bias is multiplied with 1024 when determining the final z_index and should be a rather small number.
    pub z_bias: i32,
}

#[derive(Debug)]
pub enum DivContent {
    Text(TextContent),
    Children(Vec<DivId>),
}

#[derive(Debug)]
pub struct TextContent {
    text: Text,
    /// None means text layout has not been computed yet or was since invalidated
    cached: YoloCell<Option<CachedTextLayout>>,
}

impl TextContent {
    /// invalidates the text layout cache if it differs from old cache
    pub fn set(&mut self, text: Text) {
        // invalidate cache if the text we want to set differs.
        if self.text.font != text.font || self.text.string != text.string {
            self.cached = YoloCell::new(None);
        }
        self.text = text;
    }

    pub fn new(text: Text) -> Self {
        TextContent {
            text,
            cached: YoloCell::new(None),
        }
    }

    #[inline(always)]
    pub fn text(&self) -> &Text {
        &self.text
    }

    // returns a Rect that covers the text
    pub fn get_cached_or_compute_layout(
        &self,
        fonts: &FontCache,
        max_width: Option<f32>,
        max_height: Option<f32>,
    ) -> Rect {
        let i_max_width = max_width.map(|e| e as i32);
        let i_max_height = max_height.map(|e| e as i32);

        // look for cached value and return it:
        let cached = self.cached.get_mut();
        if let Some(cached) = cached {
            if cached.max_width == i_max_width && cached.max_height == i_max_height {
                return cached.result.total_rect;
            }
        }

        // otherwise layout the text:
        let layout_settings = fontdue::layout::LayoutSettings {
            x: 0.0,
            y: 0.0,
            max_width,
            max_height,
            ..Default::default() //  todo!() add more of these options to the Text struct.
        };
        let result =
            fonts.perform_text_layout(&self.text.string, None, &layout_settings, self.text.font);
        let total_rect = result.total_rect;
        *cached = Some(CachedTextLayout {
            max_width: i_max_width,
            max_height: i_max_height,
            result,
        });
        total_rect
    }

    /// # Panics
    ///
    /// Expects that the text layout has been computed before. Panics if it is None
    pub fn get_cached_layout(&self) -> &TextLayoutResult {
        &self.cached.get().as_ref().unwrap().result
    }
}

#[derive(Debug)]
pub struct Text {
    pub color: Color,
    pub string: Cow<'static, str>,
    pub font: Key<RasterizedFont>,
}

#[derive(Debug)]
pub struct CachedTextLayout {
    pub max_width: Option<i32>,
    pub max_height: Option<i32>,
    pub result: TextLayoutResult,
}

#[derive(Debug)]
pub struct LayoutProps {
    // Determines width of Self
    width: Size,
    /// Determines height of Self
    height: Size,
    /// Determines how children are layed out.
    axis: Axis,
}

#[derive(Debug, Default)]
pub enum Axis {
    X,
    #[default]
    Y,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum Size {
    Px(f64),
    FractionOfParent(f64),
    #[default]
    HugContent,
}

impl Size {
    pub fn px_value(&self, parent_px_size: f64) -> Option<f64> {
        match self {
            Size::Px(x) => Some(*x),
            Size::FractionOfParent(fr) => Some(*fr * parent_px_size),
            Size::HugContent => None,
        }
    }
}
