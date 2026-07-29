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
    global::{
        functions::set_envs,
        structs::{
            ChannelDisplayPage, ChannelDisplayPageType, Item, KeyAction, MiniVideoItem, Page,
            StateEnvs, Status, Subscriptions, Task, Tasks,
        },
    },
};

use super::{ItemInfo, SubSelect};


use super::*;

fn get_options(is_channel: bool) -> &'static [&'static str] {
    // different predefined options to display depending on if "all feeds" is selected
    if is_channel {
        &["Sync feed", "View channel", "Remove subscription"]
    } else {
        &["Sync all feeds"]
    }
}

impl FrameworkItem for VideoList {
    fn message(
        &mut self,
        framework: &mut FrameworkClean,
        data: std::collections::HashMap<String, Box<dyn std::any::Any>>,
    ) -> bool {
        if !data.contains_key("type") {
            return false;
        }

        let previously_selected = self.selector.selected;

        let updated = data.get("type").is_some_and(|v| {
            v.downcast_ref::<String>()
                .is_some_and(|v| match v.as_str() {
                    "scrollup" => self.selector.up().is_ok(),
                    "scrolldown" => self.selector.down().is_ok(),
                    _ => false,
                })
        });

        if updated {
            let offset = if self.channel_id.is_some() { 3 } else { 1 };
            if self.selector.selected < offset {
                self.display.item = None;
                if previously_selected >= offset {
                    let tasks = framework.data.state.get_mut::<Tasks>().unwrap();
                    tasks.priority.push(Task::ClearPage);
                }
            } else {
                self.display.item = self
                    .items
                    .get(self.selector.selected - offset)
                    .map(|item| Item::MiniVideo(item.clone()));
            }
            framework
                .data
                .state
                .insert::<VidSelect>(VidSelect(self.selector.selected > 1));
            framework
                .data
                .global
                .get_mut::<Status>()
                .unwrap()
                .render_image = true;

            self.set_env(framework);
        }

        updated
    }

    fn load_item(
        &mut self,
        framework: &mut tui_additions::framework::FrameworkClean,
        _info: tui_additions::framework::ItemInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let appearance = framework.data.global.get::<AppearanceConfig>().unwrap();

        self.grid.set_border_type(appearance.borders);
        self.selector.set_border_type(appearance.borders);

        let filter = framework
            .data
            .global
            .get::<Status>()
            .unwrap()
            .storage
            .get::<SubSelect>()
            .map(|s| s.0.clone())
            .unwrap_or_else(|| self.previous.clone());
        // put the items into self.items
        self.update_items(
            framework.data.global.get::<Subscriptions>().unwrap(),
            filter,
        );
        // update textlist to display the items in self.items
        self.selector
            .set_items(
                &get_options(self.channel_id.is_some())
                    .iter()
                    .map(|s| s.to_string())
                    .chain(self.items.iter().map(|vid| vid.title.clone()))
                    .collect::<Vec<_>>(),
            )
            .unwrap();

        // tell channellist whether an image is being displayed
        // if yes, then the page is cleared if channellist cursor is moved
        framework.data.state.insert::<VidSelect>(VidSelect(
            self.selector.selected > if self.channel_id.is_some() { 2 } else { 0 },
        ));
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

        if let Some(subselect) = framework
            .data
            .global
            .get::<Status>()
            .unwrap()
            .storage
            .get::<SubSelect>()
        {
            self.update_items(
                framework.data.global.get::<Subscriptions>().unwrap(),
                subselect.0.clone(),
            );
            framework
                .data
                .state
                .insert::<VidSelect>(VidSelect(self.selector.selected > 1));
            self.selector
                .set_items(
                    &get_options(self.channel_id.is_some())
                        .iter()
                        .map(|s| s.to_string())
                        .chain(self.items.iter().map(|vid| vid.title.clone()))
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            self.selector.scroll = 0;
            self.selector.selected = 0;
            self.set_env(framework);
        }

        let chunks = self.grid.chunks(area).unwrap()[0].clone();

        self.update_appearance(
            &info,
            framework.data.global.get::<AppearanceConfig>().unwrap(),
        );
        frame.render_widget(self.grid.clone(), area);
        if framework
            .data
            .global
            .get::<Subscriptions>()
            .unwrap()
            .0
            .is_empty()
        {
            return;
        }

        self.selector.set_height(chunks[1].height);
        frame.render_widget(self.selector.clone(), chunks[0]);

        if self.channel_id.is_some() {
            match self.selector.selected {
                0 => {}
                1 => {}
                2 => {}
                _ => self
                    .display
                    .render(frame, framework, chunks[1], popup_render, info),
            }
        } else {
            match self.selector.selected {
                0 => {}
                _ => self
                    .display
                    .render(frame, framework, chunks[1], popup_render, info),
            }
        }
    }

    fn key_event(
        &mut self,
        framework: &mut tui_additions::framework::FrameworkClean,
        key: crossterm::event::KeyEvent,
        _info: tui_additions::framework::ItemInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tasks = framework.data.state.get_mut::<Tasks>().unwrap();
        let previously_selected = self.selector.selected;

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
            KeyAction::MoveDown if self.selector.down().is_ok() => {
                tasks.priority.push(Task::RenderAll)
            }
            KeyAction::MoveUp if self.selector.up().is_ok() => tasks.priority.push(Task::RenderAll),
            KeyAction::MoveLeft | KeyAction::First => {
                if self.selector.first().is_ok() {
                    tasks.priority.push(Task::RenderAll)
                }
            }
            KeyAction::MoveRight | KeyAction::End => {
                if self.selector.last().is_ok() {
                    tasks.priority.push(Task::RenderAll)
                }
            }
            KeyAction::Select => {
                self.select_at_cursor(framework);
                return Ok(());
            }
            _ => return Ok(()),
        }

        let offset = if self.channel_id.is_some() { 3 } else { 1 };
        if self.selector.selected != previously_selected {
            if self.selector.selected < offset {
                self.display.item = None;
                if previously_selected >= offset {
                    tasks.priority.push(Task::ClearPage);
                }
            } else {
                self.display.item = self
                    .items
                    .get(self.selector.selected - offset)
                    .map(|item| Item::MiniVideo(item.clone()));
            }
            framework
                .data
                .state
                .insert::<VidSelect>(VidSelect(self.selector.selected > 1));
            framework
                .data
                .global
                .get_mut::<Status>()
                .unwrap()
                .render_image = true;

            self.set_env(framework);
            return Ok(());
        }

        if self.selector.selected < offset {
            return Ok(());
        }

        match framework
            .data
            .global
            .get::<Subscriptions>()
            .unwrap()
            .0
            .get(self.selector.selected - offset)
        {
            Some(item)
                if item.channel.id
                    != match &self.display.item {
                        Some(displayed) => displayed.id().unwrap_or_default(),
                        None => "",
                    } => {}
            Some(_) => {}
            None => self.display.item = None,
        };

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
        let chunk = self
            .grid
            .chunks(
                if let Some(prev_frame) = framework.data.global.get::<Status>().unwrap().prev_frame
                {
                    prev_frame
                } else {
                    return false;
                },
            )
            .unwrap()[0][0];

        if !chunk.intersects(Rect::new(x, y, 1, 1)) {
            return false;
        }

        let previously_selected = self.selector.selected;
        let y = (y - chunk.y) as usize + self.selector.scroll;

        if y == self.selector.selected
            || y == self.selector.selected + 2
            || y == self.selector.selected + 1
        {
            self.select_at_cursor(framework);
            return true;
        }

        // clicking on rows after the last item
        if y > self.selector.items.len() + 1 {
            let _ = self.selector.last();
        } else if y <= self.selector.selected {
            self.selector.selected = y;
        } else if y >= self.selector.selected + 2 {
            self.selector.selected = y - 2;
        }

        if self.selector.selected == previously_selected {
            return false;
        }

        self.update(framework);
        self.set_env(framework);
        // render the new image
        let status = framework.data.global.get_mut::<Status>().unwrap();
        status.render_image = true;
        status
            .storage
            .insert::<VidSelect>(VidSelect(self.selector.selected > 1));

        if framework.data.state.get::<VidSelect>().unwrap().0 {
            framework
                .data
                .state
                .get_mut::<Tasks>()
                .unwrap()
                .priority
                .push(Task::ClearPage);
        }
        true
    }
}
