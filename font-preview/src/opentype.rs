use druid::kurbo::Shape;
use norad::{GlyphName, Ufo};
use runebender_lib::BezCache;

use std::collections::HashMap;

use std::convert::TryInto;

pub(crate) fn test(font: &Ufo) {
    //let font = make_test_font();
    let thing = VirtualFont::new(font.clone());
    //let cmap = thing.make_cmap_table();
    let table = ttf_parser::cmap::parse(&thing.cmap())
        .unwrap()
        .next()
        .unwrap();
    table.codepoints(|c| eprintln!("{}", c));
}

fn debug_print_cmap(map: &[u8]) {
    for (i, slice) in map.chunks(2).enumerate() {
        if i % 8 == 0 {
            eprintln!("");
        }
        eprintln!("{:02X} {:02X}", slice[0], slice[1]);
    }
}

type GlyphId = u16;

#[derive(Debug, Clone)]
pub struct VirtualFont {
    ufo: Ufo,
    paths: BezCache,
    glyph_ids: HashMap<GlyphId, GlyphName>,
    cmap: Vec<u8>,
    hhea: Vec<u8>,
    hmtx: Vec<u8>,
}

impl VirtualFont {
    pub fn new(ufo: Ufo) -> Self {
        let mut paths = BezCache::default();
        paths.reset(&ufo, &|name| ufo.get_glyph(name));
        let glyph_ids = ufo
            .get_default_layer()
            .unwrap()
            .iter_contents()
            .enumerate()
            .map(|(i, glyph)| (i as u16, glyph.name.clone()))
            .collect();
        let cmap = make_cmap_table(&ufo);
        let (hhea, hmtx) = make_horiz_tables(&ufo, &paths);
        VirtualFont {
            ufo,
            paths,
            glyph_ids,
            cmap,
            hhea,
            hmtx,
        }
    }

    fn glyph_for_id(&self, id: GlyphId) -> Option<GlyphName> {
        self.glyph_ids.get(&id).cloned()
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
}

fn make_cmap_table(font: &Ufo) -> Vec<u8> {
    let mut data = font
        .get_default_layer()
        .unwrap()
        .iter_contents()
        .enumerate()
        .flat_map(|(i, glyph)| {
            glyph
                .codepoints
                .as_ref()
                .and_then(|v| v.first().copied())
                .map(|code| (code, i))
        })
        .flat_map(|(chr, glyph_id)| {
            let chr: u16 = (chr as u32).try_into().ok()?;
            let glyph_id: u16 = glyph_id.try_into().ok()?;
            Some((chr, glyph_id))
        })
        .collect::<Vec<_>>();

    data.sort();

    let mut start_codes = Vec::new();
    let mut end_codes: Vec<u16> = Vec::new();
    let mut offsets = Vec::new();
    let mut deltas = Vec::new();
    let mut glyph_ids = Vec::with_capacity(data.len() + 1);
    //glyph_ids.push(0_u16);

    for (chr, glyph_id) in data.iter() {
        if end_codes.last().map(|c| c + 1) == Some(*chr) {
            *end_codes.last_mut().unwrap() += 1;
        } else {
            start_codes.push(*chr);
            end_codes.push(*chr);
            let delta = glyph_ids.len() as isize - *chr as isize;
            deltas.push(delta as i16);
            offsets.push(0_u16);
        }
        glyph_ids.push(*glyph_id);
    }

    // and required end segment
    start_codes.push(0xffff);
    end_codes.push(0xffff);
    deltas.push(1);
    offsets.push(0);

    let length = 16 + start_codes.len() * 2 * 4 + glyph_ids.len() * 2;
    let length = length as u16;
    //for i in 0..start_codes.len() {
    //eprintln!("{}..{} {}, {}", start_codes[i], end_codes[i], deltas[i], offsets[i]);
    //}
    //eprintln!("len: {}", length);
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
    glyph_ids
        .iter()
        .for_each(|int| result.extend_from_slice(&int.to_be_bytes()));
    result
}

fn make_horiz_tables(font: &Ufo, paths: &BezCache) -> (Vec<u8>, Vec<u8>) {
    let records = font
        .get_default_layer()
        .unwrap()
        .iter_contents()
        .map(|glyph| HorizontalMetricRecord {
            advance_width: glyph
                .advance_width()
                .map(|adv| adv as u16)
                .unwrap_or_default(),
            left_side_bearing: paths
                .get(&glyph.name)
                .map(|path| path.bounding_box().x0 as i16)
                .unwrap_or_default(),
        })
        .collect();
    let metrics = HorizontalMetrics {
        records,
        left_side_bearings: Vec::new(),
    };

    let hhea = HorizontalHeader {
        version: (1, 0),
        ascender: font
            .font_info
            .as_ref()
            .and_then(|info| info.ascender.map(|n| n.get() as i16))
            .unwrap_or_default(),
        descender: font
            .font_info
            .as_ref()
            .and_then(|info| info.descender.map(|n| n.get() as i16))
            .unwrap_or_default(),
        line_gap: font
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
