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

mod video;
pub use video::*;
mod playlist;
pub use playlist::*;
mod framework;

impl Default for SingleItem {
    fn default() -> Self {
        Self {
            item: None,
            iteminfo: ItemInfo::default(),
            grid: Grid::new(
                vec![Constraint::Percentage(30), Constraint::Percentage(70)],
                vec![Constraint::Percentage(100)],
            )
            .unwrap(),
            r#type: SingleItemType::None,
        }
    }
}
/// main item in the `SingleItem(_)` page
#[derive(Clone)]
pub struct SingleItem {
    pub item: Option<Item>,
    pub iteminfo: ItemInfo,
    pub grid: Grid,
    pub r#type: SingleItemType,
}
/// variations that the struct can hold
#[derive(Clone)]
pub enum SingleItemType {
    None,
    Video(SingleVideoItem),
    Playlist(Box<SinglePlaylistItem>),
}
impl SingleItemType {
    /// self is none
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// update colours every render
    pub fn update_appearance(
        &mut self,
        appearance: &AppearanceConfig,
        iteminfo: &tui_additions::framework::ItemInfo,
        grid: &mut Grid,
    ) {
        match self {
            Self::None => {}
            Self::Playlist(playlistitem) => {
                playlistitem.update_appearance(appearance, iteminfo, grid)
            }
            Self::Video(videoitem) => videoitem.update_appearance(appearance, iteminfo),
        }
    }

    pub fn inflate_load(
        &self,
        mainconfig: &MainConfig,
        status: &Status,
        item: &Option<Item>,
    ) -> Vec<(String, String)> {
        let item = if let Some(item) = item.as_ref() {
            item
        } else {
            return Vec::new();
        };

        match self {
            Self::Video(singlevideoitem) => singlevideoitem.inflate_load(mainconfig, status, item),
            Self::Playlist(singleplaylistitem) => singleplaylistitem.inflate_load(item, mainconfig),
            Self::None => Vec::new(),
        }
    }
}
