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


mod framework;

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
#[derive(Clone, PartialEq, Eq, Debug)]
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
#[cfg(test)]
mod tests {
    use super::*;

    // Mock MiniVideoItem for testing
    fn mock_video(id: &str, timestamp: u64) -> crate::global::structs::MiniVideoItem {
        crate::global::structs::MiniVideoItem {
            id: id.to_string(),
            title: format!("Video {}", id),
            thumbnail_url: String::new(),
            length: String::new(),
            views: None,
            channel: String::new(),
            channel_id: String::new(),
            published: None,
            timestamp: Some(timestamp),
            description: None,
        }
    }

    fn mock_channel(id: &str, name: &str) -> crate::global::structs::FullChannelItem {
        crate::global::structs::FullChannelItem {
            id: id.to_string(),
            name: name.to_string(),
            thumbnail_url: String::new(),
            sub_count: 0,
            sub_count_text: String::new(),
            total_views: String::new(),
            created: String::new(),
            autogenerated: false,
            description: String::new(),
        }
    }

    fn mock_sub_item(
        channel_id: &str,
        channel_name: &str,
        videos: Vec<crate::global::structs::MiniVideoItem>,
        tags: Vec<&str>,
    ) -> crate::global::structs::SubItem {
        crate::global::structs::SubItem {
            channel: mock_channel(channel_id, channel_name),
            videos,
            last_sync: 0,
            last_sync_channel: 0,
            has_new: false,
            tags: tags.into_iter().map(String::from).collect(),
        }
    }

    fn mock_subscriptions() -> crate::global::structs::Subscriptions {
        crate::global::structs::Subscriptions(vec![
            mock_sub_item(
                "ch1",
                "Channel 1",
                vec![mock_video("v1", 3000), mock_video("v2", 2000)],
                vec!["music", "rock"],
            ),
            mock_sub_item(
                "ch2",
                "Channel 2",
                vec![mock_video("v3", 1500)],
                vec!["tech"],
            ),
            mock_sub_item("ch3", "Channel 3", vec![], vec![]),
        ])
    }

    // ─── Tag list logic ───────────────────────────────────────────────────────

    #[test]
    fn video_filter_all() {
        let filter = VideoFilter::All;
        assert!(matches!(filter, VideoFilter::All));
    }

    #[test]
    fn video_filter_tag() {
        let filter = VideoFilter::Tag("music".to_string());
        assert!(matches!(filter, VideoFilter::Tag(ref t) if t == "music"));
    }

    #[test]
    fn video_filter_channel() {
        let filter = VideoFilter::Channel("ch1".to_string());
        assert!(matches!(filter, VideoFilter::Channel(ref c) if c == "ch1"));
    }

    #[test]
    fn video_filter_clone() {
        let filter = VideoFilter::Tag("music".to_string());
        let cloned = filter.clone();
        assert_eq!(filter, cloned);
    }

    #[test]
    fn video_filter_default() {
        let filter = VideoFilter::default();
        assert!(matches!(filter, VideoFilter::All));
    }

    #[test]
    fn active_pane_default() {
        let pane = ActivePane::default();
        assert!(matches!(pane, ActivePane::Tags));
    }

    #[test]
    fn channel_list_default() {
        let list = ChannelList::default();
        assert!(list.tags.is_empty());
        assert!(list.channels.is_empty());
        assert!(matches!(list.active_pane, ActivePane::Tags));
    }

    // ─── Tag grouping logic (simulated) ─────────────────────────────────────────

    #[test]
    fn subscriptions_get_channels() {
        let subs = mock_subscriptions();
        let channels = subs.get_channels();
        assert_eq!(channels.len(), 3);
        assert_eq!(channels[0].name, "Channel 1");
        assert_eq!(channels[1].name, "Channel 2");
        assert_eq!(channels[2].name, "Channel 3");
    }

    #[test]
    fn sub_item_ord_by_newest_video() {
        let mut subs = mock_subscriptions();
        subs.0.sort(); // SubItem implements Ord
                       // ch1 has v1 (timestamp 3000, newest), should be first after sort descending
        assert_eq!(subs.0[0].channel.id, "ch1");
    }

    #[test]
    fn sub_item_with_no_videos_comes_last() {
        let mut subs = mock_subscriptions();
        subs.0.sort();
        // ch3 has no videos (timestamp 0), should be last
        assert_eq!(subs.0.last().unwrap().channel.id, "ch3");
    }
}
