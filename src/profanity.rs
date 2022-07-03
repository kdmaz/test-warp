use std::env;

use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ApiResponse(String);

#[derive(Deserialize, Serialize, Debug, Clone)]
struct BadWord {
    original: String,
    word: String,
    deviations: i64,
    info: i64,
    #[serde(rename = "replacedLen")]
    replaced_len: i64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct BadWordsResponse {
    content: String,
    bad_words_total: i64,
    bad_words_list: Vec<BadWord>,
    censored_content: String,
}

pub async fn check_profanity(content: String) -> Result<String, handle_errors::Error> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let client = ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    let api_key = env::var("BAD_WORDS_API_KEY").unwrap();

    let res = client
        .post("https://api.apilayer.com/bad_words?censor_character=*")
        .header("apiKey", api_key)
        .body(content)
        .send()
        .await
        .map_err(handle_errors::Error::MiddlwareReqwestApiError)?;

    if !res.status().is_success() {
        let status = res.status().as_u16();
        let message = res.json::<ApiResponse>().await.unwrap();

        let err = handle_errors::ApiLayerError {
            status,
            message: message.0,
        };

        if status < 500 {
            return Err(handle_errors::Error::ClientError(err));
        } else {
            return Err(handle_errors::Error::ServerError(err));
        }
    }

    match res.json::<BadWordsResponse>().await {
        Ok(res) => Ok(res.censored_content),
        Err(e) => Err(handle_errors::Error::ReqwestApiError(e)),
    }
}
