use crate::{store::Store, types::answer::NewAnswer};
use warp::hyper::StatusCode;

pub async fn add_answer(
    store: Store,
    answer: NewAnswer,
) -> Result<impl warp::Reply, warp::Rejection> {
    match store.add_answer(answer).await {
        Ok(_) => Ok(warp::reply::with_status("Answer added", StatusCode::OK)),
        Err(e) => Err(warp::reject::custom(e)),
    }
}
