// Import the necessary modules for the application.
// The `api` module contains the API routes for the application.
mod api;
// The `domain` module contains the domain logic for the application.
mod domain;
// The `infrastructure` module contains the infrastructure code for the application.
mod infrastructure;
// The `utils` module contains utility functions and modules that are used throughout the application.
mod utils;

/// The `main` function is the entry point of the application.
///
/// It initializes the environment variables, sets up the API routes, and starts the server.
/// The server listens on the IP address `0.0.0.0` and the port `8080`.
///
/// The `main` function is asynchronous because it uses the `tokio` runtime to handle asynchronous tasks.
///
/// # Environment
///
/// This function loads the environment variables from the `.env` file using the `dotenv` crate.
///
/// # API Routes
///
/// This function sets up the API routes for file uploading using the `api::upload::upload_routes` function.
///
/// # Server
///
/// This function starts the server with the specified IP address and port using the `warp::serve` function.
/// The server uses the API routes for handling requests.
///
/// # Asynchronous
///
/// This function is asynchronous because it uses the `tokio` runtime to handle asynchronous tasks.
/// It is annotated with the `#[tokio::main]` attribute to indicate that it is the entry point of the `tokio` runtime.
#[tokio::main]
async fn main() {
    // Load the environment variables from the `.env` file.
    dotenv::dotenv().ok();
    // Set up the API routes for file uploading.
    let routes = api::upload::upload_routes();
    // Start the server with the specified IP address and port, and use the API routes for handling requests.
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}
