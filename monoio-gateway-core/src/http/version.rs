#[derive(Debug)]
pub enum Version {
    HTTP11,
    HTTP2,
}

#[derive(Debug)]
pub enum Type {
    HTTP,
    HTTPS,
}
