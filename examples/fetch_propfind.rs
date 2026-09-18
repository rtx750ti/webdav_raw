use base64::engine::Engine;
use base64::engine::general_purpose::STANDARD;

use reqwest::header::AUTHORIZATION;
use url::Url;
use wiremock::http::{HeaderMap, HeaderValue}; // 需要在 Cargo.toml 中添加 base64 = "0.21" // 引入 trait 才能使用 encode

use webdav_raw::WebdavAuth;
use webdav_raw::{Depth, PropFindBuilder};
use webdav_raw::{FindProp, PropFindSelector};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let username = "alice";
    let password = "secret";
    let base_url_str = "https://example.com/webdav";

    let auth = WebdavAuth::new(username, password, base_url_str)?;

    // 第一种办法
    let response1 = auth
        .propfind()
        .path("Documents/")
        .depth(Depth::One)
        .allprop()
        .send()
        .await?;
    println!("[1] 状态: {}", response1.status());
    let body1 = response1.text().await?;
    println!("[1] 响应体:\n{}\n", body1);

    // 第二种办法
    let builder2 = auth
        .propfind()
        .path("Documents/")
        .depth(Depth::One)
        .allprop();

    let request2 = builder2.build()?;
    let client2 = auth.get_client().clone();
    let response2 = client2.execute(request2).await?;
    println!("[2] 状态: {}", response2.status());
    let body2 = response2.text().await?;
    println!("[2] 响应体:\n{}\n", body2);

    // 第三种办法
    let credentials = format!("{}:{}", username, password);
    let auth_header_value = format!("Basic {}", STANDARD.encode(credentials.as_bytes()));
    let mut default_headers = HeaderMap::new();
    default_headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_header_value)?);

    let client = reqwest::Client::builder()
        .default_headers(default_headers)
        .build()?;

    let base_url = Url::parse("https://example.com/webdav/")?;

    let props = vec![
        FindProp::Resourcetype,
        FindProp::Getetag,
        FindProp::Getcontenttype,
    ];
    let builder3 = PropFindBuilder::new(client.clone(), base_url)
        .path("Documents/")
        .depth(Depth::Zero)
        .selector(PropFindSelector::Props(props.into_iter().collect()));

    let request3 = builder3.build()?;
    let response3 = client.execute(request3).await?;
    println!("[3] 状态: {}", response3.status());
    let body3 = response3.text().await?;
    println!("[3] 响应体:\n{}", body3);

    Ok(())
}
