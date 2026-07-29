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


use super::*;

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

        let padded_chunk0 = chunks[0].inner(ratatui::layout::Margin { vertical: 0, horizontal: 1 });
        let padded_chunk1 = chunks[1].inner(ratatui::layout::Margin { vertical: 0, horizontal: 1 });
        frame.render_widget(self.tags_selector.clone(), padded_chunk0);
        frame.render_widget(self.channels_selector.clone(), padded_chunk1);
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
