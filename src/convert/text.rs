// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use svgdom::{
    self,
    FuzzyEq,
};

use tree;

use short::{
    AId,
    EId,
};

use traits::{
    GetValue,
};

use super::{
    fill,
    stroke,
};


pub fn convert(
    text_elem: &svgdom::Node,
    depth: usize,
    doc: &mut tree::RenderTree,
) {
    let attrs = text_elem.attributes();
    let ts = attrs.get_transform(AId::Transform).unwrap_or_default();

    doc.append_node(depth, tree::NodeKind::Text(tree::Text {
        id: text_elem.id().clone(),
        transform: ts,
    }));

    convert_chunks(text_elem, depth + 1, doc);
}

fn convert_chunks(
    text_elem: &svgdom::Node,
    depth: usize,
    doc: &mut tree::RenderTree,
) {
    let ref root_attrs = text_elem.attributes();
    let mut prev_x = resolve_pos(root_attrs, AId::X).unwrap_or(0.0);
    let mut prev_y = resolve_pos(root_attrs, AId::Y).unwrap_or(0.0);

    doc.append_node(depth, tree::NodeKind::TextChunk(tree::TextChunk {
        x: prev_x,
        y: prev_y,
        anchor: conv_text_anchor(root_attrs),
    }));

    for tspan in text_elem.children() {
        debug_assert!(tspan.is_tag_name(EId::Tspan));

        let text = if let Some(node) = tspan.first_child() {
            node.text().clone()
        } else {
            continue;
        };


        let ref attrs = tspan.attributes();
        let x = resolve_pos(attrs, AId::X);
        let y = resolve_pos(attrs, AId::Y);

        if x.is_some() || y.is_some() {
            let tx = x.unwrap_or(prev_x);
            let ty = y.unwrap_or(prev_y);

            if tx.fuzzy_ne(&prev_x) || ty.fuzzy_ne(&prev_y) {
                doc.append_node(depth, tree::NodeKind::TextChunk(tree::TextChunk {
                    x: tx,
                    y: ty,
                    anchor: conv_text_anchor(attrs),
                }));
            }

            prev_x = tx;
            prev_y = ty;
        }

        let fill = fill::convert(doc, attrs);
        let stroke = stroke::convert(doc, attrs);
        let decoration = conv_tspan_decoration2(doc, text_elem, &tspan);
        doc.append_node(depth + 1, tree::NodeKind::TSpan(tree::TSpan {
            fill,
            stroke,
            font: convert_font(attrs),
            decoration,
            text,
        }));
    }
}

fn resolve_pos(
    attrs: &svgdom::Attributes,
    aid: AId,
) -> Option<f64> {
    if let Some(ref list) = attrs.get_number_list(aid) {
        if !list.is_empty() {
            if list.len() > 1 {
                warn!("List of 'x', 'y' coordinates are not supported in a 'text' element.");
            }

            return Some(list[0]);
        }
    }

    None
}

struct TextDecoTypes {
    has_underline: bool,
    has_overline: bool,
    has_line_through: bool,
}

// 'text-decoration' defined in the 'text' element
// should be generated by 'prepare_text_decoration'.
fn conv_text_decoration(node: &svgdom::Node) -> TextDecoTypes {
    debug_assert!(node.is_tag_name(EId::Text));

    let attrs = node.attributes();

    let def = String::new();
    let text = attrs.get_string(AId::TextDecoration).unwrap_or(&def);

    TextDecoTypes {
        has_underline: text.contains("underline"),
        has_overline: text.contains("overline"),
        has_line_through: text.contains("linethrough"),
    }
}

// 'text-decoration' in 'tspan' does not depend on parent elements.
fn conv_tspan_decoration(tspan: &svgdom::Node) -> TextDecoTypes {
    debug_assert!(tspan.is_tag_name(EId::Tspan));

    let attrs = tspan.attributes();

    let has_attr = |decoration_id: svgdom::ValueId| {
        if let Some(id) = attrs.get_predef(AId::TextDecoration) {
            if id == decoration_id {
                return true;
            }
        }

        false
    };

    TextDecoTypes {
        has_underline: has_attr(svgdom::ValueId::Underline),
        has_overline: has_attr(svgdom::ValueId::Overline),
        has_line_through: has_attr(svgdom::ValueId::LineThrough),
    }
}

fn conv_tspan_decoration2(
    doc: &tree::RenderTree,
    node: &svgdom::Node,
    tspan: &svgdom::Node
) -> tree::TextDecoration {
    let text_dec = conv_text_decoration(node);
    let tspan_dec = conv_tspan_decoration(tspan);

    let gen_style = |in_tspan: bool, in_text: bool| {
        let n = if in_tspan {
            tspan.clone()
        } else if in_text {
            node.clone()
        } else {
            return None;
        };

        let ref attrs = n.attributes();
        let fill = fill::convert(doc, attrs);
        let stroke = stroke::convert(doc, attrs);

        Some(tree::TextDecorationStyle {
            fill,
            stroke,
        })
    };

    tree::TextDecoration {
        underline: gen_style(tspan_dec.has_underline, text_dec.has_underline),
        overline: gen_style(tspan_dec.has_overline, text_dec.has_overline),
        line_through: gen_style(tspan_dec.has_line_through, text_dec.has_line_through),
    }
}

fn conv_text_anchor(attrs: &svgdom::Attributes) -> tree::TextAnchor {
    let av = attrs.get_predef(AId::TextAnchor).unwrap_or(svgdom::ValueId::Start);

    match av {
        svgdom::ValueId::Start => tree::TextAnchor::Start,
        svgdom::ValueId::Middle => tree::TextAnchor::Middle,
        svgdom::ValueId::End => tree::TextAnchor::End,
        _ => tree::TextAnchor::Start,
    }
}

fn convert_font(attrs: &svgdom::Attributes) -> tree::Font {
    let style = attrs.get_predef(AId::FontStyle).unwrap_or(svgdom::ValueId::Normal);
    let style = match style {
        svgdom::ValueId::Normal => tree::FontStyle::Normal,
        svgdom::ValueId::Italic => tree::FontStyle::Italic,
        svgdom::ValueId::Oblique => tree::FontStyle::Oblique,
        _ => tree::FontStyle::Normal,
    };

    let variant = attrs.get_predef(AId::FontVariant).unwrap_or(svgdom::ValueId::Normal);
    let variant = match variant {
        svgdom::ValueId::Normal => tree::FontVariant::Normal,
        svgdom::ValueId::SmallCaps => tree::FontVariant::SmallCaps,
        _ => tree::FontVariant::Normal,
    };

    let weight = attrs.get_predef(AId::FontWeight).unwrap_or(svgdom::ValueId::Normal);
    let weight = match weight {
        svgdom::ValueId::Normal => tree::FontWeight::Normal,
        svgdom::ValueId::Bold => tree::FontWeight::Bold,
        svgdom::ValueId::Bolder => tree::FontWeight::Bolder,
        svgdom::ValueId::Lighter => tree::FontWeight::Lighter,
        svgdom::ValueId::N100 => tree::FontWeight::W100,
        svgdom::ValueId::N200 => tree::FontWeight::W200,
        svgdom::ValueId::N300 => tree::FontWeight::W300,
        svgdom::ValueId::N400 => tree::FontWeight::W400,
        svgdom::ValueId::N500 => tree::FontWeight::W500,
        svgdom::ValueId::N600 => tree::FontWeight::W600,
        svgdom::ValueId::N700 => tree::FontWeight::W700,
        svgdom::ValueId::N800 => tree::FontWeight::W800,
        svgdom::ValueId::N900 => tree::FontWeight::W900,
        _ => tree::FontWeight::Normal,
    };

    let stretch = attrs.get_predef(AId::FontStretch).unwrap_or(svgdom::ValueId::Normal);
    let stretch = match stretch {
        svgdom::ValueId::Normal => tree::FontStretch::Normal,
        svgdom::ValueId::Wider => tree::FontStretch::Wider,
        svgdom::ValueId::Narrower => tree::FontStretch::Narrower,
        svgdom::ValueId::UltraCondensed => tree::FontStretch::UltraCondensed,
        svgdom::ValueId::ExtraCondensed => tree::FontStretch::ExtraCondensed,
        svgdom::ValueId::Condensed => tree::FontStretch::Condensed,
        svgdom::ValueId::SemiCondensed => tree::FontStretch::SemiCondensed,
        svgdom::ValueId::SemiExpanded => tree::FontStretch::SemiExpanded,
        svgdom::ValueId::Expanded => tree::FontStretch::Expanded,
        svgdom::ValueId::ExtraExpanded => tree::FontStretch::ExtraExpanded,
        svgdom::ValueId::UltraExpanded => tree::FontStretch::UltraExpanded,
        _ => tree::FontStretch::Normal,
    };

    // TODO: remove text nodes with font-size <= 0
    let size = attrs.get_number(AId::FontSize).unwrap_or(::DEFAULT_FONT_SIZE);
    debug_assert!(size > 0.0);

    let family = attrs.get_string(AId::FontFamily)
                      .unwrap_or(&::DEFAULT_FONT_FAMILY.to_owned()).clone();

    tree::Font {
        family,
        size,
        style,
        variant,
        weight,
        stretch,
    }
}
