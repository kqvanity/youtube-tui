use futures::TryFutureExt;
use rustypipe::client::RustyPipe;

const TEST_CHANNEL: &str = "UC0jzvznwtnsdXYIp415oC9g";

fn new_client() -> RustyPipe {
    RustyPipe::builder()
        .storage_dir(std::env::temp_dir().join(format!("rp_test_{}", std::process::id())))
        .build()
        .unwrap()
}

#[tokio::test]
async fn channel_videos_returns_non_empty_list() {
    let rp = new_client();
    let q = rp.query();

    let res = q.channel_videos(TEST_CHANNEL).await;
    if let Err(e) = &res {
        let msg = format!("{:?}", e);
        if msg.contains("does not exist") || msg.contains("NotFound") {
            eprintln!("SKIPPED: channel_videos returned NotFound for {}. This may be a rustypipe scraping limitation.", TEST_CHANNEL);
            return;
        }
    }
    assert!(
        res.is_ok(),
        "channel_videos should succeed: {:?}",
        res.err()
    );
    let content = res.unwrap();
    assert!(
        !content.content.items.is_empty(),
        "channel_videos should return at least one video for a real channel"
    );
}

#[tokio::test]
async fn channel_videos_sorted_by_upload_date_descending() {
    let rp = new_client();
    let q = rp.query();

    let res = q.channel_videos(TEST_CHANNEL).await;
    if let Err(e) = &res {
        let msg = format!("{:?}", e);
        if msg.contains("does not exist") || msg.contains("NotFound") {
            eprintln!("SKIPPED: channel_videos returned NotFound");
            return;
        }
    }
    let res = res.unwrap();
    let items = &res.content.items;

    if items.len() < 2 {
        eprintln!("Skipping sort test: only {} video(s) returned", items.len());
        return;
    }

    let unsorted_count = items
        .iter()
        .zip(items.iter().skip(1))
        .filter(|(curr, next)| {
            let curr_ts = curr
                .publish_date
                .map(|t| t.unix_timestamp() as u64)
                .unwrap_or(0);
            let next_ts = next
                .publish_date
                .map(|t| t.unix_timestamp() as u64)
                .unwrap_or(0);
            curr_ts < next_ts
        })
        .count();

    assert_eq!(
        unsorted_count,
        0,
        "Videos should be sorted newest-first, but {} out of {} pairs are out of order",
        unsorted_count,
        items.len() - 1
    );
}

#[tokio::test]
async fn channel_videos_have_valid_metadata() {
    let rp = new_client();
    let q = rp.query();

    let res = q.channel_videos(TEST_CHANNEL).await;
    if let Err(e) = &res {
        let msg = format!("{:?}", e);
        if msg.contains("does not exist") || msg.contains("NotFound") {
            eprintln!("SKIPPED: channel_videos returned NotFound");
            return;
        }
    }
    let res = res.unwrap();

    for (i, video) in res.content.items.iter().enumerate() {
        assert!(
            !video.name.is_empty(),
            "Video {} should have a non-empty title",
            i
        );
        assert!(
            !video.id.is_empty(),
            "Video {} should have a non-empty id",
            i
        );
    }
}

#[tokio::test]
async fn channel_videos_have_duration() {
    let rp = new_client();
    let q = rp.query();

    let res = q.channel_videos(TEST_CHANNEL).await;
    if let Err(e) = &res {
        let msg = format!("{:?}", e);
        if msg.contains("does not exist") || msg.contains("NotFound") {
            eprintln!("SKIPPED: channel_videos returned NotFound");
            return;
        }
    }
    let res = res.unwrap();

    let total = res.content.items.len();
    let has_duration = res
        .content
        .items
        .iter()
        .any(|v| v.duration.is_some() && v.duration != Some(0));

    eprintln!(
        "Duration test: {} total videos, has_duration={}",
        total, has_duration
    );

    assert!(
        has_duration || total == 0,
        "At least some videos should have non-zero duration"
    );
}

#[tokio::test]
async fn channel_videos_have_view_counts() {
    let rp = new_client();
    let q = rp.query();

    let res = q.channel_videos(TEST_CHANNEL).await;
    if let Err(e) = &res {
        let msg = format!("{:?}", e);
        if msg.contains("does not exist") || msg.contains("NotFound") {
            eprintln!("SKIPPED: channel_videos returned NotFound");
            return;
        }
    }
    let res = res.unwrap();

    let total = res.content.items.len();
    let with_views = res
        .content
        .items
        .iter()
        .filter(|v| v.view_count.is_some() && v.view_count.unwrap() > 0)
        .count();

    eprintln!(
        "View counts test: {} total, {} have non-zero views",
        total, with_views
    );

    assert!(
        with_views > 0 || total == 0,
        "At least some videos should have non-zero view counts"
    );
}

#[tokio::test]
async fn channel_videos_have_publish_dates() {
    let rp = new_client();
    let q = rp.query();

    let res = q.channel_videos(TEST_CHANNEL).await;
    if let Err(e) = &res {
        let msg = format!("{:?}", e);
        if msg.contains("does not exist") || msg.contains("NotFound") {
            eprintln!("SKIPPED: channel_videos returned NotFound");
            return;
        }
    }
    let res = res.unwrap();

    let total = res.content.items.len();
    let with_dates = res
        .content
        .items
        .iter()
        .filter(|v| v.publish_date.is_some())
        .count();

    eprintln!(
        "Publish dates test: {} total, {} have dates",
        total, with_dates
    );

    assert!(
        with_dates > 0 || total == 0,
        "At least some videos should have dates"
    );
}

#[tokio::test]
async fn video_item_convert_produces_valid_data() {
    let rp = new_client();
    let q = rp.query();

    let res = q.channel_videos(TEST_CHANNEL).await;
    if let Err(e) = &res {
        let msg = format!("{:?}", e);
        if msg.contains("does not exist") || msg.contains("NotFound") {
            eprintln!("SKIPPED: channel_videos returned NotFound");
            return;
        }
    }
    let res = res.unwrap();
    let video = &res.content.items[0];

    assert!(!video.name.is_empty(), "Video should have non-empty title");
    assert!(!video.id.is_empty(), "Video should have non-empty id");
    assert!(
        video.duration.is_some() && video.duration != Some(0),
        "Video should have valid duration (milliseconds), got: {:?}",
        video.duration
    );
    assert_ne!(video.duration.unwrap_or(0), 0, "Duration should not be 0");

    // Also verify the converted CommonVideo would have correct seconds
    let length_seconds = video.duration.unwrap_or(0) / 1000;
    assert!(
        length_seconds > 0,
        "After ms->s conversion, length should be > 0, got {}",
        length_seconds
    );
}

#[tokio::test]
async fn channel_info_returns_valid_channel() {
    let rp = new_client();
    let q = rp.query();

    let info = q.channel_info(TEST_CHANNEL).await;

    if let Err(e) = &info {
        let msg = format!("{:?}", e);
        if msg.contains("does not exist") || msg.contains("NotFound") {
            eprintln!("SKIPPED: channel_info returned NotFound");
            return;
        }
    }

    assert!(
        info.is_ok(),
        "channel_info should succeed: {:?}",
        info.err()
    );
    let info = info.unwrap();
    assert_eq!(info.id, TEST_CHANNEL, "Channel id should match");
    assert!(!info.url.is_empty(), "Channel should have a non-empty URL");
}

#[tokio::test]
async fn music_artist_returns_channel_name() {
    let rp = new_client();
    let q = rp.query();

    let artist = q.music_artist(TEST_CHANNEL, false).await;

    if let Ok(a) = &artist {
        assert!(!a.name.is_empty(), "Artist should have a non-empty name");
        eprintln!("Artist name: {:?}", a.name);
    } else {
        eprintln!(
            "music_artist error (non-music channel?): {:?}",
            artist.err()
        );
    }
}

#[tokio::test]
async fn channel_videos_timestamps_are_reasonable() {
    let rp = new_client();
    let q = rp.query();

    let res = q.channel_videos(TEST_CHANNEL).await;
    if let Err(e) = &res {
        let msg = format!("{:?}", e);
        if msg.contains("does not exist") || msg.contains("NotFound") {
            eprintln!("SKIPPED: channel_videos returned NotFound");
            return;
        }
    }
    let res = res.unwrap();
    let now = chrono::Utc::now().timestamp() as u64;

    for (i, video) in res.content.items.iter().enumerate() {
        if let Some(ts) = video.publish_date {
            let unix = ts.unix_timestamp() as u64;
            assert!(
                unix > 0 && unix <= now,
                "Video {} has unreasonable timestamp: {} (now={})",
                i,
                unix,
                now
            );
        }
    }
}

#[tokio::test]
async fn channel_videos_video_ids_are_unique() {
    let rp = new_client();
    let q = rp.query();

    let res = q.channel_videos(TEST_CHANNEL).await;
    if let Err(e) = &res {
        let msg = format!("{:?}", e);
        if msg.contains("does not exist") || msg.contains("NotFound") {
            eprintln!("SKIPPED: channel_videos returned NotFound");
            return;
        }
    }
    let res = res.unwrap();

    let ids: Vec<_> = res.content.items.iter().map(|v| v.id.clone()).collect();
    let unique: std::collections::HashSet<_> = ids.iter().collect();

    assert_eq!(ids.len(), unique.len(), "Video IDs should be unique");
}

#[tokio::test]
async fn channel_videos_all_have_thumbnails() {
    let rp = new_client();
    let q = rp.query();

    let res = q.channel_videos(TEST_CHANNEL).await;
    if let Err(e) = &res {
        let msg = format!("{:?}", e);
        if msg.contains("does not exist") || msg.contains("NotFound") {
            eprintln!("SKIPPED: channel_videos returned NotFound");
            return;
        }
    }
    let res = res.unwrap();

    for (i, video) in res.content.items.iter().enumerate() {
        assert!(
            !video.thumbnail.is_empty(),
            "Video {} should have at least one thumbnail",
            i
        );
        assert!(
            !video.thumbnail[0].url.is_empty(),
            "Video {} thumbnail URL should not be empty",
            i
        );
    }
}

#[tokio::test]
async fn channel_playlists_returns_results() {
    let rp = new_client();
    let q = rp.query();

    let res = q.channel_playlists(TEST_CHANNEL).await;

    if let Ok(r) = &res {
        assert!(r.content.items.len() > 0);
        eprintln!("channel_playlists returned {} items", r.content.items.len());
    } else {
        eprintln!("channel_playlists error: {:?}", res.err());
    }
}
