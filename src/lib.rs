pub mod cache;
pub mod cookie;
mod errors;
pub mod filters;

use warp::{self, reject, http, Rejection, http::Response as HttpResponse};
use oauth2::basic::BasicClient;
use oauth2::{AuthorizationCode, AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope, PkceCodeVerifier, TokenResponse, TokenUrl};
use oauth2::reqwest::async_http_client;
use std::collections::HashMap;
use uuid::Uuid;

pub fn redirect(auth_client: BasicClient, scopes: Vec<Scope>) -> HttpResponse<String> {

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let scopes_iter = scopes.into_iter();
    let (auth_url, _csrf_token) = auth_client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(scopes_iter)
        .set_pkce_challenge(pkce_challenge)
        .url();

    let auth_url = auth_url.to_string();

    let cookie = cookie::Cookie::new("pkce", pkce_verifier.secret().to_string());
    HttpResponse::builder()
        .status(http::StatusCode::TEMPORARY_REDIRECT)
        .header("Location", auth_url)
        .header(http::header::SET_COOKIE, cookie.to_string())
        .body("".to_string())
        .unwrap()
}

pub async fn token(
    cookie: Option<String>,
    redirect_url: Option<String>,
    query: HashMap<String, String>,
    auth_client: BasicClient,
    cache: cache::Cache
) -> Result<HttpResponse<String>, Rejection> {

    let code = query.get("code");
    let code = match code {
        Some(code) => code,
        None => {
            let r = HttpResponse::builder()
                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(String::from("Internal server error, missing code from query"))
                .unwrap();
            return  Ok(r)
        }
    };

    if None == cookie {
        return Err(reject::custom(errors::CookieIsMissing));
    }
    let cookie = cookie.unwrap().to_string();
    let pkce_verifier = PkceCodeVerifier::new(cookie);
    let token_result = auth_client
        .exchange_code(AuthorizationCode::new(code.to_string()))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await;

    if let Err(err) = token_result {
        let more_details = match &err {
            oauth2::RequestTokenError::ServerResponse(error) => {
                let mut error_details = String::from("auth server reason: ");
                error_details.push_str(&error.to_string());
                error_details
            },
            oauth2::RequestTokenError::Parse(_, reason) => String::from_utf8(reason.to_vec()).expect("Found invalid utf-8"),
            oauth2::RequestTokenError::Other(_) => String::from("other"),
            _ => String::from("Something else")
        };
        let mut tmp = String::from("Auth server response not okay\n");
        tmp.push_str(&err.to_string());
        tmp.push_str(" | ");
        tmp.push_str(&more_details.to_string());
        let resp = HttpResponse::builder()
            .body(tmp);
        return match resp {
            Ok(resp) => Ok(resp),
            Err(_err) => Err(reject::custom(errors::ResponseBuildError)),
        }
    }
    let token_result = token_result.expect("Unexpected token_result can never be an Err");
    let token = token_result.access_token().secret();
    let session_id = Uuid::new_v4();
    let mut hash = cache.lock().await;
    hash.insert(session_id.to_string(), token.to_string());

    let redirect_url = redirect_url.unwrap_or("/".to_string());

    let cookie = cookie::Cookie::new("proxy_token", session_id.to_string());
    let resp = HttpResponse::builder()
        .status(http::StatusCode::TEMPORARY_REDIRECT)
        .header(http::header::SET_COOKIE, cookie.to_string())
        .header(http::header::LOCATION, redirect_url)
        .body("".to_string());
    match resp {
        Ok(resp) => Ok(resp),
        Err(_err) => Err(reject::custom(errors::ResponseBuildError)),
    }
}

/// Builds an oauth2 client
pub fn build_client(auth_url: String, token_url: String, client_id: String, client_secret: String, redirect_url: String) -> BasicClient {
    BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new(auth_url).unwrap(),
        Some(TokenUrl::new(token_url).unwrap())
        )
        .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap())
}
