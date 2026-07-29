use rustypipe::client::RustyPipe;
use rustypipe::param::search_filter::SearchFilter;

#[tokio::main]
async fn main() {
    let rp = RustyPipe::default();
    let q = rp.query();
    let res = q.search_filter("Rust programming", &SearchFilter::new()).await.unwrap();
    println!("First page items: {}", res.items.items.len());
    if let Some(cont) = res.items.continuation {
        println!("Has continuation: {}", cont);
        // how to call next page? 
    } else {
         println!("No continuation");
    }
}
