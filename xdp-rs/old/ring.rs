#[derive(Debug)]
struct RingProducer {
    cached_prod: u32,
    cached_cons: u32,
    mask: u32,
    size: u32,
}

#[derive(Debug)]
struct RingConsumer {
    cached_prod: u32,
    cached_cons: u32,
    mask: u32,
    size: u32,
}
