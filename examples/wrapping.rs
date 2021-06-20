#![deny(warnings)]
use std::{collections::HashMap, convert::Infallible, str::FromStr, sync::Arc};
use tokio::sync::RwLock;
use warp::{Filter, Rejection};

// convenience method that puts the store down into the chain
fn with_store(
    store: Arc<RwLock<HashMap<String, u64>>>,
) -> impl Filter<Extract = (Arc<RwLock<HashMap<String, u64>>>,), Error = Infallible> + Clone {
    warp::any().map(move || store.clone())
}

// async because getting the lock on the hashmap, simple hashmap incrementer
async fn inc_path<T>(path: T, store: Arc<RwLock<HashMap<String, u64>>>) -> Result<(), Rejection>
where
    T: Send + Sync + ToString,
{
    let mut s = store.write().await;
    let c = s.entry(path.to_string()).or_insert(0);
    *c += 1;
    println!("Called path {:?} {} times.", path.to_string(), c);
    Ok(())
}

// the wrapping function that takes F (along with the store) and returns F, applying inc_path
fn inc_by_path_wrapper<F, T>(
    filter: F,
    store: Arc<RwLock<HashMap<String, u64>>>,
) -> impl Filter<Extract = (T,)> + Clone + Send + Sync + 'static
where
    F: Filter<Extract = (T,), Error = Infallible> + Clone + Send + Sync + 'static,
    F::Extract: warp::Reply,
    // FromStr / ToString corresponds to the return type of map and recover in routes
    T: FromStr + Send + Sync + 'static + ToString,
{
    println!("Called inc_by_path_wrapper during init.");
    warp::path::param()
        .and(with_store(store))
        .and_then(inc_path)
        // because inc_path returns () on success we need to dump it using untuple
        .untuple_one()
        .and(filter)
}

#[tokio::main]
async fn main() {
    let h = Arc::new(RwLock::new(HashMap::new()));
    // Match any request and return hello world!
    let routes = warp::any()
        .map(|| "hello world\n".to_string())
        .boxed()
        .recover(|_err| async { Ok("recovered".to_string()) })
        // unify is needed because we're actually passing routes into and back out, which creates Either
        .unify()
        // we capture the store into the closure using move
        .with(warp::wrap_fn(move |f| inc_by_path_wrapper(f, h.clone())));

    println!("Use curl localhost:3030/path_name to demo.");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
