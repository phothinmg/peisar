use peisar::Peisar;

fn main() {
    let raw_md = std::fs::read_to_string("test.md").unwrap();
    let mut peisar = Peisar::new(raw_md, None);
    let html = peisar.html();
    std::fs::write("aa.html", html).ok();
}
