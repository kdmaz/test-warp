use reqwest::Error as ReqwestError;
use argon2::Error as ArgonError;
use reqwest_middleware::Error as MiddlwareReqwestError;
use std::fmt::Display;
use tracing::{event, Level};
use warp::{body::BodyDeserializeError, hyper::StatusCode, reject::Reject, Rejection, Reply};

#[derive(Debug)]
pub enum Error {
    ParseError(std::num::ParseIntError),
    MissingParameters,
    WrongPassword,
    ArgonLibraryError(ArgonError),
    DatabaseQueryError(sqlx::Error),
    ReqwestApiError(ReqwestError),
    MiddlwareReqwestApiError(MiddlwareReqwestError),
    ClientError(ApiLayerError),
    ServerError(ApiLayerError),
}

#[derive(Debug, Clone)]
pub struct ApiLayerError {
    pub status: u16,
    pub message: String,
}

impl Display for ApiLayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Status: {}, Message: {}", self.status, self.message)
    }
}

impl Reject for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ParseError(err) => write!(f, "Cannot parse parameter: {}", err),
            Error::MissingParameters => write!(f, "Missing parameter"),
            Error::WrongPassword => write!(f, "Wrong password"),
            Error::ArgonLibraryError(_) => write!(f, "Cannot verify password"),
            Error::DatabaseQueryError(_) => write!(f, "Query could not be executed"),
            Error::ReqwestApiError(err) => write!(f, "External API error: {}", err),
            Error::MiddlwareReqwestApiError(err) => write!(f, "External API error: {}", err),
            Error::ClientError(err) => write!(f, "External Client error: {}", err),
            Error::ServerError(err) => write!(f, "External Server error: {}", err),
        }
    }
}

pub async fn return_error(r: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(Error::DatabaseQueryError(e)) = r.find() {
        event!(Level::ERROR, "Database query error");

        match e {
            sqlx::Error::Database(err) => {
                if err.code().unwrap().parse::<i32>().unwrap() == 23505 {
                    Ok(warp::reply::with_status(
                        "Account already exists".to_string(), 
                        StatusCode::UNPROCESSABLE_ENTITY,
                    ))
                } else {
                    Ok(warp::reply::with_status(
                        "Cannot update data".to_string(),
                        StatusCode::UNPROCESSABLE_ENTITY,
                    ))
                }
            },
            _ => {
                Ok(warp::reply::with_status(
                    "Cannot update data".to_string(), 
                    StatusCode::UNPROCESSABLE_ENTITY,
                ))
            }
        }
    } else if let Some(Error::ReqwestApiError(e)) = r.find() {
        event!(Level::ERROR, "{}", e);
        Ok(warp::reply::with_status(
            "Internal Server Error".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    } else if let Some(crate::Error::WrongPassword) = r.find() {
        event!(Level::ERROR, "Entered wrong password");
        Ok(warp::reply::with_status(
            "Wrong E-Mail/Password combination".to_string(), 
            StatusCode::UNAUTHORIZED
        ))
    } else if let Some(Error::MiddlwareReqwestApiError(e)) = r.find() {
        event!(Level::ERROR, "{}", e);
        Ok(warp::reply::with_status(
            "Internal Server Error".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    } else if let Some(Error::ClientError(e)) = r.find() {
        event!(Level::ERROR, "{}", e);
        Ok(warp::reply::with_status(
            "Internal Server Error".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    } else if let Some(Error::ServerError(e)) = r.find() {
        event!(Level::ERROR, "{}", e);
        Ok(warp::reply::with_status(
            "Internal Server Error".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        )) 
    } else if let Some(error) = r.find::<Error>() {
        Ok(warp::reply::with_status(
            error.to_string(),
            StatusCode::RANGE_NOT_SATISFIABLE,
        ))
    } else if let Some(error) = r.find::<BodyDeserializeError>() {
        Ok(warp::reply::with_status(
            error.to_string(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ))
    } else {
        Ok(warp::reply::with_status(
            "Route not found".to_owned(),
            StatusCode::NOT_FOUND,
        ))
    }
}
