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

#[derive(Clone)]
pub struct SinglePlaylistItem {
    pub commands_view: TextList,
    pub videos_view: TextList,
    pub commands: Vec<(String, String)>,
    pub is_commands_view: bool,
    pub hovered_video: ItemInfo,
}
impl SinglePlaylistItem {
    pub fn new(
        commands: &CommandsConfig,
        mainconfig: &MainConfig,
        id: &str,
        playlist_items: &[Item],
    ) -> Self {
        let saved = find_library_item(id, mainconfig).is_some();
        if saved {
            Self::new_with_map(
                commands.saved_playlist.clone().into_iter().collect(),
                playlist_items,
            )
        } else {
            Self::new_with_map(
                commands.playlist.clone().into_iter().collect(),
                playlist_items,
            )
        }
    }

    pub fn new_with_map(commands: Vec<(String, String)>, playlist_items: &[Item]) -> Self {
        let hovered_video = ItemInfo::new(if playlist_items.is_empty() {
            None
        } else {
            Some(playlist_items[0].clone())
        });

        Self {
            commands_view: TextList::default()
                .items(
                    &commands
                        .iter()
                        .map(|command| &command.0)
                        .collect::<Vec<_>>(),
                )
                .unwrap(),
            videos_view: TextList::default()
                .items(&{
                    let mut items = vec!["Switch view"];
                    items.extend(
                        playlist_items
                            .iter()
                            .map(|item| item.minivideo().unwrap().title.as_str()),
                    );
                    items
                })
                .unwrap(),
            commands,
            hovered_video,
            is_commands_view: true,
        }
    }

    pub fn update_appearance(
        &mut self,
        appearance: &AppearanceConfig,
        iteminfo: &tui_additions::framework::ItemInfo,
        grid: &mut Grid,
    ) {
        if self.is_commands_view {
            grid.widths = vec![Constraint::Percentage(30), Constraint::Percentage(70)];
            self.commands_view.set_border_type(appearance.borders);
            self.commands_view
                .set_style(Style::default().fg(appearance.colors.text));

            if iteminfo.selected {
                self.commands_view
                    .set_cursor_style(Style::default().fg(appearance.colors.outline_hover));
                self.commands_view
                    .set_selected_style(Style::default().fg(appearance.colors.text_special));
            } else {
                self.commands_view
                    .set_cursor_style(Style::default().fg(appearance.colors.outline_secondary));
                self.commands_view
                    .set_selected_style(Style::default().fg(appearance.colors.text_secondary));
            }
        } else {
            grid.widths = if self.videos_view.selected == 0 {
                vec![Constraint::Percentage(30), Constraint::Percentage(70)]
            } else {
                vec![
                    Constraint::Percentage(30),
                    Constraint::Percentage(40),
                    Constraint::Percentage(30),
                ]
            };
            self.videos_view.set_border_type(appearance.borders);
            self.videos_view
                .set_style(Style::default().fg(appearance.colors.text));

            if iteminfo.selected {
                self.videos_view
                    .set_cursor_style(Style::default().fg(appearance.colors.outline_hover));
                self.videos_view
                    .set_selected_style(Style::default().fg(appearance.colors.text_special));
            } else {
                self.videos_view
                    .set_cursor_style(Style::default().fg(appearance.colors.outline_secondary));
                self.videos_view
                    .set_selected_style(Style::default().fg(appearance.colors.text_secondary));
            }
        }
    }

    /// find all occurances of ${provider}
    pub fn update_provider(&mut self) -> Vec<usize> {
        self.commands
            .iter()
            .enumerate()
            .filter(|(_index, (display, _command))| display.contains("${provider}"))
            .map(|(index, _)| index)
            .collect()
    }

    /// creates a hashmap from `self`, containing info of the current item
    pub fn inflate_load(&self, item: &Item, mainconfig: &MainConfig) -> Vec<(String, String)> {
        let playlist_item = item.fullplaylist().unwrap();
        let path = find_library_item(&playlist_item.id, mainconfig);

        vec![
            (String::from("id"), playlist_item.id.clone()),
            (String::from("channel-id"), playlist_item.channel_id.clone()),
            (String::from("title"), playlist_item.title.clone()),
            (
                String::from("all-ids"),
                playlist_item
                    .videos
                    .iter()
                    .map(|video| video.minivideo().unwrap().id.as_str())
                    .collect::<Vec<&str>>()
                    .join(" "),
            ),
            (
                String::from("offline-path"),
                path.clone()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap()
                    .to_string(),
            ),
            (
                String::from("offline-queuelist"),
                match path {
                    Some(path) => match fs::read_dir(path) {
                        Ok(entries) => entries
                            .map(|entry| {
                                format!(
                                    "mpv loadfile '{}' append",
                                    match entry {
                                        Ok(entry) =>
                                            entry.path().as_os_str().to_str().unwrap().to_string(),
                                        Err(_) => String::new(),
                                    }
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(" ;; "),
                        Err(e) => e.to_string(),
                    },
                    None => String::from("bad path"),
                },
            ),
        ]
    }
}
impl SingleItem {
    fn infalte_item_update(
        &self,
        mainconfig: &MainConfig,
        status: &Status,
    ) -> Vec<(String, String)> {
        if let SingleItemType::Playlist(singleplaylistitem) = &self.r#type {
            if singleplaylistitem.videos_view.selected == 0 {
                return vec![(String::from("hover-url"), String::from("not avaliable"))];
            }
            vec![(
                String::from("hover-url"),
                match &self.item {
                    Some(item) => format!(
                        "{}/watch?v={}",
                        match status.provider {
                            Provider::YouTube => "https://youtube.com",
                            Provider::Invidious => &mainconfig.invidious_instance,
                        },
                        item.fullplaylist().unwrap().videos
                            [singleplaylistitem.videos_view.selected - 1]
                            .minivideo()
                            .unwrap()
                            .id
                    ),
                    None => String::from("not avaliable"),
                },
            )]
        } else {
            vec![(String::from("hover-url"), String::from("not avaliable"))]
        }
    }
    /// update colours and layout every render
    fn update_appearance(
        &mut self,
        appearance: &AppearanceConfig,
        iteminfo: &tui_additions::framework::ItemInfo,
    ) {
        self.grid.set_border_type(appearance.borders);

        if iteminfo.selected {
            self.grid
                .set_border_style(Style::default().fg(appearance.colors.outline_selected));
        } else if iteminfo.hover {
            self.grid
                .set_border_style(Style::default().fg(appearance.colors.outline_hover));
        } else {
            self.grid
                .set_border_style(Style::default().fg(appearance.colors.outline));
        }

        self.r#type
            .update_appearance(appearance, iteminfo, &mut self.grid);
    }

    /// update hover item preview
    fn update(&mut self) {
        if let SingleItemType::Playlist(singleplaylistitem) = &mut self.r#type {
            let SinglePlaylistItem {
                hovered_video,
                videos_view,
                ..
            } = &mut **singleplaylistitem;
            if videos_view.items.is_empty() || videos_view.selected == 0 {
                hovered_video.item = None;
                return;
            }

            if hovered_video.item.is_none()
                || self.item.as_ref().unwrap().fullplaylist().unwrap().videos
                    [videos_view.selected - 1]
                    .id()
                    != hovered_video.item.as_ref().unwrap().id()
            {
                hovered_video.item = Some(
                    self.item.as_ref().unwrap().fullplaylist().unwrap().videos
                        [videos_view.selected - 1]
                        .clone(),
                );
            }
        }
    }

    /// handle enter presses
    fn select_at_cursor(
        &mut self,
        framework: &mut FrameworkClean,
        // info: tui_additions::framework::ItemInfo,
    ) {
        match &mut self.r#type {
            SingleItemType::Video(singlevideoitem) => {
                let command_string = singlevideoitem.commands[singlevideoitem.textlist.selected]
                    .1
                    .clone();

                // check if the command starts with an ':' which case should be captured
                framework
                    .data
                    .state
                    .get_mut::<Tasks>()
                    .unwrap()
                    .priority
                    .push(Task::Command(apply_envs(command_string)));
            }
            SingleItemType::Playlist(singleplaylistitem) => {
                let command_string = singleplaylistitem.commands
                    [singleplaylistitem.commands_view.selected]
                    .1
                    .clone();

                // checks for special cases
                match command_string.as_str() {
                    "%switch-view%" => {
                        singleplaylistitem.is_commands_view = !singleplaylistitem.is_commands_view;
                        // self.update_appearance(
                        //     framework.data.global.get::<AppearanceConfig>().unwrap(),
                        //     &info,
                        // );
                        *framework.data.global.get_mut::<Message>().unwrap() =
                            Message::Success(String::from("Switched view"));
                    }
                    _ => {
                        // check if the command starts with an ':' which case should be captured
                        framework
                            .data
                            .state
                            .get_mut::<Tasks>()
                            .unwrap()
                            .priority
                            .push(Task::Command(apply_envs(command_string)));
                    }
                };
            }
            _ => return,
        }

        framework
            .data
            .state
            .get_mut::<Tasks>()
            .unwrap()
            .priority
            .push(Task::RenderAll);
    }
}
