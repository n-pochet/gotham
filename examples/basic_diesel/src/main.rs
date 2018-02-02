extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;
extern crate gotham_middleware_diesel;
extern crate diesel;
extern crate r2d2_diesel;
extern crate r2d2;
extern crate basic_diesel;

use hyper::{Response, StatusCode};

use gotham::state::State;
use gotham::router::Router;
use gotham::pipeline::new_pipeline;
use gotham::router::builder::*;
use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set};
use gotham_middleware_diesel::DieselMiddleware;
use diesel::sqlite::SqliteConnection;
use r2d2_diesel::ConnectionManager;
use r2d2::{Pool, PooledConnection};

// The URL of the database.
static DATABASE_URL: &'static str = ".posts.db";

/// Handler function. Responsible of getting and displaying the posts from the DB
fn handler(state: State) -> (State, Response) {
    let conn: PooledConnection<ConnectionManager<SqliteConnection>> =
        gotham_middleware_diesel::state_data::connection(&state);
    let posts = basic_diesel::get_posts(&conn);

    (
        state,
        Response::new().with_status(StatusCode::Ok).with_body(
            format!(
                "{:?}",
                posts
            ),
        ),
    )
}

/// Create a `Router`
///
/// The resulting tree looks like:
///
/// /                         --> GET
///
/// It returns the content of the SQLite DB file located in `.posts.db`
/// This DB consists of `Post` entries.
fn router() -> Router {
    let manager = ConnectionManager::new(DATABASE_URL);
    let pool = Pool::<ConnectionManager<SqliteConnection>>::new(manager).unwrap();
    // Create the `DieselMiddleware`
    let middleware = DieselMiddleware::with_pool(pool);


    // Create a new pipeline set
    let editable_pipeline_set = new_pipeline_set();

    // Add the middleware to a new pipeline
    let (editable_pipeline_set, pipeline) =
        editable_pipeline_set.add(new_pipeline().add(middleware).build());
    let pipeline_set = finalize_pipeline_set(editable_pipeline_set);

    let default_pipeline_chain = (pipeline, ());

    build_router(default_pipeline_chain, pipeline_set, |route| {
        route.get("/").to(handler);
    })
}



/// Start a server and use a `Router` to dispatch requests
fn main() {
    let addr = "127.0.0.1:7878";

    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router());
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::test::TestServer;
    use hyper::StatusCode;
    use std::str;

    #[test]
    fn index_get() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        let str_body = str::from_utf8(&body).unwrap();
        let index = "[Post { \
        id: Some(1), \
        title: \"test\", \
        body: \"this a test post\", \
        published: true }, \
        Post { \
        id: Some(2), \
        title: \"another\", \
        body: \"another post\", \
        published: true }]";
        assert_eq!(str_body, index);
    }
}
