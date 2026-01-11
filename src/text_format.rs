#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Script {
    Normal,
    Sub,
    Sup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub script: Script,
    pub text: String,
}

fn push_span(out: &mut Vec<Span>, script: Script, text: String) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut() {
        if last.script == script {
            last.text.push_str(&text);
            return;
        }
    }
    out.push(Span { script, text });
}

fn parse_braced(it: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    if it.peek().copied() != Some('{') {
        return String::new();
    }
    it.next();
    let mut depth = 1usize;
    let mut out = String::new();
    while let Some(ch) = it.next() {
        if ch == '\\' {
            if let Some(next) = it.next() {
                out.push(next);
            } else {
                out.push(ch);
            }
            continue;
        }
        match ch {
            '{' => {
                depth += 1;
                out.push(ch);
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

pub fn parse_rich_text(input: &str) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    let mut it = input.chars().peekable();
    let mut buf = String::new();
    let mut script = Script::Normal;

    while let Some(ch) = it.next() {
        if ch == '\\' {
            if let Some(next) = it.next() {
                buf.push(next);
            } else {
                buf.push(ch);
            }
            continue;
        }
        if ch == '^' || ch == '_' {
            let new_script = if ch == '^' { Script::Sup } else { Script::Sub };
            if script != Script::Normal {
                push_span(&mut out, script, std::mem::take(&mut buf));
                script = Script::Normal;
            } else {
                push_span(&mut out, Script::Normal, std::mem::take(&mut buf));
            }
            if it.peek().copied() == Some('{') {
                let inner = parse_braced(&mut it);
                push_span(&mut out, new_script, inner);
            } else if it.peek().is_some_and(|c| c.is_whitespace()) || it.peek().is_none() {
                buf.push(ch);
            } else {
                script = new_script;
            }
            continue;
        }
        if script != Script::Normal && ch.is_whitespace() {
            push_span(&mut out, script, std::mem::take(&mut buf));
            script = Script::Normal;
            buf.push(ch);
        } else {
            buf.push(ch);
        }
    }

    push_span(&mut out, script, buf);

    out
}

pub fn visual_char_count(input: &str) -> usize {
    parse_rich_text(input)
        .into_iter()
        .map(|s| s.text.chars().filter(|&c| c != '\n').count())
        .sum()
}

pub fn split_spans_by_lines(spans: Vec<Span>) -> Vec<Vec<Span>> {
    let mut lines: Vec<Vec<Span>> = vec![vec![]];
    for span in spans {
        let parts: Vec<&str> = span.text.split('\n').collect();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                lines.push(vec![]);
            }
            if !part.is_empty() {
                if let Some(last) = lines.last_mut().unwrap().last_mut() {
                    if last.script == span.script {
                        last.text.push_str(part);
                        continue;
                    }
                }
                lines.last_mut().unwrap().push(Span {
                    script: span.script,
                    text: part.to_string(),
                });
            }
        }
    }
    lines
}

pub fn parse_rich_text_lines(input: &str) -> Vec<Vec<Span>> {
    split_spans_by_lines(parse_rich_text(input))
}
