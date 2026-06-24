use crate::config::BlockList;
use crate::global::structs::Item;

pub trait Filter {
    fn matches(&self, _black_list: BlockList) -> bool { false }
}

impl Filter for Item {
    fn matches(&self, black_list: BlockList) -> bool {
        let blocked_channels = crate::global::structs::DatabaseManager::get_all_blocked_channels().unwrap_or_default();
        let blocked_playlists = crate::global::structs::DatabaseManager::get_all_blocked_playlists().unwrap_or_default();
        
        let self_id = self.id().unwrap_or_default();
        
        struct Result {
            title: String,
            channel_title: String,
            channel_id: String,
        }
        let result = match self.clone() {
            Item::MiniVideo(video) => Result{
                title: video.title,
                channel_id: video.channel_id,
                channel_title: video.channel,
            },
            Item::MiniPlaylist(playlist) => Result{
                title: playlist.title,
                channel_id: playlist.channel_id,
                channel_title: playlist.channel,
            },
            Item::FullVideo(video) => Result{
                title: video.title,
                channel_id: video.channel_id,
                channel_title: video.channel,
            },
            Item::FullPlaylist(playlist) => Result{
                title: playlist.title,
                channel_id: playlist.channel_id,
                channel_title: playlist.channel,
            },
            Item::MiniChannel(channel) => Result{
                title: channel.name.clone(),
                channel_id: channel.id,
                channel_title: channel.name,
            },
            Item::FullChannel(channel) => Result{
                title: channel.name.clone(),
                channel_id: channel.id,
                channel_title: channel.name,
            },
            _ => Result{
                title: "".to_string(),
                channel_id: "".to_string(),
                channel_title: "".to_string(),
            },
        };
        
        blocked_channels.contains(&result.channel_id) ||
        blocked_playlists.contains(self_id) ||
        black_list.channels.contains(&result.channel_id) ||
            black_list.keywords.contains(&result.channel_title.to_lowercase()) ||
            black_list.keywords.iter().any(|x| {
                result.title.to_lowercase().contains(x)
            })
    }
}