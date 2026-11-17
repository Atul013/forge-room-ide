//! Native argument capture for task quoting tests, with no shell/script execution-policy changes.
fn main() {
    let mut args = std::env::args().skip(1);
    let output = args.next().expect("output path");
    std::fs::write(output, args.collect::<Vec<_>>().join("\n")).unwrap();
}
