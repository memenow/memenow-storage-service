// Import the necessary modules from the warp crate.
use warp::Filter;
// Import the `handle_upload` function from the `domain::services` module.
use crate::domain::services::handle_upload;

/// The `upload_routes` function returns a warp filter that represents the routes for file uploading.
/// This function is used to define the route handler for the "/upload" path.
///
/// # Returns
///
/// This function returns an implementation of `Filter` that extracts a type implementing `warp::Reply`.
/// This means that the filter can be used to handle incoming POST requests to the "/upload" path and produce responses.
///
/// The filter expects multipart form data with a maximum length of 5,000,000 bytes.
/// The form data is then passed to the `handle_upload` function for processing.
///
/// # Errors
///
/// This function will return a `warp::Rejection` if the `handle_upload` function fails to process the form data.
pub fn upload_routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("upload") // Define the path for the route.
        .and(warp::post()) // Specify that this route is for POST requests.
        .and(warp::multipart::form().max_length(5_000_000)) // Expect multipart form data with a maximum length of 5,000,000 bytes.
        .and_then(handle_upload) // Pass the form data to the `handle_upload` function for processing.
}
