//! 管理端普通 JSON 响应的压缩范围与大小门槛

use http::{Response, header};
use http_body::Body;
use tower_http::compression::predicate::Predicate;

#[derive(Clone, Copy)]
pub(super) struct AdminJson;

impl Predicate for AdminJson {
    fn should_compress<B: Body>(&self, response: &Response<B>) -> bool {
        // 只处理长度已知的 JSON，不为判断门槛收集流式正文
        response
            .body()
            .size_hint()
            .exact()
            .is_some_and(|size| size >= 1024)
            && response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(';').next())
                .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
    }
}
