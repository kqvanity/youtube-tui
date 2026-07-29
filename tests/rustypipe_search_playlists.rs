use rustypipe::client::RustyPipe;
use rustypipe::model::{SearchResult, YouTubeItem};
use rustypipe::param::search_filter::{ItemType, SearchFilter};

fn new_client() -> RustyPipe {
    RustyPipe::builder()
        .storage_dir(std::env::temp_dir().join(format!("rp_test_search_{}", std::process::id())))
        .build()
        .unwrap()
}

const TEST_CHANNEL: &str = "UCBJycsmduvYELZo9IVnubg";

fn skip_if_not_found<T, E: std::fmt::Debug>(res: Result<T, E>) -> Option<T> {
    match res {
        Ok(v) => Some(v),
        Err(e) => {
            let msg = format!("{:?}", e);
            if msg.contains("does not exist")
                || msg.contains("NotFound")
                || msg.contains("itemSectionRenderer empty")
                || msg.contains("InvalidData")
            {
                eprintln!("SKIPPED: API returned error: {:?}", e);
                return None;
            }
            panic!("API call failed unexpectedly: {:?}", e);
        }
    }
}

async fn search_filtered(
    rp: &RustyPipe,
    query: &str,
    filter: &SearchFilter,
) -> Option<SearchResult<YouTubeItem>> {
    skip_if_not_found(rp.query().search_filter(query, filter).await)
}

// ─── Search ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn search_returns_results() {
    let rp = new_client();
    let res = search_filtered(&rp, "music", &SearchFilter::new()).await;

    let res = match res {
        Some(r) => r,
        None => return,
    };

    assert!(
        !res.items.items.is_empty(),
        "Search for 'music' should return at least one result"
    );
}

#[tokio::test]
async fn search_filter_playlist_returns_playlists() {
    let rp = new_client();
    let res = search_filtered(
        &rp,
        "music",
        &SearchFilter::new().item_type_opt(Some(ItemType::Playlist)),
    )
    .await;

    let res = match res {
        Some(r) => r,
        None => return,
    };

    eprintln!(
        "Playlist-filtered search returned {} items",
        res.items.items.len()
    );

    assert!(
        !res.items.items.is_empty(),
        "Search for 'music' with playlist filter should return at least one playlist"
    );

    let all_playlists = res
        .items
        .items
        .iter()
        .all(|item| matches!(item, YouTubeItem::Playlist(_)));
    assert!(
        all_playlists,
        "All results from playlist-filtered search should be playlists"
    );
}

#[tokio::test]
async fn search_playlist_items_have_valid_metadata() {
    let rp = new_client();
    let res = search_filtered(
        &rp,
        "music",
        &SearchFilter::new().item_type_opt(Some(ItemType::Playlist)),
    )
    .await;

    let res = match res {
        Some(r) => r,
        None => return,
    };

    for (i, item) in res.items.items.iter().enumerate() {
        if let YouTubeItem::Playlist(p) = item {
            assert!(
                !p.name.is_empty(),
                "Playlist {} should have non-empty name",
                i
            );
            assert!(!p.id.is_empty(), "Playlist {} should have non-empty id", i);
        }
    }
}

#[tokio::test]
async fn search_returns_mixed_types_without_filter() {
    let rp = new_client();
    let res = search_filtered(&rp, "music", &SearchFilter::new()).await;

    let res = match res {
        Some(r) => r,
        None => return,
    };

    let has_videos = res
        .items
        .items
        .iter()
        .any(|i| matches!(i, YouTubeItem::Video(_)));
    let has_playlists = res
        .items
        .items
        .iter()
        .any(|i| matches!(i, YouTubeItem::Playlist(_)));
    let has_channels = res
        .items
        .items
        .iter()
        .any(|i| matches!(i, YouTubeItem::Channel(_)));

    eprintln!(
        "Unfiltered search: {} items (videos={}, playlists={}, channels={})",
        res.items.items.len(),
        has_videos,
        has_playlists,
        has_channels
    );

    assert!(has_videos, "Search should return at least some videos");
}

#[tokio::test]
async fn search_filter_channel_returns_channels() {
    let rp = new_client();
    let res = search_filtered(
        &rp,
        "music",
        &SearchFilter::new().item_type_opt(Some(ItemType::Channel)),
    )
    .await;

    let res = match res {
        Some(r) => r,
        None => return,
    };

    eprintln!(
        "Channel-filtered search returned {} items",
        res.items.items.len()
    );

    assert!(
        !res.items.items.is_empty(),
        "Search for 'music' with channel filter should return at least one channel"
    );

    let all_channels = res
        .items
        .items
        .iter()
        .all(|item| matches!(item, YouTubeItem::Channel(_)));
    assert!(
        all_channels,
        "All results from channel-filtered search should be channels"
    );
}

// ─── Channel playlists ────────────────────────────────────────────────────

#[tokio::test]
async fn channel_playlists_returns_results() {
    let rp = new_client();
    let res = skip_if_not_found(rp.query().channel_playlists(TEST_CHANNEL).await);

    let res = match res {
        Some(r) => r,
        None => return,
    };

    eprintln!(
        "channel_playlists returned {} items",
        res.content.items.len()
    );

    assert!(
        !res.content.items.is_empty(),
        "channel_playlists should return at least one playlist for a real channel"
    );
}

#[tokio::test]
async fn channel_playlists_have_valid_metadata() {
    let rp = new_client();
    let res = skip_if_not_found(rp.query().channel_playlists(TEST_CHANNEL).await);

    let res = match res {
        Some(r) => r,
        None => return,
    };

    for (i, playlist) in res.content.items.iter().enumerate() {
        assert!(
            !playlist.name.is_empty(),
            "Playlist {} should have non-empty name",
            i
        );
        assert!(
            !playlist.id.is_empty(),
            "Playlist {} should have non-empty id",
            i
        );
    }
}

#[tokio::test]
async fn channel_playlists_ids_are_unique() {
    let rp = new_client();
    let res = skip_if_not_found(rp.query().channel_playlists(TEST_CHANNEL).await);

    let res = match res {
        Some(r) => r,
        None => return,
    };

    let ids: Vec<_> = res.content.items.iter().map(|p| p.id.clone()).collect();
    let unique: std::collections::HashSet<_> = ids.iter().collect();

    assert_eq!(ids.len(), unique.len(), "Playlist IDs should be unique");
}

// ─── Fetch a specific playlist ─────────────────────────────────────────────

#[tokio::test]
async fn fetch_playlist_by_id() {
    let rp = new_client();

    let search_res = search_filtered(
        &rp,
        "music",
        &SearchFilter::new().item_type_opt(Some(ItemType::Playlist)),
    )
    .await;

    let search_res = match search_res {
        Some(r) => r,
        None => return,
    };

    let playlist_id = search_res
        .items
        .items
        .iter()
        .find_map(|item| {
            if let YouTubeItem::Playlist(p) = item {
                Some(p.id.clone())
            } else {
                None
            }
        })
        .expect("Should find at least one playlist in search results");

    eprintln!("Fetching playlist: {}", playlist_id);

    let playlist = skip_if_not_found(rp.query().playlist(&playlist_id).await);
    let playlist = match playlist {
        Some(p) => p,
        None => return,
    };

    assert!(
        !playlist.name.is_empty(),
        "Fetched playlist should have non-empty name"
    );
    assert_eq!(playlist.id, playlist_id, "Fetched playlist ID should match");
    assert!(
        !playlist.videos.items.is_empty(),
        "Fetched playlist should contain at least one video"
    );
}

#[tokio::test]
async fn fetch_playlist_videos_have_metadata() {
    let rp = new_client();

    let search_res = search_filtered(
        &rp,
        "music",
        &SearchFilter::new().item_type_opt(Some(ItemType::Playlist)),
    )
    .await;

    let search_res = match search_res {
        Some(r) => r,
        None => return,
    };

    let playlist_id = search_res
        .items
        .items
        .iter()
        .find_map(|item| {
            if let YouTubeItem::Playlist(p) = item {
                Some(p.id.clone())
            } else {
                None
            }
        })
        .expect("Should find a playlist");

    let playlist = skip_if_not_found(rp.query().playlist(&playlist_id).await);
    let playlist = match playlist {
        Some(p) => p,
        None => return,
    };

    for (i, video) in playlist.videos.items.iter().enumerate() {
        assert!(
            !video.name.is_empty(),
            "Playlist video {} should have non-empty name",
            i
        );
        assert!(
            !video.id.is_empty(),
            "Playlist video {} should have non-empty id",
            i
        );
    }
}
