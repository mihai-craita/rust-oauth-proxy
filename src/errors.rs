use warp::reject::Reject;

#[derive(Debug)]
pub struct ResponseBuildError;

impl Reject for ResponseBuildError {}

#[derive(Debug)]
pub struct CookieIsMissing;

impl Reject for CookieIsMissing {}

#[derive(Debug)]
pub struct UnauthenticatedUser {
    pub path: String,
}

impl Reject for UnauthenticatedUser {}
