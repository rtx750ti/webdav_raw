//! 内容类型推断与校验的独立路径。
//!
//! `U8Metadata::from_name` 提供默认推断：交给 `mime_guess`，本库不维护扩展名表。
//! 推断结果永远是合法 MIME，因此该入口只在文件名为空时报错。
//!
//! 字段公开后本库不再在写入时校验，`validate` 供调用方自行检查。

use webdav_core::put::put_body::u8_bytes::U8Metadata;

/// 命中推断表：常见扩展名映射到对应内容类型。
#[test]
fn known_extensions_map_to_their_content_type() {
    let cases = [
        ("report.txt", "text/plain"),
        ("data.csv", "text/csv"),
        ("data.json", "application/json"),
        ("page.html", "text/html"),
        ("doc.pdf", "application/pdf"),
        ("photo.png", "image/png"),
        ("photo.jpg", "image/jpeg"),
        ("archive.zip", "application/zip"),
    ];

    for (name, expected) in cases {
        let metadata = U8Metadata::from_name(name.to_owned()).expect("推断必须成功");

        assert_eq!(
            metadata.content_type, expected,
            "文件名 {name} 的推断结果不符"
        );
    }
}

/// 扩展名匹配不区分大小写。
#[test]
fn extension_lookup_ignores_case() {
    let upper = U8Metadata::from_name("PHOTO.PNG".to_owned()).expect("推断必须成功");
    let mixed = U8Metadata::from_name("Report.TxT".to_owned()).expect("推断必须成功");

    assert_eq!(upper.content_type, "image/png");
    assert_eq!(mixed.content_type, "text/plain");
}

/// 扩展名无法识别时回落为默认内容类型。
#[test]
fn unknown_extension_falls_back_to_default() {
    let metadata = U8Metadata::from_name("installer.exe".to_owned()).expect("推断必须成功");

    assert_eq!(metadata.content_type, "application/octet-stream");
}

/// 没有扩展名时回落为默认内容类型。
#[test]
fn missing_extension_falls_back_to_default() {
    let metadata = U8Metadata::from_name("README".to_owned()).expect("推断必须成功");

    assert_eq!(metadata.content_type, "application/octet-stream");
}

/// 文件名被原样保留，其余字段为空。
#[test]
fn name_is_preserved_and_optional_fields_stay_empty() {
    let metadata = U8Metadata::from_name("report.zip".to_owned()).expect("推断必须成功");

    assert_eq!(metadata.name, "report.zip");
    assert_eq!(metadata.etag, None);
    assert_eq!(metadata.last_modified, None);
    assert_eq!(metadata.create_time, None);
    assert_eq!(metadata.update_time, None);
}

/// 空文件名与纯空白文件名都被拒绝。
#[test]
fn empty_name_is_rejected() {
    assert!(U8Metadata::from_name(String::new()).is_err());
    assert!(U8Metadata::from_name("   ".to_owned()).is_err());
}

/// 推断出来的内容类型本身通过校验。
#[test]
fn inferred_metadata_passes_validation() {
    let metadata = U8Metadata::from_name("report.zip".to_owned()).expect("推断必须成功");

    assert!(metadata.validate().is_ok());
}

/// 默认元数据（名称为空）不通过校验。
#[test]
fn empty_metadata_fails_validation() {
    let metadata = U8Metadata::default();

    assert!(
        metadata.validate().is_err(),
        "空名称必须被校验发现，否则会发出匿名的内存上传"
    );
}

/// 字段公开后直接改写内容类型，校验由调用方主动调用。
#[test]
fn validation_detects_caller_written_invalid_content_type() {
    let mut metadata = U8Metadata::from_name("a.txt".to_owned()).expect("推断必须成功");
    assert!(metadata.validate().is_ok());

    metadata.content_type = "no-slash".to_owned();
    assert!(metadata.validate().is_err(), "非法 MIME 必须被校验发现");

    metadata.content_type = "bad\nvalue".to_owned();
    assert!(
        metadata.validate().is_err(),
        "写不成合法请求头取值的内容类型必须被校验发现"
    );
}

/// 校验同样能发现被改写为空的文件名。
#[test]
fn validation_detects_empty_name() {
    let mut metadata = U8Metadata::from_name("a.txt".to_owned()).expect("推断必须成功");

    metadata.name = "  ".to_owned();

    assert!(metadata.validate().is_err());
}

/// 自定义内容类型可以直接赋值，不被构造点拦截。
#[test]
fn custom_content_type_can_be_assigned_directly() {
    let mut metadata = U8Metadata::from_name("a.bin".to_owned()).expect("推断必须成功");
    metadata.content_type = "application/x-custom".to_owned();

    assert_eq!(metadata.content_type, "application/x-custom");
    assert!(metadata.validate().is_ok());
}
