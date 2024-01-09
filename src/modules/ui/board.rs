use std::{
    borrow::{Borrow, Cow},
    cell::{Cell, RefCell, UnsafeCell},
    collections::{
        hash_map::{Entry, OccupiedEntry},
        HashMap,
    },
    hash::{Hash, Hasher},
    iter::Map,
    marker::PhantomData,
    ops::{Add, Deref, DerefMut},
};

use crate::{
    elements::{rect::Aabb, Color, Rect},
    modules::{arenas::Key, input::PressState, Input},
    prelude::{glam::Vec2, winit::event::MouseButton},
    utils::ChillCell,
};
use egui::ahash::HashSet;
use etagere::euclid::default;
use fontdue::layout::{HorizontalAlign, Layout, VerticalAlign};
use glam::{dvec2, vec2, DVec2, IVec2};
use smallvec::{smallvec, SmallVec};

use super::{
    batching::{get_batches, BatchingResult},
    font_cache::{FontCache, RasterizedFont, TextLayoutResult},
    widgets::Widget,
};

/// A wrapper around a non-text div that can be used as a parent key when inserting a child div.
/// (text divs cannot have children).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContainerId {
    /// you cannot set this manually, to ensure only DivIds that belong to a Div with DivContent::Children.
    _priv: Id,
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

    pub fn add<'a, W: Widget>(
        &'a mut self,
        widget: W,
        id: Id,
        parent: Option<ContainerId>,
    ) -> W::Response<'a> {
        widget.add_to_board(self, id, parent)
    }

    pub fn add_non_text_div<'a>(
        &'a mut self,
        props: DivProps,
        style: DivStyle,
        id: Id,
        parent: Option<ContainerId>,
    ) -> ContainerResponse<'a> {
        let (comm, entry) = self._add_div(props, style, id, DivContent::Children(vec![]), parent);
        let div_id = ContainerId { _priv: id };
        ContainerResponse {
            id: div_id,
            comm,
            entry,
        }
    }

    pub fn add_text_div<'a>(
        &'a mut self,
        props: DivProps,
        style: DivStyle,
        text: Text,
        id: Id,
        parent: Option<ContainerId>,
    ) -> TextResponse<'a> {
        let (comm, entry): (Option<Comm>, OccupiedEntry<'_, Id, Div>) =
            self._add_div(props, style, id, DivContent::Text(text), parent);
        TextResponse { comm, entry }
    }

    fn _add_div<'a>(
        &'a mut self,
        props: DivProps,
        style: DivStyle,
        id: Id,
        content: DivContent,
        parent: Option<ContainerId>,
    ) -> (Option<Comm>, OccupiedEntry<'a, Id, Div>) {
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
        let entry: OccupiedEntry<'a, Id, Div>;
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
                entry = e;
            }
            Entry::Vacant(vacant) => {
                entry = vacant.insert_entry(Div {
                    id,
                    props,
                    z_index,
                    last_frame: self.last_frame,
                    style,
                    content,
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
        (comm, entry)
    }

    /// call to transition from  BoardPhase::AddDivs -> BoardPhase::LayoutDone
    pub fn end_frame(&mut self, fonts: &FontCache) {
        assert_eq!(self.phase, BoardPhase::AddDivs);
        self.phase = BoardPhase::Rendering;

        // Remove Nodes that have not been added/updated this frame
        self.divs.retain(|_, v| v.last_frame == self.last_frame);
        self.last_frame += 1;

        // Perform Layout (set sizes and positions for all divs in the tree)
        let layouter = Layouter::new(&self.divs, fonts);
        layouter.perform_layout(&self.top_level_children, self.top_level_size);
    }
}

pub struct ContainerResponse<'a> {
    /// to be used as a parent for another div
    pub id: ContainerId,
    comm: Option<Comm>,
    pub entry: OccupiedEntry<'a, Id, Div>,
}

impl<'a> Deref for ContainerResponse<'a> {
    type Target = DivStyle;

    fn deref(&self) -> &Self::Target {
        &self.entry.get().style
    }
}

impl<'a> DerefMut for ContainerResponse<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entry.get_mut().style
    }
}

impl<'a> ContainerResponse<'a> {
    pub fn is_hovered(&self) -> bool {
        if let Some(comm) = &self.comm {
            comm.hovered
        } else {
            false
        }
    }

    pub fn style_mut(&mut self) -> &mut DivStyle {
        &mut self.entry.get_mut().style
    }
}

pub struct TextResponse<'a> {
    comm: Option<Comm>,
    pub entry: OccupiedEntry<'a, Id, Div>,
}

impl<'a> TextResponse<'a> {
    pub fn is_hovered(&self) -> bool {
        if let Some(comm) = &self.comm {
            comm.hovered
        } else {
            false
        }
    }

    pub fn style_mut(&mut self) -> &mut DivStyle {
        &mut self.entry.get_mut().style
    }

    pub fn text_mut(&mut self) -> &mut Text {
        match &mut self.entry.get_mut().content {
            DivContent::Text(text) => text,
            DivContent::Children(_) => panic!("This should always be text on a text div"),
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
    pub rect: Rect,
    // Some, if the mouse is hovering, clicking or releasing?
    pub hovered: bool,
    pub clicked: bool,
}

#[derive(Debug)]
pub struct Div {
    id: Id,
    pub(crate) content: DivContent,
    pub(crate) props: DivProps,
    pub(crate) style: DivStyle,
    // last_frame is reset every frame.
    last_frame: u64,
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
    divs: &'a HashMap<Id, Div>,
    fonts: &'a FontCache,
}

impl<'a> Layouter<'a> {
    fn new(divs: &'a HashMap<Id, Div>, fonts: &'a FontCache) -> Self {
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
    fn perform_layout(&self, top_level_children: &[Id], top_level_size: DVec2) {
        // Note: Right now top level divs have no relationship to each other, they are all individually positioned on the screen.
        // That means: adding another top level div never changes the position of other top level divs.
        for id in top_level_children.iter() {
            let top_div = self.divs.get(id).unwrap();
            // set the size of each div in the tree:
            _ = self.get_and_set_size(top_div, top_level_size);
            top_div.c_pos.set(DVec2::ZERO);
            // set the position of each div in the tree:
            self.set_child_positions(top_div);
        }
    }

    /// sets the position of this div.
    ///
    /// Expects that sizes and child_sizes of all divs have already been computed.
    fn set_child_positions(&self, div: &Div) {
        match div.props.axis {
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

        #[inline(always)]
        fn _monomorphized_set_child_positions<A: AssembleDisassemble>(
            sel: &Layouter<'_>,
            div: &Div,
        ) {
            match &div.content {
                DivContent::Text(text) => {
                    // todo!() maybe in future store offset of text or something, in case the parent pos is the same?
                    // right now, we just store the glyphs as a layout result independent of the divs pos in here,
                    // every frame we build up a glyoh buffer adding the position of the div to each glyph individually.
                }
                DivContent::Children(children) => {
                    let n_children = children.len();
                    if n_children == 0 {
                        return;
                    }

                    let my_pos = div.c_pos.get();
                    let my_size = div.c_size.get();
                    let my_content_size = div.c_content_size.get();

                    let (main_pos, cross_pos) = A::disassemble(div.c_pos.get());
                    let (main_size, cross_size) = A::disassemble(div.c_size.get());
                    let (main_content_size, cross_content_size) =
                        A::disassemble(div.c_content_size.get());

                    let main_axis_offset: f64; // initial offset on main axis for the first child
                    let main_axis_step: f64; // step that gets added for each child on main axis after its own size on main axis.

                    match div.props.main_align {
                        MainAlign::Start => {
                            main_axis_offset = 0.0;
                            main_axis_step = 0.0;
                        }
                        MainAlign::Center => {
                            main_axis_offset = (main_size - main_content_size) * 0.5;
                            main_axis_step = 0.0;
                        }
                        MainAlign::End => {
                            main_axis_offset = main_size - main_content_size;
                            main_axis_step = 0.0;
                        }
                        MainAlign::SpaceBetween => {
                            main_axis_offset = 0.0;

                            if n_children == 1 {
                                main_axis_step = 0.0;
                            } else {
                                main_axis_step =
                                    (main_size - main_content_size) / (children.len() - 1) as f64;
                            }
                        }
                        MainAlign::SpaceAround => {
                            main_axis_step =
                                (main_size - main_content_size) / children.len() as f64;
                            main_axis_offset = main_axis_step / 2.0;
                        }
                    }

                    let calc_cross_offset = match div.props.cross_align {
                        CrossAlign::Start => |cross_parent: f64, cross_item: f64| -> f64 { 0.0 },
                        CrossAlign::Center => |cross_parent: f64, cross_item: f64| -> f64 {
                            (cross_parent - cross_item) * 0.5
                        },
                        CrossAlign::End => |cross_parent: f64, cross_item: f64| -> f64 {
                            cross_parent - cross_item
                        },
                    };

                    let children = children.iter().map(|e| sel.divs.get(e).unwrap());

                    let mut main_v = main_axis_offset;
                    for ch in children {
                        let (ch_main_size, ch_cross_size) = A::disassemble(ch.c_size.get());
                        let cross = calc_cross_offset(cross_size, ch_cross_size);
                        let ch_pos = A::assemble(main_v, cross); // 0.0 if we always assume crossaxis align start.
                        ch.c_pos.set(ch_pos);
                        main_v += ch_main_size + main_axis_step;
                    }
                }
            }
        }
    }

    /// Gets monomorphized into two functions: One for Y being the Main Axis and one for X being the Main Axis.

    /// Calculates and sets the sizes of the given div and all of its children recursively.
    ///
    /// This follows 3 simple steps:
    /// 1. find out if width or height are contrained to a fixed size, or if they should hug the content.
    /// 2. figure out own size and content size
    /// 3. sache own size and content size in the div, then return own size.
    fn get_and_set_size(&self, div: &Div, parent_max_size: DVec2) -> DVec2 {
        let fixed_w = div.props.width.px_value(parent_max_size.x);
        let fixed_h = div.props.height.px_value(parent_max_size.y);

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
        let cached = c_text_layout.get_mut();
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
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            line_height: 1.0,
            wrap_style: fontdue::layout::WrapStyle::Word,
            wrap_hard_breaks: true, // todo!() needle expose these functions
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
    children_ids: &'a [Id],
    divs: &'a HashMap<Id, Div>,
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
    pub border_color: Color,
    pub border_radius: BorderRadius,
    pub border_thickness: f32,
    // set to 0.0 for very crisp inner border. set to 20.0 for like an inset shadow effect.
    pub border_softness: f32,
    // todo: margin and padding
    /// Note: z_bias is multiplied with 1024 when determining the final z_index and should be a rather small number.
    pub z_bias: i32,
}

impl Default for DivStyle {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            border_radius: BorderRadius::default(),
            z_bias: 0,
            border_thickness: 0.0,
            border_softness: 1.0,
            border_color: Color::BLACK,
        }
    }
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
pub enum DivContent {
    Text(Text),
    Children(Vec<Id>),
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
pub struct DivProps {
    // Determines width of Self
    pub width: Len,
    /// Determines height of Self
    pub height: Len,
    /// Determines how children are layed out.
    pub axis: Axis,
    pub main_align: MainAlign,
    pub cross_align: CrossAlign,
    // todo! translation, absolute, padding, margin
}

impl Default for DivProps {
    fn default() -> Self {
        Self {
            width: Len::HugContent,
            height: Len::HugContent,
            axis: Axis::Y,
            main_align: MainAlign::Start,
            cross_align: CrossAlign::Start,
        }
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
pub enum CrossAlign {
    #[default]
    Start,
    Center,
    End,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Len {
    Px(f64),
    FractionOfParent(f64),
    #[default]
    HugContent,
}

impl Len {
    pub fn px_value(&self, parent_px_size: f64) -> Option<f64> {
        match self {
            Len::Px(x) => Some(*x),
            Len::FractionOfParent(fr) => Some(*fr * parent_px_size),
            Len::HugContent => None,
        }
    }
}
