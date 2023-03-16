use warp::Filter;
use oauth2::basic::BasicClient;
use warp::{self, hyper::Body, Rejection, Reply, http::Response as HttpResponse};

pub async fn log_response(response: HttpResponse<Body>) -> Result<impl Reply, Rejection> {
    println!("{:?}", response);
    Ok(response)
}

pub fn with_auth_client(auth_client: BasicClient) -> impl Filter<Extract = (BasicClient,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || auth_client.clone())
}
