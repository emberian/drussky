//! A deliberately small, inert Markdown vocabulary. Every prose block gets its own
//! visibility observer; a long answer is never credited when only its start appears.
use crate::{exposure_view::PresentedRussian, model::AppState};
use dioxus::prelude::*;
use pulldown_cmark::{Event, Parser, Tag};
use rufren_core::PresentationPurpose;

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Root,
    Paragraph,
    Quote,
    List(Option<u64>),
    Item,
    Strong,
    Emphasis,
    Link(Option<String>),
    Code,
    Text(String),
    Break,
}
#[derive(Clone, Debug, PartialEq)]
struct Node {
    kind: Kind,
    children: Vec<Node>,
}
impl Node {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            children: Vec::new(),
        }
    }
    fn inline(&self) -> bool {
        matches!(
            self.kind,
            Kind::Strong
                | Kind::Emphasis
                | Kind::Link(_)
                | Kind::Code
                | Kind::Text(_)
                | Kind::Break
        )
    }
    fn plain(&self) -> String {
        match &self.kind {
            Kind::Text(text) => text.clone(),
            Kind::Break => "\n".into(),
            _ => self.children.iter().map(Node::plain).collect(),
        }
    }
}
fn safe_link(url: &str) -> Option<String> {
    // Absolute web links only. Controls and whitespace are rejected, including
    // obfuscated schemes; Dioxus escapes the resulting attribute value.
    let lower = url.to_ascii_lowercase();
    ((lower.starts_with("https://") || lower.starts_with("http://"))
        && !url.chars().any(|c| c.is_control() || c.is_whitespace()))
    .then(|| url.to_owned())
}
fn parse(text: &str) -> Node {
    let mut end = text.len().min(128 * 1024);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut stack = vec![Node::new(Kind::Root)];
    let mut overflow = 0usize;
    for event in Parser::new(&text[..end]) {
        match event {
            Event::Start(tag) => {
                if stack.len() >= 32 {
                    overflow += 1;
                    continue;
                }
                if overflow > 0 {
                    overflow += 1;
                    continue;
                }
                let kind = match tag {
                    Tag::Strong => Kind::Strong,
                    Tag::Emphasis => Kind::Emphasis,
                    Tag::Link { dest_url, .. } => Kind::Link(safe_link(&dest_url)),
                    // Images retain their readable alternative text without fetching anything.
                    Tag::Image { .. } => Kind::Link(None),
                    Tag::List(start) => Kind::List(start),
                    Tag::Item => Kind::Item,
                    Tag::BlockQuote(_) => Kind::Quote,
                    _ => Kind::Paragraph,
                };
                stack.push(Node::new(kind));
            }
            Event::End(_) => {
                if overflow > 0 {
                    overflow -= 1;
                    continue;
                }
                if stack.len() > 1 {
                    let node = stack.pop().unwrap();
                    stack.last_mut().unwrap().children.push(node);
                }
            }
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => stack
                .last_mut()
                .unwrap()
                .children
                .push(Node::new(Kind::Text(text.into_string()))),
            Event::Code(text) => {
                let mut node = Node::new(Kind::Code);
                node.children
                    .push(Node::new(Kind::Text(text.into_string())));
                stack.last_mut().unwrap().children.push(node);
            }
            Event::SoftBreak | Event::HardBreak => stack
                .last_mut()
                .unwrap()
                .children
                .push(Node::new(Kind::Break)),
            Event::Rule => stack
                .last_mut()
                .unwrap()
                .children
                .push(Node::new(Kind::Break)),
            _ => (),
        }
    }
    stack.remove(0)
}

#[component]
pub(crate) fn TeacherProse(
    app: Signal<AppState>,
    text: String,
    purpose: PresentationPurpose,
    #[props(default)] class: String,
    #[props(default = "ru".to_string())] language: String,
) -> Element {
    let tree = parse(&text);
    rsx! {div {class: "teacher-prose {class}", lang: language.clone(), {render_children(&tree.children, app, purpose, &language)}}}
}
fn render_children(
    nodes: &[Node],
    app: Signal<AppState>,
    purpose: PresentationPurpose,
    language: &str,
) -> Element {
    let mut output = Vec::new();
    let mut at = 0;
    while at < nodes.len() {
        if nodes[at].inline() {
            let start = at;
            while at < nodes.len() && nodes[at].inline() {
                at += 1;
            }
            let run = &nodes[start..at];
            let text: String = run.iter().map(Node::plain).collect();
            // New visible content needs a fresh observer even if a saved answer
            // is replaced in the same position of a dynamic list.
            let key = format!("{start}:{text}");
            output.push(rsx! {PresentedRussian {key: "{key}", app, text, purpose, language:language.to_owned(), p {for node in run {{render_inline(node)}}}}});
        } else {
            let node = &nodes[at];
            let children = render_children(&node.children, app, purpose, language);
            output.push(match node.kind {
                Kind::List(Some(start)) => rsx! {ol {start: "{start}", {children}}},
                Kind::List(None) => rsx! {ul {{children}}},
                Kind::Item => rsx! {li {{children}}},
                Kind::Quote => rsx! {blockquote {{children}}},
                _ => children,
            });
            at += 1;
        }
    }
    rsx! {for element in output {{element}}}
}
fn render_inline(node: &Node) -> Element {
    let children = || rsx! {for child in &node.children {{render_inline(child)}}};
    match &node.kind {
        Kind::Text(text) => rsx! {"{text}"},
        Kind::Break => rsx! {br {}},
        Kind::Strong => rsx! {strong {{children()}}},
        Kind::Emphasis => rsx! {em {{children()}}},
        Kind::Code => rsx! {code {{children()}}},
        Kind::Link(Some(url)) => {
            rsx! {a {href:url.clone(), rel:"noreferrer noopener", {children()}}}
        }
        _ => children(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_dom_escapes_html_and_never_loads_images_or_unsafe_links() {
        fn inline_nodes(node: &Node, out: &mut Vec<Element>) {
            if node.inline() {
                out.push(render_inline(node));
            } else {
                for child in &node.children {
                    inline_nodes(child, out);
                }
            }
        }
        let tree = parse(
            "**Приве́т** *мир* `x²` [да](https://example.org/) [нет](javascript:alert%281%29) ![кот](https://example.org/cat.png)\n\n<script>alert(1)</script>",
        );
        let mut elements = Vec::new();
        inline_nodes(&tree, &mut elements);
        let html = dioxus_ssr::render_element(rsx! { for element in elements {{element}} });
        assert!(html.contains("<strong>Приве́т</strong>"), "{html}");
        assert!(html.contains("<em>мир</em>"), "{html}");
        assert!(html.contains("<code>x²</code>"), "{html}");
        assert!(html.contains("href=\"https://example.org/\""), "{html}");
        assert!(!html.contains("javascript:"), "{html}");
        assert!(!html.contains("<img"), "{html}");
        assert!(!html.contains("cat.png"), "{html}");
        assert!(!html.contains("<script>"), "{html}");
        assert!(html.contains("&#60;script&#62;"), "{html}");
    }
    #[test]
    fn plain_history_excludes_markup_and_destinations() {
        let tree = parse(
            "**Приве́т** *мир* `x²` [да](https://example.org/скрыто) ![кот](https://example.org/cat.png)",
        );
        assert_eq!(tree.plain(), "Приве́т мир x² да кот");
        assert!(matches!(tree.children[0].children[0].kind, Kind::Strong));
    }
    #[test]
    fn html_is_literal_and_unsafe_links_are_inert() {
        let tree = parse("<script>alert(1)</script>\n\n[да](javascript:alert%281%29)");
        assert!(tree.plain().contains("<script>"));
        assert_eq!(safe_link("javascript:alert(1)"), None);
        assert_eq!(safe_link("data:text/html,test"), None);
        assert_eq!(safe_link("//example.org"), None);
        assert!(safe_link("HTTPS://example.org/путь").is_some());
        assert_eq!(safe_link("https://example.org/\nfoo"), None);
    }
    #[test]
    fn paragraphs_and_list_items_remain_separate_exposure_blocks() {
        let tree = parse("Первый.\n\nВторой.\n\n- Один\n- Два");
        assert_eq!(tree.children.len(), 3);
        assert!(matches!(tree.children[2].kind, Kind::List(None)));
        assert_eq!(tree.children[2].children.len(), 2);
        assert_eq!(tree.children[2].children[1].plain(), "Два");
    }
    #[test]
    fn preserves_unicode_unmatched_syntax_and_limits_depth() {
        assert_eq!(
            parse("Приве́т **незакрыто ∑ x²").plain(),
            "Приве́т **незакрыто ∑ x²"
        );
        let nested = format!("{}да{}", "> ".repeat(200), "\n");
        assert!(parse(&nested).plain().contains("да"));
    }
}
