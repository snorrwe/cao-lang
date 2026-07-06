#[derive(Debug, thiserror::Error)]
pub enum LowerError {
    #[error("unsupported node kind `{kind}` at byte {start}..{end}")]
    UnsupportedNode {
        kind: &'static str,
        start: usize,
        end: usize,
    },
    #[error(
        "dynamic call targets (index access in a call's callee, e.g. `arr[i](x)`) are not supported yet, at byte {start}..{end}"
    )]
    DynamicCallUnsupported { start: usize, end: usize },
    #[error("invalid integer literal `{text}`")]
    BadInteger { text: String },
    #[error("invalid float literal `{text}`")]
    BadFloat { text: String },
}
