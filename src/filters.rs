use warp::Filter;
use oauth2::basic::BasicClient;
use oauth2::Scope;
use warp::path::FullPath;
use warp::{self, hyper::Body, Rejection,Reply};
use warp::http;
use crate::{errors, cache::Cache, cookie};

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
    } else if let Some(_) = err.find::<errors::CookieIsMissing>() {
        eprintln!("Missing a cookie");
        let res = res.status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .body("A cookie is missing from the request".to_string())
            .unwrap();

        Ok(res)

    } else if let Some(e) = err.find::<errors::UnauthenticatedUser>() {
        eprintln!("User is not authenticated on path {}", e.path);
        if e.path.eq("/favicon.ico") {
            return Ok(res.status(http::StatusCode::NOT_FOUND).body("".to_string()).unwrap());
        };
        let cookie = cookie::Cookie::new("redirect_url", e.path.clone());
        // cookie is missing so we redirect the user for login
        let res = res.status(http::StatusCode::TEMPORARY_REDIRECT)
            .header("Location", "/oauth/auth")
            .header(http::header::SET_COOKIE, cookie.to_string())
            .body("".to_string())
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

pub fn handle_auth_cookie(cache: Cache) -> impl Filter<Extract = ((),), Error = Rejection> + Clone 
{
    warp::filters::cookie::optional::<String>("proxy_token")
        .and(with_cache(cache.clone()))
        .and(warp::path::full())
        .and_then(|cookie_value: Option<String>, cache: Cache, full_path: FullPath| async move {
            let found = get_value_from_cache(cache, cookie_value);
            match found.await {
                Some(_) => Ok::<(), Rejection>(()),
                None => Err(warp::reject::custom(errors::UnauthenticatedUser{path: full_path.as_str().to_string()}))
            }
        })
}

async fn get_value_from_cache(arc_data: Cache, key: Option<String>) -> Option<String> {
    let lock = arc_data.lock().await; // Acquire the lock asynchronously
    match key {
        Some(k) => lock.get(&k).cloned(),
        None => None
    }
}
