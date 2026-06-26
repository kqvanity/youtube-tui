use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
};
use tui_additions::{
    framework::{FrameworkClean, FrameworkItem},
    widgets::{Grid, TextList},
};
use typemap::Key;

use crate::{
    config::{AppearanceConfig, KeyBindingsConfig, MainConfig, Provider},
    global::{functions::set_envs, structs::*},
};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivePane {
    #[default]
    Tags,
    Channels,
}

#[derive(Clone)]
pub struct ChannelList {
    pub tags_selector: TextList,
    pub channels_selector: TextList,
    pub tags: Vec<String>,
    pub channels: Vec<FullChannelItem>,
    pub grid: Grid,
    pub active_pane: ActivePane,
}

impl Default for ChannelList {
    fn default() -> Self {
        Self {
            tags_selector: TextList::default(),
            channels_selector: TextList::default(),
            tags: Vec::new(),
            channels: Vec::new(),
            grid: Grid::new(
                vec![Constraint::Percentage(30), Constraint::Percentage(70)],
                vec![Constraint::Percentage(100)],
            )
            .unwrap(),
            active_pane: ActivePane::Tags,
        }
    }
}

impl ChannelList {
    fn update_appearance(
        &mut self,
        info: &tui_additions::framework::ItemInfo,
        appearance: &AppearanceConfig,
    ) {
        let default_style = Style::default().fg(appearance.colors.outline);
        let hover_style = Style::default().fg(appearance.colors.outline_hover);
        let selected_style = Style::default().fg(appearance.colors.outline_selected);
        let secondary_style = Style::default().fg(appearance.colors.outline_secondary);

        if info.selected {
            self.grid.set_border_style(selected_style);
            self.tags_selector
                .set_cursor_style(if self.active_pane == ActivePane::Tags {
                    hover_style
                } else {
                    secondary_style
                });
            self.channels_selector
                .set_cursor_style(if self.active_pane == ActivePane::Channels {
                    hover_style
                } else {
                    secondary_style
                });
        } else if info.hover {
            self.grid.set_border_style(hover_style);
            self.tags_selector.set_cursor_style(secondary_style);
            self.channels_selector.set_cursor_style(secondary_style);
        } else {
            self.grid.set_border_style(default_style);
            self.tags_selector.set_cursor_style(default_style);
            self.channels_selector.set_cursor_style(default_style);
        }
    }

    fn update_lists(&mut self, subscriptions: &Subscriptions) {
        let mut tags: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut has_untagged = false;

        for item in &subscriptions.0 {
            if item.tags.is_empty() {
                has_untagged = true;
            } else {
                for tag in &item.tags {
                    tags.insert(tag.clone());
                }
            }
        }

        self.tags = tags.into_iter().collect();
        self.tags.sort();

        let mut tag_items = vec!["All subscriptions".to_string()];
        if has_untagged {
            tag_items.push("Untagged".to_string());
        }
        tag_items.extend(self.tags.iter().map(|t| t.clone()));

        self.tags_selector.set_items(&tag_items).unwrap();
        self.update_channels_pane(subscriptions);
    }

    fn update_channels_pane(&mut self, subscriptions: &Subscriptions) {
        let selected_tag_idx = self.tags_selector.selected;
        let mut filtered_channels = Vec::new();

        if selected_tag_idx == 0 {
            filtered_channels = subscriptions.0.iter().map(|s| s.channel.clone()).collect();
        } else {
            let has_untagged = subscriptions.0.iter().any(|s| s.tags.is_empty());
            let is_untagged_idx = selected_tag_idx == 1 && has_untagged;

            if is_untagged_idx {
                filtered_channels = subscriptions
                    .0
                    .iter()
                    .filter(|s| s.tags.is_empty())
                    .map(|s| s.channel.clone())
                    .collect();
            } else {
                let actual_tag_idx = if has_untagged {
                    selected_tag_idx - 2
                } else {
                    selected_tag_idx - 1
                };
                if let Some(tag) = self.tags.get(actual_tag_idx) {
                    filtered_channels = subscriptions
                        .0
                        .iter()
                        .filter(|s| s.tags.contains(tag))
                        .map(|s| s.channel.clone())
                        .collect();
                }
            }
        }

        self.channels = filtered_channels;

        let mut channel_items = vec![];
        if selected_tag_idx == 0 {
            channel_items.push("All channels".to_string());
        } else {
            let has_untagged = subscriptions.0.iter().any(|s| s.tags.is_empty());
            let is_untagged_idx = selected_tag_idx == 1 && has_untagged;
            if is_untagged_idx {
                channel_items.push("All untagged channels".to_string());
            } else {
                let actual_tag_idx = if has_untagged {
                    selected_tag_idx - 2
                } else {
                    selected_tag_idx - 1
                };
                let tag_name = self
                    .tags
                    .get(actual_tag_idx)
                    .unwrap_or(&String::new())
                    .clone();
                channel_items.push(format!("All '{}' channels", tag_name));
            }
        }

        channel_items.extend(self.channels.iter().map(|c| c.name.clone()));
        self.channels_selector.set_items(&channel_items).unwrap();

        if self.channels_selector.selected >= channel_items.len() {
            self.channels_selector.selected = 0;
            self.channels_selector.scroll = 0;
        }
    }

    fn emit_filter(&mut self, framework: &mut FrameworkClean) {
        let subscriptions = framework.data.global.get::<Subscriptions>().unwrap();
        let selected_tag_idx = self.tags_selector.selected;
        let selected_channel_idx = self.channels_selector.selected;

        let filter = if selected_tag_idx == 0 {
            if selected_channel_idx == 0 {
                VideoFilter::All
            } else if let Some(channel) = self.channels.get(selected_channel_idx - 1) {
                VideoFilter::Channel(channel.id.clone())
            } else {
                VideoFilter::All
            }
        } else {
            let has_untagged = subscriptions.0.iter().any(|s| s.tags.is_empty());
            let is_untagged_idx = selected_tag_idx == 1 && has_untagged;

            if selected_channel_idx == 0 {
                if is_untagged_idx {
                    VideoFilter::Tag(String::new())
                } else if let Some(tag) = self
                    .tags
                    .get(selected_tag_idx - 1 - if has_untagged { 1 } else { 0 })
                {
                    VideoFilter::Tag(tag.clone())
                } else {
                    VideoFilter::All
                }
            } else if let Some(channel) = self.channels.get(selected_channel_idx - 1) {
                VideoFilter::Channel(channel.id.clone())
            } else {
                VideoFilter::All
            }
        };

        framework
            .data
            .global
            .get_mut::<Status>()
            .unwrap()
            .storage
            .insert::<SubSelect>(SubSelect(filter));
        framework
            .data
            .global
            .get_mut::<Status>()
            .unwrap()
            .render_image = true;
    }

    fn set_env(&self, framework: &mut FrameworkClean) {
        let selected_channel_idx = self.channels_selector.selected;
        let id = if selected_channel_idx == 0 {
            "invalid".to_string()
        } else if let Some(channel) = self.channels.get(selected_channel_idx - 1) {
            channel.id.clone()
        } else {
            "invalid".to_string()
        };

        let mainconfig = framework.data.global.get::<MainConfig>().unwrap();
        set_envs(
            [
                (
                    String::from("hover-channel-url"),
                    format!(
                        "{}channel/{id}",
                        match framework.data.global.get::<Status>().unwrap().provider {
                            Provider::YouTube => "https://youtube.com/",
                            Provider::Invidious => mainconfig.invidious_instance.as_str(),
                        }
                    ),
                ),
                (String::from("hover-channel-id"), id),
            ]
            .into_iter(),
            &mut framework.data.state.get_mut::<StateEnvs>().unwrap().0,
        )
    }
}

impl FrameworkItem for ChannelList {
    fn load_item(
        &mut self,
        framework: &mut tui_additions::framework::FrameworkClean,
        _info: tui_additions::framework::ItemInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let appearance = framework.data.global.get::<AppearanceConfig>().unwrap();

        let subscriptions = framework.data.global.get::<Subscriptions>().unwrap();
        self.tags_selector.set_border_type(appearance.borders);
        self.channels_selector.set_border_type(appearance.borders);
        self.grid.set_border_type(appearance.borders);

        self.update_lists(subscriptions);
        self.emit_filter(framework);

        Ok(())
    }

    fn render(
        &mut self,
        frame: &mut ratatui::Frame,
        framework: &mut tui_additions::framework::FrameworkClean,
        area: ratatui::layout::Rect,
        popup_render: bool,
        info: tui_additions::framework::ItemInfo,
    ) {
        if popup_render {
            return;
        }

        let appearance = framework.data.global.get::<AppearanceConfig>().unwrap();

        self.update_appearance(&info, appearance);
        let chunks = self.grid.chunks(area).unwrap()[0].clone();
        frame.render_widget(self.grid.clone(), area);
        self.tags_selector.set_height(chunks[0].height);
        self.channels_selector.set_height(chunks[1].height);

        frame.render_widget(self.tags_selector.clone(), chunks[0]);
        frame.render_widget(self.channels_selector.clone(), chunks[1]);
    }

    fn message(
        &mut self,
        framework: &mut FrameworkClean,
        data: std::collections::HashMap<String, Box<dyn std::any::Any>>,
    ) -> bool {
        if !data.contains_key("type") {
            return false;
        }

        let old_tag_idx = self.tags_selector.selected;
        let old_channel_idx = self.channels_selector.selected;

        let updated = data.get("type").is_some_and(|v| {
            v.downcast_ref::<String>()
                .is_some_and(|v| match v.as_str() {
                    "scrollup" => {
                        if self.active_pane == ActivePane::Tags {
                            self.tags_selector.up().is_ok()
                        } else {
                            self.channels_selector.up().is_ok()
                        }
                    }
                    "scrolldown" => {
                        if self.active_pane == ActivePane::Tags {
                            self.tags_selector.down().is_ok()
                        } else {
                            self.channels_selector.down().is_ok()
                        }
                    }
                    _ => false,
                })
        });

        if updated {
            let mut changed = false;
            if self.tags_selector.selected != old_tag_idx {
                let subscriptions = framework.data.global.get::<Subscriptions>().unwrap();
                self.update_channels_pane(subscriptions);
                changed = true;
            }
            if self.channels_selector.selected != old_channel_idx || changed {
                self.emit_filter(framework);
            }

            framework
                .data
                .global
                .get_mut::<Status>()
                .unwrap()
                .render_image = true;
        }

        updated
    }

    fn key_event(
        &mut self,
        framework: &mut tui_additions::framework::FrameworkClean,
        key: crossterm::event::KeyEvent,
        _info: tui_additions::framework::ItemInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tasks = framework.data.state.get_mut::<Tasks>().unwrap();
        let old_tag_idx = self.tags_selector.selected;
        let old_channel_idx = self.channels_selector.selected;

        let action = if let Some(action) = framework
            .data
            .global
            .get::<KeyBindingsConfig>()
            .unwrap()
            .get(key)
        {
            action
        } else {
            return Ok(());
        };

        match action {
            KeyAction::MoveDown => {
                if self.active_pane == ActivePane::Tags {
                    if self.tags_selector.down().is_ok() {
                        tasks.priority.push(Task::RenderAll)
                    }
                } else if self.channels_selector.down().is_ok() {
                    tasks.priority.push(Task::RenderAll)
                }
            }
            KeyAction::MoveUp => {
                if self.active_pane == ActivePane::Tags {
                    if self.tags_selector.up().is_ok() {
                        tasks.priority.push(Task::RenderAll)
                    }
                } else if self.channels_selector.up().is_ok() {
                    tasks.priority.push(Task::RenderAll)
                }
            }
            KeyAction::MoveLeft => {
                if self.active_pane == ActivePane::Channels {
                    self.active_pane = ActivePane::Tags;
                    tasks.priority.push(Task::RenderAll);
                }
            }
            KeyAction::MoveRight => {
                if self.active_pane == ActivePane::Tags {
                    self.active_pane = ActivePane::Channels;
                    tasks.priority.push(Task::RenderAll);
                }
            }
            KeyAction::First => {
                if self.active_pane == ActivePane::Tags {
                    if self.tags_selector.first().is_ok() {
                        tasks.priority.push(Task::RenderAll)
                    }
                } else if self.channels_selector.first().is_ok() {
                    tasks.priority.push(Task::RenderAll)
                }
            }
            KeyAction::End => {
                if self.active_pane == ActivePane::Tags {
                    if self.tags_selector.last().is_ok() {
                        tasks.priority.push(Task::RenderAll)
                    }
                } else if self.channels_selector.last().is_ok() {
                    tasks.priority.push(Task::RenderAll)
                }
            }
            KeyAction::Select => {
                return Ok(());
            }
            _ => return Ok(()),
        }

        let mut changed = false;

        if self.tags_selector.selected != old_tag_idx {
            let subscriptions = framework.data.global.get::<Subscriptions>().unwrap();
            self.update_channels_pane(subscriptions);
            changed = true;
        }

        if self.channels_selector.selected != old_channel_idx || changed {
            self.emit_filter(framework);
            self.set_env(framework);
            return Ok(());
        }

        Ok(())
    }

    fn mouse_event(
        &mut self,
        framework: &mut FrameworkClean,
        x: u16,
        y: u16,
        _absolute_x: u16,
        _absolute_y: u16,
    ) -> bool {
        let chunks = self
            .grid
            .chunks(
                if let Some(prev_frame) = framework.data.global.get::<Status>().unwrap().prev_frame
                {
                    prev_frame
                } else {
                    return false;
                },
            )
            .unwrap()[0]
            .clone();

        let mut changed = false;
        let old_tag_idx = self.tags_selector.selected;
        let old_chan_idx = self.channels_selector.selected;

        if chunks[0].intersects(Rect::new(x, y, 1, 1)) {
            self.active_pane = ActivePane::Tags;
            let y_rel = (y - chunks[0].y) as usize + self.tags_selector.scroll;
            if y_rel < self.tags_selector.items.len() {
                self.tags_selector.selected = y_rel;
                if self.tags_selector.selected != old_tag_idx {
                    let subscriptions = framework.data.global.get::<Subscriptions>().unwrap();
                    self.update_channels_pane(subscriptions);
                    changed = true;
                }
            }
        } else if chunks[1].intersects(Rect::new(x, y, 1, 1)) {
            self.active_pane = ActivePane::Channels;
            let y_rel = (y - chunks[1].y) as usize + self.channels_selector.scroll;
            if y_rel < self.channels_selector.items.len() {
                self.channels_selector.selected = y_rel;
                if self.channels_selector.selected != old_chan_idx {
                    changed = true;
                }
            }
        }

        if changed {
            self.emit_filter(framework);
            self.set_env(framework);
            framework
                .data
                .global
                .get_mut::<Status>()
                .unwrap()
                .render_image = true;
            return true;
        }

        false
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum VideoFilter {
    All,
    Tag(String),
    Channel(String),
}

impl Default for VideoFilter {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Clone, Default)]
pub struct SubSelect(pub VideoFilter);

impl Key for SubSelect {
    type Value = Self;
}
