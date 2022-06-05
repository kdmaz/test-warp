use warp::Filter;

#[tokio::main]
async fn main() {
    let filter = warp::path("hi").map(|| format!("hi!"));
    warp::serve(filter).run(([127,0,0,1], 8080)).await;
}
