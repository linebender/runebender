use norad::{GlyphName, Ufo};
use std::collections::HashMap;

use std::convert::TryInto;

pub(crate) fn test(font: &Ufo) {
    //let font = make_test_font();
    let thing = UfOtf::new(font.clone());
    let cmap = thing.make_cmap_table();
    let table = ttf_parser::cmap::parse(&cmap).unwrap().next().unwrap();
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

pub struct UfOtf {
    ufo: Ufo,
    glyph_ids: HashMap<GlyphId, GlyphName>,
}

impl UfOtf {
    pub fn new(ufo: Ufo) -> Self {
        let glyph_ids = ufo
            .get_default_layer()
            .unwrap()
            .iter_contents()
            .enumerate()
            .map(|(i, glyph)| (i as u16, glyph.name.clone()))
            .collect();
        UfOtf { ufo, glyph_ids }
    }

    fn glyph_for_id(&self, id: GlyphId) -> Option<GlyphName> {
        self.glyph_ids.get(&id).cloned()
    }

    pub fn make_cmap_table(&self) -> Vec<u8> {
        let mut data = self
            .ufo
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
}

//fn make_test_font() -> Ufo {
//let mut ufo = norad::Ufo::new();
//ufo.font_info = norad::FontInfo {
//family_name: Some("Untitled".into()),
//style_name: Some("Regular".into()),
//units_per_em: Some(TryFrom::try_from(1000.0f64).unwrap()),
//descender: Some(From::from(-200.0)),
//ascender: Some(800.0.into()),
//cap_height: Some(700.0.into()),
//x_height: Some(500.0.into()),
//..Default::default()
//}
//.into();

//let layer = ufo.get_default_layer_mut().unwrap();
//let mut glyph = norad::Glyph::new_named("A".to_string());
//glyph.codepoints = Some(vec!['A']);
//layer.insert_glyph(glyph);
//ufo
//}
