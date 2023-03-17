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
        // .recover(handle_errors)
        //.and(warp::header::headers_cloned())
        .and_then(token);

    let proxy_route = warp::any()
        .and(
            reverse_proxy_filter("".to_string(), "http://127.0.0.1:8089/".to_string())
            .and_then(log_response),
            );

    let routes = warp::any()
        .and(ping_route)
        .or(auth_route)
        .or(auth_step)
        .or(proxy_route);

    let port = 3030;
    println!("The server starts on port: {} \n", port);
    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;
}
