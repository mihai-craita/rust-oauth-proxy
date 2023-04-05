mod errors;

use warp::{self, Filter};
use warp_reverse_proxy::reverse_proxy_filter;
use proxy::*;
use proxy::filters::*;
use std::{env, collections::HashMap};
use dotenvy::dotenv;

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

    let ping_route = warp::path!("ping").map(|| "pong".to_string());

    let auth_route = warp::path!("oauth" / "auth")
        .and(with_auth_client(auth_client.clone()))
        .and(with_scopes(scopes))
        .map(redirect);

    let callback_route = warp::path!("oauth" / "callback")
        .and(warp::filters::cookie::optional("pkce"))
        .and(warp::query::<HashMap<String, String>>())
        .and(with_auth_client(auth_client))
        .and(with_cache(cache.clone()))
        .and_then(token);

    let proxy_route = warp::any()
        .and(handle_auth_cookie(cache.clone()))
        .untuple_one()
        .and(
            reverse_proxy_filter("".to_string(), "http://127.0.0.1:8089/".to_string())
            .and_then(log_response)
            );

        let routes = warp::any()
            .and(ping_route
                 .or(auth_route)
                 .or(callback_route)
                 .or(proxy_route)
                 .recover(filters::handle_rejection)
                );

    let port = 3030;
    println!("The server starts on port: {} \n", port);
    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;
}
