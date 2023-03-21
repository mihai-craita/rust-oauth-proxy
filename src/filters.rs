use warp::Filter;
use oauth2::basic::BasicClient;
use oauth2::Scope;
use warp::{self, hyper::Body, Rejection,Reply};
use warp::http;
use crate::cache::Cache;
use futures_util::future;
use tokio::runtime::Runtime;

pub async fn log_response(response: http::Response<Body>) -> Result<impl Reply, Rejection> {
    println!("{:?}", response);
    Ok(response)
}

pub fn with_auth_client(auth_client: BasicClient) -> impl Filter<Extract = (BasicClient,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || auth_client.clone())
}

pub fn with_scopes(scopes: String) -> impl Filter<Extract = (Vec<Scope>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || scopes
                    .split(" ")
                    .map(|s| Scope::new(s.to_string()))
                    .collect()
                   )
}

pub fn with_cache(cache: Cache) -> impl Filter<Extract = (Cache,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || cache.clone())
}

pub async fn handle_rejection(err: warp::Rejection) -> Result<warp::http::Response<String>, std::convert::Infallible> {
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

pub fn handle_auth_cookie(cache: Cache) -> impl Filter<Extract = (bool,), Error = Rejection> + Clone 
{
    warp::filters::cookie::cookie::<String>("proxy_token")
        .and(with_cache(cache.clone()))
        .and_then(move |cookie_value: String, cache: Cache| {
            let found = get_value_from_cache(cache, cookie_value);
            let rt = Runtime::new().unwrap();
            let v = match rt.block_on(found) {
                Some(_) => true,
                None => false
            };
            if v == false {
                future::ready(Err(warp::reject()))
            } else {
                future::ready(Ok::<bool, Rejection>(v))
            }
        })
}

async fn get_value_from_cache(
    arc_data: Cache,
    key: String,
    ) -> Option<String> {
    let lock = arc_data.lock().await; // Acquire the lock asynchronously
    lock.get(&key).cloned() // Check if the key exists and return a cloned value if it does
}

#[derive(Debug)]
struct InvalidCookie;

impl warp::reject::Reject for InvalidCookie {}

// pub fn redirect_if_cookie_missing()
//     -> Result<impl Reply, Rejection> {
//     warp::cookie::optional("cookie-name")
//         .and_then(|cookie: Option<String>| async move // { Ok(()) }
//              {
//             if cookie.is_none() {
                
//                 // Redirect to the desired link
//                 let redirect_url = "https://example.com";
//                 let uri = Uri::from_static(redirect_url);
//                 let response = redirect(uri);
//                 Ok(response)
//             } else {
//                 // let response = warp::reply::with_status(warp::reply(), warp::http::StatusCode::OK);
//                 // let response = response.into_response();
//                 Err(warp::reject::custom(InvalidCookie))
//             }
//         }
//         ).boxed()
// }
