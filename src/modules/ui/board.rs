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
    utils::ChillCell,
};
use egui::ahash::HashSet;
use etagere::euclid::default;
use fontdue::layout::Layout;
use glam::{dvec2, vec2, DVec2, IVec2};
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
        let comm = self._add_div(props, style, id, DivContent::Children(vec![]), parent);
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
        let comm = self._add_div(props, style, id, DivContent::Text(text), parent);
        comm
    }

    fn _add_div(
        &mut self,
        props: LayoutProps,
        style: DivStyle,
        id: DivId,
        content: DivContent,
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
                div.content = content;
                // technically we could also invalidate the font cache here, if the content is children and not text. But doe not matter much.

                // return the Rect. (must be set, because the node was already inserted at a previous frame.)
                let size = div.c_size.get();
                let pos = div.c_pos.get();
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
                    props,
                    z_index,
                    last_frame: self.last_frame,
                    style,
                    content,
                    i_id: Cell::new(usize::MAX),
                    c_size: Cell::new(DVec2::ZERO),
                    c_pos: Cell::new(DVec2::ZERO),
                    c_content_size: Cell::new(DVec2::ZERO),
                    c_text_layout: ChillCell::new(None),
                });

                // rect not known yet.
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
    pub fn end_frame(&mut self, fonts: &FontCache) {
        assert_eq!(self.phase, BoardPhase::AddDivs);
        self.phase = BoardPhase::Rendering;

        // Remove Nodes that have not been added/updated this frame
        self.divs.retain(|_, v| v.last_frame == self.last_frame);

        // Perform Layout

        let layouter = Layouter::new(&self.divs, fonts);
        layouter.perform_full_layout(&self.top_level_children, self.top_level_size);
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

    // computed sizes and position
    pub(crate) c_size: Cell<DVec2>,
    pub(crate) c_content_size: Cell<DVec2>,
    pub(crate) c_pos: Cell<DVec2>,
    pub(crate) c_text_layout: ChillCell<Option<CachedTextLayout>>,
}

impl Div {
    #[inline(always)]
    pub fn computed_aabb(&self) -> Aabb {
        let size = self.c_size.get();
        let pos = self.c_pos.get();

        Aabb {
            min_x: pos.x as f32,
            min_y: pos.y as f32,
            max_x: (pos.x + size.x) as f32,
            max_y: (pos.y + size.y) as f32,
        }
    }

    #[inline(always)]
    pub fn computed_rect(&self) -> Rect {
        let size = self.c_size.get();
        let pos = self.c_pos.get();

        Rect {
            min_x: pos.x as f32,
            min_y: pos.y as f32,
            width: size.x as f32,
            height: size.y as f32,
        }
    }
}

struct Layouter<'a> {
    divs: &'a HashMap<DivId, Div>,
    fonts: &'a FontCache,
}

impl<'a> Layouter<'a> {
    fn new(divs: &'a HashMap<DivId, Div>, fonts: &'a FontCache) -> Self {
        Self { divs, fonts }
    }

    /// determine the Rect of each div on this board.
    /// ### Step 1: Determine Sizes of all Rects.
    /// - go down from top level (known px size) recursively and set a size for each div.
    ///   - if a div has a fixed size (px or percent of parent), use this size.
    ///   - if a div has a hugchildren size:
    ///     - if it has children: use sum of sizes of children in the axis direction.
    ///     - if it has text: layout the text and use the size of the text
    ///
    /// So: determine own size + determine size of children (order depends on fixed vs. hug)
    /// If Children: children_size = sum of all children, (0,0) if no children
    /// If Text: text size.
    ///
    /// Children with Absolute Positioning: just ignore during determining own size?
    ///
    /// ### Step 2: Determine Positioning:
    /// - Positioning depends on:
    ///    - Parent Axis (X or Y)
    ///    - Parent MainAxisAlignment (Start, Center, End, SpaceBetween, SpaceAround)
    ///    - Parent CrossAxisAlignment (Start, Center, End)
    ///
    fn perform_full_layout(&self, top_level_children: &[DivId], top_level_size: DVec2) {
        for id in top_level_children.iter() {
            let top_div = self.divs.get(id).unwrap();
            self.get_and_set_size(top_div, top_level_size);
        }
    }

    /// Calculates and sets the sizes of the given div and all of its children recursively.
    ///
    /// This follows 3 simple steps:
    /// 1. find out if width or height are contrained to a fixed size, or if they should hug the content.
    /// 2. figure out own size and content size
    /// 3. sache own size and content size in the div, then return own size.
    fn get_and_set_size(&self, div: &Div, parent_max_size: DVec2) -> DVec2 {
        let fixed_w = div.props.width.px_value(parent_max_size.x);
        let fixed_h = div.props.width.px_value(parent_max_size.y);

        let own_size: DVec2;
        let content_size: DVec2;
        // None values indicate, that the size value is not known yet.
        match (fixed_w, fixed_h) {
            (Some(x), Some(y)) => {
                own_size = dvec2(x, y);
                content_size = self.get_and_set_content_size(div, own_size);
            }
            (Some(x), None) => {
                // x is fixed, y height is the sum/max of children height (depending on axis y/x)
                let max_size = dvec2(x, parent_max_size.y);

                content_size = self.get_and_set_content_size(div, max_size);
                own_size = dvec2(x, content_size.y);
            }
            (None, Some(y)) => {
                // y is fixed, x width is the sum/max of children width (depending on axis y/x)
                let max_size = dvec2(parent_max_size.x, y);

                content_size = self.get_and_set_content_size(div, max_size);
                own_size = dvec2(content_size.x, y);
            }
            (None, None) => {
                // nothing is fixed, x width and y height are the sum/max of children widths and heights (depending on axis y/x)
                content_size = self.get_and_set_content_size(div, parent_max_size);
                own_size = content_size;
            }
        }

        div.c_size.set(own_size);
        div.c_content_size.set(content_size);

        own_size
    }

    /// Returns the size of the content of this div.
    ///   - if content is text, that is the size of the layouted text
    ///   - if content is other divs, sum up their
    ///
    /// This function caches the content size in `c_content_size` and then returns `c_content_size`.
    /// `content_max_size` is the max size the content (text or all children together) is allowed to take.
    fn get_and_set_content_size(&self, div: &Div, content_max_size: DVec2) -> DVec2 {
        let content_size: DVec2;
        match &div.content {
            DivContent::Text(text) => {
                content_size = self.get_text_size_or_layout_and_set(
                    text,
                    &div.c_text_layout,
                    content_max_size,
                );
            }
            DivContent::Children(children) => {
                content_size =
                    self.get_and_set_child_sizes(children, content_max_size, div.props.axis);
            }
        }
        div.c_content_size.set(content_size);
        content_size
    }

    /// Returns the size the children take all together.
    fn get_and_set_child_sizes(
        &self,
        children: &[DivId],
        parent_max_size: DVec2,
        parent_axis: Axis,
    ) -> DVec2 {
        let children = children.iter().map(|id| self.divs.get(id).unwrap());

        let mut all_children_size = DVec2::ZERO;
        match parent_axis {
            Axis::X => {
                for c in children {
                    let child_size = self.get_and_set_size(c, parent_max_size);
                    all_children_size.x += child_size.x;
                    all_children_size.y = all_children_size.y.max(child_size.y);
                }
            }
            Axis::Y => {
                for c in children {
                    let child_size = self.get_and_set_size(c, parent_max_size);
                    all_children_size.x = all_children_size.x.max(child_size.x);
                    all_children_size.y += child_size.y;
                }
            }
        }
        all_children_size
    }

    /// Returns the size of the layouted text.
    fn get_text_size_or_layout_and_set(
        &self,
        text: &Text,
        c_text_layout: &ChillCell<Option<CachedTextLayout>>,
        max_size: DVec2,
    ) -> DVec2 {
        let i_max_size = max_size.as_ivec2();
        // look for cached value and return it:
        let mut cached = c_text_layout.get_mut();
        if let Some(cached) = cached {
            if cached.max_size == i_max_size {
                return cached.result.total_rect.d_size();
            }
        }

        // otherwise layout the text:
        let layout_settings = fontdue::layout::LayoutSettings {
            x: 0.0,
            y: 0.0,
            max_width: Some(max_size.x as f32),
            max_height: Some(max_size.y as f32),
            ..Default::default() //  todo!() add more of these options to the Text struct.
        };
        let result =
            self.fonts
                .perform_text_layout(&text.string, None, &layout_settings, text.font);
        let text_size = result.total_rect.d_size();
        *cached = Some(CachedTextLayout {
            max_size: i_max_size,
            result,
        });
        text_size
    }
}

pub struct DivChildIter<'a> {
    i: usize,
    children_ids: &'a [DivId],
    divs: &'a HashMap<DivId, Div>,
}

impl<'a> DivChildIter<'a> {}

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
    Text(Text),
    Children(Vec<DivId>),
}

#[derive(Debug)]
pub struct Text {
    pub color: Color,
    pub string: Cow<'static, str>,
    pub font: Key<RasterizedFont>,
}

#[derive(Debug)]
pub struct CachedTextLayout {
    /// Width and Height that the text can take at Max. Right now the assumption is that the text is always bounded by some way (e.g. the entire screen).
    /// These can be integers, so that minor float differences do not cause a new layout.
    pub max_size: IVec2,
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
    main_align: MainAlign,
    cross_align: CrossAlign,
    // todo! translation, absolute
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    #[default]
    Y,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MainAlign {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CrossAlign {
    #[default]
    Start,
    Center,
    End,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
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
