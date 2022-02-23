

async fn serve(_: usize) {
    println!("Hello, world!");
}


fn main() {
    let mut handlers = Vec::new();

    let h = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(serve(0));
    });

    handlers.push(h);

    for handler in handlers {
        handler.join().unwrap();
    }
}
