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


use super::*;

impl FrameworkItem for SingleItem {
    fn message(
        &mut self,
        framework: &mut FrameworkClean,
        data: std::collections::HashMap<String, Box<dyn std::any::Any>>,
    ) -> bool {
        if !data.contains_key("type") {
            return false;
        }

        match &mut self.r#type {
            SingleItemType::None => false,
            SingleItemType::Video(SingleVideoItem { textlist, .. }) => {
                data.get("type").is_some_and(|v| {
                    v.downcast_ref::<String>()
                        .is_some_and(|v| match v.as_str() {
                            "scrollup" => textlist.up().is_ok(),
                            "scrolldown" => textlist.down().is_ok(),
                            _ => false,
                        })
                })
            }
            SingleItemType::Playlist(item) => {
                if item.is_commands_view {
                    data.get("type").is_some_and(|v| {
                        v.downcast_ref::<String>()
                            .is_some_and(|v| match v.as_str() {
                                "scrollup" => item.commands_view.up().is_ok(),
                                "scrolldown" => item.commands_view.down().is_ok(),
                                _ => false,
                            })
                    })
                } else {
                    let updated = data.get("type").is_some_and(|v| {
                        v.downcast_ref::<String>()
                            .is_some_and(|v| match v.as_str() {
                                "scrollup" => {
                                    if item.videos_view.selected == 1 {
                                        // going from a hovering video to not hovering will make the image
                                        // stay on the screen, therefore it needs to be removed by clearing
                                        // the screen
                                        framework
                                            .data
                                            .state
                                            .get_mut::<Tasks>()
                                            .unwrap()
                                            .priority
                                            .push(Task::ClearPage);
                                    }
                                    item.videos_view.up().is_ok()
                                }
                                "scrolldown" => item.videos_view.down().is_ok(),
                                _ => false,
                            })
                    });

                    if updated {
                        if item.videos_view.selected != 0 {
                            item.hovered_video.item = Some(
                                self.item.as_ref().unwrap().fullplaylist().unwrap().videos
                                    [item.videos_view.selected - 1]
                                    .clone(),
                            );
                        }

                        framework
                            .data
                            .global
                            .get_mut::<Status>()
                            .unwrap()
                            .render_image = true;
                        set_envs(
                            self.infalte_item_update(
                                framework.data.global.get::<MainConfig>().unwrap(),
                                framework.data.global.get::<Status>().unwrap(),
                            )
                            .into_iter(),
                            &mut framework.data.state.get_mut::<StateEnvs>().unwrap().0,
                        );
                    }

                    updated
                }
            }
        }
    }
    fn render(
        &mut self,
        frame: &mut ratatui::Frame,
        framework: &mut FrameworkClean,
        area: Rect,
        popup_render: bool,
        info: tui_additions::framework::ItemInfo,
    ) {
        let status = framework.data.global.get::<Status>().unwrap();
        if popup_render {
            return;
        }

        let appearance = framework.data.global.get::<AppearanceConfig>().unwrap();

        if self.item.is_none() {
            frame.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(appearance.borders),
                area,
            );
            return;
        }

        if status.provider_updated {
            set_envs(
                self.infalte_item_update(
                    framework.data.global.get::<MainConfig>().unwrap(),
                    status,
                )
                .into_iter(),
                &mut framework.data.state.get_mut::<StateEnvs>().unwrap().0,
            );
        }

        self.update_appearance(appearance, &info);

        let chunks = self.grid.chunks(area).unwrap()[0].clone();

        frame.render_widget(self.grid.clone(), area);

        match &mut self.r#type {
            SingleItemType::Video(typeinfo) => {
                // 2 by 1 grid, item info in the first cell and textlist at the second
                if status.provider_updated {
                    typeinfo.update_provider().into_iter().for_each(|index| {
                        typeinfo.textlist.items[index] = typeinfo.commands[index].0.clone().replace(
                            "${provider}",
                            framework
                                .data
                                .global
                                .get::<Status>()
                                .unwrap()
                                .provider
                                .as_str(),
                        )
                    });
                }
                self.iteminfo
                    .render(frame, framework, chunks[0], popup_render, info);
                typeinfo.textlist.set_height(chunks[1].height);
                frame.render_widget(typeinfo.textlist.clone(), chunks[1]);
            }
            SingleItemType::Playlist(typeinfo) => {
                // 3 by 1 grid if hovering a video inside the playlist
                // if not then 2 by 1
                //
                // item info in the first cell, textlists in the second, hovering video on 3rd (if
                // present)
                if typeinfo.is_commands_view {
                    if status.provider_updated {
                        typeinfo.update_provider().into_iter().for_each(|index| {
                            typeinfo.commands_view.items[index] =
                                typeinfo.commands[index].0.clone().replace(
                                    "${provider}",
                                    framework
                                        .data
                                        .global
                                        .get::<Status>()
                                        .unwrap()
                                        .provider
                                        .as_str(),
                                )
                        });
                    }
                    typeinfo.commands_view.set_height(chunks[1].height);
                    frame.render_widget(typeinfo.commands_view.clone(), chunks[1]);
                } else {
                    typeinfo.videos_view.set_height(chunks[1].height);
                    frame.render_widget(typeinfo.videos_view.clone(), chunks[1]);

                    if typeinfo.videos_view.selected != 0 {
                        typeinfo.hovered_video.render(
                            frame,
                            framework,
                            chunks[2],
                            popup_render,
                            info,
                        );
                    }
                }
                self.iteminfo
                    .render(frame, framework, chunks[0], popup_render, info);
            }
            SingleItemType::None => {}
        }
    }

    fn load_item(
        &mut self,
        framework: &mut tui_additions::framework::FrameworkClean,
        _info: tui_additions::framework::ItemInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        *self = Self::default();

        let page = framework.data.state.get::<Page>().unwrap();

        let r#type = if let Page::SingleItem(r#type) = page {
            r#type
        } else {
            unreachable!("item `SingleItem` cannot be used in {page:?}")
        };

        let mainconfig = framework.data.global.get::<MainConfig>().unwrap();
        // load items using the invidious api
        // gets the item that it needs to load from `data.state.Page`
        let mut is_new = true;
        let (item, r#type) = match r#type {
            SingleItemPage::Video(id) => {
                let video = if let Some(item) = LocalStore::get_info(id) {
                    is_new = false;
                    item
                } else {
                    load_video(id, mainconfig)?
                };
                (
                    video,
                    SingleItemType::Video(SingleVideoItem::new(
                        framework.data.global.get::<CommandsConfig>().unwrap(),
                        mainconfig,
                        id,
                    )),
                )
            }
            SingleItemPage::Playlist(id) => {
                let playlist = if let Some(item) = LocalStore::get_info(id) {
                    is_new = false;
                    item
                } else {
                    load_playlist(id, mainconfig)?
                };

                let r#type = SingleItemType::Playlist(
                    SinglePlaylistItem::new(
                        framework.data.global.get::<CommandsConfig>().unwrap(),
                        mainconfig,
                        id,
                        &playlist.fullplaylist().unwrap().videos,
                    )
                    .into(),
                );

                (playlist, r#type)
            }
        };

        LocalStore::set_info(item.id().unwrap().to_string(), item.clone(), is_new);

        self.item = Some(item);
        self.r#type = r#type;
        self.iteminfo.item = self.item.clone();

        if let Some(item) = &self.item {
            // if item.is_unknown() {
            //     return Ok(());
            // }

            let item = item.clone();
            // push to watch history
            let watch_history = framework.data.global.get_mut::<WatchHistory>().unwrap();
            watch_history.push(item)?;
            // watch_history.save()?;
        }

        let mainconfig = framework.data.global.get::<MainConfig>().unwrap();
        // need to update provider every time the item loads or else it will display `${provider}`
        // instead of the actual provider (e.g. `YouTube`)

        set_envs(
            self.r#type
                .inflate_load(
                    mainconfig,
                    framework.data.global.get::<Status>().unwrap(),
                    &self.item,
                )
                .into_iter(),
            &mut framework.data.state.get_mut::<StateEnvs>().unwrap().0,
        );

        set_envs(
            self.infalte_item_update(mainconfig, framework.data.global.get::<Status>().unwrap())
                .into_iter(),
            &mut framework.data.state.get_mut::<StateEnvs>().unwrap().0,
        );

        Ok(())
    }

    fn key_event(
        &mut self,
        framework: &mut tui_additions::framework::FrameworkClean,
        key: crossterm::event::KeyEvent,
        info: tui_additions::framework::ItemInfo,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // discard any key inputs if `self.is_none()` becuase nothing can happen if self is none
        if self.r#type.is_none() {
            return Ok(());
        }

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

        let updated = match &mut self.r#type {
            SingleItemType::Video(singlevideoitem) => match action {
                // move the cursor in the textlist, only update the screen if it is changed
                KeyAction::MoveUp => singlevideoitem.textlist.up().is_ok(),
                KeyAction::MoveDown => singlevideoitem.textlist.down().is_ok(),
                KeyAction::MoveLeft | KeyAction::First => singlevideoitem.textlist.first().is_ok(),
                KeyAction::MoveRight | KeyAction::End => singlevideoitem.textlist.last().is_ok(),
                KeyAction::Select => {
                    self.select_at_cursor(framework);
                    return Ok(());
                }
                _ => false,
            },
            SingleItemType::Playlist(singleplaylistitem) => {
                // there are 2 possible states in a playlist item
                // they are handelled separately
                if singleplaylistitem.is_commands_view {
                    match action {
                        KeyAction::MoveUp => singleplaylistitem.commands_view.up().is_ok(),
                        KeyAction::MoveDown => singleplaylistitem.commands_view.down().is_ok(),
                        KeyAction::MoveLeft | KeyAction::First => {
                            singleplaylistitem.commands_view.first().is_ok()
                        }
                        KeyAction::MoveRight | KeyAction::End => {
                            singleplaylistitem.commands_view.last().is_ok()
                        }
                        KeyAction::Select => {
                            self.select_at_cursor(framework);
                            return Ok(());
                        }
                        _ => false,
                    }
                } else {
                    let updated = match action {
                        // checks if it is updated, if it is and selected is not 0 (is hovering on
                        // a video), then also need to update the iteminfo
                        KeyAction::MoveUp => {
                            if singleplaylistitem.videos_view.selected == 1 {
                                // going from a hovering video to not hovering will make the image
                                // stay on the screen, therefore it needs to be removed by clearing
                                // the screen
                                framework
                                    .data
                                    .state
                                    .get_mut::<Tasks>()
                                    .unwrap()
                                    .priority
                                    .push(Task::ClearPage);
                            } else if singleplaylistitem.videos_view.selected == 0 {
                                return Ok(());
                            }

                            let updated = singleplaylistitem.videos_view.up().is_ok();
                            if singleplaylistitem.videos_view.selected != 0 {
                                singleplaylistitem.hovered_video.item = Some(
                                    self.item.as_ref().unwrap().fullplaylist()?.videos
                                        [singleplaylistitem.videos_view.selected - 1]
                                        .clone(),
                                );
                            }
                            updated
                        }
                        KeyAction::MoveDown => {
                            if singleplaylistitem.videos_view.selected
                                == singleplaylistitem.videos_view.items.len() - 1
                            {
                                return Ok(());
                            }

                            let updated = singleplaylistitem.videos_view.down().is_ok();
                            if updated && singleplaylistitem.videos_view.selected != 0 {
                                singleplaylistitem.hovered_video.item = Some(
                                    self.item.as_ref().unwrap().fullplaylist()?.videos
                                        [singleplaylistitem.videos_view.selected - 1]
                                        .clone(),
                                );
                            }
                            updated
                        }
                        KeyAction::MoveLeft => {
                            if singleplaylistitem.videos_view.selected != 0 {
                                framework
                                    .data
                                    .state
                                    .get_mut::<Tasks>()
                                    .unwrap()
                                    .priority
                                    .push(Task::ClearPage);
                            } else if singleplaylistitem.videos_view.selected == 0 {
                                return Ok(());
                            }

                            let updated = singleplaylistitem.videos_view.first().is_ok();
                            singleplaylistitem.hovered_video.item = None;
                            updated
                        }
                        KeyAction::MoveRight => {
                            if singleplaylistitem.videos_view.selected
                                == singleplaylistitem.videos_view.items.len() - 1
                            {
                                return Ok(());
                            }
                            let updated = singleplaylistitem.videos_view.last().is_ok();
                            if updated && singleplaylistitem.videos_view.selected != 0 {
                                singleplaylistitem.hovered_video.item = Some(
                                    self.item.as_ref().unwrap().fullplaylist()?.videos
                                        [singleplaylistitem.videos_view.selected - 1]
                                        .clone(),
                                );
                            }
                            updated
                        }
                        KeyAction::Select => {
                            if singleplaylistitem.videos_view.selected == 0 {
                                singleplaylistitem.is_commands_view =
                                    !singleplaylistitem.is_commands_view;
                                self.update_appearance(
                                    framework.data.global.get::<AppearanceConfig>().unwrap(),
                                    &info,
                                );
                                *framework.data.global.get_mut::<Message>().unwrap() =
                                    Message::Success(String::from("Switched view"));
                            } else {
                                framework
                                    .data
                                    .state
                                    .get_mut::<Tasks>()
                                    .unwrap()
                                    .priority
                                    .push(Task::LoadPage(Page::SingleItem(SingleItemPage::Video(
                                        self.item.as_ref().unwrap().fullplaylist()?.videos
                                            [singleplaylistitem.videos_view.selected - 1]
                                            .minivideo()?
                                            .id
                                            .clone(),
                                    ))));
                            }

                            true
                        }
                        _ => false,
                    };

                    if updated {
                        framework
                            .data
                            .global
                            .get_mut::<Status>()
                            .unwrap()
                            .render_image = true;
                        set_envs(
                            self.infalte_item_update(
                                framework.data.global.get::<MainConfig>().unwrap(),
                                framework.data.global.get::<Status>().unwrap(),
                            )
                            .into_iter(),
                            &mut framework.data.state.get_mut::<StateEnvs>().unwrap().0,
                        );
                    }

                    updated
                }
            }
            SingleItemType::None => false,
        };

        if updated {
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
            .unwrap()[0][1];

        if !chunk.intersects(Rect::new(x, y, 1, 1)) {
            return false;
        }

        let textlist = match &mut self.r#type {
            SingleItemType::Video(SingleVideoItem { textlist, .. }) => textlist,
            SingleItemType::Playlist(singleplaylistitem) => {
                if singleplaylistitem.is_commands_view {
                    &mut singleplaylistitem.commands_view
                } else {
                    let y = (y - chunk.y) as usize + singleplaylistitem.videos_view.scroll;
                    if singleplaylistitem.videos_view.selected != 0 && y == 0 {
                        framework
                            .data
                            .state
                            .get_mut::<Tasks>()
                            .unwrap()
                            .priority
                            .push(Task::ClearPage);
                    }
                    &mut singleplaylistitem.videos_view
                }
            }
            _ => return false,
        };

        let y = (y - chunk.y) as usize + textlist.scroll;

        // clicking on already selected item
        if y == textlist.selected || y == textlist.selected + 2 || y == textlist.selected + 1 {
            self.select_at_cursor(framework);
            return true;
        }

        // clicking on rows after the last item
        if y > textlist.items.len() + 1 {
            let _ = textlist.last();
        } else if y <= textlist.selected {
            textlist.selected = y;
        } else if y >= textlist.selected + 2 {
            textlist.selected = y - 2;
        }

        self.update();

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
