use crate::{model::Error, status, with_context};

/// C-compatible complete style snapshot. Width/height: -1 fill, -2 fit, otherwise
/// nonnegative logical pixels. Colors are 0xRRGGBBAA, independent of endianness.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct GmStyle {
    pub width: f64,
    pub height: f64,
    pub spacing: f64,
    pub padding: f64,
    pub margin: f64,
    pub font_size: f64,
    pub corner_radius: f64,
    pub align_x: f64,
    pub align_y: f64,
    pub color: u32,
    pub background: u32,
}
impl Default for GmStyle {
    fn default() -> Self {
        Self { width: -1.0, height: -2.0, spacing: 12.0, padding: 0.0, margin: 0.0, font_size: 16.0, corner_radius: 0.0, align_x: 0.0, align_y: 0.0, color: 0xe8eaf0ff, background: 0 }
    }
}
impl GmStyle {
    pub fn validate(&self) -> Result<(), Error> {
        for size in [self.width, self.height] {
            if !size.is_finite() || size > f32::MAX as f64 || (size < 0.0 && size != -1.0 && size != -2.0) { return Err(Error::new(-2, "invalid style size")); }
        }
        for value in [self.spacing, self.padding, self.margin, self.font_size, self.corner_radius] {
            if !value.is_finite() || !(0.0..=f32::MAX as f64).contains(&value) { return Err(Error::new(-2, "style metrics must be finite and nonnegative")); }
        }
        if self.font_size == 0.0 || !self.align_x.is_finite() || !self.align_y.is_finite() || !(0.0..=1.0).contains(&self.align_x) || !(0.0..=1.0).contains(&self.align_y) {
            return Err(Error::new(-2, "invalid font size or alignment"));
        }
        Ok(())
    }
}
/// # Safety
/// `style` must point to an initialized, readable GmStyle.
#[no_mangle]
pub unsafe extern "C" fn gm_node_set_style(ctx: u64, id: u64, style: *const GmStyle) -> i32 {
    status(|| {
        if style.is_null() { return Err(Error::new(-2, "style pointer is null")); }
        let style = *style;
        style.validate()?;
        with_context(ctx, |c| {
            let node = c.nodes.get_mut(&id).ok_or_else(|| Error::new(-3, "invalid widget handle"))?;
            if node.style != style { node.style = style; c.mark(id, 1); }
            Ok(())
        })
    })
}
/// # Safety
/// `style` must point to writable storage for a GmStyle.
#[no_mangle]
pub unsafe extern "C" fn gm_node_get_style(ctx: u64, id: u64, style: *mut GmStyle) -> i32 {
    status(|| {
        if style.is_null() { return Err(Error::new(-2, "style pointer is null")); }
        with_context(ctx, |c| { *style = c.node(id)?.style; Ok(()) })
    })
}
