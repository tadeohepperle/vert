use std::{borrow::Cow, path::PathBuf};

const VERTEX_ENTRY_POINT: &str = "vs_main";
const FRAGMENT_ENTRY_POINT: &str = "fs_main";

use crate::constants::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT};
use indoc::indoc;
use smallvec::{smallvec, SmallVec};
use wgpu::{BindGroupLayout, PrimitiveState};

use super::{graphics_context::GraphicsContext, settings::GraphicsSettings};

pub mod renderer;
pub mod vertex;
