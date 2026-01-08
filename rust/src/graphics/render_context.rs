//! Render context registry for DCQ resource resolution.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::graphics::cmap::ColorMapInner as ColorMap;
use crate::graphics::font::FontPage;
use crate::graphics::tfb_draw::{Canvas, TFImage};

/// Identifies the render target screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum ScreenType {
    /// Primary main screen.
    Main = 0,
    /// Auxiliary extra screen.
    Extra = 1,
    /// Transition screen used for fades.
    Transition = 2,
}

impl ScreenType {
    const fn index(self) -> usize {
        self as usize
    }
}

/// Handle for a registered screen canvas.
#[derive(Debug, Clone)]
pub struct ScreenHandle {
    /// Unique identifier for the screen canvas.
    pub id: u32,
    /// Shared canvas storage for the screen.
    pub canvas: Arc<RwLock<Canvas>>,
}

impl ScreenHandle {
    /// Return the unique identifier for this screen.
    pub fn id(&self) -> u32 {
        self.id
    }
}

/// Resource metadata for lifecycle tracking.
#[derive(Debug, Clone)]
pub struct ResourceMetadata {
    /// Unique ID of the resource.
    pub id: u32,
    /// Type of resource.
    pub resource_type: ResourceType,
    /// Whether resource was explicitly registered vs auto-generated.
    pub explicit: bool,
    /// Reference count (for tracking usage).
    pub ref_count: u32,
    /// Creation time (optional, for debugging).
    pub created_at: Option<Instant>,
}

/// Type of tracked resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Screen,
    Canvas,
    Image,
    Font,
    ColorMap,
    DataPtr,
}

/// Helper structure for resource tracking info.
#[derive(Debug, Clone)]
pub struct ResourceTrackingInfo {
    pub total_resources: usize,
    pub screens: usize,
    pub canvases: usize,
    pub images: usize,
    pub fonts: usize,
    pub color_maps: usize,
    pub data_ptrs: usize,
}

/// Registry for render resources resolved by DCQ.
pub struct RenderContext {
    screens: [Option<ScreenHandle>; 3],
    canvases: HashMap<u32, Arc<RwLock<Canvas>>>,
    images: HashMap<u32, Arc<TFImage>>,
    font_pages: HashMap<u32, Arc<FontPage>>,
    color_maps: HashMap<u32, Arc<ColorMap>>,
    data_ptrs: HashMap<u64, u32>,
    next_id: u32,
    metadata: HashMap<u32, ResourceMetadata>,
    creation_order: Vec<u32>,
}

impl RenderContext {
    const FIRST_DYNAMIC_ID: u32 = 3;

    /// Create a new empty render context.
    pub fn new() -> Self {
        Self {
            screens: std::array::from_fn(|_| None),
            canvases: HashMap::new(),
            images: HashMap::new(),
            font_pages: HashMap::new(),
            color_maps: HashMap::new(),
            data_ptrs: HashMap::new(),
            next_id: Self::FIRST_DYNAMIC_ID,
            metadata: HashMap::new(),
            creation_order: Vec::new(),
        }
    }

    /// Generate the next non-zero resource identifier.
    pub fn next_id(&mut self) -> u32 {
        let id = self.next_id;
        let mut next = self.next_id.wrapping_add(1);
        if next < Self::FIRST_DYNAMIC_ID {
            next = Self::FIRST_DYNAMIC_ID;
        }
        if next == id {
            next = next.wrapping_add(1);
            if next < Self::FIRST_DYNAMIC_ID {
                next = Self::FIRST_DYNAMIC_ID;
            }
        }
        self.next_id = next;
        id
    }

    /// Register a canvas resource and return its id.
    pub fn register_canvas(&mut self, canvas: Arc<RwLock<Canvas>>) -> u32 {
        let id = self.next_id();
        self.metadata.insert(
            id,
            ResourceMetadata {
                id,
                resource_type: ResourceType::Canvas,
                explicit: true,
                ref_count: 1,
                created_at: Some(Instant::now()),
            },
        );
        self.creation_order.push(id);
        self.canvases.insert(id, Arc::clone(&canvas));
        id
    }

    /// Fetch a canvas by id.
    pub fn get_canvas(&self, id: u32) -> Option<Arc<RwLock<Canvas>>> {
        self.canvases.get(&id).map(Arc::clone)
    }

    /// Register an image resource and return its id.
    pub fn register_image(&mut self, image: Arc<TFImage>) -> u32 {
        let id = self.next_id();
        self.metadata.insert(
            id,
            ResourceMetadata {
                id,
                resource_type: ResourceType::Image,
                explicit: true,
                ref_count: 1,
                created_at: Some(Instant::now()),
            },
        );
        self.creation_order.push(id);
        self.images.insert(id, Arc::clone(&image));
        id
    }

    /// Fetch an image by id.
    pub fn get_image(&self, id: u32) -> Option<Arc<TFImage>> {
        self.images.get(&id).map(Arc::clone)
    }

    /// Register a font page resource and return its id.
    pub fn register_font_page(&mut self, page: Arc<FontPage>) -> u32 {
        let id = self.next_id();
        self.metadata.insert(
            id,
            ResourceMetadata {
                id,
                resource_type: ResourceType::Font,
                explicit: true,
                ref_count: 1,
                created_at: Some(Instant::now()),
            },
        );
        self.creation_order.push(id);
        self.font_pages.insert(id, Arc::clone(&page));
        id
    }

    /// Fetch a font page by id.
    pub fn get_font_page(&self, id: u32) -> Option<Arc<FontPage>> {
        self.font_pages.get(&id).map(Arc::clone)
    }

    /// Register a data pointer (allocated externally).
    pub fn register_data_ptr(&mut self, ptr: u64) {
        if ptr != 0 {
            let id = self.next_id();
            self.metadata.insert(
                id,
                ResourceMetadata {
                    id,
                    resource_type: ResourceType::DataPtr,
                    explicit: true,
                    ref_count: 1,
                    created_at: Some(Instant::now()),
                },
            );
            self.creation_order.push(id);
            self.data_ptrs.insert(ptr, id);
        }
    }

    /// Remove a registered data pointer.
    pub fn remove_data_ptr(&mut self, ptr: u64) -> bool {
        if let Some(id) = self.data_ptrs.remove(&ptr) {
            self.metadata.remove(&id);
            true
        } else {
            false
        }
    }

    /// Purge data pointers with zero references.
    pub fn purge_data_ptrs(&mut self) -> usize {
        let mut purged = 0;
        let mut to_remove = Vec::new();

        for (&ptr, &id) in &self.data_ptrs {
            if let Some(meta) = self.metadata.get(&id) {
                if meta.ref_count == 0 && meta.resource_type == ResourceType::DataPtr {
                    to_remove.push(ptr);
                }
            }
        }

        for ptr in to_remove {
            if let Some(id) = self.data_ptrs.remove(&ptr) {
                self.metadata.remove(&id);
                purged += 1;
            }
        }

        purged
    }

    #[cfg(test)]
    pub fn data_ptr_count(&self) -> usize {
        self.data_ptrs.len()
    }

    /// Get metadata for a resource.
    pub fn get_metadata(&self, id: u32) -> Option<&ResourceMetadata> {
        self.metadata.get(&id)
    }

    /// Check if a resource is tracked.
    pub fn has_resource(&self, id: u32) -> bool {
        self.metadata.contains_key(&id)
    }

    /// Get all tracking information for debugging.
    pub fn get_tracking_info(&self) -> ResourceTrackingInfo {
        ResourceTrackingInfo {
            total_resources: self.metadata.len(),
            screens: self.screens.iter().filter(|s| s.is_some()).count(),
            canvases: self.canvases.len(),
            images: self.images.len(),
            fonts: self.font_pages.len(),
            color_maps: self.color_maps.len(),
            data_ptrs: self.data_ptrs.len(),
        }
    }

    /// Increment reference count for a resource.
    pub fn increment_ref(&mut self, id: u32) -> bool {
        if let Some(meta) = self.metadata.get_mut(&id) {
            meta.ref_count += 1;
            true
        } else {
            false
        }
    }

    /// Decrement reference count for a resource.
    pub fn decrement_ref(&mut self, id: u32) -> u32 {
        if let Some(meta) = self.metadata.get_mut(&id) {
            meta.ref_count = meta.ref_count.saturating_sub(1);
            meta.ref_count
        } else {
            0
        }
    }

    /// Find resources with zero references (candidates for cleanup).
    pub fn find_orphaned_resources(&self) -> Vec<u32> {
        self.metadata
            .iter()
            .filter(|(_, meta)| meta.ref_count == 0)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get list of resources by type.
    pub fn get_resources_by_type(&self, resource_type: ResourceType) -> Vec<u32> {
        self.metadata
            .iter()
            .filter(|(_, meta)| meta.resource_type == resource_type)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Register a color map resource and return its id.
    pub fn register_color_map(&mut self, cmap: Arc<ColorMap>) -> u32 {
        let id = self.next_id();
        self.metadata.insert(
            id,
            ResourceMetadata {
                id,
                resource_type: ResourceType::ColorMap,
                explicit: true,
                ref_count: 1,
                created_at: Some(Instant::now()),
            },
        );
        self.creation_order.push(id);
        self.color_maps.insert(id, Arc::clone(&cmap));
        id
    }

    /// Fetch a color map by id.
    pub fn get_color_map(&self, id: u32) -> Option<Arc<ColorMap>> {
        self.color_maps.get(&id).map(Arc::clone)
    }

    /// Remove a canvas resource.
    pub fn remove_canvas(&mut self, id: u32) -> Option<Arc<RwLock<Canvas>>> {
        let canvas = self.canvases.remove(&id);
        if canvas.is_some() {
            self.metadata.remove(&id);
        }
        canvas
    }

    /// Remove an image resource.
    pub fn remove_image(&mut self, id: u32) -> Option<Arc<TFImage>> {
        let image = self.images.remove(&id);
        if image.is_some() {
            self.metadata.remove(&id);
        }
        image
    }

    /// Remove a font page resource.
    pub fn remove_font_page(&mut self, id: u32) -> Option<Arc<FontPage>> {
        let page = self.font_pages.remove(&id);
        if page.is_some() {
            self.metadata.remove(&id);
        }
        page
    }

    /// Remove a color map resource.
    pub fn remove_color_map(&mut self, id: u32) -> Option<Arc<ColorMap>> {
        let cmap = self.color_maps.remove(&id);
        if cmap.is_some() {
            self.metadata.remove(&id);
        }
        cmap
    }

    /// Set the canvas associated with a screen slot.
    pub fn set_screen(&mut self, screen: ScreenType, canvas: Arc<RwLock<Canvas>>) {
        let id = screen.index() as u32;
        self.metadata.insert(
            id,
            ResourceMetadata {
                id,
                resource_type: ResourceType::Screen,
                explicit: true,
                ref_count: 1,
                created_at: Some(Instant::now()),
            },
        );
        self.screens[screen.index()] = Some(ScreenHandle { id, canvas });
    }

    /// Fetch the canvas for a screen slot.
    pub fn get_screen(&self, screen: ScreenType) -> Option<Arc<RwLock<Canvas>>> {
        self.screens[screen.index()]
            .as_ref()
            .map(|handle| Arc::clone(&handle.canvas))
    }

    /// Get the resource id associated with a data pointer.
    pub fn get_data_ptr_id(&self, ptr: u64) -> Option<u32> {
        self.data_ptrs.get(&ptr).copied()
    }

    /// Get metadata for a data pointer.
    pub fn get_data_ptr_metadata(&self, ptr: u64) -> Option<&ResourceMetadata> {
        self.data_ptrs
            .get(&ptr)
            .and_then(|id| self.metadata.get(id))
    }
}

impl Default for RenderContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_register_canvas() {
        let mut ctx = RenderContext::new();
        let canvas = Arc::new(RwLock::new(Canvas::new_rgba(10, 10)));

        let id = ctx.register_canvas(Arc::clone(&canvas));
        let fetched = ctx.get_canvas(id).expect("canvas should exist");

        assert!(Arc::ptr_eq(&canvas, &fetched));
    }

    #[test]
    fn test_register_image() {
        let mut ctx = RenderContext::new();
        let image = Arc::new(TFImage::new_rgba(8, 8));

        let id = ctx.register_image(Arc::clone(&image));
        let fetched = ctx.get_image(id).expect("image should exist");

        assert!(Arc::ptr_eq(&image, &fetched));
    }

    #[test]
    fn test_metadata_created_on_registration() {
        let mut ctx = RenderContext::new();
        let canvas = Arc::new(RwLock::new(Canvas::new_rgba(10, 10)));
        let image = Arc::new(TFImage::new_rgba(8, 8));

        let canvas_id = ctx.register_canvas(Arc::clone(&canvas));
        let image_id = ctx.register_image(Arc::clone(&image));

        let canvas_meta = ctx
            .get_metadata(canvas_id)
            .expect("canvas metadata missing");
        let image_meta = ctx.get_metadata(image_id).expect("image metadata missing");

        assert_eq!(canvas_meta.resource_type, ResourceType::Canvas);
        assert_eq!(image_meta.resource_type, ResourceType::Image);
        assert_eq!(canvas_meta.ref_count, 1);
        assert_eq!(image_meta.ref_count, 1);
    }

    #[test]
    fn test_ref_counting_and_orphan_detection() {
        let mut ctx = RenderContext::new();
        let canvas = Arc::new(RwLock::new(Canvas::new_rgba(10, 10)));

        let id = ctx.register_canvas(Arc::clone(&canvas));
        assert!(ctx.increment_ref(id));
        assert_eq!(ctx.decrement_ref(id), 1);
        assert_eq!(ctx.decrement_ref(id), 0);

        let orphans = ctx.find_orphaned_resources();
        assert!(orphans.contains(&id));
    }

    #[test]
    fn test_filter_resources_by_type() {
        let mut ctx = RenderContext::new();
        let canvas = Arc::new(RwLock::new(Canvas::new_rgba(10, 10)));
        let image = Arc::new(TFImage::new_rgba(8, 8));

        let canvas_id = ctx.register_canvas(Arc::clone(&canvas));
        let image_id = ctx.register_image(Arc::clone(&image));

        let mut canvases = ctx.get_resources_by_type(ResourceType::Canvas);
        let mut images = ctx.get_resources_by_type(ResourceType::Image);

        canvases.sort_unstable();
        images.sort_unstable();

        assert_eq!(canvases, vec![canvas_id]);
        assert_eq!(images, vec![image_id]);
    }

    #[test]
    fn test_register_font_page() {
        let mut ctx = RenderContext::new();
        let page = Arc::new(FontPage::new(0x0000, 0x0020, 1));

        let id = ctx.register_font_page(Arc::clone(&page));
        let fetched = ctx.get_font_page(id).expect("font page should exist");

        assert!(Arc::ptr_eq(&page, &fetched));
    }

    #[test]
    fn test_register_color_map() {
        let mut ctx = RenderContext::new();
        let cmap = Arc::new(ColorMap::new(0));

        let id = ctx.register_color_map(Arc::clone(&cmap));
        let fetched = ctx.get_color_map(id).expect("color map should exist");

        assert!(Arc::ptr_eq(&cmap, &fetched));
    }

    #[test]
    fn test_set_get_screen() {
        let mut ctx = RenderContext::new();
        let canvas = Arc::new(RwLock::new(Canvas::new_rgba(640, 480)));

        ctx.set_screen(ScreenType::Main, Arc::clone(&canvas));

        let fetched = ctx
            .get_screen(ScreenType::Main)
            .expect("screen should exist");
        assert!(Arc::ptr_eq(&canvas, &fetched));
        assert!(ctx.get_screen(ScreenType::Extra).is_none());
    }

    #[test]
    fn test_next_id_unique() {
        let mut ctx = RenderContext::new();

        let id1 = ctx.next_id();
        let id2 = ctx.next_id();

        assert_ne!(id1, id2);
    }
}
