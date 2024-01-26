//! Module for layout of ui elements on a board.
//!
//! The board represents a screen, card or anything. Elements can be added in an immediate mode API.

use std::{
    borrow::Cow,
    cell::Cell,
    collections::{
        hash_map::{Entry, OccupiedEntry},
        HashMap,
    },
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::{Add, Deref, DerefMut, Mul, Sub},
};

use crate::{
    elements::{rect::Aabb, BindableTexture, Color, Rect},
    ext::glam::Vec2,
    modules::{input::MouseButtonState, Input},
    utils::ChillCell,
    Ptr,
};
use egui::Ui;

use fontdue::{
    layout::{HorizontalAlign, VerticalAlign},
    Font,
};
use glam::{dvec2, DVec2, IVec2};
use rand::Rng;
use smallvec::smallvec;

use super::{
    font_cache::{FontCache, FontSize, TextLayoutItem, TextLayoutResult},
    widgets::Widget,
};

/// A wrapper around a non-text div that can be used as a parent key when inserting a child div.
/// (text divs cannot have children).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContainerId {
    /// you cannot set this manually, to ensure only DivIds that belong to a Div with DivContent::Children.
    _priv: Id,
}

/// A wrapper around a div that has been inserted into the tree with no parent, but is not a top level div.
/// It can be set as the child of another div later. A bit hacky.
///
/// The UnboundId can also not be cloned, such that the div cannot be set as the child of multiple divs.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnboundId {
    /// you cannot set this manually, to ensure only DivIds that belong to a Div with DivContent::Children.
    _priv: Id,
}

impl From<()> for ContainerId {
    fn from(_value: ()) -> Self {
        ContainerId::NONE
    }
}

impl ContainerId {
    ///  Warning: this is an illegal value!
    const NONE: ContainerId = ContainerId {
        _priv: Id(u64::MAX),
    };

    pub fn id(&self) -> Id {
        self._priv
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub u64);

impl Add<u64> for Id {
    type Output = Id;

    fn add(self, rhs: u64) -> Self::Output {
        Id(self.0 + rhs)
    }
}

impl From<&'static str> for Id {
    fn from(value: &'static str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut hasher);
        let h = hasher.finish();
        Self(h)
    }
}

impl From<u64> for Id {
    fn from(value: u64) -> Self {
        Id(value)
    }
}

impl From<()> for Id {
    fn from(_: ()) -> Self {
        Self(rand::thread_rng().gen())
    }
}

/// A Board represents a canvas/screen, that we can add UI-elements too. It has a bounded fixed size.
/// The Board could just represent the window screen directly, or be somewhere in the 3d space.
/// If a Board is in 3d space in the world, we just need to render it differently
/// and pass in the mouse pos via raycasting.
pub struct Board {
    last_frame: u64,
    phase: BoardPhase,
    input: BoardInput,
    top_level_size: DVec2,
    top_level_children: Vec<Id>,
    divs: HashMap<Id, Div>,
    divs_added_this_frame: usize,

    // experimental:
    hot_active: HotActiveWithId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotActiveWithId {
    None,
    Hot(Id),
    Active(Id),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotActive {
    Nil,
    Hot,
    Active,
}

impl HotActive {
    pub fn is_none(&self) -> bool {
        matches!(self, HotActive::Nil)
    }

    pub fn is_hot(&self) -> bool {
        matches!(self, HotActive::Hot)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, HotActive::Active)
    }
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
    pub fn start_frame(&mut self, input: BoardInput, top_level_size: DVec2) {
        assert_eq!(self.phase, BoardPhase::Rendering);
        self.input = input;
        self.phase = BoardPhase::AddDivs;
        self.top_level_children.clear();
        self.top_level_size = top_level_size;
    }

    pub fn iter_divs(&self) -> impl Iterator<Item = &Div> {
        self.divs.values()
    }

    pub fn input(&self) -> &BoardInput {
        &self.input
    }

    pub fn hot_active(&self, id: Id) -> HotActive {
        match self.hot_active {
            HotActiveWithId::Hot(i) if i == id => HotActive::Hot,
            HotActiveWithId::Active(i) if i == id => HotActive::Active,
            _ => HotActive::Nil,
        }
    }

    pub fn set_hot_active(&mut self, id: Id, state: HotActive) {
        match state {
            HotActive::Nil => {
                // dont allow change to none if currently other item is hot or active
                if matches!(self.hot_active, HotActiveWithId::Hot(i) | HotActiveWithId::Active(i) if i != id)
                {
                    return;
                }
                self.hot_active = HotActiveWithId::None;
            }
            HotActive::Hot => self.hot_active = HotActiveWithId::Hot(id),
            HotActive::Active => self.hot_active = HotActiveWithId::Active(id),
        }
    }

    pub fn new(board_size: DVec2) -> Self {
        let last_frame = 0;

        Board {
            last_frame,
            input: BoardInput::default(),
            divs: HashMap::new(),
            phase: BoardPhase::Rendering,
            top_level_size: board_size,
            top_level_children: vec![],
            hot_active: HotActiveWithId::None,
            divs_added_this_frame: 0,
        }
    }

    pub fn add<W: Widget>(
        &mut self,
        widget: W,
        id: impl Into<Id>,
        parent: Option<ContainerId>,
    ) -> W::Response<'_> {
        widget.add_to_board(self, id.into(), parent)
    }

    pub fn add_div(
        &mut self,
        id: impl Into<Id>,
        parent: Option<ContainerId>,
    ) -> Response<'_, ContainerId> {
        let id: Id = id.into();
        let parent = match parent {
            Some(p) => DivParent::Parent(p._priv),
            None => DivParent::TopLevel,
        };
        let (comm, entry) = self._add_div(id, None, parent);
        let div_id = ContainerId { _priv: id };
        Response {
            id: div_id,
            comm,
            entry,
        }
    }

    pub fn add_unbound_div(&mut self, id: impl Into<Id>) -> Response<'_, UnboundId> {
        let id: Id = id.into();
        let (comm, entry) = self._add_div(id, None, DivParent::Unbound);
        let div_id = UnboundId { _priv: id };
        Response {
            id: div_id,
            comm,
            entry,
        }
    }

    pub fn add_text_div(
        &mut self,
        text: Text,
        id: impl Into<Id>,
        parent: Option<ContainerId>,
    ) -> Response<'_, TextMarker> {
        let id: Id = id.into();
        let parent = match parent {
            Some(p) => DivParent::Parent(p._priv),
            None => DivParent::TopLevel,
        };
        let (comm, entry): (Comm, OccupiedEntry<'_, Id, Div>) =
            self._add_div(id, Some(text), parent);
        Response {
            comm,
            entry,
            id: TextMarker,
        }
    }

    fn _add_div<'a>(
        &'a mut self,
        id: Id,
        text: Option<Text>,
        parent: DivParent,
    ) -> (Comm, OccupiedEntry<'a, Id, Div>) {
        //Node: opt!() currently we need to do two hash table resolves, which (might be??) the bulk of the work? Not sure, probably totally Fine.
        self.divs_added_this_frame += 1;
        // go into the parent and register the child:

        match parent {
            DivParent::Unbound => {}
            DivParent::TopLevel => {
                self.top_level_children.push(id);
            }
            DivParent::Parent(parent) => {
                let parent = self.divs.get_mut(&parent).expect("Invalid Parent...");
                match &mut parent.content {
                    DivContent::Text { .. } => {
                        panic!("Invalid Parent... Text Div cannnot be parent")
                    }
                    DivContent::Children(children) => children.push(id),
                };
            }
        }

        // Note: This is super naive and should be changed in the future.
        let z_index = self.divs_added_this_frame as i32;
        let rect: Option<Rect>;
        let entry: OccupiedEntry<'a, Id, Div>;
        match self.divs.entry(id) {
            Entry::Occupied(mut e) => {
                let div = e.get_mut();
                if div.last_frame == self.last_frame {
                    panic!("Div with id {id:?} inserted twice in one frame!");
                }
                div.last_frame = self.last_frame;

                match text {
                    Some(new_text) => match &mut div.content {
                        DivContent::Text(old_text) if old_text.text.same_glyphs(&new_text) => {
                            // keep the computed layout, just set the new text (color, offset, etc...; font and size should be the same)
                            old_text.text = new_text
                        }
                        e => *e = DivContent::Text(TextEntry::new(new_text)),
                    },
                    None => div.content = DivContent::Children(vec![]),
                }
                div.z_index.set(z_index);
                // return the Rect. (must be set, because the node was already inserted at a previous frame. Maybe not up to date anymore, but good enough.)
                let size = div.c_size.get();
                let pos = div.c_pos.get();
                rect = Some(Rect {
                    min_x: pos.x as f32,
                    min_y: pos.y as f32,
                    width: size.x as f32,
                    height: size.y as f32,
                });
                entry = e;
            }
            Entry::Vacant(vacant) => {
                entry = vacant.insert_entry(Div {
                    z_index: Cell::new(z_index),
                    last_frame: self.last_frame,
                    style: DivStyle::default(),
                    content: match text {
                        Some(text) => DivContent::Text(TextEntry::new(text)),
                        None => DivContent::Children(vec![]),
                    },
                    c_size: Cell::new(DVec2::ZERO),
                    c_pos: Cell::new(DVec2::ZERO),
                    c_content_size: Cell::new(DVec2::ZERO),
                    c_padding: Cell::new(ComputedPadding::ZERO),
                });

                // rect not known yet.
                rect = None;
            }
        };

        // build up the response
        let mut comm = Comm {
            mouse_in_rect: false,
        };

        if let Some(rect) = rect {
            if let Some(cursor_pos) = self.input.cursor_pos {
                if rect.contains(cursor_pos) {
                    comm.mouse_in_rect = true;
                }
            }
        };

        (comm, entry)
    }

    /// call to transition from  BoardPhase::AddDivs -> BoardPhase::LayoutDone
    pub fn end_frame(&mut self, fonts: &mut FontCache) {
        assert_eq!(self.phase, BoardPhase::AddDivs);
        self.phase = BoardPhase::Rendering;

        // Remove Nodes that have not been added/updated this frame
        self.divs.retain(|_, v| v.last_frame == self.last_frame);
        self.divs_added_this_frame = 0;
        self.last_frame += 1;

        // Perform Layout (set sizes and positions for all divs in the tree)
        let mut layouter = Layouter::new(&self.divs, fonts);
        layouter.perform_layout(&self.top_level_children, self.top_level_size);
    }
}

enum DivParent {
    Unbound,
    TopLevel,
    Parent(Id),
}

pub struct Response<'a, ID> {
    /// to be used as a parent for another div
    pub id: ID,
    comm: Comm,
    pub entry: OccupiedEntry<'a, Id, Div>,
}

impl<'a, ID> Deref for Response<'a, ID> {
    type Target = DivStyle;

    fn deref(&self) -> &Self::Target {
        &self.entry.get().style
    }
}

impl<'a> Response<'a, ContainerId> {
    /// Warning: This is not well tested yet.
    pub fn add_child(&mut self, id: UnboundId) {
        let div = self.entry.get_mut();
        match &mut div.content {
            DivContent::Text(t) => unreachable!("The should never be a text in a parent div"),
            DivContent::Children(children) => children.push(id._priv),
        }
    }
}

impl<'a, ID> DerefMut for Response<'a, ID> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entry.get_mut().style
    }
}

impl<'a, ID> Response<'a, ID> {
    pub fn mouse_in_rect(&self) -> bool {
        self.comm.mouse_in_rect
    }

    pub fn add_z_bias(&mut self, z_bias: i32) {
        let entry = self.entry.get_mut();
        entry.z_index.set(entry.z_index.get() + z_bias);
    }
}

pub struct TextMarker;
impl<'a> Response<'a, TextMarker> {
    pub fn text(&mut self) -> &mut Text {
        match &mut self.entry.get_mut().content {
            DivContent::Text(text_e) => &mut text_e.text,
            DivContent::Children(_) => unreachable!("This should always be text on a text div"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BoardInput {
    pub mouse_buttons: MouseButtonState,
    pub scroll: f32,
    pub cursor_pos: Option<Vec2>,
    pub cursor_delta: Vec2,
}

impl BoardInput {
    /// todo! other function from input module + camera + plane in 3d space => 3d game world ui!
    pub fn from_input_module(input: &Input) -> Self {
        BoardInput {
            mouse_buttons: *input.mouse_buttons(),
            scroll: input.scroll().unwrap_or(0.0),
            cursor_pos: Some(input.cursor_pos()),
            cursor_delta: input.cursor_delta(),
        }
    }
}

/// Communication for each Rect
pub struct Comm {
    // Some, if the mouse is hovering, clicking or releasing?
    pub mouse_in_rect: bool,
}

struct Layouter<'a> {
    divs: &'a HashMap<Id, Div>,
    fonts: &'a mut FontCache,
}

impl<'a> Layouter<'a> {
    fn new(divs: &'a HashMap<Id, Div>, fonts: &'a mut FontCache) -> Self {
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
    fn perform_layout(&mut self, top_level_children: &[Id], top_level_size: DVec2) {
        // Note: Right now top level divs have no relationship to each other, they are all individually positioned on the screen.
        // That means: adding another top level div never changes the position of other top level divs.
        for id in top_level_children.iter() {
            let top_div = self.divs.get(id).unwrap();
            // set the size of each div in the tree:
            _ = self.get_and_set_size(top_div, top_level_size);
            let offset = offset_dvec2(
                top_div.style.offset_x,
                top_div.style.offset_y,
                top_level_size,
            );
            top_div.c_pos.set(offset);
            // set the position of each div in the tree:
            self.set_child_positions(top_div);
        }
    }

    /// Calculates and sets the sizes of the given div and all of its children recursively.
    ///
    /// This follows 3 simple steps:
    /// 1. find out if width or height are contrained to a fixed size, or if they should hug the content.
    /// 2. figure out own size and content size
    /// 3. sache own size and content size in the div, then return own size.
    fn get_and_set_size(&mut self, div: &Div, parent_max_size: DVec2) -> DVec2 {
        // fixed width
        let mut fixed_width: Option<f64> = None;
        let mut fixed_height: Option<f64> = None;

        if let Some(width) = div.style.width {
            fixed_width = Some(width.fixed(parent_max_size.x));
        };

        if let Some(height) = div.style.height {
            fixed_height = Some(height.fixed(parent_max_size.y));
        };

        let padding = &div.style.padding;

        let own_size: DVec2;
        let content_size: DVec2;
        let mut c_padding: ComputedPadding = ComputedPadding::ZERO;
        // None values indicate, that the size value is not known yet.
        match (fixed_width, fixed_height) {
            (Some(x), Some(y)) => {
                // both x and y are fixed, padding and own size can be calculated in advance
                own_size = dvec2(x, y);

                padding.compute_x(own_size.x, &mut c_padding);
                padding.compute_y(own_size.y, &mut c_padding);
                let max_size = own_size - dvec2(c_padding.width(), c_padding.height());

                content_size = self.get_and_set_content_size(div, max_size);
            }
            (Some(x), None) => {
                // x is fixed, y height is the sum/max of children height (depending on axis y/x)
                padding.compute_x(x, &mut c_padding);

                let max_size = dvec2(x - c_padding.width(), parent_max_size.y);
                content_size = self.get_and_set_content_size(div, max_size);

                padding.compute_y_from_content(content_size.y, &mut c_padding);

                // in y direction add the padding on top of the content size:
                own_size = dvec2(x, content_size.y + c_padding.height());
            }
            (None, Some(y)) => {
                // y is fixed, x width is the sum/max of children width (depending on axis y/x)
                padding.compute_y(y, &mut c_padding);

                let max_size = dvec2(parent_max_size.x, y);
                content_size = self.get_and_set_content_size(div, max_size);

                padding.compute_x_from_content(content_size.x, &mut c_padding);

                // in x direction add the padding on top of the content size:
                own_size = dvec2(content_size.x + c_padding.width(), y);
            }
            (None, None) => {
                // nothing is fixed, x width and y height are the sum/max of children widths and heights (depending on axis y/x)
                content_size = self.get_and_set_content_size(div, parent_max_size);

                padding.compute_x_from_content(content_size.x, &mut c_padding);
                padding.compute_y_from_content(content_size.y, &mut c_padding);

                own_size = dvec2(
                    content_size.x + c_padding.width(),
                    content_size.y + c_padding.height(),
                );
            }
        }

        div.c_size.set(own_size);
        div.c_content_size.set(content_size);
        div.c_padding.set(c_padding);

        own_size
    }

    /// Returns the size of the content of this div.
    ///   - if content is text, that is the size of the layouted text
    ///   - if content is other divs, sum up their
    ///
    /// This function caches the content size in `c_content_size` and then returns `c_content_size`.
    /// `content_max_size` is the max size the content (text or all children together) is allowed to take.
    fn get_and_set_content_size(&mut self, div: &Div, content_max_size: DVec2) -> DVec2 {
        let content_size: DVec2;
        match &div.content {
            DivContent::Text(text_entry) => {
                content_size = self.get_text_size_or_layout_and_set(text_entry, content_max_size);
            }
            DivContent::Children(children) => {
                content_size =
                    self.get_and_set_child_sizes(children, content_max_size, div.style.axis);
            }
        }
        div.c_content_size.set(content_size);
        content_size
    }

    /// Returns the size the children take all together.
    fn get_and_set_child_sizes(
        &mut self,
        children: &[Id],
        parent_max_size: DVec2,
        parent_axis: Axis,
    ) -> DVec2 {
        let children = children.iter().map(|id| self.divs.get(id).unwrap());

        let mut all_children_size = DVec2::ZERO;
        match parent_axis {
            Axis::X => {
                for c in children {
                    let child_size = self.get_and_set_size(c, parent_max_size);

                    // children with absolute positioning should not contribute to the size of the parent.
                    if !c.style.absolute {
                        all_children_size.x += child_size.x;
                        all_children_size.y = all_children_size.y.max(child_size.y);
                    }
                }
            }
            Axis::Y => {
                for c in children {
                    let child_size = self.get_and_set_size(c, parent_max_size);

                    // children with absolute positioning should not contribute to the size of the parent.
                    if !c.style.absolute {
                        all_children_size.x = all_children_size.x.max(child_size.x);
                        all_children_size.y += child_size.y;
                    }
                }
            }
        }
        all_children_size
    }

    /// Returns the size of the layouted text.
    fn get_text_size_or_layout_and_set(
        &mut self,
        text_entry: &TextEntry,
        max_size: DVec2,
    ) -> DVec2 {
        // iterate over all the divs inside of the text spans (hopefully strictly sized, and size them)
        for span in text_entry.text.spans.iter() {
            let Span::FixedSizeDiv {
                id,
                width,
                font_size,
            } = span
            else {
                continue;
            };
            let div = self.divs.get(&id._priv).expect("Div was inserted");
            self.get_and_set_size(div, dvec2(*width as f64, font_size.0 as f64));
        }

        let i_max_size = max_size.as_ivec2();
        // look for cached value and return it:
        let cached = text_entry.c_text_layout.get_mut();
        if cached.max_size == i_max_size {
            return cached.result.total_rect.d_size();
        }

        // otherwise layout the text:
        // dbg!(text_entry);
        // dbg!(max_size);
        let layout_settings = fontdue::layout::LayoutSettings {
            x: 0.0,
            y: 0.0,
            max_width: Some(max_size.x as f32),
            max_height: Some(max_size.y as f32),
            // We only support Left right now, because there are issues with fontdues text layout:
            // If you specify e.g. Center, it will always center it to the provided max_size. This is a bit bad,
            // because it then returns a way bigger size than the text actually takes and the text is then drawn in the top right corner.
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            line_height: 1.0,
            wrap_style: fontdue::layout::WrapStyle::Word,
            wrap_hard_breaks: true, // todo!() needle expose these functions
        };

        let spans = text_entry.text.spans.iter().map(|e| match e {
            Span::Text(t) => TextLayoutItem::Text(t),
            Span::FixedSizeDiv {
                id,
                width,
                font_size,
            } => TextLayoutItem::Space {
                width: *width,
                fontsize: *font_size,
            },
        });

        let result = self
            .fonts
            .perform_text_layout(spans, &layout_settings, text_entry.text.font);
        // dbg!(&result);
        let text_size = result.total_rect.d_size();
        *cached = CachedTextLayout {
            max_size: i_max_size,
            result,
        };
        // dbg!(text_size);
        text_size
    }

    /// sets the position of this div.
    ///
    /// Expects that sizes and child_sizes of all divs have already been computed.
    fn set_child_positions(&self, div: &Div) {
        match div.style.axis {
            Axis::X => _monomorphized_set_child_positions::<XMain>(self, div),
            Axis::Y => _monomorphized_set_child_positions::<YMain>(self, div),
        }

        pub trait AssembleDisassemble {
            /// returns (main_axis, cross_axis)
            fn disassemble(v: DVec2) -> (f64, f64);
            fn assemble(main: f64, cross: f64) -> DVec2;
        }

        struct XMain;
        struct YMain;

        impl AssembleDisassemble for XMain {
            #[inline(always)]
            fn disassemble(v: DVec2) -> (f64, f64) {
                // (main_axis, cross_axis)
                (v.x, v.y)
            }
            #[inline(always)]
            fn assemble(main: f64, cross: f64) -> DVec2 {
                DVec2 { x: main, y: cross }
            }
        }

        impl AssembleDisassemble for YMain {
            #[inline(always)]
            fn disassemble(v: DVec2) -> (f64, f64) {
                // (main_axis, cross_axis)
                (v.y, v.x)
            }
            #[inline(always)]
            fn assemble(main: f64, cross: f64) -> DVec2 {
                DVec2 { x: cross, y: main }
            }
        }

        /// Gets monomorphized into two functions: One for Y being the Main Axis and one for X being the Main Axis.
        #[inline(always)]
        fn _monomorphized_set_child_positions<A: AssembleDisassemble>(
            sel: &Layouter<'_>,
            div: &Div,
        ) {
            let n_children = match &div.content {
                DivContent::Text(_) => 1,
                DivContent::Children(children) => children.len(),
            };
            if n_children == 0 {
                return;
            }

            // get cached values from the previous layout step (sizing)
            let div_size = div.c_size.get();
            let div_pos = div.c_pos.get();
            let content_size = div.c_content_size.get();
            let div_padding = div.c_padding.get();

            // redefine div_size and div_pos to be the inner size of the div (div size - padding) and the
            // top left corner of the inner area instead of the top left corner of the div itself
            let div_size = dvec2(
                div_size.x - div_padding.width(),
                div_size.y - div_padding.height(),
            ); // div size - padding size on all sides
            let div_pos = div_pos + dvec2(div_padding.left, div_padding.top);

            let (main_size, cross_size) = A::disassemble(div_size);
            let (main_content_size, cross_content_size) = A::disassemble(content_size);
            let (mut main_offset, main_step) = main_offset_and_step(
                div.style.main_align,
                main_size,
                main_content_size,
                n_children,
            );

            let calc_cross_offset = match div.style.cross_align {
                Align::Start => |_: f64, _: f64| -> f64 { 0.0 },
                Align::Center => |cross_parent: f64, cross_item: f64| -> f64 {
                    (cross_parent - cross_item) * 0.5
                },
                Align::End => {
                    |cross_parent: f64, cross_item: f64| -> f64 { cross_parent - cross_item }
                }
            };

            match &div.content {
                DivContent::Text(t) => {
                    let cross = calc_cross_offset(cross_size, cross_content_size);
                    let text_pos = A::assemble(main_offset, cross);
                    let text_offset = offset_dvec2(t.text.offset_x, t.text.offset_y, div_size);
                    let absolute_text_pos = text_pos + text_offset + div_pos;
                    t.c_pos.set(absolute_text_pos);

                    // set the positions of the unbound divs saved in the spans:
                    let mut i: usize = 0;
                    for span in t.text.spans.iter() {
                        let Span::FixedSizeDiv { id, .. } = span else {
                            continue;
                        };
                        let div = sel.divs.get(&id._priv).unwrap();
                        let div_pos_relative_in_text =
                            t.c_text_layout.get().result.space_sections[i].as_dvec2();
                        div.c_pos.set(absolute_text_pos + div_pos_relative_in_text);
                        sel.set_child_positions(div);
                        i += 1;
                    }
                }
                DivContent::Children(children) => {
                    let children = children.iter().map(|e| sel.divs.get(e).unwrap());
                    for ch in children {
                        let (ch_main_size, ch_cross_size) = A::disassemble(ch.c_size.get());
                        let cross = calc_cross_offset(cross_size, ch_cross_size);

                        let ch_rel_pos: DVec2;
                        if ch.style.absolute {
                            // for absolute positioning just position the widget roughly like it was the only one.
                            let main_offset = main_offset_of_absolute_div(
                                div.style.main_align,
                                main_size,
                                ch_main_size,
                            );
                            ch_rel_pos = A::assemble(main_offset, cross);
                        } else {
                            ch_rel_pos = A::assemble(main_offset, cross);
                            main_offset += ch_main_size + main_step;
                        }

                        let ch_offset =
                            offset_dvec2(ch.style.offset_x, ch.style.offset_y, div_size);

                        ch.c_pos.set(ch_rel_pos + ch_offset + div_pos);
                        sel.set_child_positions(ch);
                    }
                }
            }

            // Question: maybe in future store offset of text or something, in case the parent pos is the same?
            // right now, we just store the glyphs as a layout result independent of the divs pos in here,
            // every frame we build up a glyoh buffer adding the position of the div to each glyph individually.
        }

        /// The main offset is the offset on the main axis at the start of layout.
        /// After each child with relative positioning it is incremented by the childs size, plus the step value.
        ///
        /// This function computes the initial main offset and this step value for different main axis alignment modes.
        #[inline]
        fn main_offset_and_step(
            main_align: MainAlign,
            main_size: f64,
            main_content_size: f64,
            n_children: usize,
        ) -> (f64, f64) {
            let offset: f64; // initial offset on main axis for the first child
            let step: f64; //  step that gets added for each child on main axis after its own size on main axis.
            match main_align {
                MainAlign::Start => {
                    offset = 0.0;
                    step = 0.0;
                }
                MainAlign::Center => {
                    offset = (main_size - main_content_size) * 0.5;
                    step = 0.0;
                }
                MainAlign::End => {
                    offset = main_size - main_content_size;
                    step = 0.0;
                }
                MainAlign::SpaceBetween => {
                    offset = 0.0;

                    if n_children == 1 {
                        step = 0.0;
                    } else {
                        step = (main_size - main_content_size) / (n_children - 1) as f64;
                    }
                }
                MainAlign::SpaceAround => {
                    step = (main_size - main_content_size) / n_children as f64;
                    offset = step / 2.0;
                }
            };
            (offset, step)
        }

        /// used to calculate the main axis offset of absolute divs.
        #[inline]
        fn main_offset_of_absolute_div(
            parent_main_align: MainAlign,
            parent_main_size: f64,
            self_main_size: f64,
        ) -> f64 {
            match parent_main_align {
                MainAlign::Start => 0.0,
                MainAlign::Center | MainAlign::SpaceBetween | MainAlign::SpaceAround => {
                    (parent_main_size - self_main_size) * 0.5
                }
                MainAlign::End => parent_main_size - self_main_size,
            }
        }
    }
}

#[derive(Debug)]
pub struct Div {
    pub(super) content: DivContent,
    pub(crate) style: DivStyle,
    // last_frame is reset every frame.
    last_frame: u64,
    // calculated as parent.z_index + 1, important for sorting in batching.
    pub(crate) z_index: Cell<i32>,
    // computed sizes and position
    pub(crate) c_size: Cell<DVec2>,
    pub(crate) c_content_size: Cell<DVec2>,
    pub(crate) c_pos: Cell<DVec2>,
    pub(crate) c_padding: Cell<ComputedPadding>,
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

#[derive(Debug)]
pub struct DivStyle {
    /// None means, the div has a non-fixed width, the children dictate the size of this div
    pub width: Option<Len>,
    /// None means, the div has a non-fixed height, the children dictate the size of this div
    pub height: Option<Len>,
    /// Determines how children are layed out: X = horizontally, Y = vertically.
    pub axis: Axis,
    pub main_align: MainAlign,
    pub cross_align: Align,
    pub padding: Padding,
    /// true = in CSS `position: absolute;`
    /// Since we do not have attributes top, right, left, bottom,
    /// the position is the same as if this was the single child of the div.
    /// Place it last if
    pub absolute: bool,
    // todo! translation, absolute, padding, margin
    pub color: Color,
    pub border_color: Color,
    pub border_radius: BorderRadius,
    pub border_thickness: f32,
    pub offset_x: Len,
    pub offset_y: Len,
    // set to 0.0 for very crisp inner border. set to 20.0 for like an inset shadow effect.
    pub border_softness: f32,
    pub texture: Option<DivTexture>,
}

impl Default for DivStyle {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            axis: Axis::Y,
            padding: Default::default(),
            main_align: MainAlign::Start,
            cross_align: Align::Start,
            absolute: false,
            color: Color::TRANSPARENT,
            border_radius: BorderRadius::default(),
            border_thickness: 0.0,
            border_softness: 0.0,
            border_color: Color::TRANSPARENT,
            offset_x: Len::ZERO,
            offset_y: Len::ZERO,
            texture: None,
        }
    }
}

impl DivStyle {
    #[inline]
    pub fn width(&mut self, width: Len) {
        self.width = Some(width);
    }

    #[inline]
    pub fn height(&mut self, height: Len) {
        self.height = Some(height);
    }
}

/// Padding always goes to the inside. Padding never affects a divs width or height, IF the width or height is fixed (not None).
/// Padding is added on top of size of children if the Div has a non-fixed size.
///
/// The parent fraction component of the Len value considers the size of the div itself and not the parent of the div
#[derive(Debug, Default)]
pub struct Padding {
    pub left: Len,
    pub right: Len,
    pub top: Len,
    pub bottom: Len,
}

impl Padding {
    pub fn new() -> Self {
        Padding {
            left: Len::ZERO,
            right: Len::ZERO,
            top: Len::ZERO,
            bottom: Len::ZERO,
        }
    }

    pub fn left(mut self, len: Len) -> Self {
        self.left = len;
        self
    }

    pub fn right(mut self, len: Len) -> Self {
        self.right = len;
        self
    }

    pub fn top(mut self, len: Len) -> Self {
        self.top = len;
        self
    }

    pub fn bottom(mut self, len: Len) -> Self {
        self.bottom = len;
        self
    }

    pub fn horizontal(mut self, len: Len) -> Self {
        self.left = len;
        self.right = len;
        self
    }

    pub fn vertical(mut self, len: Len) -> Self {
        self.top = len;
        self.bottom = len;
        self
    }

    pub fn all(len: Len) -> Self {
        Self {
            left: len,
            right: len,
            top: len,
            bottom: len,
        }
    }

    /// how big is the padding in total (left + right) and (top + bottom)
    ///
    /// cache the results in the ComputedPadding struct
    #[inline]
    fn compute_x(&self, div_width_px: f64, computed: &mut ComputedPadding) {
        computed.left = self.left.fixed(div_width_px);
        computed.right = self.right.fixed(div_width_px);
    }

    /// how big is the padding in total (left + right) and (top + bottom)
    ///
    /// cache the results in the ComputedPadding struct
    #[inline]
    fn compute_y(&self, div_height_px: f64, computed: &mut ComputedPadding) {
        computed.top = self.top.fixed(div_height_px);
        computed.bottom = self.bottom.fixed(div_height_px);
    }

    /// compute the padding in x direction by just knowing the contents width.
    ///
    /// calculates the parent_size in x direction as well and returns it.
    #[inline]
    fn compute_x_from_content(&self, content_width_px: f64, computed: &mut ComputedPadding) -> f64 {
        /*
        Idea: first compute the parent size:

        example:
        - if child is found to be 80px
        - padding on the left side is 5px + 0.05 parent fraction on each side.
        - padding on the right is 10px flat
        then the parent needs to be 100px tall. Because we then: 80px + 10px + 5px + 0.05 * 100px  = 100px;

        Formula:

        parent                                    = content + left px + right px + left fract * parent + right fract * parent.
        parent * (1.0 - left fract - right fract) = content + left px + right px
        parent                                    = (content + left px + right px) / (1.0 - left fract - right fract)

        */
        let parent_width_px = (content_width_px + self.left.px + self.right.px)
            / (1.0 - self.left.parent_fraction - self.right.parent_fraction); // make sure left and right do not add up to 0.
                                                                              // make sure left and right do not add up to 0.

        computed.left = self.left.px + self.left.parent_fraction * parent_width_px;
        computed.right = self.right.px + self.right.parent_fraction * parent_width_px;

        parent_width_px
    }

    /// compute the padding in x direction by just knowing the contents height.
    ///
    /// calculates the parent size in y direction as well and returns it.
    #[inline]
    fn compute_y_from_content(
        &self,
        content_height_px: f64,
        computed: &mut ComputedPadding,
    ) -> f64 {
        let parent_height_px = (content_height_px + self.top.px + self.bottom.px)
            / (1.0 - self.top.parent_fraction - self.bottom.parent_fraction); // make sure left and right do not add up to 0.
                                                                              // make sure left and right do not add up to 0.

        computed.top = self.top.px + self.top.parent_fraction * parent_height_px;
        computed.bottom = self.bottom.px + self.bottom.parent_fraction * parent_height_px;

        parent_height_px
    }

    // #[inline]
    // pub fn fixed_size_from_content(&self, content_size_px: &DVec2) -> DVec2 {
    //     DVec2 {
    //         x: self.fixed_width_from_content(content_size_px.x),
    //         y: self.fixed_height_from_content(content_size_px.y),
    //     }
    // }

    //     /// how big precisely in px is this padding, if we know how big the content is, that it wraps.
    //     #[inline]
    //     pub fn fixed_width_from_content(&self, content_width_px: f64) -> f64 {
    //        let len = self.left + self.right;

    //         //

    //         // example

    //         let px = len.px;

    //     }

    //     #[inline]
    //     pub fn fixed_height_from_content(&self, content_height_px: f64) -> f64 {
    //         let len = self.bottom + self.top;
    //         todo!()
    //     }
}

#[derive(Clone, Copy, Debug)]
pub struct ComputedPadding {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl ComputedPadding {
    #[inline]
    fn width(&self) -> f64 {
        self.left + self.right
    }

    #[inline]
    fn height(&self) -> f64 {
        self.top + self.bottom
    }

    const ZERO: ComputedPadding = ComputedPadding {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        bottom: 0.0,
    };
}

#[derive(Debug, Clone, Copy)]
pub struct DivTexture {
    pub texture: Ptr<BindableTexture>,
    pub uv: Aabb,
}

/// todo! make BorderRadius have not only f32 pixels but also PercentOfParent(f32).
#[repr(C)]
#[derive(Debug, Clone, bytemuck::Pod, bytemuck::Zeroable, Copy, Default)]
pub struct BorderRadius {
    top_left: f32,
    top_right: f32,
    bottom_right: f32,
    bottom_left: f32,
}

impl BorderRadius {
    pub const fn all(value: f32) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_right: value,
            bottom_left: value,
        }
    }

    pub const fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }
}

#[derive(Debug)]
pub(super) enum DivContent {
    Text(TextEntry),
    Children(Vec<Id>),
}

#[derive(Debug)]
pub(super) struct TextEntry {
    pub text: Text,
    pub c_pos: Cell<DVec2>,
    pub c_text_layout: ChillCell<CachedTextLayout>,
}

impl TextEntry {
    fn new(text: Text) -> Self {
        TextEntry {
            text,
            c_pos: Cell::new(DVec2::ZERO),
            c_text_layout: ChillCell::new(CachedTextLayout::zeroed()),
        }
    }
}

#[derive(Debug)]
pub struct Text {
    pub spans: smallvec::SmallVec<[Span; 1]>,
    /// None means the default font will be used insteads
    pub font: Option<Ptr<Font>>,
    // is this here maybe in the wrong place for offset? Maybe an extra div for this stuff would be better than putting it in the text itself!
    // on the other hand it is very useful to adjust the font baseline in a quick and dirty way.
    pub offset_x: Len,
    pub offset_y: Len,
}

#[derive(Debug)]
pub enum Span {
    Text(TextSection),
    FixedSizeDiv {
        id: UnboundId,
        width: f32,
        font_size: FontSize,
    },
}

impl Span {
    pub fn text_mut(&mut self) -> &mut TextSection {
        match self {
            Span::Text(t) => t,
            Span::FixedSizeDiv { .. } => panic!("cannot get text if it is a fixed size div!"),
        }
    }
}

#[derive(Debug)]
pub struct TextSection {
    pub color: Color,
    pub string: Cow<'static, str>,
    pub size: FontSize,
}

impl Text {
    pub fn new(string: impl Into<Cow<'static, str>>) -> Self {
        Self {
            spans: smallvec![Span::Text(TextSection {
                color: Color::BLACK,
                string: string.into(),
                size: FontSize(24)
            })],
            ..Default::default()
        }
    }

    pub fn font(mut self, font: Ptr<Font>) -> Self {
        self.font = Some(font);
        self
    }

    fn same_glyphs(&self, other: &Self) -> bool {
        let same = self.font == other.font && self.spans.len() == other.spans.len();
        if !same {
            return false;
        }

        for i in 0..self.spans.len() {
            let a = &self.spans[i];
            let b = &other.spans[i];

            let same = match (a, b) {
                (Span::Text(a), Span::Text(b)) => a.size == b.size && a.string == b.string,
                (
                    Span::FixedSizeDiv {
                        id,
                        width,
                        font_size,
                    },
                    Span::FixedSizeDiv {
                        id: id2,
                        width: width2,
                        font_size: font_size2,
                    },
                ) => id == id && width == width && font_size == font_size,
                _ => false,
            };
            if !same {
                return false;
            }
        }

        true
    }
}

impl Default for Text {
    fn default() -> Self {
        Self {
            spans: smallvec![Span::Text(TextSection {
                color: Color::RED,
                string: "Hello".into(),
                size: FontSize(24),
            })],
            font: None,
            offset_x: Len::ZERO,
            offset_y: Len::ZERO,
        }
    }
}

pub struct CachedTextLayout {
    /// Width and Height that the text can take at Max. Right now the assumption is that the text is always bounded by some way (e.g. the entire screen).
    /// These can be integers, so that minor float differences do not cause a new layout.
    pub max_size: IVec2,
    pub result: TextLayoutResult,
}

impl CachedTextLayout {
    pub fn zeroed() -> Self {
        CachedTextLayout {
            max_size: IVec2::ZERO,
            result: TextLayoutResult {
                layouted_glyphs: vec![],
                total_rect: Rect::ZERO,
                space_sections: smallvec![],
            },
        }
    }
}

impl Debug for CachedTextLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedTextLayout")
            .field("max_size", &self.max_size)
            .field("result", &self.result)
            .finish()
    }
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
pub enum Align {
    #[default]
    Start,
    Center,
    End,
}

/// An explicitly set Length
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Len {
    pub px: f64,
    /// Fraction of the size of the parent, subtracting the parents inner padding (this differs from e.g. CSS!).
    ///
    /// E.g. if the parent is 80x60 px big, but has 10px padding on each side and the child size is Len::parent(1.0),
    /// then the child will be 60x40 px big.
    pub parent_fraction: f64,
}

impl Default for Len {
    fn default() -> Self {
        Len::ZERO
    }
}

impl Len {
    pub const ZERO: Len = Len {
        px: 0.0,
        parent_fraction: 0.0,
    };
    pub const PARENT: Len = Len {
        px: 0.0,
        parent_fraction: 1.0,
    };

    pub const fn px(px: f64) -> Self {
        Len {
            px,
            parent_fraction: 0.0,
        }
    }

    pub const fn parent(parent_fraction: f64) -> Self {
        Len {
            px: 0.0,
            parent_fraction,
        }
    }

    fn fixed(&self, parent_size_px: f64) -> f64 {
        self.px + self.parent_fraction * parent_size_px
    }
}

impl Sub for Len {
    type Output = Len;

    fn sub(self, rhs: Self) -> Self::Output {
        Len {
            px: self.px - rhs.px,
            parent_fraction: self.parent_fraction - rhs.parent_fraction,
        }
    }
}

impl Add for Len {
    type Output = Len;

    fn add(self, rhs: Self) -> Self::Output {
        Len {
            px: self.px + rhs.px,
            parent_fraction: self.parent_fraction + rhs.parent_fraction,
        }
    }
}

impl Mul<f64> for Len {
    type Output = Len;

    fn mul(self, rhs: f64) -> Self::Output {
        Len {
            px: self.px * rhs,
            parent_fraction: self.parent_fraction * rhs,
        }
    }
}

/// Warning: assumes the content_size is set already on this div
pub(super) fn offset_dvec2(offset_x: Len, offset_y: Len, parent_size: DVec2) -> DVec2 {
    DVec2 {
        x: offset_x.fixed(parent_size.x),
        y: offset_y.fixed(parent_size.y),
    }
}

fn _unsued_egui_inspect_board(ctx: &mut egui::Context, board: &mut Board) {
    fn str_split(debug: &dyn Debug) -> String {
        let div_str = format!("{debug:?}");
        let mut div_str2 = String::new();

        for (i, c) in div_str.chars().enumerate() {
            div_str2.push(c);
            if (i + 1) % 100 == 0 {
                div_str2.push('\n');
            }
        }
        div_str2
    }

    egui::Window::new("Board").max_width(700.0).show(ctx, |ui| {
        // /////////////////////////////////////////////////////////////////////////////
        // Graphics Settings
        // /////////////////////////////////////////////////////////////////////////////
        ui.label(format!(
            "Top level children: {}",
            board.top_level_children.len()
        ));
        ui.label(format!("Top level size: {}", board.top_level_size));

        fn show_widget(ui: &mut Ui, board: &Board, div: &Div, level: usize) {
            ui.horizontal(|ui| {
                ui.add_space((level * 20) as f32);
                ui.label(str_split(div));
            });

            match &div.content {
                DivContent::Text(text) => {
                    ui.horizontal(|ui| {
                        ui.add_space(((level + 1) * 20) as f32);
                        ui.label(str_split(text));
                    });
                }
                DivContent::Children(children) => {
                    for div in children.iter().map(|e| board.divs.get(e).unwrap()) {
                        show_widget(ui, board, div, level + 1)
                    }
                }
            }
        }

        for top in board.top_level_children.iter() {
            let top_level_div = board.divs.get(top).unwrap();
            show_widget(ui, board, top_level_div, 0);
        }
    });
}
