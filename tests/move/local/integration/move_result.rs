//! MOVE 的结果语义：移动成功后源消失、目标出现。
//!
//! 这是 MOVE 区别于 COPY 的唯一行为差异，也是最容易出错的认知点：调用方常以为
//! 源还在。用例用一个模拟真实文件系统的 responder 验证因果——只有在 MOVE 成功
//! 之后，源路径才返回 404，目标路径才返回 200。

use std::sync::{Arc, Mutex};

use webdav_raw::{MoveDepth, Overwrite, WebdavAuth};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, Request, Respond, ResponseTemplate};

use crate::common::local_http;

/// 一个极简的文件系统：只记录「哪些路径上存在内容」。
#[derive(Default)]
struct FileSystem {
    files: Mutex<Vec<String>>,
}

impl FileSystem {
    fn with(files: &[&str]) -> Arc<Self> {
        Arc::new(Self {
            files: Mutex::new(files.iter().map(|name| (*name).to_owned()).collect()),
        })
    }

    fn contains(&self, name: &str) -> bool {
        self.files
            .lock()
            .expect("锁不应中毒")
            .iter()
            .any(|existing| existing == name)
    }

    /// 真正的移动：目标出现、源消失。
    fn r#move(&self, from: &str, to: &str) {
        let mut files = self.files.lock().expect("锁不应中毒");
        files.retain(|existing| existing != from && existing != to);
        files.push(to.to_owned());
    }
}

/// 按文件系统状态应答 GET。
struct ReadFile {
    fs: Arc<FileSystem>,
    name: String,
}

impl Respond for ReadFile {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        if self.fs.contains(&self.name) {
            ResponseTemplate::new(200).set_body_string(self.name.clone())
        } else {
            ResponseTemplate::new(404)
        }
    }
}

/// 按文件系统状态应答 MOVE。
struct MoveFile {
    fs: Arc<FileSystem>,
    from: String,
    to: String,
    calls: Arc<std::sync::atomic::AtomicUsize>,
}

impl Respond for MoveFile {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        self.calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        if !self.fs.contains(&self.from) {
            return ResponseTemplate::new(404);
        }

        self.fs.r#move(&self.from, &self.to);

        ResponseTemplate::new(201)
    }
}

/// 移动之后源查不到、目标查得到。
#[tokio::test]
async fn source_disappears_and_target_appears_after_move() {
    let server = local_http::start_server().await;
    let fs = FileSystem::with(&["staging/report.pdf"]);
    let move_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    Mock::given(method("GET"))
        .and(path("/dav/staging/report.pdf"))
        .respond_with(ReadFile {
            fs: Arc::clone(&fs),
            name: "staging/report.pdf".to_owned(),
        })
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/dav/archive/report.pdf"))
        .respond_with(ReadFile {
            fs: Arc::clone(&fs),
            name: "archive/report.pdf".to_owned(),
        })
        .mount(&server)
        .await;
    Mock::given(method("MOVE"))
        .and(path("/dav/staging/report.pdf"))
        .and(header("depth", "infinity"))
        .respond_with(MoveFile {
            fs: Arc::clone(&fs),
            from: "staging/report.pdf".to_owned(),
            to: "archive/report.pdf".to_owned(),
            calls: Arc::clone(&move_calls),
        })
        .mount(&server)
        .await;

    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    // 移动前：源在、目标不在。
    assert_eq!(
        auth.get()
            .relative_path("staging/report.pdf")
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("GET 应发送成功")
            .status(),
        200,
        "移动前源应存在"
    );
    assert_eq!(
        auth.get()
            .relative_path("archive/report.pdf")
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("GET 应发送成功")
            .status(),
        404,
        "移动前目标不应存在"
    );

    let moved = auth
        .mv()
        .move_from_path("staging/report.pdf")
        .expect("合法相对路径应被接受")
        .target_path("archive/report.pdf")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("MOVE 应发送成功");
    assert_eq!(moved.status(), 201);

    // 移动后：源消失、目标出现。
    assert_eq!(
        auth.get()
            .relative_path("staging/report.pdf")
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("GET 应发送成功")
            .status(),
        404,
        "移动后源必须查不到"
    );
    assert_eq!(
        auth.get()
            .relative_path("archive/report.pdf")
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("GET 应发送成功")
            .status(),
        200,
        "移动后目标必须查得到"
    );
    assert_eq!(
        move_calls.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "只应发出一次 MOVE"
    );
}

/// 重命名也是 MOVE：同一目录内换名字。
#[tokio::test]
async fn rename_within_same_collection_moves_the_file() {
    let server = local_http::start_server().await;
    let fs = FileSystem::with(&["draft.txt"]);
    let move_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    Mock::given(method("GET"))
        .and(path("/dav/draft.txt"))
        .respond_with(ReadFile {
            fs: Arc::clone(&fs),
            name: "draft.txt".to_owned(),
        })
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/dav/final.txt"))
        .respond_with(ReadFile {
            fs: Arc::clone(&fs),
            name: "final.txt".to_owned(),
        })
        .mount(&server)
        .await;
    Mock::given(method("MOVE"))
        .and(path("/dav/draft.txt"))
        .respond_with(MoveFile {
            fs: Arc::clone(&fs),
            from: "draft.txt".to_owned(),
            to: "final.txt".to_owned(),
            calls: Arc::clone(&move_calls),
        })
        .mount(&server)
        .await;

    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let renamed = auth
        .mv()
        .move_from_path("draft.txt")
        .expect("合法相对路径应被接受")
        .target_path("final.txt")
        .expect("合法相对路径应被接受")
        .depth(MoveDepth::Zero)
        .overwrite(Overwrite::False)
        .send()
        .await
        .expect("MOVE 应发送成功");
    assert_eq!(renamed.status(), 201);

    assert_eq!(
        auth.get()
            .relative_path("draft.txt")
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("GET 应发送成功")
            .status(),
        404,
        "重命名后旧名字应查不到"
    );
    assert_eq!(
        auth.get()
            .relative_path("final.txt")
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("GET 应发送成功")
            .status(),
        200,
        "重命名后新名字应查得到"
    );
}

/// 源不存在时服务端回 404，本库原样透传，且源与目标状态都不变。
#[tokio::test]
async fn moving_a_missing_source_leaves_state_untouched() {
    let server = local_http::start_server().await;
    let fs = FileSystem::with(&[]);

    Mock::given(method("MOVE"))
        .and(path("/dav/missing.txt"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/dav/target.txt"))
        .respond_with(ReadFile {
            fs: Arc::clone(&fs),
            name: "target.txt".to_owned(),
        })
        .mount(&server)
        .await;

    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let response = auth
        .mv()
        .move_from_path("missing.txt")
        .expect("合法相对路径应被接受")
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("MOVE 应发送成功");

    assert_eq!(response.status(), 404);
    assert_eq!(
        auth.get()
            .relative_path("target.txt")
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("GET 应发送成功")
            .status(),
        404,
        "移动失败时目标不应出现"
    );
}
