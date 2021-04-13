use druid::kurbo::Shape;
use norad::{GlyphName, Ufo};

use crate::data::Workspace;

use std::convert::TryInto;

use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};

pub type GlyphId = u16;

/// An object that acts like a font loaded from disk.
///
/// This lets us interact with harfbuzz as if we were just a normal compiled
/// font file.
#[derive(Debug, Clone, Default)]
pub struct VirtualFont {
    glyph_ids: Vec<(char, GlyphName)>,
    cmap: Vec<u8>,
    hhea: Vec<u8>,
    hmtx: Vec<u8>,
    kern: Vec<u8>,
}

/// Given a ufo, generate a vector of (codepoint, glyph name) pairs,
/// sorted by codepoint.
///
/// This is used as our 'glyphid' table.
///
/// NOTE:
///
/// Although multiple codepoints can map to the same glyph, we do
/// not actually handle this well in practice.
fn glyph_ids(font: &Ufo) -> Vec<(char, GlyphName)> {
    let mut chars_and_names = Vec::with_capacity(font.glyph_count() + 1);
    chars_and_names.push(('\0', GlyphName::from(".notdef")));
    for glyph in font
        .get_default_layer()
        .iter()
        .flat_map(|layer| layer.iter_contents())
    {
        for codepoint in glyph.codepoints.as_ref().iter().flat_map(|cps| cps.iter()) {
            chars_and_names.push((*codepoint, glyph.name.clone()));
        }
    }
    chars_and_names.sort();
    chars_and_names
}

impl VirtualFont {
    /// Given a loaded [`Ufo`] object, resolve the glyphs and generate
    /// the font tables.
    pub fn new(workspace: &Workspace) -> Self {
        let glyph_ids = glyph_ids(&workspace.font.ufo);
        let cmap = make_cmap_table(&glyph_ids);
        let (hhea, hmtx) = make_horiz_tables(workspace, &glyph_ids);
        let kern = make_kern_table(workspace, &glyph_ids);
        VirtualFont {
            glyph_ids,
            cmap,
            hhea,
            hmtx,
            kern,
        }
    }

    //#[allow(dead_code)]
    //pub(crate) fn test_tables(&self) {
    //let table = ttf_parser::cmap::parse(self.cmap())
    //.unwrap()
    //.next()
    //.unwrap();
    //table.codepoints(|c| eprintln!("{}", c));
    //for chr in &[' ', 'A', 'B', 'F', 'a', 'b', 'c'] {
    //eprintln!("{}: {:?}", chr, table.glyph_index(*chr as u32));
    //}
    //}

    pub(crate) fn glyph_for_id(&self, id: GlyphId) -> Option<&GlyphName> {
        self.glyph_ids.get(id as usize).map(|(_, g)| g)
    }

    pub fn cmap(&self) -> &[u8] {
        &self.cmap
    }

    pub fn hhea(&self) -> &[u8] {
        &self.hhea
    }

    pub fn hmtx(&self) -> &[u8] {
        &self.hmtx
    }

    pub fn kern(&self) -> &[u8] {
        &self.kern
    }
}

fn make_cmap_table(glyphs: &[(char, GlyphName)]) -> Vec<u8> {
    let mut start_codes = Vec::new();
    let mut end_codes: Vec<u16> = Vec::new();
    let mut offsets = Vec::new();
    let mut deltas = Vec::new();

    for (i, (chr, _glyph_name)) in glyphs.iter().enumerate().skip(1) {
        let chr: u16 = (*chr as u32).try_into().unwrap();
        if end_codes.last().map(|c| c + 1) == Some(chr) {
            *end_codes.last_mut().unwrap() += 1;
        } else {
            start_codes.push(chr);
            end_codes.push(chr);
            let delta = i as isize - chr as isize;
            deltas.push(delta as i16);
            offsets.push(0_u16);
        }
    }

    // and required end segment
    start_codes.push(0xffff);
    end_codes.push(0xffff);
    deltas.push(1);
    offsets.push(0);

    let length = 16 + start_codes.len() * 2 * 4;
    let length = length as u16;
    let segment_count_x2 = (start_codes.len() * 2) as u16;

    let mut result = Vec::new();
    //header:
    result.extend(0_u16.to_be_bytes().iter()); // version
    result.extend(1_u16.to_be_bytes().iter()); // table_count
                                               // record:
    result.extend(0_u16.to_be_bytes().iter()); // platform_id
    result.extend(4_u16.to_be_bytes().iter()); // encoding_id
    result.extend(12_u32.to_be_bytes().iter()); // offset

    // encoding
    result.extend(4_u16.to_be_bytes().iter()); // format
    result.extend(length.to_be_bytes().iter());
    result.extend(0_u16.to_be_bytes().iter()); // language
    result.extend(segment_count_x2.to_be_bytes().iter());
    result.extend(0_u16.to_be_bytes().iter()); // search_range
    result.extend(0_u16.to_be_bytes().iter()); // entry_selector
    result.extend(0_u16.to_be_bytes().iter()); // range_shift
    end_codes
        .iter()
        .for_each(|int| result.extend_from_slice(&int.to_be_bytes()));
    result.extend(0_u16.to_be_bytes().iter()); // padding
    start_codes
        .iter()
        .for_each(|int| result.extend_from_slice(&int.to_be_bytes()));
    deltas
        .iter()
        .for_each(|int| result.extend_from_slice(&int.to_be_bytes()));
    offsets
        .iter()
        .for_each(|int| result.extend_from_slice(&int.to_be_bytes()));
    result
}

fn make_horiz_tables(
    workspace: &Workspace,
    glyphs: &[(char, GlyphName)],
    //paths: &BezCache,
) -> (Vec<u8>, Vec<u8>) {
    let records = glyphs
        .iter()
        .map(|(_, name)| {
            let advance_width = workspace
                .font
                .ufo
                .get_glyph(name)
                .and_then(|glyph| glyph.advance_width().map(|adv| adv as u16))
                .unwrap_or_default();
            HorizontalMetricRecord {
                advance_width,
                left_side_bearing: workspace
                    .get_bezier(name)
                    //.get(name)
                    .map(|path| path.bounding_box().x0 as i16)
                    .unwrap_or_default(),
            }
        })
        .collect();
    let metrics = HorizontalMetrics {
        records,
        left_side_bearings: Vec::new(),
    };

    let hhea = HorizontalHeader {
        version: (1, 0),
        ascender: workspace
            .info
            .font_metrics()
            .ascender
            .map(|n| n as i16)
            .unwrap_or_default(),
        descender: workspace
            .info
            .font_metrics()
            .descender
            .map(|n| n as i16)
            .unwrap_or_default(),
        line_gap: workspace
            .font
            .ufo
            .font_info
            .as_ref()
            .and_then(|info| info.open_type_hhea_line_gap)
            .unwrap_or_default() as i16,
        advance_width_max: metrics
            .records
            .iter()
            .map(|r| r.advance_width)
            .max()
            .unwrap_or_default(),
        left_side_bearing_min: metrics
            .records
            .iter()
            .map(|r| r.left_side_bearing)
            .min()
            .unwrap_or_default(),
        //FIXME: these are currently just made-up
        right_side_bearing_min: 6,
        max_x_extent: 900,
        caret_slope_rise: 1,
        caret_slope_run: 0,
        caret_offset: 0,
        reserved: 0,
        format: 0,
        number_of_h_metrics: metrics.records.len() as u16,
    };

    (hhea.encode(), metrics.encode())
}

struct HorizontalHeader {
    version: (u16, u16),
    ascender: i16,
    descender: i16,
    line_gap: i16,
    advance_width_max: u16,
    left_side_bearing_min: i16,
    right_side_bearing_min: i16,
    max_x_extent: i16,
    caret_slope_rise: i16,
    caret_slope_run: i16,
    caret_offset: i16,
    reserved: u64,
    format: i16,
    number_of_h_metrics: u16,
}

impl HorizontalHeader {
    fn encode(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(36);
        result.extend_from_slice(&self.version.0.to_be_bytes());
        result.extend_from_slice(&self.version.1.to_be_bytes());
        result.extend_from_slice(&self.ascender.to_be_bytes());
        result.extend_from_slice(&self.descender.to_be_bytes());
        result.extend_from_slice(&self.line_gap.to_be_bytes());
        result.extend_from_slice(&self.advance_width_max.to_be_bytes());
        result.extend_from_slice(&self.left_side_bearing_min.to_be_bytes());
        result.extend_from_slice(&self.right_side_bearing_min.to_be_bytes());
        result.extend_from_slice(&self.max_x_extent.to_be_bytes());
        result.extend_from_slice(&self.caret_slope_rise.to_be_bytes());
        result.extend_from_slice(&self.caret_slope_run.to_be_bytes());
        result.extend_from_slice(&self.caret_offset.to_be_bytes());
        result.extend_from_slice(&self.reserved.to_be_bytes());
        result.extend_from_slice(&self.format.to_be_bytes());
        result.extend_from_slice(&self.number_of_h_metrics.to_be_bytes());
        result
    }
}

struct HorizontalMetricRecord {
    advance_width: u16,
    left_side_bearing: i16,
}

struct HorizontalMetrics {
    records: Vec<HorizontalMetricRecord>,
    left_side_bearings: Vec<i16>,
}

impl HorizontalMetrics {
    fn encode(&self) -> Vec<u8> {
        let len = self.records.len() * 4 + self.left_side_bearings.len() * 2;
        let mut result = Vec::with_capacity(len);

        for record in &self.records {
            result.extend_from_slice(&record.advance_width.to_be_bytes());
            result.extend_from_slice(&record.left_side_bearing.to_be_bytes());
        }

        for lsb in &self.left_side_bearings {
            result.extend_from_slice(&lsb.to_be_bytes());
        }
        result
    }
}

fn expand_group(
    prefix: &str,
    workspace: &Workspace,
    glyphs: &[(char, GlyphName)],
    k: &str,
) -> Vec<u16> {
    if k.starts_with(prefix) {
        if let Some(groups) = &workspace.font.ufo.groups {
            match groups.get(k) {
                Some(v) => v
                    .iter()
                    .filter_map(|x| {
                        glyphs
                            .iter()
                            .position(|(_, n)| (*n).to_string() == (*x).to_string())
                            .map(|x| x as u16)
                    })
                    .collect(),
                None => vec![],
            }
        } else {
            vec![]
        }
    } else {
        match glyphs.iter().position(|(_, n)| (*n).to_string() == *k) {
            Some(id) => vec![id as u16],
            None => vec![],
        }
    }
}

fn make_kern_table(
    workspace: &Workspace,
    glyphs: &[(char, GlyphName)],
    //paths: &BezCache,
) -> Vec<u8> {
    let mut pairs = Vec::<KernFormat0Pair>::new();

    if let Some(kerning) = &workspace.font.ufo.kerning {
        for (kern1, kern2s) in kerning {
            let kern1ids = expand_group("public.kern1.", workspace, glyphs, kern1);
            let kern2kvs = kern2s
                .iter()
                .map(|(kern2, v)| (expand_group("public.kern2.", workspace, glyphs, kern2), *v))
                .collect::<Vec<(Vec<u16>, f32)>>();
            for k1id in kern1ids {
                for (k2ids, k2v) in &kern2kvs {
                    for k2id in k2ids {
                        pairs.push(KernFormat0Pair {
                            left: k1id,
                            right: *k2id,
                            value: *k2v as i16,
                        })
                    }
                }
            }
        }
    }

    KernTable {
        version: 0,
        subtables: vec![KernSubtable {
            version: 0,
            coverage: KernSubtableCoverage::default(),
            pairs,
        }],
    }
    .encode()
}

struct KernTable {
    version: u16,
    subtables: Vec<KernSubtable>,
}

impl KernTable {
    fn encode(&self) -> Vec<u8> {
        let mut result = Vec::<u8>::new();
        result.extend_from_slice(&self.version.to_be_bytes());
        result.extend_from_slice(&(self.subtables.len() as u16).to_be_bytes());
        for subtable in &self.subtables {
            result.extend(subtable.encode());
        }
        result
    }
}

struct KernSubtable {
    version: u16,
    coverage: KernSubtableCoverage,
    pairs: Vec<KernFormat0Pair>,
}

impl KernSubtable {
    fn encode(&self) -> Vec<u8> {
        let length = 6 + self.pairs.len() * 6;
        let mut result = Vec::with_capacity(length);
        result.extend_from_slice(&self.version.to_be_bytes());
        result.extend_from_slice(&length.to_be_bytes());
        result.extend_from_slice(&self.coverage.encode().to_be_bytes());

        let n_pairs = self.pairs.len() as u16;
        result.extend_from_slice(&n_pairs.to_be_bytes());
        let search_range: u16 = (1 << (15 - n_pairs.leading_zeros())) * 6;
        result.extend_from_slice(&search_range.to_be_bytes());
        let entry_selector: u16 = (1 << (16 - n_pairs.leading_zeros())) * 6;
        result.extend_from_slice(&entry_selector.to_be_bytes());
        let range_shift: u16 = n_pairs - search_range;
        result.extend_from_slice(&range_shift.to_be_bytes());
        let mut pairs = self.pairs.clone();
        pairs.sort();
        for pair in pairs {
            result.extend(pair.encode());
        }
        result
    }
}

struct KernSubtableCoverage {
    horizontal: bool,
    minimum: bool,
    cross_stream: bool,
    over_ride: bool,
    format: u8,
}

impl KernSubtableCoverage {
    fn default() -> Self {
        KernSubtableCoverage {
            horizontal: true,
            minimum: false,
            cross_stream: false,
            over_ride: false,
            format: 0,
        }
    }

    fn encode(&self) -> u16 {
        let mut r: u16 = 0;
        if self.horizontal {
            r |= 0b1;
        }
        if self.minimum {
            r |= 0b10;
        }
        if self.cross_stream {
            r |= 0b100;
        }
        if self.over_ride {
            r |= 0b1000;
        }
        r |= (self.format as u16) << 8;
        r
    }
}

#[derive(PartialEq, Eq, Clone)]
struct KernFormat0Pair {
    left: u16,
    right: u16,
    value: i16,
}

impl KernFormat0Pair {
    fn encode(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(6);
        result.extend_from_slice(&self.left.to_be_bytes());
        result.extend_from_slice(&self.right.to_be_bytes());
        result.extend_from_slice(&self.value.to_be_bytes());
        result
    }
}

impl PartialOrd for KernFormat0Pair {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KernFormat0Pair {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.left.cmp(&other.left) {
            Ordering::Equal => self.right.cmp(&other.right),
            o => o,
        }
    }
}
