use std::{
    hash::{Hash, Hasher},
    num::NonZeroUsize,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    body::Bytes,
    extract::Path,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::get,
    Extension, Router,
};

use anyhow::Result;
use lru::LruCache;
use pb::{filter, resize, ImageSpec, Spec};
use percent_encoding::{percent_decode_str, percent_encode, NON_ALPHANUMERIC};
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use tower::{limit::ConcurrencyLimitLayer, timeout::TimeoutLayer, ServiceBuilder};
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

mod engine;
mod pb;
use engine::{Engine, Photon};
use image::ImageFormat;

type Cache = Arc<Mutex<LruCache<u64, Bytes>>>;

#[derive(Deserialize)]
struct Params {
    spec: String,
    url: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cache: Cache = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())));

    // let middleware_stack = ServiceBuilder::new()
    //     .layer(TraceLayer::new_for_http())
    //     .layer(CompressionLayer::new())
    //     .layer(TimeoutLayer::new(Duration::from_secs(30)))
    //     .layer(ConcurrencyLimitLayer::new(100))
    //     .into_inner();
    let app = Router::new()
        .route("/image/:spec/:url", get(generate))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(ConcurrencyLimitLayer::new(100))
                .layer(CompressionLayer::new())
                // .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(Extension(cache)),
        );
    // .layer(TimeoutLayer::new(Duration::from_secs(30)));
    let addr = "0.0.0.0:3010";
    print_test_url("https://images.pexels.com/photos/1562477/pexels-photo-1562477.jpeg?auto=compress&cs=tinysrgb&dpr=3&h=750&w=1260");
    info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn generate(
    Path(Params { spec, url }): Path<Params>,
    // Extension(cache): Extension<Cache>,
) -> Result<(HeaderMap, Vec<u8>), StatusCode> {
    let spec: ImageSpec = spec
        .as_str()
        .try_into()
        .map_err(|_| StatusCode::BAD_REQUEST)
        .unwrap();

    let url: &str = &percent_decode_str(&url).decode_utf8_lossy();
    let data = retrieve_image(url)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)
        .unwrap();
    // 使用 image engine 处理
    let mut engine: Photon = data
        .try_into()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .unwrap();
    engine.apply(&spec.specs);
    let image: Vec<u8> = engine.generate(ImageFormat::Png);

    // info!("Finished processing: image size {}", image.len());
    let mut headers = HeaderMap::new();

    headers.insert("content-type", HeaderValue::from_static("image/png"));

    // (StatusCode::OK, headers, image).into_response()

    Ok((headers, image.to_vec()))
}

async fn retrieve_image(url: &str) -> Result<Bytes, reqwest::Error> {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    // let key = hasher.finish();

    // let g = &mut cache.lock().unwrap();
    // let data = match g.get(&key) {
    //     Some(v) => {
    //         info!("Match cache {}", key);
    //         v.to_owned()
    //     }
    //     None => {
    //         info!("Retrieve image {} from {}", key, url);
    //         let data = reqwest::get(url).await?.bytes().await?;
    //         g.put(key, data.clone());
    //         data
    //     }
    // };
    info!("Retrieve url: {}", url);
    let data = reqwest::get(url).await?.bytes().await?;
    Ok(data)
}

fn print_test_url(url: &str) {
    use std::borrow::Borrow;
    let spec1 = Spec::new_resize(200, 400, resize::SampleFilter::CatmullRom);
    let spec2 = Spec::new_watermark(20, 20);
    let spec3 = Spec::new_filter(filter::Filter::Marine);
    let image_spec = ImageSpec::new(vec![spec1, spec2, spec3]);
    // let image_spec = ImageSpec::new(vec![]);
    let s: String = image_spec.borrow().into();
    let test_image = percent_encode(url.as_bytes(), NON_ALPHANUMERIC).to_string();
    println!("test url: http://0.0.0.0:3010/image/{}/{}", s, test_image);
}
