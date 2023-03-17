use cookie::Cookie;
use warp::reject;
use warp::{self, http, Rejection, Reply, http::Response as HttpResponse};
use oauth2::basic::BasicClient;
use oauth2::{AuthorizationCode, AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope, PkceCodeVerifier, TokenResponse, TokenUrl};
use oauth2::reqwest::async_http_client;
use std::collections::HashMap;
use uuid::Uuid;

mod errors;
mod cookie;
mod cache;

pub fn redirect(auth_client: BasicClient, scopes: Vec<Scope>) -> HttpResponse<String> {

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let scopes_iter = scopes.into_iter();
    let (auth_url, _csrf_token) = auth_client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(scopes_iter)
        .set_pkce_challenge(pkce_challenge)
        .url();

    let auth_url = auth_url.to_string();

    let body = format!("Go here to login: \n<br><a href=\"{}\">{}</a>\n\n", auth_url, auth_url);

    let cookie = Cookie::new("pkce", pkce_verifier.secret().to_string());
    let resp = HttpResponse::builder()
        .status(http::StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(http::header::SET_COOKIE, cookie.to_string())
        .body(body).unwrap();
    resp
}

pub struct ReplyError;

impl Reply for ReplyError {
    fn into_response(self) -> warp::reply::Response {
        HttpResponse::new(format!("message: " ).into())
    }
}

pub async fn token(
    cookie: Option<String>, query: HashMap<String, String>, auth_client: BasicClient, cache: cache::Cache) -> Result<HttpResponse<String>, Rejection> {

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
        // None => "Request invalid: Missing cookie on request".to_string(),
        return Err(reject::custom(errors::ResponseBuildError));
    }
    let cookie = cookie.unwrap().to_string();
    let pkce_verifier = PkceCodeVerifier::new(cookie);
    let token_result = auth_client
        .exchange_code(AuthorizationCode::new(code.to_string()))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await;

    if let Err(err) = token_result {
        println!("{:#?}", err);
        let mut tmp = String::from("Auth server ");
        tmp.push_str(&err.to_string());
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

    let cookie = Cookie::new("proxy_token", session_id.to_string());

    let body = format!("Token was read");
    let resp = HttpResponse::builder()
        .header(http::header::SET_COOKIE, cookie.to_string())
        .body(body);
    return match resp {
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
