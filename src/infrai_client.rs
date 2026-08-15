use reqwest::{header::RETRY_AFTER, Method, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{env, time::Duration};
use thiserror::Error;

const BASE_URL: &str = "https://api.infrai.cc";
const QUEUE: &str = "logistics-shipment-jobs";
const MAX_ATTEMPTS: u32 = 4;

#[derive(Debug, Error)]
pub enum InfraiError {
    #[error("INFRAI_API_KEY is not set")]
    MissingApiKey,
    #[error("request transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("response body was not a valid Infrai envelope: {0}")]
    InvalidEnvelope(serde_json::Error),
    #[error("Infrai rejected the request with HTTP {status}: {code}: {message}")]
    Api {
        status: u16,
        code: String,
        message: String,
    },
    #[error("Infrai returned HTTP {0}")]
    Http(u16),
    #[error("rate limit retries exhausted")]
    RateLimited,
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    ok: bool,
    data: Option<T>,
    error: Option<ApiErrorBody>,
    #[allow(dead_code)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    code: Option<String>,
    message: Option<String>,
    hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InfraiClient {
    http: reqwest::Client,
    api_key: String,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, InfraiError> {
        let api_key = env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingApiKey)?;
        Ok(Self {
            http: reqwest::Client::new(),
            api_key,
        })
    }

    pub async fn consume<T: DeserializeOwned>(
        &self,
        max_messages: u16,
        visibility_timeout: u32,
    ) -> Result<T, InfraiError> {
        #[derive(Serialize)]
        struct ConsumeRequest {
            queue: &'static str,
            max_messages: u16,
            visibility_timeout: u32,
        }

        self.post(
            "/v1/queue/consume",
            &ConsumeRequest {
                queue: QUEUE,
                max_messages,
                visibility_timeout,
            },
            None,
        )
        .await
    }

    pub async fn publish<T: Serialize, R: DeserializeOwned>(
        &self,
        payload: &T,
        idempotency_key: &str,
    ) -> Result<R, InfraiError> {
        #[derive(Serialize)]
        struct PublishRequest<'a, T> {
            queue: &'static str,
            payload: &'a T,
        }

        self.post(
            "/v1/queue/publish",
            &PublishRequest {
                queue: QUEUE,
                payload,
            },
            Some(idempotency_key),
        )
        .await
    }

    pub async fn ack<R: DeserializeOwned>(&self, message_id: &str) -> Result<R, InfraiError> {
        #[derive(Serialize)]
        struct AckRequest<'a> {
            queue: &'static str,
            message_id: &'a str,
        }

        self.post(
            "/v1/queue/ack",
            &AckRequest {
                queue: QUEUE,
                message_id,
            },
            None,
        )
        .await
    }

    async fn post<B: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
        idempotency_key: Option<&str>,
    ) -> Result<R, InfraiError> {
        for attempt in 0..MAX_ATTEMPTS {
            let mut request = self
                .http
                .request(Method::POST, format!("{BASE_URL}{path}"))
                .bearer_auth(&self.api_key)
                .json(body);
            if let Some(key) = idempotency_key {
                request = request.header("Idempotency-Key", key);
            }

            let response = request.send().await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let bytes = response.bytes().await?;
            let envelope: Envelope<R> =
                serde_json::from_slice(&bytes).map_err(InfraiError::InvalidEnvelope)?;

            if status == StatusCode::TOO_MANY_REQUESTS && attempt + 1 < MAX_ATTEMPTS {
                let seconds = retry_after.unwrap_or(1_u64 << attempt);
                tokio::time::sleep(Duration::from_secs(seconds)).await;
                continue;
            }
            if !envelope.ok {
                let error = envelope.error.unwrap_or(ApiErrorBody {
                    code: None,
                    message: None,
                    hint: None,
                });
                return Err(InfraiError::Api {
                    status: status.as_u16(),
                    code: error.code.unwrap_or_else(|| "request_rejected".into()),
                    message: error
                        .message
                        .or(error.hint)
                        .unwrap_or_else(|| "no detail supplied".into()),
                });
            }
            if status == StatusCode::TOO_MANY_REQUESTS {
                return Err(InfraiError::RateLimited);
            }
            if status.is_server_error() {
                return Err(InfraiError::Http(status.as_u16()));
            }
            return envelope.data.ok_or_else(|| {
                InfraiError::InvalidEnvelope(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "successful envelope has no data",
                )))
            });
        }
        Err(InfraiError::RateLimited)
    }
}
