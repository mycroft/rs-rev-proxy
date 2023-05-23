use axum::{
    body::{Body, StreamBody, boxed},
    extract::{Path, State},
    Extension,
    http::Request,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    Router,
    routing::get,
    ServiceExt,
};
use dotenv::dotenv;
use log::{debug, error, info};
use rusoto_core::Region;
use rusoto_s3::{GetObjectRequest, S3Client, S3};
use std::{
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::RwLock;
use tower::Layer;

mod stream;
use stream::PouetStream;
mod config;
use config::{
    AppConfig,
    load_configuration,
    spawn_config_reloader
};

type Cache = Arc<RwLock<PathBuf>>;
type SharedAppConfig = Arc<RwLock<AppConfig>>;

async fn rewrite_request_uri(
    State(config): State<SharedAppConfig>,
    req: Request<Body>,
    next: Next<Body>,
) -> impl IntoResponse {
    let config = config.read().await;
    debug!("rewrite_request_uri: {:?} with config {:?}", req.uri(), config.redirect);
    let mut current_path = req.uri().to_string();

    // Look into config.redirect if we have a matching path
    for rules in config.redirect.iter() {
        for (from, to) in rules {
            if *from == current_path {
                current_path = to.clone();
            }
        }
    }
    
    debug!("Path: {} -> {}", req.uri(), current_path);

    let req = Request::builder()
        .method(req.method().clone())
        .uri(current_path)
        .body(req.into_body())
        .unwrap();

    next.run(req).await
}

async fn cache_middleware(
    req: Request<Body>,
    next: Next<Body>
) -> Response {
    debug!("cache_middleware: {}", req.uri().path());

    next.run(req).await
}

async fn handle_proxy(
    Path(key): Path<String>,
    Extension(cache): Extension<Cache>,
    Extension(client): Extension<S3Client>,
    req: Request<Body>,
) -> Response {
    let bucket_name = "hey".to_string();
    let object_key = format!("service_objects/{}", key);

    let cache = cache.read().await;

    debug!("cache is located in {:?}", cache);

    if let Some(object_key) = req.uri().path().rsplit('/').next() {
        debug!("object_key is {}", object_key);
        debug!("Path is {:?}", cache);
        let file_path = cache.join(object_key);
        if file_path.exists() {
            debug!("Path in cache exists...");

            let file = tokio::fs::File::open(file_path).await.unwrap(); // TODO(unwrap)
            let buf_reader = tokio::io::BufReader::new(file);
            let stream = tokio_util::io::ReaderStream::new(buf_reader);
            return Response::builder()
                .body(boxed(StreamBody::new(stream)))
                .unwrap();
        }
    }

    info!("Looking at s3://{}/{}", bucket_name, object_key);

    let request = GetObjectRequest {
        bucket: bucket_name,
        key: object_key,
        ..Default::default()
    };

    let s3_response = match client.get_object(request).await {
        Ok(resp) => resp,
        Err(err) => {
            error!("err: {}", err.to_string());

            return Response::builder()
                .body(boxed("Error while getting file.".to_string()))
                .unwrap();
        }
    };

    // compute expected cache file
    let file_path = cache.join(key);

    let s3_stream = s3_response.body.unwrap();
    let p = PouetStream::new(s3_stream, file_path.to_str().unwrap().to_string());

    Response::builder()
        .body(boxed(StreamBody::new(p)))
        .unwrap()
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();

    let config = Arc::new(RwLock::new(load_configuration().expect("Failed to load configuration")));
    spawn_config_reloader(Arc::clone(&config));

    // by default, set cache in /tmp/cache
    let mut cache_dir = std::env::temp_dir().join("cache");
    if config.read().await.cache.contains_key("location") {
        cache_dir = PathBuf::from(config.read().await.cache.get("location").unwrap());
    }

    std::fs::create_dir_all(&cache_dir).unwrap();
    let cache: Cache = Arc::new(RwLock::new(cache_dir));

    let mut region = Region::default();
    if config.read().await.aws.contains_key("use_custom_region") {
        region = Region::Custom {
            name: "".to_string(),
            endpoint: config.read().await.aws.get("endpoint").unwrap().clone(),
        }
    }
    
    let s3_client = S3Client::new(region);

    let app = Router::new()
        .route("/:key", get(handle_proxy))
        .layer(middleware::from_fn(cache_middleware))
        .layer(Extension(s3_client))
        .layer(Extension(cache.clone()))
        .layer(Extension(config.clone()));

    let middleware = axum::middleware::from_fn_with_state(config.clone(), rewrite_request_uri);
    let app_with_middleware = middleware.layer(app);

    let addr = "0.0.0.0:8080";
    info!("Server listening on {}", addr);
    axum::Server::bind(&addr.parse().unwrap())
        .serve(app_with_middleware.into_make_service())
        .await
        .unwrap();
}
