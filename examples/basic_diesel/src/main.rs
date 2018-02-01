extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;
extern crate gotham_middleware_diesel;
extern crate diesel;
extern crate r2d2_diesel;
extern crate r2d2;
extern crate basic_diesel;

use hyper::{Response, StatusCode, Method};

use gotham::state::State;
use gotham::router::Router;
use gotham::pipeline::new_pipeline;
use gotham::router::tree::TreeBuilder;
use gotham::router::route::{RouteImpl, Extractors, Delegation};
use gotham::router::route::matcher::MethodOnlyRouteMatcher;
use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set, DispatcherImpl};
use gotham::router::request::path::NoopPathExtractor;
use gotham::router::request::query_string::NoopQueryStringExtractor;
use gotham::router::response::finalizer::ResponseFinalizerBuilder;
use gotham_middleware_diesel::DieselMiddleware;
use diesel::sqlite::SqliteConnection;
use r2d2_diesel::ConnectionManager;
use r2d2::{Pool, PooledConnection};


static DATABASE_URL: &'static str = ".posts.db";

fn handler(state: State) -> (State, Response) {
    let conn: PooledConnection<ConnectionManager<SqliteConnection>> = gotham_middleware_diesel::state_data::connection(&state);
    let posts = basic_diesel::get_posts(&conn);

    (
        state,
        Response::new()
            .with_status(StatusCode::Ok)
            .with_body(format!("{:?}", posts)),
    )
}



/// Start a server and use a `Router` to dispatch requests
pub fn main() {
    let manager = ConnectionManager::new(DATABASE_URL);
    let pool = Pool::<ConnectionManager<SqliteConnection>>::new(manager).unwrap();
    let middleware = DieselMiddleware::with_pool(pool.clone());
    let addr = "127.0.0.1:7878";
    let editable_pipeline_set = new_pipeline_set();
    let (editable_pipeline_set, pipeline) = editable_pipeline_set.add(new_pipeline().add(middleware).build());
    let pipeline_set = finalize_pipeline_set(editable_pipeline_set);
    let mut tree_builder = TreeBuilder::new();
    let matcher = MethodOnlyRouteMatcher::new(vec![Method::Get]);
    let dispatcher = Box::new(DispatcherImpl::new(|| Ok(handler), (pipeline, ()), pipeline_set));
    let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
    let route = RouteImpl::new(matcher, dispatcher, extractors, Delegation::Internal);
    tree_builder.add_route(Box::new(route));
    let tree = tree_builder.finalize();
    let response_finalizer = ResponseFinalizerBuilder::new().finalize();
    let router = Router::new(tree, response_finalizer);

    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router);
}