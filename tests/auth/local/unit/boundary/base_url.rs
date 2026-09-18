use webdav_raw::{WebdavAuth, WebdavAuthError};

/// 验证空根地址会返回明确错误。
#[test]
fn empty_base_url_returns_empty_base_url_error() {
    let result = WebdavAuth::new("alice", "password", "  ");

    assert!(matches!(result, Err(WebdavAuthError::EmptyBaseUrl)));
}

/// 验证无法解析的根地址会返回格式错误。
#[test]
fn malformed_base_url_returns_invalid_base_url_error() {
    let result = WebdavAuth::new("alice", "password", "not a url");

    assert!(matches!(result, Err(WebdavAuthError::InvalidBaseUrl(_))));
}

/// 验证缺少主机名的 HTTP 地址会被拒绝。
#[test]
fn hostless_base_url_returns_missing_host_error() {
    let result = WebdavAuth::new("alice", "password", "http:/webdav");

    assert!(matches!(result, Err(WebdavAuthError::MissingHost)));
}

/// 验证非 HTTP 协议会被拒绝。
#[test]
fn unsupported_scheme_returns_unsupported_scheme_error() {
    let result = WebdavAuth::new("alice", "password", "ftp://example.com/dav");

    assert!(matches!(result, Err(WebdavAuthError::UnsupportedScheme(_))));
}

/// 验证根地址中的查询参数会被拒绝。
#[test]
fn query_in_base_url_returns_query_error() {
    let result = WebdavAuth::new("alice", "password", "https://example.com/dav?token=value");

    assert!(matches!(result, Err(WebdavAuthError::BaseUrlContainsQuery)));
}

/// 验证根地址中的片段会被拒绝。
#[test]
fn fragment_in_base_url_returns_fragment_error() {
    let result = WebdavAuth::new("alice", "password", "https://example.com/dav#section");

    assert!(matches!(
        result,
        Err(WebdavAuthError::BaseUrlContainsFragment)
    ));
}

/// 验证根地址会补齐末尾斜杠并保持已有斜杠。
#[test]
fn base_url_normalizes_trailing_slash() {
    let normalized = WebdavAuth::new("alice", "password", "https://example.com/dav")
        .expect("合法根地址应创建认证对象");
    let preserved = WebdavAuth::new("alice", "password", "https://example.com/dav/")
        .expect("合法根地址应创建认证对象");

    assert_eq!(
        normalized.get_base_url().as_str(),
        "https://example.com/dav/"
    );
    assert_eq!(
        preserved.get_base_url().as_str(),
        "https://example.com/dav/"
    );
}
