use std::fs;

use super::ItemInfo;
use crate::{
    config::*,
    global::{functions::*, structs::*, traits::Collection},
};
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders},
};
use tui_additions::{
    framework::{FrameworkClean, FrameworkItem},
    widgets::{Grid, TextList},
};

use super::*;
