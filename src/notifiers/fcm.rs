use std::fs;

use crate::core::error::{Error, Result};
use chrono::Utc;
use futures_util::lock::Mutex;
use reqwest::Client;
use serde::Serialize;
use serde_json::{from_str, to_string, to_vec};
use sha2::Sha256;
use sqlx::{query, query_scalar, PgPool};
use tokio::sync::RwLock;

use crate::core::notifier::Notifier;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct FCMNotifier {
    client: Client,
    pool: PgPool,
    access_token: Arc<Mutex<Option<String>>>,
}

impl FCMNotifier {
    pub fn new(pool: PgPool) -> Self {
        Self {
            client: Client::new(),
            pool,
            access_token: Arc::new(Mutex::new(None)),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Notification {
    title: String,
    body: String,
}


#[derive(Debug, Serialize)]
pub(crate) struct Message<T>
where
    T: Serialize,
{
    token: String,
    notification: Notification,
    data: T,
}

#[derive(Debug, Serialize)]
pub(crate) struct Body<T>
where
    T: Serialize,
{
    message: Message<T>,
}

impl Notifier for FCMNotifier {
    async fn get_token(&self, uid: &str) -> Result<Option<String>> {
        query_scalar!("SELECT fcm_token FROM users WHERE id = $1", uid)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                Error::wrap("failed to fetch fcm token".into(), 500, e)
            })
    }

    async fn update_token(&self, uid: &str, token: &str) -> Result<()> {
        query!("UPDATE users SET fcm_token = $1 WHERE id = $2", token, uid)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                Error::wrap("failed to update token".into(), 500, e)
            })?;
        Ok(())
    }
    async fn send_notification<T>(
        &self,
        to: &str,
        title: &str,
        body: &str,
        data: T,
    ) -> Result<()>
    where
        T: Serialize,
    {

        let mut access_token = self.access_token.lock().await;
        if access_token.is_none() {
            *access_token = Some(mint_access_token().await?);
        }
        let req = self
            .client
            .post("https://fcm.googleapis.com/v1/projects/webrtc-example-1af2b/messages:send")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", access_token.to_owned().unwrap()))
            .body(
                to_vec(&Body {
                    message: Message {
                        token: to.into(),
                        notification: Notification {
                                title: title.into(),
                                body: body.into(),
                        },
                        data
                    }
                })
                .map_err(|e| {
                    Error::wrap(
                        "failed to serialize Notification".into(),
                        500,
                        e,
                    )
                })?,
            )
            .build()
            .map_err(|e| {
                Error::wrap("failed to build request".into(), 500, e)
            })?;
        let res = self.client.execute(req).await.map_err(|e| {
            Error::wrap("failed to execute request".into(), 500, e)
        })?;
        if !res.status().is_success() {
            let reason = res.text().await.map_err(|e| {
                Error::wrap("failed to read response body".into(), 500, e)
            })?;
            return Err(Error::new(
                format!("failed to send notification: {}", reason),
                500,
            ));
        }
        Ok(())
    }
}

use base64::{engine::general_purpose, Engine};
use rsa::{
    pkcs1v15::SigningKey,
    pkcs8::DecodePrivateKey,
    signature::{SignatureEncoding, Signer},
    RsaPrivateKey,
};
use serde::{self, Deserialize};
use serde_json::from_reader;

#[derive(Debug, Deserialize)]
struct ServiceAccount {
    #[serde(rename = "type")]
    typ: String,
    project_id: String,
    private_key_id: String,
    private_key: String,
    client_email: String,
    client_id: String,
    auth_uri: String,
    token_uri: String,
    auth_provider_x509_cert_url: String,
    client_x509_cert_url: String,
    universe_domain: String,
}

fn read_service_account() -> Result<ServiceAccount> {
    let file = fs::File::open("service-account.json").map_err(|e| {
        Error::wrap("failed to read service-account.json".into(), 500, e)
    })?;
    let service_account: ServiceAccount = from_reader(&file).map_err(|e| {
        Error::wrap("failed to parse service account".into(), 500, e)
    })?;
    Ok(service_account)
}

#[derive(Debug, Serialize)]
struct Header {
    alg: String,
    typ: String,
    kid: String,
}

#[derive(Debug, Serialize)]
struct Claim {
    iss: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

fn generate_jwt_token(service_account: ServiceAccount) -> Result<String> {
    let header = Header {
        alg: "RS256".into(),
        typ: "JWT".into(),
        kid: service_account.private_key_id,
    };
    let header_encoding = general_purpose::STANDARD
        .encode(serde_json::to_string(&header).unwrap());
    let now = Utc::now().timestamp();
    let claim = Claim {
        iss: service_account.client_email,
        scope: "https://www.googleapis.com/auth/firebase.messaging".into(),
        aud: "https://oauth2.googleapis.com/token".into(),
        iat: now,
        exp: (now + 3600),
    };
    let claim_encoding = general_purpose::STANDARD
        .encode(serde_json::to_string(&claim).unwrap());
    let private_key = RsaPrivateKey::from_pkcs8_pem(
        &service_account.private_key,
    )
    .map_err(|e| Error::wrap("failed to load private key".into(), 500, e))?;
    let signing_key: SigningKey<Sha256> = SigningKey::new(private_key);
    let signature = signing_key
        .sign(format!("{}.{}", header_encoding, claim_encoding).as_bytes())
        .to_vec();
    let signature_encoding = general_purpose::STANDARD.encode(signature);
    Ok(format!(
        "{}.{}.{}",
        header_encoding, claim_encoding, signature_encoding
    ))
}

#[derive(Debug, Serialize)]
struct AcquireToken {
    grant_type: String,
    assertion: String,
}

#[derive(Debug, Deserialize)]
struct AcquireTokenResp {
    access_token: String,
    // scope: String,
    token_type: String,
    expires_in: i64,
}

async fn acquire_token(jwt_token: &str) -> Result<AcquireTokenResp> {
    let client = Client::new();
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&AcquireToken {
            grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer".into(),
            assertion: jwt_token.into(),
        })
        .send()
        .await
        .map_err(|e| Error::wrap("failed to send request".into(), 500, e))?;
    if !resp.status().is_success() {
        let reason = resp.text().await.map_err(|e| {
            Error::wrap("failed to read response body".into(), 500, e)
        })?;
        return Err(Error::new(
            format!("failed to mint token: {}", reason),
            500,
        ));
    }
    from_str(&resp.text().await.map_err(|e| {
        Error::wrap("failed to read response body".into(), 500, e)
    })?)
    .map_err(|e| {
        Error::wrap("failed to deserialize response body".into(), 500, e)
    })
}

async fn mint_access_token() -> Result<String> {
    let service_account = read_service_account()?;
    let jwt_token = generate_jwt_token(service_account)?;
    let token = acquire_token(&jwt_token).await?;
    Ok(token.access_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mint_access_token() {
        let service_account = read_service_account().unwrap();
        let jwt_token = generate_jwt_token(service_account).unwrap();
        let token = acquire_token(&jwt_token).await.unwrap();
        println!("{:?}", token);
    }
}
