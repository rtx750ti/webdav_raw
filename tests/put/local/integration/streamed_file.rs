#[path = "../../../common/local_http.rs"]
mod local_http;

use webdav_core::auth::WebdavAuth;
use webdav_core::put::put_body::{PutBody, file_handle::FileHandle};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::support::{remove_temp_file, write_temp_file};

/// 文件源以流式发送：服务端收到的字节与源文件一致，长度确定，不退化为 chunked。
#[tokio::test]
async fn file_put_streams_exact_bytes_without_chunked() {
    let content = vec![0xAB_u8; 100 * 1024];
    let source = write_temp_file("streamed", &content).await;

    let server = local_http::start_server().await;
    Mock::given(method("PUT"))
        .and(path("/dav/streamed.bin"))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;

    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");
    let file = tokio::fs::File::open(&source)
        .await
        .expect("临时文件必须可打开");

    let response = auth
        .put()
        .relative_path("streamed.bin")
        .expect("相对路径必须有效")
        .body(PutBody::from_file(FileHandle::new(file, None)))
        .send()
        .await
        .expect("文件源 PUT 应发送成功");
    assert_eq!(response.status(), 201);

    let received = server
        .received_requests()
        .await
        .expect("MockServer 默认记录收到的请求");
    assert_eq!(received.len(), 1, "应当只收到一次 PUT");
    let request = &received[0];

    assert_eq!(
        request.body, content,
        "服务端收到的字节必须与源文件逐字节一致"
    );
    assert_eq!(
        request
            .headers
            .get("content-length")
            .expect("流式请求必须带确定的 Content-Length")
            .to_str()
            .expect("Content-Length 必须是可见文本"),
        content.len().to_string()
    );
    assert_eq!(
        request.headers.get("content-type").unwrap(),
        "application/octet-stream",
        "文件源没有内容类型信息，应回落为默认值"
    );

    remove_temp_file(&source).await;
}

/// 空文件同样是合法的流式来源，长度为 0。
#[tokio::test]
async fn empty_file_put_sends_zero_length() {
    let source = write_temp_file("empty_stream", b"").await;

    let server = local_http::start_server().await;
    Mock::given(method("PUT"))
        .and(path("/dav/empty.bin"))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;

    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");
    let file = tokio::fs::File::open(&source)
        .await
        .expect("临时文件必须可打开");

    let response = auth
        .put()
        .relative_path("empty.bin")
        .expect("相对路径必须有效")
        .body(PutBody::from_file(FileHandle::new(file, None)))
        .send()
        .await
        .expect("空文件 PUT 应发送成功");
    assert_eq!(response.status(), 201);

    let received = server
        .received_requests()
        .await
        .expect("MockServer 默认记录收到的请求");
    assert!(received[0].body.is_empty());
    assert_eq!(
        received[0].headers.get("content-length").unwrap(),
        "0",
        "空文件仍应带确定的 Content-Length"
    );

    remove_temp_file(&source).await;
}

/// 单字节文件不会退化成 chunked。
#[tokio::test]
async fn single_byte_file_put_has_no_transfer_encoding() {
    let source = write_temp_file("single_byte_stream", b"Z").await;

    let server = local_http::start_server().await;
    Mock::given(method("PUT"))
        .and(path("/dav/one.bin"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");
    let file = tokio::fs::File::open(&source)
        .await
        .expect("临时文件必须可打开");

    auth.put()
        .relative_path("one.bin")
        .expect("相对路径必须有效")
        .body(PutBody::from_file(FileHandle::new(file, None)))
        .send()
        .await
        .expect("单字节文件 PUT 应发送成功");

    let received = server
        .received_requests()
        .await
        .expect("MockServer 默认记录收到的请求");
    assert_eq!(received[0].body, b"Z");
    assert!(
        received[0].headers.get("transfer-encoding").is_none(),
        "显式 Content-Length 下不应出现 transfer-encoding"
    );

    remove_temp_file(&source).await;
}
