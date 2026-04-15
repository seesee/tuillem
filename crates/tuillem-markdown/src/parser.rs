use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, Options, parse_document};

#[derive(Debug, Clone, PartialEq)]
pub enum MdElement {
    Heading(u8, Vec<InlineElement>),
    Paragraph(Vec<InlineElement>),
    CodeBlock {
        language: String,
        code: String,
    },
    List(Vec<ListItem>),
    OrderedList(Vec<ListItem>),
    BlockQuote(Vec<MdElement>),
    Table {
        headers: Vec<Vec<InlineElement>>,
        rows: Vec<Vec<Vec<InlineElement>>>,
    },
    ThematicBreak,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    pub content: Vec<InlineElement>,
    pub children: Vec<MdElement>, // nested lists, code blocks, etc.
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineElement {
    Text(String),
    Bold(Vec<InlineElement>),
    Italic(Vec<InlineElement>),
    Strikethrough(Vec<InlineElement>),
    Code(String),
    Link { text: String, url: String },
    SoftBreak,
}

pub fn parse(markdown: &str) -> Vec<MdElement> {
    let arena = Arena::new();
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;

    let root = parse_document(&arena, markdown, &options);
    collect_blocks(root)
}

fn collect_blocks<'a>(node: &'a AstNode<'a>) -> Vec<MdElement> {
    let mut elements = Vec::new();
    for child in node.children() {
        let val = &child.data.borrow().value;
        match val {
            NodeValue::Heading(heading) => {
                let inlines = collect_inlines(child);
                elements.push(MdElement::Heading(heading.level, inlines));
            }
            NodeValue::Paragraph => {
                let inlines = collect_inlines(child);
                if !inlines.is_empty() {
                    elements.push(MdElement::Paragraph(inlines));
                }
            }
            NodeValue::CodeBlock(cb) => {
                elements.push(MdElement::CodeBlock {
                    language: cb.info.clone(),
                    code: cb.literal.clone(),
                });
            }
            NodeValue::List(list) => {
                let items = collect_list_items(child);
                if list.list_type == comrak::nodes::ListType::Ordered {
                    elements.push(MdElement::OrderedList(items));
                } else {
                    elements.push(MdElement::List(items));
                }
            }
            NodeValue::BlockQuote => {
                let inner = collect_blocks(child);
                elements.push(MdElement::BlockQuote(inner));
            }
            NodeValue::Table(_) => {
                let (headers, rows) = collect_table(child);
                elements.push(MdElement::Table { headers, rows });
            }
            NodeValue::ThematicBreak => {
                elements.push(MdElement::ThematicBreak);
            }
            _ => {
                elements.extend(collect_blocks(child));
            }
        }
    }
    elements
}

fn collect_inlines<'a>(node: &'a AstNode<'a>) -> Vec<InlineElement> {
    let mut inlines = Vec::new();
    for child in node.children() {
        collect_inline_node(child, &mut inlines);
    }
    inlines
}

fn collect_inline_node<'a>(node: &'a AstNode<'a>, out: &mut Vec<InlineElement>) {
    let val = &node.data.borrow().value;
    match val {
        NodeValue::Text(t) => out.push(InlineElement::Text(t.clone())),
        NodeValue::Code(c) => out.push(InlineElement::Code(c.literal.clone())),
        NodeValue::Strong => {
            let inner = collect_inlines(node);
            out.push(InlineElement::Bold(inner));
        }
        NodeValue::Emph => {
            let inner = collect_inlines(node);
            out.push(InlineElement::Italic(inner));
        }
        NodeValue::Strikethrough => {
            let inner = collect_inlines(node);
            out.push(InlineElement::Strikethrough(inner));
        }
        NodeValue::Link(link) => {
            let text = collect_inline_text(node);
            out.push(InlineElement::Link {
                text,
                url: link.url.clone(),
            });
        }
        NodeValue::SoftBreak | NodeValue::LineBreak => out.push(InlineElement::SoftBreak),
        _ => {
            for child in node.children() {
                collect_inline_node(child, out);
            }
        }
    }
}

fn collect_inline_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();
    for child in node.children() {
        let val = &child.data.borrow().value;
        match val {
            NodeValue::Text(t) => text.push_str(t),
            NodeValue::Code(c) => text.push_str(&c.literal),
            _ => text.push_str(&collect_inline_text(child)),
        }
    }
    text
}

fn collect_list_items<'a>(node: &'a AstNode<'a>) -> Vec<ListItem> {
    let mut items = Vec::new();
    for child in node.children() {
        if matches!(&child.data.borrow().value, NodeValue::Item(_)) {
            let mut content = Vec::new();
            let mut children = Vec::new();
            for sub in child.children() {
                let sub_val = &sub.data.borrow().value.clone();
                match sub_val {
                    NodeValue::Paragraph => {
                        content.extend(collect_inlines(sub));
                    }
                    NodeValue::List(list) => {
                        let sub_items = collect_list_items(sub);
                        if list.list_type == comrak::nodes::ListType::Ordered {
                            children.push(MdElement::OrderedList(sub_items));
                        } else {
                            children.push(MdElement::List(sub_items));
                        }
                    }
                    NodeValue::CodeBlock(cb) => {
                        children.push(MdElement::CodeBlock {
                            language: cb.info.clone(),
                            code: cb.literal.clone(),
                        });
                    }
                    _ => {}
                }
            }
            items.push(ListItem { content, children });
        }
    }
    items
}

fn collect_table<'a>(
    node: &'a AstNode<'a>,
) -> (Vec<Vec<InlineElement>>, Vec<Vec<Vec<InlineElement>>>) {
    let mut headers = Vec::new();
    let mut rows = Vec::new();
    let mut is_header = true;

    for child in node.children() {
        if matches!(&child.data.borrow().value, NodeValue::TableRow(_)) {
            let mut cells = Vec::new();
            for cell_node in child.children() {
                if matches!(&cell_node.data.borrow().value, NodeValue::TableCell) {
                    cells.push(collect_inlines(cell_node));
                }
            }
            if is_header {
                headers = cells;
                is_header = false;
            } else {
                rows.push(cells);
            }
        }
    }
    (headers, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let elements = parse("# Hello World");
        assert!(matches!(&elements[0], MdElement::Heading(1, _)));
    }

    #[test]
    fn test_parse_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |";
        let elements = parse(md);
        match &elements[0] {
            MdElement::Table { headers, rows } => {
                assert_eq!(headers.len(), 2);
                assert_eq!(rows.len(), 2);
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn test_parse_bold() {
        let elements = parse("This is **bold** text");
        match &elements[0] {
            MdElement::Paragraph(inlines) => {
                assert!(inlines.iter().any(|i| matches!(i, InlineElement::Bold(_))));
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn test_parse_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let elements = parse(md);
        match &elements[0] {
            MdElement::CodeBlock { language, code } => {
                assert_eq!(language, "rust");
                assert!(code.contains("fn main()"));
            }
            _ => panic!("expected code block"),
        }
    }

    #[test]
    fn test_parse_list() {
        let md = "- one\n- two\n- three";
        let elements = parse(md);
        match &elements[0] {
            MdElement::List(items) => assert_eq!(items.len(), 3),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn test_parse_nested_list() {
        let md = "1. First\n2. Second\n   - sub-a\n   - sub-b\n3. Third";
        let elements = parse(md);
        match &elements[0] {
            MdElement::OrderedList(items) => {
                assert_eq!(items.len(), 3);
                // Item 2 should have a nested bullet list
                assert_eq!(items[1].children.len(), 1);
                match &items[1].children[0] {
                    MdElement::List(sub) => assert_eq!(sub.len(), 2),
                    other => panic!("expected nested List, got {:?}", other),
                }
                // Items 1 and 3 should have no children
                assert!(items[0].children.is_empty());
                assert!(items[2].children.is_empty());
            }
            _ => panic!("expected ordered list"),
        }
    }
}
