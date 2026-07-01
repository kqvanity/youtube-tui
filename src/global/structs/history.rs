use crate::global::{
    structs::Item,
    traits::{Collection, CollectionNoId},
};
use serde::{Deserialize, Serialize};
use typemap::Key;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct WatchHistory(pub Vec<Item>);

impl Key for WatchHistory {
    type Value = Self;
}

impl Collection<Item> for WatchHistory {
    const INDEX_PATH: &'static str = ".local/share/youtube-tui/watch_history.json";

    fn items(&self) -> &Vec<Item> {
        &self.0
    }

    fn items_mut(&mut self) -> &mut Vec<Item> {
        &mut self.0
    }

    fn from_items(items: Vec<Item>) -> Self {
        Self(items)
    }
}

#[derive(Clone, Default)]
pub struct SearchHistory(pub Vec<String>);

impl Key for SearchHistory {
    type Value = Self;
}

impl SearchHistory {
    pub fn load() -> Self {
        match crate::global::structs::DatabaseManager::get_search_history() {
            Ok(items) => Self(items),
            Err(_) => Self(Vec::new()),
        }
    }

    pub fn push(&mut self, query: String) {
        if query.is_empty() {
            return;
        }
        if let Some(pos) = self.0.iter().position(|q| q == &query) {
            self.0.remove(pos);
        }
        self.0.insert(0, query);
        let _ =
            crate::global::structs::DatabaseManager::add_search_history(self.0.first().unwrap());
    }

    pub fn trim(&mut self, limit: usize) {
        if self.0.len() > limit {
            self.0.truncate(limit);
        }
        let _ = crate::global::structs::DatabaseManager::trim_search_history(limit);
    }

    pub fn save(&self) {
        let _ = crate::global::structs::DatabaseManager::trim_search_history(self.0.len());
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct CommandHistory(pub Vec<String>);

impl Key for CommandHistory {
    type Value = Self;
}

impl CollectionNoId<String> for CommandHistory {
    const INDEX_PATH: &'static str = ".local/share/youtube-tui/command_history.json";

    fn items(&self) -> &Vec<String> {
        &self.0
    }

    fn items_mut(&mut self) -> &mut Vec<String> {
        &mut self.0
    }

    fn from_items(items: Vec<String>) -> Self {
        Self(items)
    }
}

/*
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ChannelHistory(pub Vec<String>);

impl Key for ChannelHistory {
    type Value = Self;
}

impl CollectionNoId<String> for ChannelHistory {
    const INDEX_PATH: &'static str = ".local/share/youtube-tui/channel_history.json";

    fn items(&self) -> &Vec<String> {
        &self.0
    }

    fn items_mut(&mut self) -> &mut Vec<String> {
        &mut self.0
    }

    fn from_items(items: Vec<String>) -> Self {
        Self(items)
    }
}
*/

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_history_default_is_empty() {
        let h = SearchHistory::default();
        assert!(h.0.is_empty());
    }

    #[test]
    fn watch_history_default_is_empty() {
        let h = WatchHistory::default();
        assert!(h.0.is_empty());
    }

    #[test]
    fn watch_history_items_accessor() {
        let items = vec![crate::global::structs::Item::MiniVideo(
            crate::global::structs::MiniVideoItem {
                title: "Test".to_string(),
                id: "v1".to_string(),
                thumbnail_url: String::new(),
                length: String::new(),
                views: None,
                channel: String::new(),
                channel_id: String::new(),
                published: None,
                timestamp: Some(1000),
                description: None,
            },
        )];
        let h = WatchHistory(items.clone());
        assert_eq!(h.0.len(), 1);
        assert_eq!(h.items().len(), 1);
    }

    #[test]
    fn watch_history_from_items() {
        let items = vec![crate::global::structs::Item::MiniVideo(
            crate::global::structs::MiniVideoItem {
                title: "Test".to_string(),
                id: "v1".to_string(),
                thumbnail_url: String::new(),
                length: String::new(),
                views: None,
                channel: String::new(),
                channel_id: String::new(),
                published: None,
                timestamp: Some(1000),
                description: None,
            },
        )];
        let h = WatchHistory::from_items(items.clone());
        assert_eq!(h.0[0].id().unwrap(), "v1");
    }

    #[test]
    fn command_history_default_is_empty() {
        let h = CommandHistory::default();
        assert!(h.0.is_empty());
    }

    #[test]
    fn command_history_items_accessor() {
        let h = CommandHistory(vec!["sync".to_string(), "search rust".to_string()]);
        assert_eq!(h.items().len(), 2);
    }

    #[test]
    fn command_history_items_mut() {
        let mut h = CommandHistory::default();
        h.items_mut().push("test".to_string());
        assert_eq!(h.0.len(), 1);
    }

    #[test]
    fn command_history_from_items() {
        let items = vec!["sync".to_string(), "sub test".to_string()];
        let h = CommandHistory::from_items(items.clone());
        assert_eq!(h.0, items);
    }
}
