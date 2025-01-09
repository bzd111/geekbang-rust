use axum::extract::Path;
use tokio::fs::File;

use axum::response::IntoResponse;
// use axum::{body::Bytes, extract::Extension, response::IntoResponse, routing::get, Router};
// use anyhow::Result;
use axum::{body::Bytes, extract::Extension, routing::get, Router};
use lru::LruCache;
use reqwest::StatusCode;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncReadExt;

// use bytes::Bytes;

type Cache = Arc<Mutex<LruCache<u64, Bytes>>>;

#[tokio::main]
async fn main() {
    // 创建一个容量为100的LRU缓存
    let cache: Cache = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(2).unwrap())));

    // 创建路由器并注册路由
    let app = Router::new()
        .route("/image/:spec/:url", get(handler))
        .layer(Extension(cache));

    // 启动服务器
    let addr = "0.0.0.0:3010";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handler(
    Path((_spec, _url)): Path<(String, String)>,
    Extension(cache): Extension<Cache>,
) -> impl IntoResponse {
    // 从缓存中获取值,如果不存在则插入新值
    let value = {
        let mut cache = cache.lock().unwrap();
        cache.get(&42).cloned().unwrap_or_else(|| {
            let new_value = Bytes::from("Hello, Axum with LRU Cache!");
            cache.put(42, new_value.clone());
            new_value
        })
    };
    // // 读取图片文件
    // let image_data = include_bytes!("path/to/your/image.png");
    //
    // // 创建响应
    // let mut response = Response::new(Vec::new());
    // response
    //     .headers_mut()
    //     .insert("content-type", HeaderValue::from_static("image/png"));
    // *response.status_mut() = StatusCode::OK;
    //
    // // 将图片数据写入响应体
    // response.body_mut().write_all(image_data).unwrap();
    //
    // response

    // 从文件中读取图片
    let mut file = match File::open("/cnsp/test/rust/geekbang/thumbor/rust-logo.png").await {
        Ok(file) => file,
        Err(_) => return (StatusCode::NOT_FOUND, "Image not found").into_response(),
    };

    // 获取文件大小
    let metadata = file
        .metadata()
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read file metadata",
            )
                .into_response()
        })
        .unwrap();
    let file_size = metadata.len() as usize;

    // 创建一个足够大的 buffer
    let mut contents = vec![0; file_size];

    // 读取整个文件
    match file.read_exact(&mut contents).await {
        Ok(_) => (),
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read image").into_response()
        }
    }

    // 设置正确的 Content-Type
    let headers = [(axum::http::header::CONTENT_TYPE, "image/png")];

    // let image_data = include_bytes!("/cnsp/test/rust/geekbang/thumbor/rust-logo.png");

    // (StatusCode::OK, headers, image_data).into_response()
    (StatusCode::OK, headers, contents).into_response()
}
