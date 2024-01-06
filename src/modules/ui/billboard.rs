use std::{
    alloc::Layout,
    borrow::Cow,
    cell::UnsafeCell,
    collections::{hash_map::Entry, HashMap},
    iter::Map,
};

use crate::{
    elements::Rect,
    modules::{arenas::Key, input::PressState, Input},
    prelude::{glam::Vec2, winit::event::MouseButton},
};
use morphorm::{Cache, LayoutType, Node, PositionType, Units};

use super::font_cache::{CachedFont, FontCache};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DivId(u64);

impl From<u64> for DivId {
    fn from(value: u64) -> Self {
        DivId(value)
    }
}

impl DivId {
    const TOP_LEVEL: DivId = DivId(u64::MAX);
}

/// A Billboard represents a screen, that contains UI-elements.
/// The Billboard could just represent the window screen directly, or be somewhere in the 3d space.
/// If a Billboard is in 3d space in the world, we just need to render it differently
/// and pass in the mouse pos via raycasting.
pub struct Billboard {
    last_frame: u32,
    input: BillboardInput,
    top_level_div: Div,
    div_store: UnsafeCell<DivStore>,
    text_layout_cache: HashMap<DivText, DivTextLayout>,
}

pub struct DivStore(HashMap<DivId, Div>);

impl Billboard {
    #[inline]
    fn div_store(&self) -> &mut DivStore {
        unsafe { &mut (*self.div_store.get()) }
    }
}

impl Billboard {
    pub fn set_input(&mut self, input: BillboardInput) {
        self.input = input
    }

    pub fn iter_divs(&self) -> impl Iterator<Item = &Div> {
        self.div_store().0.values()
    }

    pub fn new(width_px: f32, height_px: f32, layout_type: LayoutType) -> Self {
        let last_frame = 0;
        let top_level_div_appearance = Appearance {
            width: Some(Units::Pixels(width_px)),
            height: Some(Units::Pixels(height_px)),
            layout_type,
            ..Default::default()
        };
        let top_level_div = Div {
            id: DivId::TOP_LEVEL,
            text: None,

            children: vec![],
            parent: None,
            appearance: top_level_div_appearance,
            last_frame,
            rect: Some(Rect::new(0.0, 0.0, width_px, height_px)),
        };

        Billboard {
            last_frame,
            input: BillboardInput::default(),
            div_store: UnsafeCell::new(DivStore(HashMap::new())),
            top_level_div,
            text_layout_cache: HashMap::new(),
        }
    }

    pub fn add_div(
        &mut self,
        text: Option<DivText>,
        appearance: Appearance,
        id: DivId,
        parent: Option<DivId>,
    ) -> Response {
        // go into the parent:
        if let Some(parent) = parent {
            let parent = self
                .div_store()
                .0
                .get_mut(&parent)
                .expect("Invalid Parent...");
            // Note: Children should be added in every frame After their parent is added.
            parent.children.push(id);
        } else {
            self.top_level_div.children.push(id);
        }

        // insert child entry.
        let rect: Option<Rect>;
        match self.div_store().0.entry(id) {
            Entry::Occupied(mut e) => {
                let div = e.get_mut();
                div.parent = parent;
                div.children = vec![];
                div.appearance = appearance;
                div.text = text;
                // leave div.rect untouched...
                div.last_frame = self.last_frame;
                assert!(div.rect.is_some());
                rect = div.rect;
            }
            Entry::Vacant(vacant) => {
                vacant.insert(Div {
                    id,
                    text,
                    children: vec![],
                    parent,
                    appearance,
                    last_frame: self.last_frame,
                    rect: None,
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

        Response { id, comm }
    }

    pub fn end_frame(&mut self, font_cache: &FontCache) {
        // /////////////////////////////////////////////////////////////////////////////
        // Remove Nodes that have not been added/updated this frame
        // /////////////////////////////////////////////////////////////////////////////
        self.div_store
            .get_mut()
            .0
            .retain(|_, v| v.last_frame == self.last_frame);

        // /////////////////////////////////////////////////////////////////////////////
        // Perform Layout
        // /////////////////////////////////////////////////////////////////////////////

        // Note: This gets around the weird restrictions of the morphorm Node trait via an UnsafeCell: I want cache, tree and store to refer to the same thing.
        // I want to store all Div information in just one hashmap and not spread it out in multiple HashMaps.
        // Having it in multiple data structures would not be too bad if they were Arenas or Vectors, but with the random persistent keys that we use, that is not so easy to do.
        // Later we can implement our own layout system, I am not convinced that morphorm is super fast.

        let mut text_layout_cache = std::mem::take(&mut self.text_layout_cache);
        let cache = self.div_store();
        let tree = &*self.div_store();
        let store = tree;
        self.top_level_div
            .layout(cache, tree, store, &mut text_layout_cache);
        self.text_layout_cache = text_layout_cache;
    }
}

impl Cache for DivStore {
    type Node = Div;

    fn width(&self, node: &Self::Node) -> f32 {
        self.0.get(&node.id).unwrap().rect.unwrap().width
    }

    fn height(&self, node: &Self::Node) -> f32 {
        self.0.get(&node.id).unwrap().rect.unwrap().height
    }

    fn posx(&self, node: &Self::Node) -> f32 {
        self.0.get(&node.id).unwrap().rect.unwrap().min_x
    }

    fn posy(&self, node: &Self::Node) -> f32 {
        self.0.get(&node.id).unwrap().rect.unwrap().min_y
    }

    fn set_bounds(&mut self, node: &Self::Node, posx: f32, posy: f32, width: f32, height: f32) {
        let rect = Rect::new(posx, posy, width, height);
        self.0.get_mut(&node.id).unwrap().rect = Some(rect);
    }
}

#[derive(Debug, Clone, Default)]
pub struct BillboardInput {
    pub left_mouse_button: PressState,
    pub right_mouse_button: PressState,
    pub scroll: f32,
    pub cursor_pos: Option<Vec2>,
    pub cursor_delta: Vec2,
}

impl BillboardInput {
    pub fn from_input_module(input: &Input) -> Self {
        let left_mouse_button = input.mouse_buttons().press_state(MouseButton::Left);
        let right_mouse_button = input.mouse_buttons().press_state(MouseButton::Left);

        BillboardInput {
            left_mouse_button,
            right_mouse_button,
            scroll: input.scroll().unwrap_or(0.0),
            cursor_pos: Some(input.cursor_pos()),
            cursor_delta: input.cursor_delta(),
        }
    }
}

pub struct Response {
    id: DivId,
    comm: Option<Comm>,
}

pub struct Comm {
    rect: Rect,
    // Some, if the mouse is hovering, clicking or releasing?
    hovered: bool,
    clicked: bool,
}

pub struct Div {
    id: DivId,
    text: Option<DivText>,
    children: Vec<DivId>,
    parent: Option<DivId>,
    appearance: Appearance,
    // computed every frame
    last_frame: u32,
    rect: Option<Rect>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DivText {
    font: Key<CachedFont>,
    text: Cow<'static, str>,
}

pub struct DivTextLayout {}

#[derive(Default, Clone)]
pub struct Appearance {
    pub width: Option<Units>,
    pub height: Option<Units>,
    pub min_width: Option<Units>,
    pub max_width: Option<Units>,
    pub min_height: Option<Units>,
    pub max_height: Option<Units>,
    pub left: Option<Units>,
    pub right: Option<Units>,
    pub top: Option<Units>,
    pub bottom: Option<Units>,
    pub min_left: Option<Units>,
    pub max_left: Option<Units>,
    pub max_right: Option<Units>,
    pub min_right: Option<Units>,
    pub min_top: Option<Units>,
    pub max_top: Option<Units>,
    pub min_bottom: Option<Units>,
    pub max_bottom: Option<Units>,
    pub child_left: Option<Units>,
    pub child_right: Option<Units>,
    pub child_top: Option<Units>,
    pub child_bottom: Option<Units>,
    pub row_between: Option<Units>,
    pub col_between: Option<Units>,
    pub layout_type: LayoutType,
    pub position_type: PositionType,
}

pub struct DivChildIter<'a> {
    i: usize,
    len: usize,
    div: &'a Div,
    div_store: &'a DivStore,
}

impl<'a> DivChildIter<'a> {
    pub fn new(div: &'a Div, div_store: &'a DivStore) -> Self {
        Self {
            i: 0,
            len: div.children.len(),
            div,
            div_store,
        }
    }
}

impl<'a> Iterator for DivChildIter<'a> {
    type Item = &'a Div;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i == self.len {
            None
        } else {
            let i = self.i;
            self.i += 1;
            let child = self
                .div_store
                .0
                .get(&self.div.children[i])
                .expect("Node not found");
            Some(child)
        }
    }
}

impl Node for Div {
    type Store = DivStore;
    type Tree = DivStore;

    type ChildIter<'t> = DivChildIter<'t>;

    type CacheKey = ();

    type SubLayout<'a> = HashMap<DivText, DivTextLayout>;

    fn key(&self) -> Self::CacheKey {}

    fn children<'t>(&'t self, screen: &'t Self::Tree) -> Self::ChildIter<'t> {
        DivChildIter::new(self, screen)
    }

    fn visible(&self, store: &Self::Store) -> bool {
        true
    }

    fn layout_type(&self, store: &Self::Store) -> Option<LayoutType> {
        Some(self.appearance.layout_type)
    }

    fn position_type(&self, store: &Self::Store) -> Option<PositionType> {
        Some(self.appearance.position_type)
    }

    fn width(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.width
    }

    fn height(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.height
    }

    fn left(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.left
    }

    fn right(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.right
    }

    fn top(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.top
    }

    fn bottom(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.bottom
    }

    fn content_size(
        &self,
        store: &Self::Store,
        sublayout: &mut Self::SubLayout<'_>,
        parent_width: Option<f32>,
        parent_height: Option<f32>,
    ) -> Option<(f32, f32)> {
        if let Some(text) = &self.text {
            if let Some(e) = sublayout.get(text) {
                todo!() // needle
            }
            Some((200.0, 100.0))
        } else {
            None
        }
    }

    fn child_left(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.child_left
    }

    fn child_right(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.child_right
    }

    fn child_top(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.child_top
    }

    fn child_bottom(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.child_bottom
    }

    fn row_between(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.row_between
    }

    fn col_between(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.col_between
    }

    fn min_width(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.min_width
    }

    fn min_height(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.min_height
    }

    fn max_width(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.max_width
    }

    fn max_height(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.max_height
    }

    fn min_left(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.min_left
    }

    fn min_right(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.min_right
    }

    fn min_top(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.min_top
    }

    fn min_bottom(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.min_bottom
    }

    fn max_left(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.max_left
    }

    fn max_right(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.max_right
    }

    fn max_top(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.max_top
    }

    fn max_bottom(&self, store: &Self::Store) -> Option<Units> {
        self.appearance.max_bottom
    }

    fn border_left(&self, store: &Self::Store) -> Option<Units> {
        None
    }

    fn border_right(&self, store: &Self::Store) -> Option<Units> {
        None
    }

    fn border_top(&self, store: &Self::Store) -> Option<Units> {
        None
    }

    fn border_bottom(&self, store: &Self::Store) -> Option<Units> {
        None
    }
}

// pub fn main() {
//     let mut sm: SlotMap<DivId, Div> = SlotMap::<DivId, Div>::with_key();
//     let foo = sm.insert(Div {
//         id:  Some(self.appearance.layout_type),
//         children:  Some(self.appearance.layout_type),
//         div_layout:  Some(self.appearance.layout_type),
//     }); // Key generated on insert.
//     let bar = sm.insert("bar");
//     assert_eq!(sm[foo], "foo");
// }

/*

We take a screen as our context.


// begin frame:

Then on our code paths, we add Divs to the screen.

every time we add a div, we are given back Option<(Rect, Interaction)> which is the layout of this div
of the previous frame and the interaction result, if the widget existed in the frame before already.
We also set a frame counter on this div, meaning that we touched it.

we can add a div as a child of another div also.


// end frame


perform layout on the entire tree, caching the results.
we discard all the nodes that were not touched this frame.





*/
