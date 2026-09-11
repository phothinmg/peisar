use crate::ast::{Document, Inline};
use crate::plugin_factory::{Plugin, PluginContext};

/// GitHub-flavored alert types.
///
/// See <https://docs.github.com/en/get-started/writing-on-github/get-started/writing-on-github#alerts>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AlertKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl AlertKind {
    /// Parse a `[!XXX]` marker into an [`AlertKind`].
    pub fn from_marker(marker: &str) -> Option<Self> {
        match marker.trim().to_ascii_uppercase().as_str() {
            "[!NOTE]" => Some(Self::Note),
            "[!TIP]" => Some(Self::Tip),
            "[!IMPORTANT]" => Some(Self::Important),
            "[!WARNING]" => Some(Self::Warning),
            "[!CAUTION]" => Some(Self::Caution),
            _ => None,
        }
    }

    /// Human-readable label (uppercase).
    pub fn label(&self) -> &'static str {
        match self {
            Self::Note => "NOTE",
            Self::Tip => "TIP",
            Self::Important => "IMPORTANT",
            Self::Warning => "WARNING",
            Self::Caution => "CAUTION",
        }
    }

    /// Badge background color for the inline label span.
    pub fn color(&self) -> &'static str {
        match self {
            Self::Note => "#0969da",
            Self::Tip => "#1a7f37",
            Self::Important => "#8250df",
            Self::Warning => "#9a6700",
            Self::Caution => "#cf222e",
        }
    }

    /// Build the inline HTML badge for this alert kind.
    pub fn badge_html(&self) -> String {
        format!(
            "<p style=\"font-family: Arial, Helvetica, sans-serif;background-color: {};color: white;padding: 4px;text-align: center;border-radius: 5px;font-size: 12px;font-weight: 600;max-width: 100px;\">{}</p>",
            self.color(),
            self.label()
        )
    }
}

/// A plugin that renders GitHub-flavored alert markers inside blockquote
/// (`[!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`, `[!CAUTION]`) as
/// styled inline badges.
pub struct BlockquoteAlert;

impl Default for BlockquoteAlert {
    fn default() -> Self {
        Self
    }
}

impl Plugin for BlockquoteAlert {
    fn name(&self) -> &str {
        "blockquote_alert"
    }
    fn transform_ast(&self, doc: &mut Document, _ctx: &PluginContext) {
        use crate::ast::Block;
        for block in &mut doc.children {
            if let Block::BlockQuote { children, .. } = block {
                if let Some(Block::Paragraph {
                    children: inlines, ..
                }) = children.first_mut()
                {
                    if let Some(Inline::Text { value, .. }) = inlines.first() {
                        if let Some(kind) = AlertKind::from_marker(value) {
                            *inlines = vec![Inline::HtmlInline {
                                html: kind.badge_html(),
                                position: Default::default(),
                            }];
                        }
                    }
                }
            }
        }
    }
}
