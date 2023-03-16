use warp::Filter;
use oauth2::basic::BasicClient;
use oauth2::Scope;
use warp::{self, hyper::Body, Rejection, Reply, http::Response as HttpResponse};

pub async fn log_response(response: HttpResponse<Body>) -> Result<impl Reply, Rejection> {
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
