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


use super::*;

impl FrameworkItem for ItemList {
    fn message(
        &mut self,
        framework: &mut FrameworkClean,
        data: std::collections::HashMap<String, Box<dyn std::any::Any>>,
    ) -> bool {
        if !data.contains_key("type") {
            return false;
        }

        let updated = data.get("type").is_some_and(|v| {
            v.downcast_ref::<String>()
                .is_some_and(|v| match v.as_str() {
                    "scrollup" => self.textlist.up().is_ok(),
                    "scrolldown" => self.textlist.down().is_ok(),
                    _ => false,
                })
        });

        if updated && !self.items.is_empty() {
            self.update(framework);
            set_envs(
                self.infalte_item_update(
                    framework.data.global.get::<MainConfig>().unwrap(),
                    framework.data.global.get::<Status>().unwrap(),
                )
                .into_iter(),
                &mut framework.data.state.get_mut::<StateEnvs>().unwrap().0,
            );

            framework
                .data
                .global
                .get_mut::<Status>()
                .unwrap()
                .render_image = true;
            self.info.item = Some(self.items[self.textlist.selected].clone());
            framework
                .data
                .state
                .get_mut::<Tasks>()
                .unwrap()
                .priority
                .push(Task::RenderAll);
        }

        updated
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

        let status = framework.data.global.get::<Status>().unwrap();
        let appearance = framework.data.global.get::<AppearanceConfig>().unwrap();
        let mainconfig = framework.data.global.get::<MainConfig>().unwrap();

        if status.provider_updated {
            set_envs(
                self.infalte_item_update(mainconfig, status).into_iter(),
                &mut framework.data.state.get_mut::<StateEnvs>().unwrap().0,
            );
        }

        self.update_appearance(appearance, mainconfig, &info);

        // creates the grid
        let grid = self.grid.clone();
        let chunks = grid.chunks(area).unwrap()[0].clone();

        // creates the text list in cell (0, 1)
        self.textlist.set_height(chunks[0].height);
        self.textlist
            .set_cursor_style(Style::default().fg(if info.selected {
                appearance.colors.outline_hover
            } else {
                appearance.colors.outline_secondary
            }));

        let textlist = self.textlist.clone();

        frame.render_widget(grid, area);
        let padded_chunk0 = chunks[0].inner(ratatui::layout::Margin { vertical: 0, horizontal: 1 });
        frame.render_widget(textlist, padded_chunk0);

        // used the `.render()` function in self.info because it is an ItemInfo and impls FrameworkItem instead of Widget
        self.info
            .render(frame, framework, chunks[1], popup_render, info);
    }

    fn selectable(&self) -> bool {
        true
    }

    fn load_item(
        &mut self,
        framework: &mut tui_additions::framework::FrameworkClean,
        _info: tui_additions::framework::ItemInfo,
    ) -> Result<(), Box<dyn Error>> {
        *self = Self::default();

        let page = framework.data.state.get::<Page>().unwrap();
        let image_index = framework
            .data
            .global
            .get::<MainConfig>()
            .unwrap()
            .image_index;

        let mainconfig = framework.data.global.get::<MainConfig>().unwrap();

        // fetch the items using the invidious api
        match page {
            Page::MainMenu(MainMenuPage::Trending) => {
                self.items = SearchProviderWrapper::trending()?
                    .into_iter()
                    .map(|item| Item::from_common_video(item, image_index))
                    .collect();
            }
            Page::MainMenu(MainMenuPage::Popular) => {
                self.items = SearchProviderWrapper::popular()?
                    .into_iter()
                    .map(|item| Item::from_popular_item(item, image_index))
                    .collect();
            }
            Page::MainMenu(MainMenuPage::Library) => {
                let history = framework.data.global.get::<Library>().unwrap();
                self.items = history.0.clone().into_iter().rev().collect();
            }
            Page::MainMenu(MainMenuPage::History) => {
                // the vector needs to be reversed because the latest watch history is pushed to
                // the back, meaning it needs to be reversed so that the latests one are on top
                let history = framework.data.global.get::<WatchHistory>().unwrap();
                self.items = history.0.clone().into_iter().rev().collect();
            }
            Page::Search(search) => {
                let mut collected = Vec::new();
                for p in 1..=search.page {
                    let mut cur_search = search.clone();
                    cur_search.page = p;
                    let items = SearchProviderWrapper::search(&cur_search)?;
                    let filtered: Vec<_> = items
                        .into_iter()
                        .map(|item| Item::from_search_item(item, image_index))
                        .filter(|item| !item.matches(mainconfig.block_list.clone()))
                        .collect();
                    collected.extend(filtered);
                }
                self.items = collected;
            }
            _ => unreachable!("item `ItemList` cannot be used in `{page:?}`"),
        }

        // update the items in text list
        self.refresh_textlist(framework);
        self.update(framework);

        {
            let status = framework.data.global.get_mut::<Status>().unwrap();
            if let Some((selected, scroll)) = status.saved_list_state.take() {
                self.textlist.selected = selected;
                self.textlist.scroll = scroll;
                if !self.items.is_empty() && self.textlist.selected >= self.items.len() {
                    self.textlist.selected = self.items.len() - 1;
                }
            }
        }

        let mainconfig = framework.data.global.get::<MainConfig>().unwrap();
        let status = framework.data.global.get::<Status>().unwrap();

        if mainconfig.images.display() {
            // download thumbnails of all videos in the list
            download_all_images(self.items.iter().map(|item| item.into()).collect());
        }

        set_envs(
            self.infalte_item_update(mainconfig, status).into_iter(),
            &mut framework.data.state.get_mut::<StateEnvs>().unwrap().0,
        );
        update_provider(framework.data);

        Ok(())
    }

    fn key_event(
        &mut self,
        framework: &mut tui_additions::framework::FrameworkClean,
        key: crossterm::event::KeyEvent,
        _info: tui_additions::framework::ItemInfo,
    ) -> Result<(), Box<dyn Error>> {
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

        // move the textlist cursor in the corresponding directions
        let updated = match action {
            KeyAction::MoveUp => self.textlist.up().is_ok(),
            KeyAction::MoveDown => {
                let ok = self.textlist.down().is_ok();

                // Infinite scrolling check near bottom
                if self.textlist.selected >= self.items.len().saturating_sub(5) && !self.exhausted {
                    if let Some(Page::Search(search)) = framework.data.state.get_mut::<Page>() {
                        let mainconfig = framework.data.global.get::<MainConfig>().unwrap();
                        let image_index = mainconfig.image_index;
                        let block_list = mainconfig.block_list.clone();

                        for _ in 0..5 {
                            let mut next_search = search.clone();
                            next_search.page += 1;

                            if let Ok(new_items) = SearchProviderWrapper::search(&next_search) {
                                if new_items.is_empty() {
                                    self.exhausted = true;
                                    break;
                                }

                                let filtered: Vec<_> = new_items
                                    .into_iter()
                                    .map(|item| Item::from_search_item(item, image_index))
                                    .filter(|item| !item.matches(block_list.clone()))
                                    .collect();

                                search.page += 1;

                                if !filtered.is_empty() {
                                    let filtered_len = filtered.len();
                                    self.items.extend(filtered);
                                    self.refresh_textlist(framework);
                                    if self.textlist.update().is_err() {
                                        break;
                                    }

                                    if mainconfig.images.display() {
                                        let offset = self.items.len() - filtered_len;
                                        download_all_images(
                                            self.items[offset..]
                                                .iter()
                                                .map(|item| item.into())
                                                .collect(),
                                        );
                                    }
                                    break;
                                }
                            } else {
                                self.exhausted = true;
                                break;
                            }
                        }
                    }
                }
                ok
            }
            KeyAction::MoveLeft | KeyAction::First => self.textlist.first().is_ok(),
            KeyAction::MoveRight | KeyAction::End => self.textlist.last().is_ok(),
            KeyAction::Select => {
                self.select_at_cursor(framework);
                false
            }
            _ => false,
        };

        // only create a render task if the key event actually changed something
        if updated && !self.items.is_empty() {
            self.update(framework);
            set_envs(
                self.infalte_item_update(
                    framework.data.global.get::<MainConfig>().unwrap(),
                    framework.data.global.get::<Status>().unwrap(),
                )
                .into_iter(),
                &mut framework.data.state.get_mut::<StateEnvs>().unwrap().0,
            );

            framework
                .data
                .global
                .get_mut::<Status>()
                .unwrap()
                .render_image = true;
            self.info.item = Some(self.items[self.textlist.selected].clone());
            framework
                .data
                .state
                .get_mut::<Tasks>()
                .unwrap()
                .priority
                .push(Task::RenderAll);
        }

        Ok(())
    }

    fn mouse_event(
        &mut self,
        framework: &mut tui_additions::framework::FrameworkClean,
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

        let previously_selected = self.textlist.selected;
        let y = (y - chunk.y) as usize + self.textlist.scroll;

        // clicking on already selected item
        if y == self.textlist.selected
            || y == self.textlist.selected + 2
            || y == self.textlist.selected + 1
        {
            self.select_at_cursor(framework);
            return true;
        }

        // clicking on rows after the last item
        if y > self.textlist.items.len() + 1 {
            let _ = self.textlist.last();
        } else if y <= self.textlist.selected {
            self.textlist.selected = y;
        } else if y >= self.textlist.selected + 2 {
            self.textlist.selected = y - 2;
        }

        if self.textlist.selected == previously_selected {
            return false;
        }

        self.update(framework);
        set_envs(
            self.infalte_item_update(
                framework.data.global.get::<MainConfig>().unwrap(),
                framework.data.global.get::<Status>().unwrap(),
            )
            .into_iter(),
            &mut framework.data.state.get_mut::<StateEnvs>().unwrap().0,
        );

        // render the new image
        framework
            .data
            .global
            .get_mut::<Status>()
            .unwrap()
            .render_image = true;

        true
    }
}
