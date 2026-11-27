use peisar::{Peisar, PeisarOptions};

#[derive(serde::Deserialize)]
struct CommonMarkSpecs {
    pub markdown: String,
    pub html: String,
    pub example: u32,
    #[allow(unused)]
    pub start_line: u32,
    #[allow(unused)]
    pub end_line: u32,
    pub section: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct FailedTests {
    example: u32,
    input: String,
    expected: String,
    actual: String,
    section: String,
}

// Run CommonMark with GFM and Kramdown disabled
fn parse_markdown(code: &str) -> String {
    let options = PeisarOptions {
        kramdown: Some(false),
        gfm: Some(false),
        fragment: Some(true),
        ..Default::default()
    };
    let mut peisar = Peisar::new(format!("\n{code}"), Some(options));
    peisar.html()
}

fn get_specs() -> Vec<CommonMarkSpecs> {
    let txt = std::fs::read_to_string("spec-tests/specs/cm_v0_31_2.json").unwrap();
    let specs: Vec<CommonMarkSpecs> = serde_json::from_str(&txt).unwrap();
    specs
}
// ================================================
// 1.
// Basic syntax: Tabs, Backslash escapes, Entity and numeric character references.
// Status : Fixed
// ================================================
#[test]
fn basic_syntax_v0_31_2() {
    let specs = get_specs();
    let mut failed: Vec<FailedTests> = Vec::new();
    for spec in specs {
        let section = spec.section.clone();
        if section == "Tabs"
            || section == "Backslash escapes"
            || section == "Entity and numeric character references"
        {
            let input = spec.markdown;
            let example = spec.example;
            let expected = parse_markdown(&input);
            let actual = spec.html;
            if expected != actual {
                failed.push(FailedTests {
                    example,
                    input,
                    expected: expected.clone(),
                    actual: actual.clone(),
                    section: spec.section.clone(),
                });
                assert_ne!(expected.clone(), actual.clone())
            } else {
                assert_eq!(expected.clone(), actual.clone())
            }
        }
    }
    let failed_json = serde_json::to_string_pretty(&failed).unwrap();
    std::fs::write(
        "spec-tests/failed_records/basic_syntax_v0_31_2_r.json",
        failed_json,
    )
    .ok();
}
// =========================================================
// 2.
// Simple block elements:
// Blank lines, Paragraphs, ATX headings, Setext headings,
// Thematic breaks, Indented code blocks, Fenced code blocks.
// Status : Fixed
// ==========================================================
#[test]
fn simple_blocks_v0_31_2() {
    let specs = get_specs();
    let mut failed: Vec<FailedTests> = Vec::new();
    for spec in specs {
        let section = spec.section.clone();
        if section == "Blank lines"
            || section == "Paragraphs"
            || section == "ATX headings"
            || section == "Setext headings"
            || section == "Thematic breaks"
            || section == "Indented code blocks"
            || section == "Fenced code blocks"
        {
            let input = spec.markdown;
            let example = spec.example;
            let expected = parse_markdown(&input);
            let actual = spec.html;
            if expected != actual {
                failed.push(FailedTests {
                    example,
                    input,
                    expected: expected.clone(),
                    actual: actual.clone(),
                    section: spec.section.clone(),
                });
                assert_ne!(expected.clone(), actual.clone())
            } else {
                assert_eq!(expected.clone(), actual.clone())
            }
        }
    }
    let failed_json = serde_json::to_string_pretty(&failed).unwrap();
    std::fs::write(
        "spec-tests/failed_records/simple_blocks_v0_31_2_r.json",
        failed_json,
    )
    .ok();
}
// =========================================================
// 3.
// Block containers:
// Block quotes, List items, Lists.
// Keep list items and lists together; they’re closely related.
// Status : Fixed
// ==========================================================
#[test]
fn block_containers_v0_31_2() {
    let specs = get_specs();
    let mut failed: Vec<FailedTests> = Vec::new();
    for spec in specs {
        let section = spec.section.clone();
        if section == "Block quotes" || section == "List items" || section == "Lists" {
            let input = spec.markdown;
            let example = spec.example;
            let expected = parse_markdown(&input);
            let actual = spec.html;
            if expected != actual {
                failed.push(FailedTests {
                    example,
                    input,
                    expected: expected.clone(),
                    actual: actual.clone(),
                    section: spec.section.clone(),
                });
                assert_ne!(expected.clone(), actual.clone())
            } else {
                assert_eq!(expected.clone(), actual.clone())
            }
        }
    }
    let failed_json = serde_json::to_string_pretty(&failed).unwrap();
    std::fs::write(
        "spec-tests/failed_records/block_containers_v0_31_2_r.json",
        failed_json,
    )
    .ok();
}
// =========================================================
// 4.
// HTML blocks: HTML blocks
// This is substantial enough to tackle separately.
// Status : Fixed
// ==========================================================
#[test]
fn html_blocks_v0_31_2() {
    let specs = get_specs();
    let mut failed: Vec<FailedTests> = Vec::new();
    for spec in specs {
        let section = spec.section.clone();
        if section == "HTML blocks" {
            let input = spec.markdown;
            let example = spec.example;
            let expected = parse_markdown(&input);
            let actual = spec.html;
            if expected != actual {
                failed.push(FailedTests {
                    example,
                    input,
                    expected: expected.clone(),
                    actual: actual.clone(),
                    section: spec.section.clone(),
                });
                assert_ne!(expected.clone(), actual.clone())
            } else {
                assert_eq!(expected.clone(), actual.clone())
            }
        }
    }
    let failed_json = serde_json::to_string_pretty(&failed).unwrap();
    std::fs::write(
        "spec-tests/failed_records/html_blocks_v0_31_2_r.json",
        failed_json,
    )
    .ok();
}
// =========================================================
// 5.
// Inline parsing:
// Code spans, Emphasis and strong emphasis, Links, Images, Autolinks,
// Raw HTML, Hard line breaks, Soft line breaks, Textual content
// ==========================================================
#[test]
fn inline_parsing_v0_31_2() {
    let specs = get_specs();
    let mut failed: Vec<FailedTests> = Vec::new();
    for spec in specs {
        let section = spec.section.clone();
        if section == "Code spans"
            || section == "Emphasis and strong emphasis"
            || section == "Links"
            || section == "Images"
            || section == "Autolinks"
            || section == "Raw HTML"
            || section == "Hard line breaks"
            || section == "Soft line breaks"
            || section == "Textual content"
        {
            let input = spec.markdown;
            let example = spec.example;
            let expected = parse_markdown(&input);
            let actual = spec.html;
            if expected != actual {
                failed.push(FailedTests {
                    example,
                    input,
                    expected: expected.clone(),
                    actual: actual.clone(),
                    section: spec.section.clone(),
                });
                assert_ne!(expected.clone(), actual.clone())
            } else {
                assert_eq!(expected.clone(), actual.clone())
            }
        }
    }
    let failed_json = serde_json::to_string_pretty(&failed).unwrap();
    std::fs::write(
        "spec-tests/failed_records/inline_parsing_v0_31_2_r.json",
        failed_json,
    )
    .ok();
}
