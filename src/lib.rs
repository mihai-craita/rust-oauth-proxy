use warp::reject;
use warp::{self, http, hyper::Body, Rejection, Reply, http::Response as HttpResponse};
use oauth2::basic::BasicClient;
use oauth2::{AuthorizationCode, AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope, PkceCodeVerifier, TokenResponse, TokenUrl};
use oauth2::reqwest::async_http_client;
use std::collections::HashMap;

mod errors;

pub async fn log_response(response: HttpResponse<Body>) -> Result<impl Reply, Rejection> {
    println!("{:?}", response);
    Ok(response)
}

pub fn redirect(auth_client: BasicClient) -> HttpResponse<String> {

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, _csrf_token) = auth_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("*".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let auth_url = auth_url.to_string();

    let body = format!("Go here to login: \n\n{}\n\n", auth_url);
    let mut cookie_value = String::from("pkce=");
    cookie_value.push_str(pkce_verifier.secret());
    cookie_value.push_str("; Expires=2024-03-13T00:00:00.000Z; HttpOnly");
    let resp = HttpResponse::builder()
        .status(http::StatusCode::OK)
        .header(http::header::SET_COOKIE, cookie_value)
        .body(body).unwrap();
    resp
}

pub struct ReplyError;

impl Reply for ReplyError {
    fn into_response(self) -> warp::reply::Response {
        HttpResponse::new(format!("message: " ).into())
    }
}
pub async fn token(cookie: Option<String>, query: HashMap<String, String>, auth_client: BasicClient) -> Result<HttpResponse<String>, Rejection> {

    let code = query.get("code");
    let r = HttpResponse::builder()
        .status(http::StatusCode::INTERNAL_SERVER_ERROR)
        .body(String::from("Internal server error, missing code from query")).unwrap();
    let code = match code {
        Some(code) => code,
        //None => return Err(reject::custom(errors::ResponseBuildError))
        None => return  Ok(r)
    };

    let body = match cookie {
        Some(cookie) => {
            let pkce_verifier = PkceCodeVerifier::new(cookie.to_string());
            let token_result = auth_client
                .exchange_code(AuthorizationCode::new(code.to_string()))
                // Set the PKCE code verifier.
                .set_pkce_verifier(pkce_verifier)
                .request_async(async_http_client)
                .await;

            match token_result {
                Ok(token) => {
                    let r = token.access_token();
                    format!("Token was read: {}", r.secret())
                },
                Err(err) => {
                    println!("{:#?}", err);
                    let mut tmp = String::from("Auth server ");
                    tmp.push_str(&err.to_string());
                    tmp
                },
            }
        },
        None => "Request invalid: Missing cookie on request".to_string(),
    };
    let resp = HttpResponse::builder()
        .body(body);
    match resp {
        Ok(resp) => Ok(resp),
        Err(_err) => Err(reject::custom(errors::ResponseBuildError)),
    }
    //resp
}

pub fn build_client(auth_url: String, token_url: String, client_id: String, client_secret: String, redirect_url: String) -> BasicClient {

    BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new(auth_url).unwrap(),
        Some(TokenUrl::new(token_url).unwrap())
        )
        .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap())
}
