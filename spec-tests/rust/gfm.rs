use peisar::{Peisar, PeisarOptions};

fn render(markdown: &str, gfm: bool) -> String {
    let options = PeisarOptions {
        fragment: Some(true),
        gfm: Some(gfm),
        kramdown: Some(false),
        ..Default::default()
    };
    Peisar::new(markdown.to_string(), Some(options)).html()
}

#[test]
fn renders_gfm_extensions() {
    let html = render(
        "~~gone~~ and https://example.com\n\n- [x] done\n\n| A | B |\n| --- | ---: |\n| one | two |\n",
        true,
    );

    assert!(html.contains("<del>gone</del>"));
    assert!(html.contains("<a href=\"https://example.com\">https://example.com</a>"));
    assert!(html.contains("class=\"task-list-item\""));
    assert!(html.contains("<table>"));
    assert!(html.contains("<td style=\"text-align: right\">two</td>"));
}

#[test]
fn leaves_gfm_extensions_disabled() {
    let html = render("~~gone~~\n\n| A | B |\n| --- | --- |\n", false);

    assert!(html.contains("~~gone~~"));
    assert!(!html.contains("<del>"));
    assert!(!html.contains("<table>"));
}
