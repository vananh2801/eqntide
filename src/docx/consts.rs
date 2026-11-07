//! XML namespaces, relationship types and package part names used by the
//! conversion.

// ── Namespaces ────────────────────────────────────────────────────────────────

pub(super) const NS_W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub(super) const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub(super) const NS_V: &str = "urn:schemas-microsoft-com:vml";
pub(super) const NS_O: &str = "urn:schemas-microsoft-com:office:office";
pub(super) const NS_MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
pub(super) const NS_PKG_REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
pub(super) const NS_CT: &str = "http://schemas.openxmlformats.org/package/2006/content-types";

pub(super) const REL_OLEOBJECT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject";
pub(super) const CT_OLEOBJECT: &str = "application/vnd.openxmlformats-officedocument.oleObject";

/// Custom namespace used in `Requires=` so that Word picks the OMML fallback.
pub(super) const MTYPE_NS: &str = "urn:mathtype:oleobject";

pub(super) const DOC_XML_PATH: &str = "word/document.xml";
pub(super) const CT_PATH: &str = "[Content_Types].xml";

pub(super) const EMPTY_RELS: &[u8] = br#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#;

pub(super) const REL_IMAGE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
