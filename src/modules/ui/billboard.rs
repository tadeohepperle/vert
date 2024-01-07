use std::{
    borrow::{Borrow, Cow},
    cell::{Cell, RefCell, UnsafeCell},
    collections::{hash_map::Entry, HashMap},
    iter::Map,
};

use crate::{
    elements::{Color, Rect},
    modules::{arenas::Key, input::PressState, Input},
    prelude::{glam::Vec2, winit::event::MouseButton},
};
use fontdue::layout::Layout;
use morphorm::{Cache, LayoutType, Node, PositionType, Units};
use smallvec::{smallvec, SmallVec};

use super::{
    batching::{get_batches, BatchingResult},
    font_cache::{FontCache, LayoutTextResult, RasterizedFont},
};

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
    phase: BillboardPhase,
    input: BillboardInput,
    top_level_div: Div,
    divs: HashMap<DivId, Div>,
    text_cache: HashMap<DivText, LayoutTextResult>,
}

/// The Billboard alternates between two phases:
/// - in AddDivs you can add elements to the billboard.
/// - in Render you cannot change the billboard, but you can extract batches to render from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillboardPhase {
    /// In this phase elements can be added
    AddDivs,
    /// Static, extract batches from the divs and their texts in this phase.
    Rendering,
}

impl Billboard {
    #[inline]
    pub fn text_layout_cache(&self) -> &HashMap<DivText, LayoutTextResult> {
        &self.text_cache
    }

    #[inline]
    pub fn phase(&self) -> BillboardPhase {
        self.phase
    }
}

impl Billboard {
    /// call to transition from  BillboardPhase::Rendering -> BillboardPhase::AddDivs.
    pub fn start_frame(&mut self, input: BillboardInput) {
        assert_eq!(self.phase, BillboardPhase::Rendering);
        self.input = input;
        self.phase = BillboardPhase::AddDivs;
    }

    pub fn iter_divs(&self) -> impl Iterator<Item = &Div> {
        self.divs.values()
    }

    pub fn new(width_px: f32, height_px: f32, layout_type: LayoutType) -> Self {
        println!("new Billboard created");
        let last_frame = 0;
        let top_level_div_props = DivProps {
            width: Some(Units::Pixels(width_px)),
            height: Some(Units::Pixels(height_px)),
            max_width: Some(Units::Pixels(width_px)),
            max_height: Some(Units::Pixels(height_px)),
            layout_type,
            ..Default::default()
        };
        let top_level_div = Div {
            id: DivId::TOP_LEVEL,
            text: None,
            z_index: 0,
            children: vec![],
            parent: None,
            props: top_level_div_props,
            last_frame,
            rect: Cell::new(Some(Rect {
                min_x: 0.0,
                min_y: 0.0,
                width: width_px as f32,
                height: height_px as f32,
            })),
        };

        Billboard {
            last_frame,
            input: BillboardInput::default(),
            divs: HashMap::new(),
            top_level_div,
            text_cache: HashMap::new(),
            phase: BillboardPhase::Rendering,
        }
    }

    /// Note: z_bias is multiplied with 1024 and should be a rather small number.
    pub fn add_div(
        &mut self,
        text: Option<DivText>,
        props: DivProps,
        id: DivId,
        parent: Option<DivId>,
        z_bias: i32,
    ) -> Response {
        // go into the parent:
        let parent_z_index = if let Some(parent) = parent {
            let parent = self.divs.get_mut(&parent).expect("Invalid Parent...");
            // Note: Children should be added in every frame After their parent is added.
            parent.children.push(id);
            parent.z_index
        } else {
            self.top_level_div.children.push(id);
            self.top_level_div.z_index
        };

        // insert child entry. z_index is always 1 more than parent to render on top.
        let z_index = parent_z_index + 1 + z_bias * 1024;
        let rect: Option<Rect>;
        match self.divs.entry(id) {
            Entry::Occupied(mut e) => {
                let div = e.get_mut();
                div.parent = parent;
                div.children = vec![];
                div.props = props;
                div.text = text;
                div.z_index = z_index;
                div.rect = Cell::new(None);
                // // leave div.rect untouched...
                // div.last_frame = self.last_frame;
                // let rect_o = div.rect.get();
                // assert!(rect_o.is_some());
                // rect = rect_o;
                rect = None;
            }
            Entry::Vacant(vacant) => {
                vacant.insert(Div {
                    id,
                    z_index,
                    text,
                    children: vec![],
                    parent,
                    props,
                    last_frame: self.last_frame,
                    rect: Cell::new(None),
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

    /// call to transition from  BillboardPhase::AddDivs -> BillboardPhase::LayoutDone
    pub fn end_frame(&mut self, font_cache: &FontCache) {
        assert_eq!(self.phase, BillboardPhase::AddDivs);
        self.phase = BillboardPhase::Rendering;
        // /////////////////////////////////////////////////////////////////////////////
        // Remove Nodes that have not been added/updated this frame
        // /////////////////////////////////////////////////////////////////////////////
        self.divs.retain(|_, v| v.last_frame == self.last_frame);

        // /////////////////////////////////////////////////////////////////////////////
        // Perform Layout
        // /////////////////////////////////////////////////////////////////////////////

        // Do text layout on all divs that have text in them:
        for div in self.divs.values() {
            if let Some(text) = &div.text {
                if self.text_cache.get(text).is_none() {
                    println!("performed text layout for {text:?}");
                    let layout_settings = text.layout_settings(div);
                    let result = font_cache.perform_text_layout(
                        &text.string,
                        None,
                        &layout_settings,
                        text.font,
                    );
                    self.text_cache.insert(text.clone(), result);
                }
            }
        }

        // Note: This gets around the weird restrictions of the morphorm Node trait via an UnsafeCell: I want cache, tree and store to refer to the same thing.
        // I want to store all Div information in just one hashmap and not spread it out in multiple HashMaps.
        // Having it in multiple data structures would not be too bad if they were Arenas or Vectors, but with the random persistent keys that we use, that is not so easy to do.
        // Later we can implement our own layout system, I am not convinced that morphorm is super fast.

        let mut text_cache = std::mem::take(&mut self.text_cache);
        self.top_level_div.layout(
            &mut NotNeededBecauseNodesStoreRects,
            &self.divs,
            &(),
            &mut text_cache,
        );

        dbg!(&self.divs);
        // child
        // dbg!(self.top_level_div.rect.get());
        self.text_cache = text_cache;
    }
}

struct NotNeededBecauseNodesStoreRects;
impl Cache for NotNeededBecauseNodesStoreRects {
    type Node = Div;

    fn width(&self, node: &Self::Node) -> f32 {
        node.rect.get().unwrap().width
    }

    fn height(&self, node: &Self::Node) -> f32 {
        node.rect.get().unwrap().height
    }

    fn posx(&self, node: &Self::Node) -> f32 {
        node.rect.get().unwrap().min_x
    }

    fn posy(&self, node: &Self::Node) -> f32 {
        node.rect.get().unwrap().min_y
    }

    fn set_bounds(&mut self, node: &Self::Node, posx: f32, posy: f32, width: f32, height: f32) {
        let rect = Rect::new(posx, posy, width, height);
        node.rect.set(Some(rect));
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

#[derive(Debug)]
pub struct Div {
    pub id: DivId,
    pub text: Option<DivText>,
    pub children: Vec<DivId>,
    pub parent: Option<DivId>,
    pub props: DivProps,
    pub z_index: i32,
    // computed every frame
    last_frame: u32,
    pub rect: Cell<Option<Rect>>,
}

/// Serves as key to a cache that saves layout results.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DivText {
    pub font: Key<RasterizedFont>,
    pub string: Cow<'static, str>,
}

impl DivText {
    pub fn layout_settings(&self, div: &Div) -> fontdue::layout::LayoutSettings {
        // todo!()   // needle

        fontdue::layout::LayoutSettings {
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct DivProps {
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
    pub color: Color,
    pub text_color: Color,
}

pub struct DivChildIter<'a> {
    i: usize,
    len: usize,
    div: &'a Div,
    div_store: &'a HashMap<DivId, Div>,
}

impl<'a> DivChildIter<'a> {
    pub fn new(div: &'a Div, div_store: &'a HashMap<DivId, Div>) -> Self {
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
                .get(&self.div.children[i])
                .expect("Node not found");
            Some(child)
        }
    }
}

impl Node for Div {
    type Store = ();
    type Tree = HashMap<DivId, Div>;
    type ChildIter<'t> = DivChildIter<'t>;
    type CacheKey = ();
    type SubLayout<'a> = HashMap<DivText, LayoutTextResult>;

    fn key(&self) -> Self::CacheKey {}

    fn children<'t>(&'t self, screen: &'t Self::Tree) -> Self::ChildIter<'t> {
        DivChildIter::new(self, screen)
    }

    fn visible(&self, store: &Self::Store) -> bool {
        true
    }

    fn layout_type(&self, store: &Self::Store) -> Option<LayoutType> {
        Some(self.props.layout_type)
    }

    fn position_type(&self, store: &Self::Store) -> Option<PositionType> {
        Some(self.props.position_type)
    }

    fn width(&self, store: &Self::Store) -> Option<Units> {
        self.props.width
    }

    fn height(&self, store: &Self::Store) -> Option<Units> {
        self.props.height
    }

    fn left(&self, store: &Self::Store) -> Option<Units> {
        self.props.left
    }

    fn right(&self, store: &Self::Store) -> Option<Units> {
        self.props.right
    }

    fn top(&self, store: &Self::Store) -> Option<Units> {
        self.props.top
    }

    fn bottom(&self, store: &Self::Store) -> Option<Units> {
        self.props.bottom
    }

    /// assumes that the text layout has been computed before.
    fn content_size(
        &self,
        store: &Self::Store,
        sublayout: &mut Self::SubLayout<'_>,
        parent_width: Option<f32>,
        parent_height: Option<f32>,
    ) -> Option<(f32, f32)> {
        println!("get content size for {:?}", self.id);
        if let Some(text) = &self.text {
            let text_rect = sublayout.get(text).unwrap().total_rect;
            Some((text_rect.width, text_rect.height))
        } else {
            None
        }
    }

    fn child_left(&self, store: &Self::Store) -> Option<Units> {
        self.props.child_left
    }

    fn child_right(&self, store: &Self::Store) -> Option<Units> {
        self.props.child_right
    }

    fn child_top(&self, store: &Self::Store) -> Option<Units> {
        self.props.child_top
    }

    fn child_bottom(&self, store: &Self::Store) -> Option<Units> {
        self.props.child_bottom
    }

    fn row_between(&self, store: &Self::Store) -> Option<Units> {
        self.props.row_between
    }

    fn col_between(&self, store: &Self::Store) -> Option<Units> {
        self.props.col_between
    }

    fn min_width(&self, store: &Self::Store) -> Option<Units> {
        self.props.min_width
    }

    fn min_height(&self, store: &Self::Store) -> Option<Units> {
        self.props.min_height
    }

    fn max_width(&self, store: &Self::Store) -> Option<Units> {
        self.props.max_width
    }

    fn max_height(&self, store: &Self::Store) -> Option<Units> {
        self.props.max_height
    }

    fn min_left(&self, store: &Self::Store) -> Option<Units> {
        self.props.min_left
    }

    fn min_right(&self, store: &Self::Store) -> Option<Units> {
        self.props.min_right
    }

    fn min_top(&self, store: &Self::Store) -> Option<Units> {
        self.props.min_top
    }

    fn min_bottom(&self, store: &Self::Store) -> Option<Units> {
        self.props.min_bottom
    }

    fn max_left(&self, store: &Self::Store) -> Option<Units> {
        self.props.max_left
    }

    fn max_right(&self, store: &Self::Store) -> Option<Units> {
        self.props.max_right
    }

    fn max_top(&self, store: &Self::Store) -> Option<Units> {
        self.props.max_top
    }

    fn max_bottom(&self, store: &Self::Store) -> Option<Units> {
        self.props.max_bottom
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

// pub enum Primitive(&)

// pub struct BillboardTreeIter<'a> {
//     billboard: &'a Billboard,
//     // the divs and the current child indexes
//     active_divs: SmallVec<[(&'a Div, usize); 10]>,
// }

// impl<'a> BillboardTreeIter<'a> {
//     pub fn new(billboard: &'a Billboard) -> Self {
//         Self {
//             billboard,
//             active_divs: smallvec![(&billboard.top_level_div, usize::MAX)],
//         }
//     }
// }

// impl<'a> Iterator for BillboardTreeIter<'a> {
//     type Item = &'a Div;

//     fn next(&mut self) -> Option<Self::Item> {
//         let (div, child_i) = self.active_divs.last_mut().unwrap();
//         let i = *child_i;
//         *child_i += 1;

//         if i == usize::MAX {
//             return Some(*div);
//         } else if i < div.children.len() {
//             let child_id = &div.children[i];
//             self.active_divs.
//             self.active_divs.push((child_id, usize::MAX));
//         }

//         let active = self.active_divs.last().unwrap();

//         todo!()
//     }
// }

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
