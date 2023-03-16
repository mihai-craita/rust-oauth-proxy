mod errors;
mod filters;

use warp::{self, http, Filter};
use warp_reverse_proxy::reverse_proxy_filter;
use proxy::*;
use std::collections::HashMap;
use dotenvy::dotenv;
use std::env;
use filters::with_auth_client;

#[tokio::main]
async fn main() {

    dotenv().ok();
    let auth_server = env::var("AUTH_SERVER").expect("Missing .env variable AUTH_SERVER");
    let token_url = env::var("TOKEN_URL").expect("Missing .env variable TOKEN_URL");
    let client_id = env::var("CLIENT_ID").expect("Missing .env variable CLIENT_ID");
    let client_secret = env::var("CLIENT_SECRET").expect("Missing .env variable CLIENT_SECRET");
    let redirect_url = env::var("REDIRECT_URL").expect("Missing .env REDIRECT_URL");

    let auth_client = build_client(auth_server, token_url, client_id, client_secret, redirect_url);

    let health_route = warp::path!("health")
        .map(|| http::StatusCode::OK);

    let auth_step = warp::path!("oauth" / "auth")
        .and(with_auth_client(auth_client.clone()))
        .map(redirect);

    let auth_route = warp::path!("oauth" / "callback")
        .and(warp::filters::cookie::optional("pkce"))
        .and(warp::query::<HashMap<String, String>>())
        .and(with_auth_client(auth_client))
        // .recover(handle_errors)
        //.and(warp::header::headers_cloned())
        .and_then(token);
        // .map(|c| token);

    let proxy_route = warp::any()
        // .and(warp::query::<HashMap<String, String>>())
        // .and(warp::path::full())
        // .and(warp::method())
        .and(
            reverse_proxy_filter("".to_string(), "http://127.0.0.1:8089/".to_string())
            .and_then(log_response),
            );
        // .map(|q: HashMap<_, _>, p: FullPath, method| {
        //     let path = p.as_str();
        //     format!("method: {}\npath: {}\nquery: {:?}", method, path, q)
        // });


    let routes = warp::any()
        .and(health_route)
        .or(auth_route)
        .or(auth_step)
        .or(proxy_route);

    println!("Start server\n");
    warp::serve(routes)
        .run(([0, 0, 0, 0], 3030))
        .await;

}
