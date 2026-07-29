use std::error::Error;

use crate::items::filter::Filter;
use crate::{
    config::*,
    global::{functions::*, structs::*, traits::SearchProviderWrapper},
    items::ItemInfo,
};
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
};
use tui_additions::{
    framework::{FrameworkClean, FrameworkItem},
    widgets::{Grid, TextList},
};


mod framework;

/// An item list displays a list of items
// It consists of a 1 x 2 grid, with the left cell displaying a text list, the right displaying item info of the currently hovered item
#[derive(Clone)]
pub struct ItemList {
    pub info: ItemInfo,
    pub items: Vec<Item>,
    pub textlist: TextList,
    pub grid: Grid,
    pub exhausted: bool,
}
impl ItemList {
    /// Build display strings for the text list, prepending a bookmark marker
    /// to items that are already in the user's Library.
    fn build_display_strings(&self, framework: &FrameworkClean) -> Vec<String> {
        let library = framework.data.global.get::<Library>();
        let bookmarked_ids: std::collections::HashSet<&str> = library
            .map(|lib| lib.0.iter().filter_map(|item| item.id()).collect())
            .unwrap_or_default();

        self.items
            .iter()
            .map(|item| {
                let title = match item {
                    Item::MiniVideo(video) => &video.title,
                    Item::MiniPlaylist(playlist) => &playlist.title,
                    Item::MiniChannel(channel) => &channel.name,
                    Item::FullVideo(video) => &video.title,
                    Item::FullPlaylist(playlist) => &playlist.title,
                    Item::FullChannel(channel) => &channel.name,
                    Item::Page(b) => {
                        return if *b { "Next page" } else { "Previous page" }.to_string()
                    }
                };
                if let Some(id) = item.id() {
                    if bookmarked_ids.contains(id) {
                        return format!("\u{2605} {}", title);
                    }
                }
                title.to_string()
            })
            .collect()
    }

    /// Rebuild the text list with bookmark-aware display strings.
    pub fn refresh_textlist(&mut self, framework: &FrameworkClean) {
        let display_strings = self.build_display_strings(framework);
        self.textlist.items = display_strings;
        if self.textlist.height.is_some() {
            let _ = self.textlist.update();
        }
    }
}
impl ItemList {
    pub fn infalte_item_update(
        &self,
        mainconfig: &MainConfig,
        status: &Status,
    ) -> Vec<(String, String)> {
        if self.textlist.items.is_empty() {
            return Vec::new();
        }

        match &self.items[self.textlist.selected] {
            Item::MiniVideo(MiniVideoItem {
                id,
                title,
                channel_id,
                channel,
                ..
            })
            | Item::FullVideo(FullVideoItem {
                id,
                title,
                channel_id,
                channel,
                ..
            }) => {
                vec![
                    (
                        String::from("hover-url"),
                        format!(
                            "{}/watch?v={id}",
                            match status.provider {
                                Provider::YouTube => "https://youtube.com",
                                Provider::Invidious => &mainconfig.invidious_instance,
                            }
                        ),
                    ),
                    (String::from("hover-title"), title.clone()),
                    (String::from("hover-id"), id.clone()),
                    (String::from("hover-type"), String::from("video")),
                    (String::from("hover-channel-id"), channel_id.clone()),
                    (String::from("hover-channel"), channel.clone()),
                ]
            }
            Item::MiniPlaylist(MiniPlaylistItem {
                id,
                title,
                channel_id,
                channel,
                ..
            })
            | Item::FullPlaylist(FullPlaylistItem {
                id,
                title,
                channel_id,
                channel,
                ..
            }) => {
                vec![
                    (
                        String::from("hover-url"),
                        format!(
                            "{}/playlist?list={id}",
                            match status.provider {
                                Provider::YouTube => "https://youtube.com",
                                Provider::Invidious => &mainconfig.invidious_instance,
                            }
                        ),
                    ),
                    (String::from("hover-title"), title.clone()),
                    (String::from("hover-id"), id.clone()),
                    (String::from("hover-type"), String::from("playlist")),
                    (String::from("hover-channel-id"), channel_id.clone()),
                    (String::from("hover-channel"), channel.clone()),
                ]
            }
            Item::MiniChannel(MiniChannelItem { id, name, .. })
            | Item::FullChannel(FullChannelItem { id, name, .. }) => {
                vec![
                    (
                        String::from("hover-url"),
                        format!(
                            "{}/channel/{id}",
                            match status.provider {
                                Provider::YouTube => "https://youtube.com",
                                Provider::Invidious => &mainconfig.invidious_instance,
                            }
                        ),
                    ),
                    (String::from("hover-title"), name.clone()),
                    (String::from("hover-id"), id.clone()),
                    (String::from("hover-channel-id"), id.clone()),
                    (String::from("hover-channel"), name.clone()),
                    (String::from("hover-type"), String::from("channel")),
                ]
            }
            Item::Page(_) => {
                vec![(String::from("hover-url"), String::from("not avaliable"))]
            }
        }
    }

    fn update_appearance(
        &mut self,
        appearance: &AppearanceConfig,
        mainconfig: &MainConfig,
        iteminfo: &tui_additions::framework::ItemInfo,
    ) {
        self.textlist.set_ascii_only(!mainconfig.allow_unicode);
        self.grid.set_border_type(appearance.borders);
        self.textlist.set_border_type(appearance.borders);
        self.textlist
            .set_style(Style::default().fg(appearance.colors.text));

        if iteminfo.selected {
            self.grid
                .set_border_style(Style::default().fg(appearance.colors.outline_selected));
            self.textlist
                .set_cursor_style(Style::default().fg(appearance.colors.outline_hover));
            self.textlist
                .set_selected_style(Style::default().fg(appearance.colors.text_special));
        } else {
            self.textlist
                .set_cursor_style(Style::default().fg(appearance.colors.outline_secondary));
            self.textlist
                .set_selected_style(Style::default().fg(appearance.colors.text_secondary));
            if iteminfo.hover {
                self.grid
                    .set_border_style(Style::default().fg(appearance.colors.outline_hover));
            } else {
                self.grid
                    .set_border_style(Style::default().fg(appearance.colors.outline));
            }
        }
    }

    /// handles select (enter)
    fn select_at_cursor(&self, framework: &mut FrameworkClean) {
        if self.items.is_empty() {
            return;
        }

        let page_to_load =
            if LocalStore::get_info(self.items[self.textlist.selected].id().unwrap_or_default())
                .is_some()
            {
                match &self.items[self.textlist.selected] {
                    Item::MiniVideo(MiniVideoItem { id, .. })
                    | Item::FullVideo(FullVideoItem { id, .. }) => {
                        Some(Page::SingleItem(SingleItemPage::Video(id.clone())))
                    }
                    Item::MiniPlaylist(MiniPlaylistItem { id, .. })
                    | Item::FullPlaylist(FullPlaylistItem { id, .. }) => {
                        Some(Page::SingleItem(SingleItemPage::Playlist(id.clone())))
                    }
                    Item::MiniChannel(MiniChannelItem { id, .. })
                    | Item::FullChannel(FullChannelItem { id, .. }) => {
                        Some(Page::ChannelDisplay(ChannelDisplayPage {
                            id: id.to_string(),
                            r#type: ChannelDisplayPageType::Main,
                        }))
                    }
                    // Item::Unknown(_) => {
                    //     *framework.data.global.get_mut::<Message>().unwrap() =
                    //         Message::Message(String::from("Unknown item"));
                    //     framework
                    //         .data
                    //         .state
                    //         .get_mut::<Tasks>()
                    //         .unwrap()
                    //         .priority
                    //         .push(Task::RenderAll);
                    //     None
                    // }
                    Item::Page(b) => match framework.data.state.get::<Page>().unwrap() {
                        Page::Search(search) => Some(Page::Search(Search {
                            page: if *b { search.page + 1 } else { search.page - 1 },
                            ..search.clone()
                        })),
                        _ => unreachable!("Page turners can only be used in search pages"),
                    },
                }
            } else {
                match &self.items[self.textlist.selected] {
                    Item::MiniVideo(MiniVideoItem { id, .. })
                    | Item::FullVideo(FullVideoItem { id, .. }) => {
                        Some(Page::SingleItem(SingleItemPage::Video(id.clone())))
                    }
                    Item::MiniPlaylist(MiniPlaylistItem { id, .. })
                    | Item::FullPlaylist(FullPlaylistItem { id, .. }) => {
                        Some(Page::SingleItem(SingleItemPage::Playlist(id.clone())))
                    }
                    Item::MiniChannel(MiniChannelItem { id, .. })
                    | Item::FullChannel(FullChannelItem { id, .. }) => {
                        Some(Page::ChannelDisplay(ChannelDisplayPage {
                            id: id.clone(),
                            r#type: ChannelDisplayPageType::Main,
                        }))
                    }
                    // Item::Unknown(_) => {
                    //     *framework.data.global.get_mut::<Message>().unwrap() =
                    //         Message::Message(String::from("Unknown item"));
                    //     framework
                    //         .data
                    //         .state
                    //         .get_mut::<Tasks>()
                    //         .unwrap()
                    //         .priority
                    //         .push(Task::RenderAll);
                    //     None
                    // }
                    Item::Page(b) => match framework.data.state.get::<Page>().unwrap() {
                        Page::Search(search) => Some(Page::Search(Search {
                            page: if *b { search.page + 1 } else { search.page - 1 },
                            ..search.clone()
                        })),
                        _ => unreachable!("Page turners can only be used in search pages"),
                    },
                }
            };

        if let Some(page_to_load) = page_to_load {
            framework
                .data
                .state
                .get_mut::<Tasks>()
                .unwrap()
                .priority
                .push(Task::LoadPage(page_to_load));
        }
    }
}
impl Default for ItemList {
    fn default() -> Self {
        Self {
            info: ItemInfo::default(),
            items: Vec::new(),
            textlist: TextList::default().non_ascii_replace(' '),
            grid: Grid::new(
                vec![Constraint::Percentage(60), Constraint::Percentage(40)],
                vec![Constraint::Percentage(100)],
            )
            .unwrap(),
            exhausted: false,
        }
    }
}
impl ItemList {
    // change `self.item` to the currently selected item
    pub fn update(&mut self, framework: &mut FrameworkClean) {
        framework
            .data
            .global
            .get_mut::<Status>()
            .unwrap()
            .last_list_state = Some((self.textlist.selected, self.textlist.scroll));

        let selected = self.items.get(self.textlist.selected);

        if selected.is_none() {
            return;
        }

        if matches!(selected, Some(Item::Page(_))) {
            framework
                .data
                .global
                .get_mut::<Status>()
                .unwrap()
                .render_image = true;
            framework
                .data
                .state
                .get_mut::<Tasks>()
                .unwrap()
                .priority
                .push(Task::ClearPage);
            self.info.item = None;
            return;
        }

        self.info.item = Some(self.items[self.textlist.selected].clone());
    }
}
