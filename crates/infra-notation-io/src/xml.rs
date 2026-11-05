//! Árvore XML mínima (DOM) com limites de profundidade/tamanho e escritor com escape.
//!
//! Entidades externas e DTD nunca são resolvidas (quick-xml não as expande); entidades
//! desconhecidas viram erro, nunca pânico.

use crate::error::{IoError, MAX_XML_DEPTH, MAX_XML_NODES};
use quick_xml::events::Event;
use quick_xml::Reader;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Element>,
    pub text: String,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.name == name)
    }

    pub fn children_named<'a>(&'a self, name: &str) -> impl Iterator<Item = &'a Element> + 'a {
        let name = name.to_owned();
        self.children.iter().filter(move |c| c.name == name)
    }

    /// Texto (aparado) do filho `name`.
    pub fn child_text(&self, name: &str) -> Option<&str> {
        self.child(name).map(|c| c.text.trim())
    }

    /// Filho numérico.
    pub fn child_num<T: std::str::FromStr>(&self, name: &str) -> Option<T> {
        self.child_text(name).and_then(|t| t.parse().ok())
    }

    /// Caminho de filhos (`a/b/c`).
    pub fn path(&self, path: &str) -> Option<&Element> {
        path.split('/').try_fold(self, |e, n| e.child(n))
    }
}

fn local(name: &[u8]) -> String {
    String::from_utf8_lossy(name).into_owned()
}

/// Lê um documento XML inteiro. Erros de sintaxe, entidades desconhecidas ou excesso de
/// profundidade/elementos retornam `Err`.
pub fn parse(bytes: &[u8]) -> Result<Element, IoError> {
    crate::error::check_size(bytes)?;
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut stack: Vec<Element> = vec![Element::default()];
    let mut nodes = 0usize;
    let open = |e: &quick_xml::events::BytesStart<'_>| -> Result<Element, IoError> {
        let mut el = Element {
            name: local(e.local_name().as_ref()),
            ..Element::default()
        };
        for a in e.attributes() {
            let a = a.map_err(IoError::malformed)?;
            let v = a.unescape_value().map_err(IoError::malformed)?;
            el.attrs
                .push((local(a.key.local_name().as_ref()), v.into_owned()));
        }
        Ok(el)
    };
    loop {
        match reader.read_event().map_err(IoError::malformed)? {
            Event::Start(e) => {
                nodes += 1;
                if stack.len() > MAX_XML_DEPTH {
                    return Err(IoError::TooLarge("profundidade XML"));
                }
                if nodes > MAX_XML_NODES {
                    return Err(IoError::TooLarge("elementos XML"));
                }
                stack.push(open(&e)?);
            }
            Event::Empty(e) => {
                nodes += 1;
                if nodes > MAX_XML_NODES {
                    return Err(IoError::TooLarge("elementos XML"));
                }
                let el = open(&e)?;
                stack
                    .last_mut()
                    .expect("raiz sempre presente")
                    .children
                    .push(el);
            }
            Event::End(_) => {
                if stack.len() <= 1 {
                    return Err(IoError::malformed("fechamento sem abertura"));
                }
                let el = stack.pop().expect("verificado acima");
                stack
                    .last_mut()
                    .expect("raiz sempre presente")
                    .children
                    .push(el);
            }
            Event::Text(t) => {
                let s = t.unescape().map_err(IoError::malformed)?;
                stack.last_mut().expect("raiz").text.push_str(&s);
            }
            Event::CData(c) => {
                let s = String::from_utf8_lossy(&c.into_inner()).into_owned();
                stack.last_mut().expect("raiz").text.push_str(&s);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if stack.len() != 1 {
        return Err(IoError::malformed("documento XML incompleto"));
    }
    let mut root = stack.pop().expect("raiz");
    let mut elements = root.children.drain(..);
    match (elements.next(), elements.next()) {
        (Some(el), None) => Ok(el),
        (None, _) => Err(IoError::malformed("documento XML vazio")),
        _ => Err(IoError::malformed("mais de um elemento raiz")),
    }
}

/// Escritor XML simples e indentado.
#[derive(Default)]
pub struct XmlWriter {
    out: String,
    depth: usize,
}

pub fn escape(s: &str) -> String {
    quick_xml::escape::escape(s).into_owned()
}

impl XmlWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn raw(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn indent(&mut self) {
        for _ in 0..self.depth {
            self.out.push_str("  ");
        }
    }

    fn tag(&mut self, name: &str, attrs: &[(&str, String)], close: bool) {
        self.indent();
        self.out.push('<');
        self.out.push_str(name);
        for (k, v) in attrs {
            self.out.push(' ');
            self.out.push_str(k);
            self.out.push_str("=\"");
            self.out.push_str(&escape(v));
            self.out.push('"');
        }
        if close {
            self.out.push('/');
        }
        self.out.push_str(">\n");
    }

    pub fn open(&mut self, name: &str, attrs: &[(&str, String)]) {
        self.tag(name, attrs, false);
        self.depth += 1;
    }

    pub fn close(&mut self, name: &str) {
        self.depth = self.depth.saturating_sub(1);
        self.indent();
        self.out.push_str("</");
        self.out.push_str(name);
        self.out.push_str(">\n");
    }

    pub fn empty(&mut self, name: &str, attrs: &[(&str, String)]) {
        self.tag(name, attrs, true);
    }

    pub fn leaf(&mut self, name: &str, text: impl std::fmt::Display) {
        self.leaf_with(name, &[], text);
    }

    pub fn leaf_with(
        &mut self,
        name: &str,
        attrs: &[(&str, String)],
        text: impl std::fmt::Display,
    ) {
        self.indent();
        self.out.push('<');
        self.out.push_str(name);
        for (k, v) in attrs {
            self.out.push(' ');
            self.out.push_str(k);
            self.out.push_str("=\"");
            self.out.push_str(&escape(v));
            self.out.push('"');
        }
        self.out.push('>');
        self.out.push_str(&escape(&text.to_string()));
        self.out.push_str("</");
        self.out.push_str(name);
        self.out.push_str(">\n");
    }

    pub fn finish(self) -> String {
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tree_with_attributes_text_and_cdata() {
        let xml = br#"<?xml version="1.0"?><!DOCTYPE x><a k="1 &amp; 2"><b>texto &lt;</b><c/><d><![CDATA[<cru>]]></d></a>"#;
        let a = parse(xml).unwrap();
        assert_eq!(a.name, "a");
        assert_eq!(a.attr("k"), Some("1 & 2"));
        assert_eq!(a.child_text("b"), Some("texto <"));
        assert!(a.child("c").is_some());
        assert_eq!(a.child_text("d"), Some("<cru>"));
    }

    #[test]
    fn rejects_malformed_and_deep_documents() {
        assert!(parse(b"<a><b></a>").is_err());
        assert!(parse(b"").is_err());
        assert!(parse(b"<a/><b/>").is_err());
        assert!(parse(b"<a>&unknown;</a>").is_err());
        let deep = "<a>".repeat(MAX_XML_DEPTH + 5);
        assert!(matches!(
            parse(deep.as_bytes()),
            Err(IoError::TooLarge(_)) | Err(IoError::Malformed(_))
        ));
    }

    #[test]
    fn writer_escapes() {
        let mut w = XmlWriter::new();
        w.open("a", &[("x", "\"<&".into())]);
        w.leaf("b", "1 < 2 & 3");
        w.empty("c", &[]);
        w.close("a");
        let s = w.finish();
        let back = parse(s.as_bytes()).unwrap();
        assert_eq!(back.attr("x"), Some("\"<&"));
        assert_eq!(back.child_text("b"), Some("1 < 2 & 3"));
    }
}
