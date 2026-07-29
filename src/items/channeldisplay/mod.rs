use super::ItemInfo;
use crate::{
    config::*,
    global::{
        functions::*,
        structs::*,
        traits::{Collection, SearchProviderWrapper},
    },
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


mod framework;

/// the 4 pages that a channel has (including the default "blank" page when loading)
#[derive(Clone, Default)]
pub enum ChannelDisplay {
    /// a blank item, will turn into one of the other variants when `.load()` depending on the page
    #[default]
    None,
    /// main channel display page
    Main {
        channel: Box<Item>,
        iteminfo: Box<ItemInfo>,
        grid: Grid,
        textlist: TextList,
        commands: Vec<(String, String)>,
    },
    /// latest videos
    Videos {
        videos: Vec<Item>,
        textlist: TextList,
        iteminfo: Box<ItemInfo>,
        grid: Grid,
    },
    /// created playlists
    Playlists {
        playlists: Vec<Item>,
        textlist: TextList,
        iteminfo: Box<ItemInfo>,
        grid: Grid,
    },
}
impl ChannelDisplay {
    pub fn new_textlist_with_map(commands: Vec<(String, String)>) -> TextList {
        TextList::default()
            .items(
                &commands
                    .iter()
                    .map(|command| &command.0)
                    .collect::<Vec<_>>(),
            )
            .unwrap()
    }

    fn inflate_load(&self, mainconfig: &MainConfig, status: &Status) -> Vec<(String, String)> {
        match self {
            Self::Main { channel, .. } => {
                vec![
                    (
                        String::from("url"),
                        match status.provider {
                            Provider::Invidious => format!(
                                "{}/channel/{}{}",
                                mainconfig.invidious_instance,
                                channel.id().unwrap_or_default(),
                                match self {
                                    Self::None | Self::Main { .. } => "",
                                    Self::Videos { .. } => "videos",
                                    Self::Playlists { .. } => "playlists",
                                }
                            ),
                            Provider::YouTube => {
                                format!("'https://youtu.be/{}'", channel.id().unwrap_or_default())
                            }
                        },
                    ),
                    (
                        String::from("id"),
                        channel.id().unwrap_or_default().to_string(),
                    ),
                    (
                        String::from("name"),
                        channel.fullchannel().unwrap().name.clone(),
                    ),
                ]
            }
            // TODO?
            _ => Vec::new(),
        }
    }

    fn infalte_item_update(
        &self,
        mainconfig: &MainConfig,
        status: &Status,
    ) -> Vec<(String, String)> {
        match self {
            ChannelDisplay::Videos {
                videos, textlist, ..
            } => {
                if textlist.items.is_empty() {
                    vec![(String::from("hover-url"), "no-videos".to_string())]
                } else {
                    vec![(
                        String::from("hover-url"),
                        format!(
                            "{}/watch?v={}",
                            match status.provider {
                                Provider::YouTube => "https://youtube.com",
                                Provider::Invidious => &mainconfig.invidious_instance,
                            },
                            videos[textlist.selected].id().unwrap_or_default()
                        ),
                    )]
                }
            }
            ChannelDisplay::Playlists {
                playlists,
                textlist,
                ..
            } => {
                if textlist.items.is_empty() {
                    vec![(String::from("hover-url"), "no-videos".to_string())]
                } else {
                    vec![(
                        String::from("hover-url"),
                        format!(
                            "{}/playlist?list={}",
                            match status.provider {
                                Provider::YouTube => "https://youtube.com",
                                Provider::Invidious => &mainconfig.invidious_instance,
                            },
                            playlists[textlist.selected].id().unwrap_or_default()
                        ),
                    )]
                }
            }
            _ => Vec::new(),
        }
    }
    /// update the style of the item (colours, etc), ran on ever render
    fn update_appearance(
        &mut self,
        info: &tui_additions::framework::ItemInfo,
        appearance: &AppearanceConfig,
    ) {
        // is runs on every render
        match self {
            ChannelDisplay::Main { textlist, grid, .. }
            | ChannelDisplay::Playlists { textlist, grid, .. }
            | ChannelDisplay::Videos { textlist, grid, .. } => {
                textlist.set_border_type(appearance.borders);
                textlist.set_style(Style::default().fg(appearance.colors.text));

                if info.selected {
                    textlist
                        .set_selected_style(Style::default().fg(appearance.colors.text_special));
                    textlist.set_cursor_style(Style::default().fg(appearance.colors.outline_hover));
                    grid.set_border_style(Style::default().fg(appearance.colors.outline_selected));
                } else {
                    if info.hover {
                        grid.set_border_style(Style::default().fg(appearance.colors.outline_hover));
                    } else {
                        grid.set_border_style(Style::default().fg(appearance.colors.outline));
                    }
                    textlist
                        .set_selected_style(Style::default().fg(appearance.colors.text_secondary));
                    textlist
                        .set_cursor_style(Style::default().fg(appearance.colors.outline_secondary));
                }
            }
            _ => {}
        }
    }

    /// handles when select (enter) is pressed, generally loads the hovered item in a
    /// `SingleItemPage`
    fn select_at_cursor(&self, framework: &mut FrameworkClean) {
        match self {
            Self::None => {}
            Self::Main {
                textlist, commands, ..
            } => {
                let command_string = commands[textlist.selected].1.clone();

                framework
                    .data
                    .state
                    .get_mut::<Tasks>()
                    .unwrap()
                    .priority
                    .push(Task::Command(apply_envs(command_string)));
            }
            Self::Videos {
                videos, textlist, ..
            } => {
                // on select loads that in singleitem
                if !videos.is_empty() {
                    framework
                        .data
                        .state
                        .get_mut::<Tasks>()
                        .unwrap()
                        .priority
                        .push(Task::LoadPage(Page::SingleItem(SingleItemPage::Video(
                            videos[textlist.selected].minivideo().unwrap().id.clone(),
                        ))));
                } else {
                    *framework.data.global.get_mut::<Message>().unwrap() =
                        Message::Error(String::from("There is nothing to select"));
                }
            }
            Self::Playlists {
                playlists,
                textlist,
                ..
            } => {
                if !playlists.is_empty() {
                    framework
                        .data
                        .state
                        .get_mut::<Tasks>()
                        .unwrap()
                        .priority
                        .push(Task::LoadPage(Page::SingleItem(SingleItemPage::Playlist(
                            playlists[textlist.selected]
                                .miniplaylist()
                                .unwrap()
                                .id
                                .clone(),
                        ))));
                } else {
                    *framework.data.global.get_mut::<Message>().unwrap() =
                        Message::Error(String::from("There is nothing to select"));
                }
            }
        }
    }

    /// updates the video/playlist preview
    fn update(&mut self) {
        match self {
            Self::Videos {
                videos: items,
                textlist,
                iteminfo,
                ..
            }
            | Self::Playlists {
                playlists: items,
                textlist,
                iteminfo,
                ..
            } => {
                if !items.is_empty()
                    && items[textlist.selected].id() != iteminfo.item.as_ref().unwrap().id()
                {
                    iteminfo.item = Some(items[textlist.selected].clone())
                }
            }
            _ => {}
        }
    }

    /// check if self should be able to be selected
    pub fn selectable(&self) -> bool {
        !matches!(self, Self::None)
    }
}
