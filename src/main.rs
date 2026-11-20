mod ocr;
mod paste;
mod store;
mod ui;

fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("search") {
        let query = args.collect::<Vec<_>>().join(" ");
        let store = store::Store::open().unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });
        let hits = if query.is_empty() {
            store.recent(8)
        } else {
            store.search(&query, 8)
        };
        println!("{} hits", hits.len());
        return;
    }
    ui::run();
}
