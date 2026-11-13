//! OMML script and prescript encoders.

use crate::error::Result;
use crate::mathml::MathNode;
use crate::xml::Element;

use super::{as_nary_op, is_empty_node, m_el, m_el_children, MathmlEncoder};

impl MathmlEncoder {
    pub(super) fn encode_msub(&mut self, node: &MathNode) -> Result<Element> {
        if let Some((chr, lim_loc, sub_opt, sup_opt)) = as_nary_op(node) {
            return self.build_nary(chr, lim_loc, sub_opt, sup_opt, Vec::new());
        }
        let base_els = match node.children.first() {
            Some(c) => self.encode_node(c)?,
            None => Vec::new(),
        };
        let sub_els = match node.children.get(1) {
            Some(c) => self.encode_node(c)?,
            None => Vec::new(),
        };
        let mut s_sub = m_el("sSub");
        s_sub.push(m_el_children("e", base_els));
        s_sub.push(m_el_children("sub", sub_els));
        Ok(s_sub)
    }

    pub(super) fn encode_msup(&mut self, node: &MathNode) -> Result<Element> {
        if let Some((chr, lim_loc, sub_opt, sup_opt)) = as_nary_op(node) {
            return self.build_nary(chr, lim_loc, sub_opt, sup_opt, Vec::new());
        }
        let base_els = match node.children.first() {
            Some(c) => self.encode_node(c)?,
            None => Vec::new(),
        };
        let sup_els = match node.children.get(1) {
            Some(c) => self.encode_node(c)?,
            None => Vec::new(),
        };
        let mut s_sup = m_el("sSup");
        s_sup.push(m_el_children("e", base_els));
        s_sup.push(m_el_children("sup", sup_els));
        Ok(s_sup)
    }

    pub(super) fn encode_msubsup(&mut self, node: &MathNode) -> Result<Element> {
        if let Some((chr, lim_loc, sub_opt, sup_opt)) = as_nary_op(node) {
            return self.build_nary(chr, lim_loc, sub_opt, sup_opt, Vec::new());
        }
        let base_els = node
            .children
            .first()
            .map(|child| self.encode_node(child))
            .transpose()?
            .unwrap_or_default();
        let sub_els = node
            .children
            .get(1)
            .map(|child| self.encode_node(child))
            .transpose()?
            .unwrap_or_default();
        let sup_els = node
            .children
            .get(2)
            .map(|child| self.encode_node(child))
            .transpose()?
            .unwrap_or_default();
        let mut s_subsup = m_el("sSubSup");
        s_subsup.push(m_el_children("e", base_els));
        s_subsup.push(m_el_children("sub", sub_els));
        s_subsup.push(m_el_children("sup", sup_els));
        Ok(s_subsup)
    }

    pub(super) fn encode_mmultiscripts(&mut self, node: &MathNode) -> Result<Vec<Element>> {
        if node.children.is_empty() {
            return Ok(Vec::new());
        }
        let base = &node.children[0];
        let mut post_pairs = Vec::new();
        let mut pre_pairs = Vec::new();
        let mut in_pre = false;
        let mut index = 1;
        while index < node.children.len() {
            if node.children[index].tag == "mprescripts" {
                in_pre = true;
                index += 1;
            } else if index + 1 < node.children.len() {
                let pair = (&node.children[index], &node.children[index + 1]);
                if in_pre {
                    pre_pairs.push(pair);
                } else {
                    post_pairs.push(pair);
                }
                index += 2;
            } else {
                break;
            }
        }

        let mut current = self.encode_node(base)?;
        for (sub_node, sup_node) in pre_pairs {
            let sub = self.encode_script_part(sub_node)?;
            let sup = self.encode_script_part(sup_node)?;
            if !sub.is_empty() || !sup.is_empty() {
                let mut pre = m_el("sPre");
                pre.push(m_el_children("sub", sub));
                pre.push(m_el_children("sup", sup));
                pre.push(m_el_children("e", current));
                current = vec![pre];
            }
        }
        for (sub_node, sup_node) in post_pairs {
            let sub = self.encode_script_part(sub_node)?;
            let sup = self.encode_script_part(sup_node)?;
            current = wrap_postscript(current, sub, sup);
        }
        Ok(current)
    }

    fn encode_script_part(&mut self, node: &MathNode) -> Result<Vec<Element>> {
        if node.tag == "none" || is_empty_node(node) {
            Ok(Vec::new())
        } else {
            self.encode_node(node)
        }
    }
}

fn wrap_postscript(base: Vec<Element>, sub: Vec<Element>, sup: Vec<Element>) -> Vec<Element> {
    match (!sub.is_empty(), !sup.is_empty()) {
        (true, true) => {
            let mut script = m_el("sSubSup");
            script.push(m_el_children("e", base));
            script.push(m_el_children("sub", sub));
            script.push(m_el_children("sup", sup));
            vec![script]
        }
        (true, false) => {
            let mut script = m_el("sSub");
            script.push(m_el_children("e", base));
            script.push(m_el_children("sub", sub));
            vec![script]
        }
        (false, true) => {
            let mut script = m_el("sSup");
            script.push(m_el_children("e", base));
            script.push(m_el_children("sup", sup));
            vec![script]
        }
        (false, false) => base,
    }
}
