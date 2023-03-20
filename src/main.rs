mod filters;
mod cache;

use warp::{self, http, Filter};
use warp_reverse_proxy::reverse_proxy_filter;
use proxy::*;
use std::collections::HashMap;
use dotenvy::dotenv;
use std::env;
use filters::{log_response, with_auth_client, with_scopes, with_cache};

#[tokio::main]
async fn main() {
    dotenv().ok();
    let auth_server = env::var("AUTH_SERVER").expect("Missing .env variable AUTH_SERVER");
    let token_url = env::var("TOKEN_URL").expect("Missing .env variable TOKEN_URL");
    let client_id = env::var("CLIENT_ID").expect("Missing .env variable CLIENT_ID");
    let client_secret = env::var("CLIENT_SECRET").expect("Missing .env variable CLIENT_SECRET");
    let redirect_url = env::var("REDIRECT_URL").expect("Missing .env REDIRECT_URL");
    let scopes = env::var("SCOPES").expect("Missing .env variable SCOPES");

    let auth_client = build_client(auth_server, token_url, client_id, client_secret, redirect_url);

    let cache = cache::new_cache();

    let ping_route = warp::path!("ping")
        .map(|| warp::reply::with_status("pong", http::StatusCode::OK));

    let auth_step = warp::path!("oauth" / "auth")
        .and(with_auth_client(auth_client.clone()))
        .and(with_scopes(scopes))
        .map(redirect);

    let auth_route = warp::path!("oauth" / "callback")
        .and(warp::filters::cookie::optional("pkce"))
        .and(warp::query::<HashMap<String, String>>())
        .and(with_auth_client(auth_client))
        .and(with_cache(cache.clone()))
        .and_then(token);

    let proxy_route = warp::any()
         .and(warp::filters::cookie::cookie::<String>("proxy_token"))
         .map(|cookie_value: String| {
             println!("before proxy filter {}", cookie_value);
         })
        .untuple_one()
        .and(
            reverse_proxy_filter("".to_string(), "http://127.0.0.1:8089/".to_string())
            .and_then(log_response)
            );

        let routes = warp::any()
            .and(ping_route
                 .or(proxy_route)
                 .or(auth_route)
                 .or(auth_step)
                 .recover(handle_rejection)
                );

    let port = 3030;
    println!("The server starts on port: {} \n", port);
    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;
}

async fn handle_rejection(err: warp::Rejection) -> Result<warp::http::Response<String>, std::convert::Infallible> {
    let res = warp::http::Response::builder();
    if err.is_not_found() {
        let res= res.status(http::StatusCode::NOT_FOUND)
            .body("Page not found".to_string())
            .unwrap();
        Ok(res)
    } else if let Some(e) = err.find::<warp::reject::MissingCookie>() {
        eprintln!("Missing cookie: {:?}", e.name());
        // cookie is missing so we redirect the user for login
        let res = res.status(http::StatusCode::TEMPORARY_REDIRECT)
            .header("Location", "/oauth/auth")
            .body("".to_string())
            .unwrap();

        Ok(res)
    } else {
        eprintln!("unhandled rejection: {:?}", err);
        let res= res.status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .body("Something went wrong!".to_string())
            .unwrap();
        Ok(res)
    }
}
