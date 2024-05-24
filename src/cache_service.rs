pub struct ResolvePayload<'a> {
    pub key: &'a str,
    pub value: &'a str,
    pub ttl: u64,
}
