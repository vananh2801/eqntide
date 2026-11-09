//! A tiny MathML tree — the intermediate form between OMML and MTEF.
//!
//! The Python version passes `lxml` elements between the two stages. Only a
//! handful of facts are ever consulted (local tag name, leaf text, a couple of
//! attributes, children), so a plain owned tree is enough and much lighter.

use crate::error::Result;
use crate::xml;

/// One MathML element.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MathNode {
    /// Local tag name (`mi`, `mfrac`, `math`, …). MathML's namespace is implied.
    pub tag: String,
    /// Character data before the first child (lxml's `.text`).
    pub text: String,
    /// Attributes (`mathvariant`, `linethickness`, …).
    pub attrs: Vec<(String, String)>,
    /// Child elements.
    pub children: Vec<MathNode>,
}

impl MathNode {
    /// An empty element.
    pub fn new(tag: &str) -> Self {
        MathNode {
            tag: tag.to_owned(),
            ..Default::default()
        }
    }

    /// A leaf element carrying `text`.
    pub fn leaf(tag: &str, text: &str) -> Self {
        MathNode {
            tag: tag.to_owned(),
            text: text.to_owned(),
            ..Default::default()
        }
    }

    /// An element with the given children.
    pub fn with_children(tag: &str, children: Vec<MathNode>) -> Self {
        MathNode {
            tag: tag.to_owned(),
            children,
            ..Default::default()
        }
    }

    /// Set (or replace) an attribute.
    pub fn set_attr(&mut self, name: &str, value: &str) {
        if let Some(a) = self.attrs.iter_mut().find(|(k, _)| k == name) {
            a.1 = value.to_owned();
        } else {
            self.attrs.push((name.to_owned(), value.to_owned()));
        }
    }

    /// Read an attribute.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// This node followed by all descendants, in document order.
    pub fn iter(&self) -> Vec<&MathNode> {
        fn walk<'a>(n: &'a MathNode, out: &mut Vec<&'a MathNode>) {
            out.push(n);
            for c in &n.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(self, &mut out);
        out
    }

    /// All *descendants* named `tag` (lxml's `el.findall(".//{mml}tag")`).
    pub fn find_all(&self, tag: &str) -> Vec<&MathNode> {
        self.iter()
            .into_iter()
            .skip(1)
            .filter(|n| n.tag == tag)
            .collect()
    }

    /// Convert a parsed XML element (e.g. a hand-written `<math>` snippet).
    pub fn from_xml(el: &xml::Element) -> MathNode {
        MathNode {
            tag: el.local.clone(),
            text: el.text(),
            attrs: el
                .attrs
                .iter()
                .map(|a| (a.local.clone(), a.value.clone()))
                .collect(),
            children: el.elements().map(MathNode::from_xml).collect(),
        }
    }

    /// Parse MathML source text into a tree.
    pub fn parse(src: &str) -> Result<MathNode> {
        Ok(MathNode::from_xml(&xml::parse(src.as_bytes())?))
    }

    /// Convert this [`MathNode`] tree into an [`xml::Element`].
    pub fn to_xml_element(&self) -> xml::Element {
        let mut el = xml::Element::new(None, None, &self.tag);
        for (k, v) in &self.attrs {
            el.set_attr(None, None, k, v);
        }
        if !self.text.is_empty() {
            el.children.push(xml::Node::Text(self.text.clone()));
        }
        for child in &self.children {
            el.children.push(xml::Node::Element(child.to_xml_element()));
        }
        el
    }

    /// Serialise this [`MathNode`] tree to a MathML XML string.
    pub fn to_xml(&self) -> String {
        xml::to_string(&self.to_xml_element())
    }

    /// Serialise this [`MathNode`] tree to a LaTeX mathematical formula.
    pub fn to_latex(&self) -> String {
        crate::mathml_to_latex::mathml_to_latex(self)
    }
}
