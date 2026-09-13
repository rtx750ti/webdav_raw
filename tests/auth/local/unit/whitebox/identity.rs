use webdav_core::WebdavAuth;

/// 验证完整身份比较同时覆盖根地址和认证凭据。
#[test]
fn equal_credentials_and_base_url_are_equal() {
    let first = WebdavAuth::new("alice", "password", "https://example.com/dav")
        .expect("认证对象应创建成功");
    let same = WebdavAuth::new("alice", "password", "https://example.com/dav/")
        .expect("认证对象应创建成功");
    let another_root = WebdavAuth::new("alice", "password", "https://example.com/other")
        .expect("认证对象应创建成功");
    let another_password =
        WebdavAuth::new("alice", "another", "https://example.com/dav").expect("认证对象应创建成功");

    assert_eq!(first, same);
    assert_ne!(first, another_root);
    assert_ne!(first, another_password);
    assert!(first.eq_only_token(&another_root));
    assert!(!first.eq_only_token(&another_password));
}

/// 验证 Debug 输出不会泄漏可使用的认证信息。
#[test]
fn debug_output_masks_credentials_and_authorization() {
    let auth = WebdavAuth::new("alice", "super-secret-password", "https://example.com/dav")
        .expect("认证对象应创建成功");
    let debug_text = format!("{auth:?}");

    assert!(!debug_text.contains("alice"));
    assert!(!debug_text.contains("super-secret-password"));
    assert!(!debug_text.contains("Basic"));
}
