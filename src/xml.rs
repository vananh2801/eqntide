//! A small, namespace-aware, *mutable* XML DOM.
//!
//! The Python original leans on `lxml` for three things: parsing, editing the
//! tree in place (swapping `m:oMath` for `mc:AlternateContent`), and
//! serialising it back with sensible namespace prefixes. Rust has no direct
//! equivalent that supports all three, so this module provides just enough:
//!
//! * [`parse`] builds an [`Element`] tree (namespaces resolved, prefixes kept),
//! * the tree can be edited freely,
//! * [`to_bytes`] writes it back, *reconciling* namespace prefixes on the fly:
//!   elements created in code only need to say "I am `{uri}local`, I would
//!   like the prefix `w`"; the serialiser reuses an in-scope prefix or adds an
//!   `xmlns` declaration where one is missing.

use std::fmt::Write as _;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::error::{Error, Result};

/// The namespace bound to the reserved `xml` prefix.
pub const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

/// Same limit libxml2 (and therefore lxml) applies by default.
const MAX_DEPTH: usize = 256;

/// `(prefix, uri)`; `prefix == None` is the default namespace.
type NsDecl = (Option<String>, String);

/// An attribute (namespace declarations are stored separately).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attr {
    /// Prefix as written (or hinted) — only meaningful when `ns` is `Some`.
    pub prefix: Option<String>,
    /// Local name.
    pub local: String,
    /// Namespace URI, if any.
    pub ns: Option<String>,
    /// Unescaped value.
    pub value: String,
}

/// A child of an [`Element`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// A nested element.
    Element(Element),
    /// Character data (already unescaped).
    Text(String),
    /// `<!-- … -->` (content only).
    Comment(String),
    /// `<? … ?>` (raw content between the delimiters).
    Pi(String),
}

/// An XML element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// Prefix as parsed / hinted. `None` means "default namespace (or none)".
    pub prefix: Option<String>,
    /// Local name.
    pub local: String,
    /// Namespace URI, if any.
    pub ns: Option<String>,
    /// Namespace declarations made *on this element*, in source order.
    pub nsdecls: Vec<(Option<String>, String)>,
    /// Attributes in source order.
    pub attrs: Vec<Attr>,
    /// Child nodes in document order.
    pub children: Vec<Node>,
}

impl Element {
    /// Create an empty element.
    pub fn new(ns: Option<&str>, prefix: Option<&str>, local: &str) -> Self {
        Element {
            prefix: prefix.map(str::to_owned),
            local: local.to_owned(),
            ns: ns.map(str::to_owned),
            nsdecls: Vec::new(),
            attrs: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Builder-style `xmlns[:prefix]="uri"` declaration on this element.
    pub fn with_nsdecl(mut self, prefix: Option<&str>, uri: &str) -> Self {
        self.nsdecls
            .push((prefix.map(str::to_owned), uri.to_owned()));
        self
    }

    /// Set (or replace) an attribute.
    pub fn set_attr(&mut self, ns: Option<&str>, prefix: Option<&str>, local: &str, value: &str) {
        if let Some(a) = self
            .attrs
            .iter_mut()
            .find(|a| a.local == local && a.ns.as_deref() == ns)
        {
            a.value = value.to_owned();
            return;
        }
        self.attrs.push(Attr {
            prefix: prefix.map(str::to_owned),
            local: local.to_owned(),
            ns: ns.map(str::to_owned),
            value: value.to_owned(),
        });
    }

    /// Look an attribute up by namespace + local name.
    pub fn attr(&self, ns: Option<&str>, local: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|a| a.local == local && a.ns.as_deref() == ns)
            .map(|a| a.value.as_str())
    }

    /// `true` when this element is `{ns}local`.
    pub fn is(&self, ns: &str, local: &str) -> bool {
        self.local == local && self.ns.as_deref() == Some(ns)
    }

    /// Append a child element.
    pub fn push(&mut self, child: Element) {
        self.children.push(Node::Element(child));
    }

    /// Iterate over direct child *elements*.
    pub fn elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|n| match n {
            Node::Element(e) => Some(e),
            _ => None,
        })
    }

    /// First direct child `{ns}local` (lxml's `el.find("{ns}local")`).
    pub fn find(&self, ns: &str, local: &str) -> Option<&Element> {
        self.elements().find(|e| e.is(ns, local))
    }

    /// All direct children `{ns}local` (lxml's `el.findall("{ns}local")`).
    pub fn find_all(&self, ns: &str, local: &str) -> Vec<&Element> {
        self.elements().filter(|e| e.is(ns, local)).collect()
    }

    /// Number of non-text children — what `len(el)` means in lxml.
    pub fn len(&self) -> usize {
        self.children
            .iter()
            .filter(|n| !matches!(n, Node::Text(_)))
            .count()
    }

    /// `true` when [`len`](Self::len) is zero.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Text before the first non-text child (lxml's `el.text`).
    pub fn text(&self) -> String {
        let mut out = String::new();
        for n in &self.children {
            match n {
                Node::Text(t) => out.push_str(t),
                _ => break,
            }
        }
        out
    }

    /// This element followed by every descendant element, in document order
    /// (lxml's `el.iter()`).
    pub fn iter(&self) -> Vec<&Element> {
        fn walk<'a>(e: &'a Element, out: &mut Vec<&'a Element>) {
            out.push(e);
            for c in e.elements() {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(self, &mut out);
        out
    }
}

// ── Parsing ───────────────────────────────────────────────────────────────────

fn xml_err(e: impl std::fmt::Display, pos: usize) -> Error {
    Error::Xml(format!("{e} (byte offset {pos})"))
}

/// Normalise `\r\n` / `\r` to `\n`, as an XML processor must.
fn normalize_newlines(s: &str) -> String {
    if !s.contains('\r') {
        return s.to_owned();
    }
    s.replace("\r\n", "\n").replace('\r', "\n")
}

fn default_known_ns(prefix: &str) -> Option<&'static str> {
    match prefix {
        "w" => Some("http://schemas.openxmlformats.org/wordprocessingml/2006/main"),
        "m" => Some("http://schemas.openxmlformats.org/officeDocument/2006/math"),
        "r" => Some("http://schemas.openxmlformats.org/officeDocument/2006/relationships"),
        "v" => Some("urn:schemas-microsoft-com:vml"),
        "o" => Some("urn:schemas-microsoft-com:office:office"),
        "pkg" => Some("http://schemas.microsoft.com/office/2006/xmlPackage"),
        "a" => Some("http://schemas.openxmlformats.org/drawingml/2006/main"),
        "pic" => Some("http://schemas.openxmlformats.org/drawingml/2006/picture"),
        "wp" => Some("http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"),
        "w14" => Some("http://schemas.microsoft.com/office/word/2010/wordml"),
        _ => None,
    }
}

fn lookup<'a>(scope: &'a [NsDecl], prefix: Option<&str>) -> Option<&'a str> {
    if prefix == Some("xml") {
        return Some(XML_NS);
    }
    for (p, u) in scope.iter().rev() {
        if p.as_deref() == prefix {
            return if u.is_empty() { None } else { Some(u.as_str()) };
        }
    }
    if let Some(p) = prefix {
        return default_known_ns(p);
    }
    None
}

fn split_qname(q: &str) -> (Option<&str>, &str) {
    match q.split_once(':') {
        Some((p, l)) => (Some(p), l),
        None => (None, q),
    }
}

/// Build an [`Element`] from a start tag; pushes its declarations onto `scope`.
fn element_from_start(e: &BytesStart<'_>, scope: &mut Vec<NsDecl>, pos: usize) -> Result<Element> {
    let qname = std::str::from_utf8(e.name().as_ref())
        .map_err(|err| xml_err(err, pos))?
        .to_owned();

    let mut nsdecls: Vec<NsDecl> = Vec::new();
    let mut raw_attrs: Vec<(String, String)> = Vec::new();
    for a in e.attributes() {
        let a = a.map_err(|err| xml_err(err, pos))?;
        let key = std::str::from_utf8(a.key.as_ref())
            .map_err(|err| xml_err(err, pos))?
            .to_owned();
        let raw = std::str::from_utf8(&a.value).map_err(|err| xml_err(err, pos))?;
        // Attribute-value normalisation: literal whitespace becomes a space.
        let normalised: String = normalize_newlines(raw)
            .chars()
            .map(|c| if c == '\n' || c == '\t' { ' ' } else { c })
            .collect();
        let value = quick_xml::escape::unescape(&normalised)
            .map_err(|err| xml_err(err, pos))?
            .into_owned();
        if key == "xmlns" {
            nsdecls.push((None, value));
        } else if let Some(p) = key.strip_prefix("xmlns:") {
            nsdecls.push((Some(p.to_owned()), value));
        } else {
            raw_attrs.push((key, value));
        }
    }
    scope.extend(nsdecls.iter().cloned());

    let (eprefix, elocal) = split_qname(&qname);
    let ens = match eprefix {
        Some(p) => Some(
            lookup(scope, Some(p))
                .ok_or_else(|| {
                    Error::Xml(format!(
                        "unbound namespace prefix '{p}' (byte offset {pos})"
                    ))
                })?
                .to_owned(),
        ),
        None => lookup(scope, None).map(str::to_owned),
    };

    let mut attrs = Vec::with_capacity(raw_attrs.len());
    for (key, value) in raw_attrs {
        let (p, l) = split_qname(&key);
        let ns = match p {
            Some(p) => Some(
                lookup(scope, Some(p))
                    .ok_or_else(|| {
                        Error::Xml(format!(
                            "unbound namespace prefix '{p}' (byte offset {pos})"
                        ))
                    })?
                    .to_owned(),
            ),
            None => None,
        };
        attrs.push(Attr {
            prefix: p.map(str::to_owned),
            local: l.to_owned(),
            ns,
            value,
        });
    }

    Ok(Element {
        prefix: eprefix.map(str::to_owned),
        local: elocal.to_owned(),
        ns: ens,
        nsdecls,
        attrs,
        children: Vec::new(),
    })
}

fn push_text(parent: &mut Element, text: String) {
    if text.is_empty() {
        return;
    }
    if let Some(Node::Text(last)) = parent.children.last_mut() {
        last.push_str(&text);
    } else {
        parent.children.push(Node::Text(text));
    }
}

/// Parse an XML document (UTF-8) into its root [`Element`].
pub fn parse(data: &[u8]) -> Result<Element> {
    let data = data.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(data);
    let text = std::str::from_utf8(data).map_err(|e| Error::Xml(format!("invalid UTF-8: {e}")))?;

    XmlParser::new(text).parse()
}

/// Stateful parser context for one XML document.
///
/// Namespace scope follows the open-element stack, while `marks` records where
/// each element's declarations begin so closing an element restores its parent.
struct XmlParser<'a> {
    reader: Reader<&'a [u8]>,
    stack: Vec<Element>,
    marks: Vec<usize>,
    scope: Vec<NsDecl>,
    root: Option<Element>,
}

impl<'a> XmlParser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            reader: Reader::from_str(text),
            stack: Vec::new(),
            marks: Vec::new(),
            scope: Vec::new(),
            root: None,
        }
    }

    fn parse(mut self) -> Result<Element> {
        let mut reader = self.reader;
        let stack = &mut self.stack;
        let marks = &mut self.marks;
        let scope = &mut self.scope;
        let root = &mut self.root;

        // Attach a finished element to its parent, or make it the root.
        fn attach(stack: &mut [Element], root: &mut Option<Element>, el: Element) -> Result<()> {
            if let Some(parent) = stack.last_mut() {
                parent.children.push(Node::Element(el));
                Ok(())
            } else if root.is_none() {
                *root = Some(el);
                Ok(())
            } else {
                Err(Error::Xml(
                    "extra content at the end of the document".into(),
                ))
            }
        }

        loop {
            let pos = reader.buffer_position();
            match reader.read_event().map_err(|e| xml_err(e, pos))? {
                Event::Eof => break,
                Event::Start(e) => {
                    if stack.len() >= MAX_DEPTH {
                        return Err(Error::Xml(format!(
                            "excessive depth in document: {MAX_DEPTH} (byte offset {pos})"
                        )));
                    }
                    if stack.is_empty() && root.is_some() {
                        return Err(Error::Xml(
                            "extra content at the end of the document".into(),
                        ));
                    }
                    marks.push(scope.len());
                    let el = element_from_start(&e, scope, pos)?;
                    stack.push(el);
                }
                Event::Empty(e) => {
                    if stack.len() >= MAX_DEPTH {
                        return Err(Error::Xml(format!(
                            "excessive depth in document: {MAX_DEPTH} (byte offset {pos})"
                        )));
                    }
                    let mark = scope.len();
                    let el = element_from_start(&e, scope, pos)?;
                    scope.truncate(mark);
                    attach(stack, root, el)?;
                }
                Event::End(_) => {
                    let el = stack
                        .pop()
                        .ok_or_else(|| xml_err("unexpected closing tag", pos))?;
                    scope.truncate(marks.pop().unwrap_or(0));
                    attach(stack, root, el)?;
                }
                Event::Text(t) => {
                    let raw = std::str::from_utf8(&t).map_err(|e| xml_err(e, pos))?;
                    let raw = normalize_newlines(raw);
                    let txt = quick_xml::escape::unescape(&raw)
                        .map_err(|e| xml_err(e, pos))?
                        .into_owned();
                    match stack.last_mut() {
                        Some(parent) => push_text(parent, txt),
                        None if txt.trim().is_empty() => {}
                        None => return Err(xml_err("text outside the root element", pos)),
                    }
                }
                Event::CData(c) => {
                    let raw = std::str::from_utf8(&c).map_err(|e| xml_err(e, pos))?;
                    let txt = normalize_newlines(raw);
                    match stack.last_mut() {
                        Some(parent) => push_text(parent, txt),
                        None => return Err(xml_err("CDATA outside the root element", pos)),
                    }
                }
                Event::Comment(c) => {
                    if let Some(parent) = stack.last_mut() {
                        let raw = std::str::from_utf8(&c).map_err(|e| xml_err(e, pos))?;
                        parent.children.push(Node::Comment(raw.to_owned()));
                    }
                }
                Event::PI(p) => {
                    if let Some(parent) = stack.last_mut() {
                        let raw = std::str::from_utf8(&p).map_err(|e| xml_err(e, pos))?;
                        parent.children.push(Node::Pi(raw.to_owned()));
                    }
                }
                Event::Decl(_) | Event::DocType(_) => {}
            }
        }

        if !stack.is_empty() {
            return Err(Error::Xml(
                "unexpected end of document (unclosed element)".into(),
            ));
        }
        root.take()
            .ok_or_else(|| Error::Xml("document is empty".into()))
    }
}

// ── Serialisation ─────────────────────────────────────────────────────────────

fn escape_text(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
}

fn escape_attr(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push_str("&#9;"),
            c => out.push(c),
        }
    }
}

/// Find (or create, by pushing a declaration) the prefix to use for `uri`.
///
/// `hint` is the prefix the element/attribute would like to have. Returns the
/// prefix to write (`None` = default namespace).
struct XmlSerializer {
    scope: Vec<NsDecl>,
    prefix_counter: usize,
    out: String,
}

impl XmlSerializer {
    fn new() -> Self {
        Self {
            scope: Vec::new(),
            prefix_counter: 0,
            out: String::new(),
        }
    }

    fn into_string(self) -> String {
        self.out
    }

    fn pick_prefix(
        &mut self,
        decls: &mut Vec<NsDecl>,
        hint: Option<&str>,
        uri: &str,
        is_attr: bool,
    ) -> Option<String> {
        // 1. The hinted prefix already means `uri` here.
        match hint {
            Some(h) if lookup(&self.scope, Some(h)) == Some(uri) => return Some(h.to_owned()),
            None if !is_attr && lookup(&self.scope, None) == Some(uri) => return None,
            _ => {}
        }
        // 2. Some other (visible) prefix already means `uri`.
        let candidate = self
            .scope
            .iter()
            .rev()
            .filter(|(p, u)| p.is_some() && u == uri)
            .map(|(p, _)| p.clone())
            .find(|p| lookup(&self.scope, p.as_deref()) == Some(uri));
        if let Some(Some(p)) = candidate {
            return Some(p);
        }
        // 3. Declare it here.
        let (prefix, decl): (Option<String>, NsDecl) = match hint {
            Some(h) if lookup(&self.scope, Some(h)).is_none() => {
                (Some(h.to_owned()), (Some(h.to_owned()), uri.to_owned()))
            }
            None if !is_attr => (None, (None, uri.to_owned())),
            _ => {
                let mut p;
                loop {
                    self.prefix_counter += 1;
                    p = format!("ns{}", self.prefix_counter);
                    if lookup(&self.scope, Some(&p)).is_none() {
                        break;
                    }
                }
                (Some(p.clone()), (Some(p), uri.to_owned()))
            }
        };
        self.scope.push(decl.clone());
        decls.push(decl);
        prefix
    }

    fn qualified_name(&mut self, e: &Element, decls: &mut Vec<NsDecl>) -> String {
        // Element name.
        let eprefix = match e.ns.as_deref() {
            Some(uri) => self.pick_prefix(decls, e.prefix.as_deref(), uri, false),
            None => {
                // No namespace: make sure no default namespace leaks in.
                if lookup(&self.scope, None).is_some() {
                    let d: NsDecl = (None, String::new());
                    self.scope.push(d.clone());
                    decls.push(d);
                }
                None
            }
        };

        match eprefix {
            Some(p) => format!("{p}:{}", e.local),
            None => e.local.clone(),
        }
    }

    fn attribute_names(&mut self, e: &Element, decls: &mut Vec<NsDecl>) -> Vec<String> {
        let mut attr_names: Vec<String> = Vec::with_capacity(e.attrs.len());
        for a in &e.attrs {
            let name = match a.ns.as_deref() {
                Some(uri) => {
                    match self.pick_prefix(decls, a.prefix.as_deref(), uri, true) {
                        Some(p) => format!("{p}:{}", a.local),
                        None => a.local.clone(), // unreachable for attributes, kept defensive
                    }
                }
                None => a.local.clone(),
            };
            attr_names.push(name);
        }

        attr_names
    }

    fn write_declarations(&mut self, decls: &[NsDecl]) {
        for (p, u) in decls {
            match p {
                Some(p) => write!(self.out, " xmlns:{p}=\"").unwrap(),
                None => self.out.push_str(" xmlns=\""),
            }
            escape_attr(u, &mut self.out);
            self.out.push('\"');
        }
    }

    fn write_attributes(&mut self, attrs: &[Attr], names: &[String]) {
        for (a, name) in attrs.iter().zip(names) {
            write!(self.out, " {name}=\"").unwrap();
            escape_attr(&a.value, &mut self.out);
            self.out.push('\"');
        }
    }

    fn write_children(&mut self, children: &[Node]) {
        for node in children {
            match node {
                Node::Element(child) => self.write_element(child),
                Node::Text(text) => escape_text(text, &mut self.out),
                Node::Comment(comment) => {
                    self.out.push_str("<!--");
                    self.out.push_str(comment);
                    self.out.push_str("-->");
                }
                Node::Pi(pi) => {
                    self.out.push_str("<?");
                    self.out.push_str(pi);
                    self.out.push_str("?>");
                }
            }
        }
    }

    fn write_element(&mut self, e: &Element) {
        let mark = self.scope.len();
        let mut decls = e.nsdecls.clone();
        self.scope.extend(e.nsdecls.iter().cloned());
        let qname = self.qualified_name(e, &mut decls);
        let attr_names = self.attribute_names(e, &mut decls);

        self.out.push('<');
        self.out.push_str(&qname);
        self.write_declarations(&decls);
        self.write_attributes(&e.attrs, &attr_names);

        if e.children.is_empty() {
            self.out.push_str("/>");
        } else {
            self.out.push('>');
            self.write_children(&e.children);
            self.out.push_str("</");
            self.out.push_str(&qname);
            self.out.push('>');
        }
        self.scope.truncate(mark);
    }
}

/// Serialise `root` without an XML declaration.
pub fn to_string(root: &Element) -> String {
    let mut serializer = XmlSerializer::new();
    serializer.write_element(root);
    serializer.into_string()
}

/// Serialise `root` as a standalone UTF-8 document, with an XML declaration
/// (`<?xml version='1.0' encoding='UTF-8' standalone='yes'?>`), exactly like
/// `lxml.etree.tostring(root, xml_declaration=True, encoding="UTF-8",
/// standalone=True)`.
pub fn to_bytes(root: &Element) -> Vec<u8> {
    let mut serializer = XmlSerializer::new();
    serializer
        .out
        .push_str("<?xml version='1.0' encoding='UTF-8' standalone='yes'?>\n");
    serializer.write_element(root);
    serializer.into_string().into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_namespace_scopes_and_retains_entity_derived_newlines() {
        let root = parse(
            br#"<w:root xmlns:w="urn:word" xmlns="urn:default" w:role="editor"><child>one&#13;&#10;two</child></w:root>"#,
        )
        .expect("test XML is valid");

        assert!(root.is("urn:word", "root"));
        assert_eq!(root.attr(Some("urn:word"), "role"), Some("editor"));
        let child = root
            .find("urn:default", "child")
            .expect("default namespace child");
        assert_eq!(child.text(), "one\r\ntwo");
    }

    #[test]
    fn serializes_namespace_hints_and_escaped_content() {
        let mut root =
            Element::new(Some("urn:word"), Some("w"), "root").with_nsdecl(Some("w"), "urn:word");
        root.set_attr(None, None, "plain", "<&\"\n\t\r");

        let mut math = Element::new(Some("urn:math"), Some("m"), "math");
        math.children.push(Node::Text("x<y & z".into()));
        root.push(math);

        assert_eq!(
            to_string(&root),
            "<w:root xmlns:w=\"urn:word\" plain=\"&lt;&amp;&quot;&#10;&#9;&#13;\"><ns1:math xmlns:ns1=\"urn:math\">x&lt;y &amp; z</ns1:math></w:root>"
        );
        assert_eq!(
            to_bytes(&root),
            b"<?xml version='1.0' encoding='UTF-8' standalone='yes'?>\n<w:root xmlns:w=\"urn:word\" plain=\"&lt;&amp;&quot;&#10;&#9;&#13;\"><ns1:math xmlns:ns1=\"urn:math\">x&lt;y &amp; z</ns1:math></w:root>"
        );
    }

    #[test]
    fn preserves_comments_and_processing_instructions_in_elements() {
        let root =
            parse(b"<root><!--note--><?target value?><child/></root>").expect("test XML is valid");

        assert_eq!(
            root.children,
            vec![
                Node::Comment("note".into()),
                Node::Pi("target value".into()),
                Node::Element(Element::new(None, None, "child")),
            ]
        );
        assert_eq!(
            to_string(&root),
            "<root><!--note--><?target value?><child/></root>"
        );
    }
}
