use rustypipe::client::RustyPipe;
use rustypipe::param::search_filter::SearchFilter;

#[tokio::main]
async fn main() {
    let rp = RustyPipe::default();
    let q = rp.query();
    let mut current_page = 1_u16;
    let target_page = 3_u16;
    
    let mut res = q.search_filter("Rust programming", &SearchFilter::new()).await.unwrap();
    println!("Got page {}, has {} items", current_page, res.items.items.len());
    
    while current_page < target_page {
        current_page += 1;
        if let Some(cont) = res.items.continuation.take() {
            res = q.search_continuation(cont).await.unwrap();
            println!("Got page {}, has {} items", current_page, res.items.items.len());
        } else {
            println!("No continuation for page {}", current_page);
            break;
        }
    }
}
