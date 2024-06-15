/// The `api` module contains the routes for the API.
pub mod upload;

use warp::Filter;

/// The `api_routes` function returns a warp filter that represents all the routes for the API.
/// This function is used to combine all the different route handlers into a single filter that can be served by warp.
///
/// # Returns
///
/// This function returns an implementation of `Filter` that extracts a type implementing `warp::Reply`.
/// This means that the filter can be used to handle incoming requests and produce responses.
///
/// # Errors
///
/// This function will return a `warp::Rejection` if any of the route handlers fail to process a request.
pub fn api_routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // The `upload_routes` function from the `upload` module is called to get the routes for file uploading.
    upload::upload_routes()
}
