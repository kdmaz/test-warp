use crate::{
    profanity::check_profanity,
    store::Store,
    types::{
        account::Session,
        pagination::{extract_pagination, Pagination},
        question::{NewQuestion, Question},
    },
};
use std::collections::HashMap;
use tracing::{event, instrument, Level};

#[instrument]
pub async fn get_questions(
    params: HashMap<String, String>,
    store: Store,
) -> Result<impl warp::Reply, warp::Rejection> {
    event!(target: "test_warp", Level::INFO, "querying questions");
    let mut pagination = Pagination::default();

    if !params.is_empty() {
        event!(Level::INFO, pagination = true);
        pagination = extract_pagination(params)?;
    }

    let res = match store
        .get_questions(pagination.limit, pagination.offset)
        .await
    {
        Ok(res) => res,
        Err(e) => return Err(warp::reject::custom(e)),
    };

    Ok(warp::reply::json(&res))
}

#[instrument]
pub async fn add_question(
    store: Store,
    question: NewQuestion,
) -> Result<impl warp::Reply, warp::Rejection> {
    let title = check_profanity(question.title);
    let content = check_profanity(question.content);

    let (title, content) = tokio::join!(title, content);

    if let Err(e) = title {
        return Err(warp::reject::custom(e));
    }

    if let Err(e) = content {
        return Err(warp::reject::custom(e));
    }

    let question = NewQuestion {
        title: title.unwrap(),
        content: content.unwrap(),
        tags: question.tags,
    };

    match store.add_question(question).await {
        Ok(question) => Ok(warp::reply::json(&question)),
        Err(e) => Err(warp::reject::custom(e)),
    }
}

pub async fn update_question(
    id: i32,
    session: Session,
    store: Store,
    question: Question,
) -> Result<impl warp::Reply, warp::Rejection> {
    let account_id = session.account_id;
    if store.is_question_owner(id, &account_id).await? {
        let title = check_profanity(question.title);
        let content = check_profanity(question.content);

        let (title, content) = tokio::join!(title, content);

        if title.is_err() {
            return Err(warp::reject::custom(title.unwrap_err()));
        }

        if content.is_err() {
            return Err(warp::reject::custom(content.unwrap_err()));
        }

        let question = Question {
            id: question.id,
            title: title.unwrap(),
            content: content.unwrap(),
            tags: question.tags,
        };

        match store.update_question(question, account_id).await {
            Ok(question) => Ok(warp::reply::json(&question)),
            Err(e) => Err(warp::reject::custom(e)),
        }
    } else {
        Err(warp::reject::custom(handle_errors::Error::Unauthorized))
    }
}

pub async fn delete_question(id: i32, store: Store) -> Result<impl warp::Reply, warp::Rejection> {
    match store.delete_question(id).await {
        Ok(_) => Ok(warp::reply::json(&id)),
        Err(e) => Err(warp::reject::custom(e)),
    }
}
