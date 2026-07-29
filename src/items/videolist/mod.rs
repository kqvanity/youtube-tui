use ratatui::{
use tui_additions::{
use typemap::Key;
use crate::{
use super::{ItemInfo, SubSelect};

mod framework;

#[derive(Clone)]
pub struct VideoList {
    pub items: Vec<MiniVideoItem>,
    pub selector: TextList,
    pub display: ItemInfo,
    pub grid: Grid,
    pub previous: super::channellist::VideoFilter,
    pub channel_id: Option<String>,
}
impl Default for VideoList {
    fn default() -> Self {
        Self {
            selector: TextList::default(),
            display: ItemInfo::default(),
            items: Vec::new(),
            grid: Grid::new(
                vec![Constraint::Percentage(70), Constraint::Percentage(30)],
                vec![Constraint::Percentage(100)],
            )
            .unwrap(),
            previous: super::channellist::VideoFilter::All,
            channel_id: None,
        }
    }
}
impl VideoList {
    fn update_appearance(
        &mut self,
        info: &tui_additions::framework::ItemInfo,
        appearance: &AppearanceConfig,
    ) {
        if info.selected {
            self.grid
                .set_border_style(Style::default().fg(appearance.colors.outline_selected));
            self.selector
                .set_cursor_style(Style::default().fg(appearance.colors.outline_hover));
        } else if info.hover {
            self.grid
                .set_border_style(Style::default().fg(appearance.colors.outline_hover));
            self.selector
                .set_cursor_style(Style::default().fg(appearance.colors.outline_secondary));
        } else {
            self.grid
                .set_border_style(Style::default().fg(appearance.colors.outline));
            self.selector
                .set_cursor_style(Style::default().fg(appearance.colors.outline));
        }
    }

    fn update_items(
        &mut self,
        subscriptions: &Subscriptions,
        filter: super::channellist::VideoFilter,
    ) {
        self.previous = filter.clone();
        match filter {
            super::channellist::VideoFilter::All => {
                self.channel_id = None;
                self.items = subscriptions.get_all_videos();
            }
            super::channellist::VideoFilter::Tag(tag) => {
                self.channel_id = None;

                let mut vids = Vec::new();
                for item in &subscriptions.0 {
                    if (tag.is_empty() && item.tags.is_empty()) || item.tags.contains(&tag) {
                        vids.extend(item.videos.clone());
                    }
                }
                vids.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                self.items = vids;
            }
            super::channellist::VideoFilter::Channel(id) => {
                self.channel_id = Some(id.clone());
                if let Some(sub) = subscriptions.0.iter().find(|s| s.channel.id == id) {
                    self.items = sub.videos.clone();
                } else {
                    self.items.clear();
                }
            }
        }
    }

    fn select_at_cursor(&self, framework: &mut FrameworkClean) {
        let filter = &self.previous;
        let offset = if self.channel_id.is_some() { 3 } else { 1 };

        match self.selector.selected {
            // index 0: sync based on current scope
            0 => match filter {
                super::channellist::VideoFilter::All => {
                    framework
                        .data
                        .state
                        .get_mut::<Tasks>()
                        .unwrap()
                        .priority
                        .push(Task::Command("syncall".to_string()));
                }
                super::channellist::VideoFilter::Tag(tag) => {
                    framework
                        .data
                        .state
                        .get_mut::<Tasks>()
                        .unwrap()
                        .priority
                        .push(Task::Command(format!("synctag {}", tag)));
                }
                super::channellist::VideoFilter::Channel(id) => {
                    framework
                        .data
                        .state
                        .get_mut::<Tasks>()
                        .unwrap()
                        .priority
                        .push(Task::Command(format!("sync {}", id)));
                }
            },
            // view channel
            1 if self.channel_id.is_some() => framework
                .data
                .state
                .get_mut::<Tasks>()
                .unwrap()
                .priority
                .push(Task::LoadPage(Page::ChannelDisplay(ChannelDisplayPage {
                    id: self.channel_id.clone().unwrap(),
                    r#type: ChannelDisplayPageType::Main,
                }))),
            // unsub
            2 if self.channel_id.is_some() => framework
                .data
                .state
                .get_mut::<Tasks>()
                .unwrap()
                .priority
                .push(Task::Command(format!(
                    "unsub {} ;; reload",
                    self.channel_id.clone().unwrap()
                ))),
            // load video
            i => framework
                .data
                .state
                .get_mut::<Tasks>()
                .unwrap()
                .priority
                .push(Task::LoadPage(Page::SingleItem(
                    crate::global::structs::SingleItemPage::Video(
                        self.items[i - offset].id.clone(),
                    ),
                ))),
        }
    }

    fn set_env(&self, framework: &mut FrameworkClean) {
        // envs to set: hover-video-url and hover-video-id
        let id = if let Some(item) = &self.display.item {
            item.id().unwrap().to_string()
        } else {
            "invalid".to_string()
        };
        let mainconfig = framework.data.global.get::<MainConfig>().unwrap();
        set_envs(
            [
                (
                    String::from("hover-video-url"),
                    format!(
                        "{}watch?v={id}",
                        match framework.data.global.get::<Status>().unwrap().provider {
                            Provider::YouTube => "https://youtube.com/",
                            Provider::Invidious => mainconfig.invidious_instance.as_str(),
                        }
                    ),
                ),
                (String::from("hover-video-id"), id),
            ]
            .into_iter(),
            &mut framework.data.state.get_mut::<StateEnvs>().unwrap().0,
        )
    }
}
// The below chunk of code is copied from channellist, which is copied from video list, i have no
// idea what it does
impl VideoList {
    pub fn update(&mut self, framework: &mut FrameworkClean) {
        let offset = if self.channel_id.is_some() { 3 } else { 1 };
        if self.selector.selected < offset
            || self.items.get(self.selector.selected - offset).is_none()
        {
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
            self.display.item = None;
            return;
        }

        self.display.item = Some(Item::MiniVideo(
            self.items[self.selector.selected - offset].clone(),
        ));
    }
}
#[derive(Clone, Copy)]
// if image is being displayed
pub struct VidSelect(pub bool);
impl Key for VidSelect {
    type Value = Self;
}
