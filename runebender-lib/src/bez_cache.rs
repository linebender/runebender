//! A mapping of component ids to glyphs that contain that id
//!
//! this is used to invalidate glyphs appropriately when components change

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use druid::kurbo::{Affine, BezPath};
use druid::Data;
use norad::{Glyph, GlyphName, Ufo};

const PRE_CACHE_SIZE: usize = 8;

/// A cache of up to date paths for each glyph.
///
/// This plays nicely with `Data` by employing a cheap-to-clone pre-cache
/// layer that will prevent needing to actually query the hashmap too often.
#[derive(Debug, Clone, Default, Data)]
pub struct BezCache {
    beziers: Arc<HashMap<GlyphName, Arc<BezPath>>>,
    pre_cache: PreCache,
    components: ComponentMap,
}

/// Tracks what glyphs are used as components in what other glyphs
#[derive(Debug, Clone, Data, Default)]
pub(crate) struct ComponentMap {
    inner: Arc<HashMap<GlyphName, Vec<GlyphName>>>,
}

/// a small array of cache entries to prevent unnecessary cloning
#[derive(Debug, Clone, Default, Data)]
struct PreCache {
    store: [Option<(GlyphName, Arc<BezPath>)>; PRE_CACHE_SIZE],
    len: usize,
}

impl PreCache {
    fn is_full(&self) -> bool {
        self.len == PRE_CACHE_SIZE
    }

    #[inline]
    fn idx_for_key(&self, key: &GlyphName) -> Option<usize> {
        self.store.iter().flatten().position(|item| item.0 == *key)
    }

    fn get(&self, key: &GlyphName) -> Option<Arc<BezPath>> {
        self.store
            .iter()
            .flatten()
            .find(|item| &item.0 == key)
            .map(|(_, bez)| bez)
            .cloned()
    }

    fn remove(&mut self, key: &GlyphName) {
        if let Some(idx) = self.idx_for_key(key) {
            let last_idx = self.len - 1;
            self.store.swap(idx, last_idx);
            self.store[last_idx] = None;
            self.len -= 1;
            if self.store.iter().take(self.len).any(Option::is_none) {
                panic!("remove failed, idx {} last_idx {}", idx, last_idx);
            }
        }
    }

    /// If the item cannot be inserted, it is returned in the error.
    fn try_insert(
        &mut self,
        key: GlyphName,
        value: Arc<BezPath>,
    ) -> Result<(), (GlyphName, Arc<BezPath>)> {
        match self.idx_for_key(&key) {
            Some(idx) => self.store[idx] = Some((key, value)),
            None if self.is_full() => return Err((key, value)),
            None => {
                assert!(!self.is_full());
                self.store[self.len] = Some((key, value));
                self.len += 1;
            }
        }
        Ok(())
    }

    fn drain(&mut self) -> impl Iterator<Item = (GlyphName, Arc<BezPath>)> {
        let items = std::mem::take(&mut self.store);
        let len = self.len;
        self.len = 0;
        let mut idx = 0;
        std::iter::from_fn(move || {
            if idx == len {
                None
            } else {
                idx += 1;
                assert!(items[idx - 1].as_ref().is_some(), "idx {} len {}", idx, len);
                Some(items[idx - 1].as_ref().unwrap().clone())
            }
        })
    }
}

impl BezCache {
    pub fn reset<'a, F>(&mut self, ufo: &Ufo, getter: &'a F)
    where
        F: Fn(&GlyphName) -> Option<&'a Arc<Glyph>> + 'a,
    {
        self.components = ComponentMap::new(ufo);
        self.pre_cache = Default::default();
        for name in ufo.iter_names() {
            self.rebuild_without_inval(&name, getter);
        }
    }

    pub fn get(&self, name: &GlyphName) -> Option<Arc<BezPath>> {
        self.pre_cache
            .get(name)
            .or_else(|| self.beziers.get(name).cloned())
    }

    pub fn set(&mut self, name: GlyphName, path: Arc<BezPath>) {
        let result = self.pre_cache.try_insert(name, path);
        // we need to actually hit the main cache
        if let Err((name, path)) = result {
            let cache = Arc::make_mut(&mut self.beziers);
            cache.insert(name, path);
            cache.extend(self.pre_cache.drain());
        }
    }

    pub fn invalidate(&mut self, name: &GlyphName) {
        let cache = Arc::make_mut(&mut self.beziers);
        for glyph in self.components.glyphs_containing_component(name).iter() {
            self.pre_cache.remove(glyph);
            cache.remove(glyph);
        }
    }

    pub fn rebuild<'a, F>(&mut self, name: &GlyphName, glyph_getter: &'a F) -> Option<Arc<BezPath>>
    where
        F: Fn(&GlyphName) -> Option<&'a Arc<Glyph>> + 'a,
    {
        self.invalidate(name);
        self.rebuild_without_inval(name, glyph_getter)
    }

    fn rebuild_without_inval<'a, F>(
        &mut self,
        name: &GlyphName,
        glyph_getter: &'a F,
    ) -> Option<Arc<BezPath>>
    where
        F: Fn(&GlyphName) -> Option<&'a Arc<Glyph>> + 'a,
    {
        let glyph = glyph_getter(name)?;
        let mut path = crate::data::path_for_glyph(glyph)?;

        for comp in glyph
            .outline
            .as_ref()
            .iter()
            .flat_map(|o| o.components.iter())
        {
            match self.rebuild_without_inval(&comp.base, glyph_getter) {
                Some(component) => {
                    let affine: Affine = comp.transform.into();
                    for comp_elem in (affine * &*component).elements() {
                        path.push(*comp_elem);
                    }
                }
                None => log::warn!("missing component {} in glyph {}", comp.base, glyph.name),
            }
        }
        let path = Arc::new(path);
        self.set(name.clone(), path.clone());
        Some(path)
    }

    pub(crate) fn glyphs_containing_component<'a>(
        &'a self,
        name: &GlyphName,
    ) -> Cow<'a, [GlyphName]> {
        self.components.glyphs_containing_component(name)
    }
}

impl ComponentMap {
    fn new(ufo: &Ufo) -> Self {
        let mut lookup: HashMap<GlyphName, Vec<GlyphName>> = HashMap::new();
        for name in ufo.iter_names() {
            if let Some(glyph) = ufo.get_glyph(&name) {
                for component in glyph
                    .outline
                    .as_ref()
                    .iter()
                    .flat_map(|o| o.components.iter())
                {
                    lookup
                        .entry(component.base.clone())
                        .or_default()
                        .push(name.clone());
                }
            }
        }

        ComponentMap {
            inner: Arc::new(lookup),
        }
    }

    fn glyphs_containing_component<'a>(&'a self, name: &GlyphName) -> Cow<'a, [GlyphName]> {
        if let Some(glyphs) = self.inner.get(name) {
            let mut component_children = glyphs
                .iter()
                .flat_map(|g| self.inner.get(g))
                .flat_map(|g| g.iter().cloned())
                .collect::<Vec<_>>();
            if component_children.is_empty() {
                Cow::Borrowed(glyphs.as_slice())
            } else {
                component_children.extend_from_slice(glyphs);
                Cow::Owned(component_children)
            }
        } else {
            Cow::Owned(Vec::new())
        }
    }
}
