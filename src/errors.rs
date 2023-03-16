use warp::reject::Reject;

#[derive(Debug)]
pub struct ResponseBuildError;

impl Reject for ResponseBuildError {}
