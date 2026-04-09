use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

#[derive(Debug, PartialEq)]
pub enum MdElement {
    Heading(u8, String),
    Paragraph(Vec<InlineElement>),
    CodeBlock {
        language: String,
        code: String,
    },
    InlineCode(String),
    List(Vec<ListItem>),
    OrderedList(Vec<ListItem>),
    BlockQuote(Vec<MdElement>),
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    ThematicBreak,
}

#[derive(Debug, PartialEq)]
pub struct ListItem {
    pub content: Vec<InlineElement>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum InlineElement {
    Text(String),
    Bold(String),
    Italic(String),
    Strikethrough(String),
    Code(String),
    Link { text: String, url: String },
}

pub fn parse(markdown: &str) -> Vec<MdElement> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(markdown, options);
    let events: Vec<Event> = parser.collect();
    let mut elements = Vec::new();
    let mut i = 0;

    while i < events.len() {
        i = parse_element(&events, i, &mut elements);
    }

    elements
}

fn parse_element(events: &[Event], start: usize, output: &mut Vec<MdElement>) -> usize {
    if start >= events.len() {
        return start + 1;
    }

    match &events[start] {
        Event::Start(Tag::Heading { level, .. }) => {
            let level_num = *level as u8;
            let mut text = String::new();
            let mut i = start + 1;
            loop {
                if i >= events.len() {
                    break;
                }
                match &events[i] {
                    Event::End(TagEnd::Heading(_)) => {
                        i += 1;
                        break;
                    }
                    Event::Text(t) => {
                        text.push_str(t);
                        i += 1;
                    }
                    Event::Code(c) => {
                        text.push_str(c);
                        i += 1;
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
            output.push(MdElement::Heading(level_num, text));
            i
        }

        Event::Start(Tag::Paragraph) => {
            let mut inlines = Vec::new();
            let mut i = start + 1;
            i = collect_inlines(events, i, &mut inlines, TagEnd::Paragraph);
            output.push(MdElement::Paragraph(inlines));
            i
        }

        Event::Start(Tag::CodeBlock(kind)) => {
            let language = match kind {
                CodeBlockKind::Fenced(lang) => lang.to_string(),
                CodeBlockKind::Indented => String::new(),
            };
            let mut code = String::new();
            let mut i = start + 1;
            loop {
                if i >= events.len() {
                    break;
                }
                match &events[i] {
                    Event::End(TagEnd::CodeBlock) => {
                        i += 1;
                        break;
                    }
                    Event::Text(t) => {
                        code.push_str(t);
                        i += 1;
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
            // Remove trailing newline if present
            if code.ends_with('\n') {
                code.pop();
            }
            output.push(MdElement::CodeBlock { language, code });
            i
        }

        Event::Start(Tag::List(first_item)) => {
            let ordered = first_item.is_some();
            let mut items = Vec::new();
            let mut i = start + 1;
            loop {
                if i >= events.len() {
                    break;
                }
                match &events[i] {
                    Event::End(TagEnd::List(_)) => {
                        i += 1;
                        break;
                    }
                    Event::Start(Tag::Item) => {
                        let mut item_inlines = Vec::new();
                        i += 1;
                        // Items can contain paragraphs or direct inlines
                        loop {
                            if i >= events.len() {
                                break;
                            }
                            match &events[i] {
                                Event::End(TagEnd::Item) => {
                                    i += 1;
                                    break;
                                }
                                Event::Start(Tag::Paragraph) => {
                                    i += 1;
                                    i = collect_inlines(
                                        events,
                                        i,
                                        &mut item_inlines,
                                        TagEnd::Paragraph,
                                    );
                                }
                                Event::Text(t) => {
                                    item_inlines.push(InlineElement::Text(t.to_string()));
                                    i += 1;
                                }
                                Event::Code(c) => {
                                    item_inlines.push(InlineElement::Code(c.to_string()));
                                    i += 1;
                                }
                                _ => {
                                    i += 1;
                                }
                            }
                        }
                        items.push(ListItem {
                            content: item_inlines,
                        });
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
            if ordered {
                output.push(MdElement::OrderedList(items));
            } else {
                output.push(MdElement::List(items));
            }
            i
        }

        Event::Start(Tag::BlockQuote(_)) => {
            let mut inner = Vec::new();
            let mut i = start + 1;
            loop {
                if i >= events.len() {
                    break;
                }
                match &events[i] {
                    Event::End(TagEnd::BlockQuote(_)) => {
                        i += 1;
                        break;
                    }
                    _ => {
                        i = parse_element(events, i, &mut inner);
                    }
                }
            }
            output.push(MdElement::BlockQuote(inner));
            i
        }

        Event::Start(Tag::Table(_)) => {
            let mut headers = Vec::new();
            let mut rows: Vec<Vec<String>> = Vec::new();
            let mut i = start + 1;
            let mut in_head = false;

            loop {
                if i >= events.len() {
                    break;
                }
                match &events[i] {
                    Event::End(TagEnd::Table) => {
                        i += 1;
                        break;
                    }
                    Event::Start(Tag::TableHead) => {
                        in_head = true;
                        i += 1;
                    }
                    Event::End(TagEnd::TableHead) => {
                        in_head = false;
                        i += 1;
                    }
                    Event::Start(Tag::TableRow) => {
                        rows.push(Vec::new());
                        i += 1;
                    }
                    Event::End(TagEnd::TableRow) => {
                        i += 1;
                    }
                    Event::Start(Tag::TableCell) => {
                        let mut cell_text = String::new();
                        i += 1;
                        loop {
                            if i >= events.len() {
                                break;
                            }
                            match &events[i] {
                                Event::End(TagEnd::TableCell) => {
                                    i += 1;
                                    break;
                                }
                                Event::Text(t) => {
                                    cell_text.push_str(t);
                                    i += 1;
                                }
                                Event::Code(c) => {
                                    cell_text.push_str(c);
                                    i += 1;
                                }
                                _ => {
                                    i += 1;
                                }
                            }
                        }
                        if in_head {
                            headers.push(cell_text);
                        } else if let Some(row) = rows.last_mut() {
                            row.push(cell_text);
                        }
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
            output.push(MdElement::Table { headers, rows });
            i
        }

        Event::Rule => {
            output.push(MdElement::ThematicBreak);
            start + 1
        }

        _ => start + 1,
    }
}

fn collect_inlines(
    events: &[Event],
    start: usize,
    inlines: &mut Vec<InlineElement>,
    end_tag: TagEnd,
) -> usize {
    let mut i = start;
    loop {
        if i >= events.len() {
            break;
        }
        match &events[i] {
            Event::End(tag) if *tag == end_tag => {
                i += 1;
                break;
            }
            Event::Text(t) => {
                inlines.push(InlineElement::Text(t.to_string()));
                i += 1;
            }
            Event::Code(c) => {
                inlines.push(InlineElement::Code(c.to_string()));
                i += 1;
            }
            Event::SoftBreak | Event::HardBreak => {
                inlines.push(InlineElement::Text(" ".to_string()));
                i += 1;
            }
            Event::Start(Tag::Strong) => {
                let mut text = String::new();
                i += 1;
                loop {
                    if i >= events.len() {
                        break;
                    }
                    match &events[i] {
                        Event::End(TagEnd::Strong) => {
                            i += 1;
                            break;
                        }
                        Event::Text(t) => {
                            text.push_str(t);
                            i += 1;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                inlines.push(InlineElement::Bold(text));
            }
            Event::Start(Tag::Emphasis) => {
                let mut text = String::new();
                i += 1;
                loop {
                    if i >= events.len() {
                        break;
                    }
                    match &events[i] {
                        Event::End(TagEnd::Emphasis) => {
                            i += 1;
                            break;
                        }
                        Event::Text(t) => {
                            text.push_str(t);
                            i += 1;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                inlines.push(InlineElement::Italic(text));
            }
            Event::Start(Tag::Strikethrough) => {
                let mut text = String::new();
                i += 1;
                loop {
                    if i >= events.len() {
                        break;
                    }
                    match &events[i] {
                        Event::End(TagEnd::Strikethrough) => {
                            i += 1;
                            break;
                        }
                        Event::Text(t) => {
                            text.push_str(t);
                            i += 1;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                inlines.push(InlineElement::Strikethrough(text));
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                let url = dest_url.to_string();
                let mut text = String::new();
                i += 1;
                loop {
                    if i >= events.len() {
                        break;
                    }
                    match &events[i] {
                        Event::End(TagEnd::Link) => {
                            i += 1;
                            break;
                        }
                        Event::Text(t) => {
                            text.push_str(t);
                            i += 1;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                inlines.push(InlineElement::Link { text, url });
            }
            _ => {
                i += 1;
            }
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let elements = parse("# Hello World");
        assert_eq!(elements.len(), 1);
        match &elements[0] {
            MdElement::Heading(1, text) => assert_eq!(text, "Hello World"),
            other => panic!("Expected Heading, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_paragraph_with_bold() {
        let elements = parse("Hello **world**!");
        assert_eq!(elements.len(), 1);
        match &elements[0] {
            MdElement::Paragraph(inlines) => {
                assert!(
                    inlines
                        .iter()
                        .any(|i| matches!(i, InlineElement::Bold(t) if t == "world"))
                );
            }
            other => panic!("Expected Paragraph, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_code_block() {
        let elements = parse("```rust\nfn main() {}\n```");
        assert_eq!(elements.len(), 1);
        match &elements[0] {
            MdElement::CodeBlock { language, code } => {
                assert_eq!(language, "rust");
                assert_eq!(code, "fn main() {}");
            }
            other => panic!("Expected CodeBlock, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_list() {
        let elements = parse("- item one\n- item two\n- item three");
        assert_eq!(elements.len(), 1);
        match &elements[0] {
            MdElement::List(items) => {
                assert_eq!(items.len(), 3);
            }
            other => panic!("Expected List, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_table() {
        let md = "| Name | Age |\n|------|-----|\n| Alice | 30 |\n| Bob | 25 |";
        let elements = parse(md);
        assert_eq!(elements.len(), 1);
        match &elements[0] {
            MdElement::Table { headers, rows } => {
                assert_eq!(headers.len(), 2);
                assert_eq!(headers[0], "Name");
                assert_eq!(headers[1], "Age");
                assert_eq!(rows.len(), 2);
            }
            other => panic!("Expected Table, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_link() {
        let elements = parse("[Click here](https://example.com)");
        assert_eq!(elements.len(), 1);
        match &elements[0] {
            MdElement::Paragraph(inlines) => {
                assert!(inlines.iter().any(|i| matches!(
                    i,
                    InlineElement::Link { text, url }
                    if text == "Click here" && url == "https://example.com"
                )));
            }
            other => panic!("Expected Paragraph, got {:?}", other),
        }
    }
}
