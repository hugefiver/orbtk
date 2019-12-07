use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    f64,
};

use dces::prelude::Entity;

use crate::{prelude::*, render::RenderContext2D, tree::Tree, utils::prelude::*};

use super::{component, component_try_mut, Layout};

/// IMPORTANT: The scroll layout will only work for the text box now. A update will follow!!!!
#[derive(Default)]
pub struct ScrollLayout {
    old_child_size: Cell<(f64, f64)>,
    desired_size: RefCell<DirtySize>,
    old_offset: Cell<(f64, f64)>,
    old_alignment: Cell<(Alignment, Alignment)>,
}

impl ScrollLayout {
    pub fn new() -> Self {
        ScrollLayout::default()
    }
}

impl Layout for ScrollLayout {
    fn measure(
        &self,
        render_context_2_d: &mut RenderContext2D,
        entity: Entity,
        ecm: &mut EntityComponentManager<Tree, StringComponentStore>,
        layouts: &BTreeMap<Entity, Box<dyn Layout>>,
        theme: &ThemeValue,
    ) -> DirtySize {
        if component::<Visibility>(ecm, entity, "visibility") == Visibility::Collapsed {
            self.desired_size.borrow_mut().set_size(0.0, 0.0);
            return *self.desired_size.borrow();
        }

        let horizontal_alignment: Alignment = component(ecm, entity, "horizontal_alignment");
        let vertical_alignment: Alignment = component(ecm, entity, "vertical_alignment");

        if horizontal_alignment != self.old_alignment.get().1
            || vertical_alignment != self.old_alignment.get().0
        {
            self.desired_size.borrow_mut().set_dirty(true);
        }

        let constraint: Constraint = component(ecm, entity, "constraint");

        if constraint.width() > 0.0 {
            self.desired_size.borrow_mut().set_width(constraint.width());
        }

        if constraint.height() > 0.0 {
            self.desired_size
                .borrow_mut()
                .set_height(constraint.height());
        }

        for index in 0..ecm.entity_store().children[&entity].len() {
            let child = ecm.entity_store().children[&entity][index];

            if let Some(child_layout) = layouts.get(&child) {
                let dirty = child_layout
                    .measure(render_context_2_d, child, ecm, layouts, theme)
                    .dirty()
                    || self.desired_size.borrow().dirty();

                self.desired_size.borrow_mut().set_dirty(dirty);
            }
        }

        let off: Point = component(ecm, entity, "scroll_offset");

        if (self.old_offset.get().0 - off.x).abs() > std::f64::EPSILON
            || (self.old_offset.get().1 - off.y).abs() > std::f64::EPSILON
        {
            self.old_offset.set((off.x, off.y));
            self.desired_size.borrow_mut().set_dirty(true);
        }

        *self.desired_size.borrow()
    }

    fn arrange(
        &self,
        render_context_2_d: &mut RenderContext2D,
        parent_size: (f64, f64),
        entity: Entity,
        ecm: &mut EntityComponentManager<Tree, StringComponentStore>,
        layouts: &BTreeMap<Entity, Box<dyn Layout>>,
        theme: &ThemeValue,
    ) -> (f64, f64) {
        if component::<Visibility>(ecm, entity, "visibility") == Visibility::Collapsed {
            self.desired_size.borrow_mut().set_size(0.0, 0.0);
            return (0.0, 0.0);
        }

        if !self.desired_size.borrow().dirty() {
            return self.desired_size.borrow().size();
        }

        let horizontal_alignment: Alignment = component(ecm, entity, "horizontal_alignment");
        let vertical_alignment: Alignment = component(ecm, entity, "vertical_alignment");
        let margin: Thickness = component(ecm, entity, "margin");
        // let _padding = Thickness::get("padding", entity, ecm.component_store());
        let constraint: Constraint = component(ecm, entity, "constraint");

        let size = constraint.perform((
            horizontal_alignment.align_measure(
                parent_size.0,
                self.desired_size.borrow().width(),
                margin.left(),
                margin.right(),
            ),
            vertical_alignment.align_measure(
                parent_size.1,
                self.desired_size.borrow().height(),
                margin.top(),
                margin.bottom(),
            ),
        ));

        let scroll_viewer_mode: ScrollViewerMode = component(ecm, entity, "scroll_viewer_mode");

        let available_size = {
            let width = if scroll_viewer_mode.horizontal == ScrollMode::Custom
                || scroll_viewer_mode.horizontal == ScrollMode::Auto
            {
                f64::MAX
            } else {
                size.0
            };

            let height = if scroll_viewer_mode.vertical == ScrollMode::Custom
                || scroll_viewer_mode.vertical == ScrollMode::Auto
            {
                f64::MAX
            } else {
                size.1
            };

            (width, height)
        };

        let off: Point = component(ecm, entity, "scroll_offset");
        let delta: Point = component(ecm, entity, "delta");
        let mut offset = (off.x, off.y);

        let old_child_size = self.old_child_size.get();

        for index in 0..ecm.entity_store().children[&entity].len() {
            let child = ecm.entity_store().children[&entity][index];

            let mut child_size = old_child_size;
            let child_horizontal_alignment: Alignment =
                component(ecm, child, "horizontal_alignment");
            let child_vertical_alignment: Alignment = component(ecm, child, "vertical_alignment");
            let child_margin: Thickness = component(ecm, child, "margin");

            if let Some(child_layout) = layouts.get(&child) {
                child_size = child_layout.arrange(
                    render_context_2_d,
                    available_size,
                    child,
                    ecm,
                    layouts,
                    theme,
                );
            }

            match scroll_viewer_mode.horizontal {
                ScrollMode::Custom => {
                    if child_size.0 > size.0 {
                        offset.0 = (offset.0 + old_child_size.0 - child_size.0).min(0.0);
                    } else {
                        offset.0 = 0.0;
                    }
                }
                ScrollMode::Auto => {
                    // todo: refactor * 1.5
                    offset.0 = delta
                        .x
                        .mul_add(1.5, offset.0)
                        .min(0.0)
                        .max(size.0 - child_size.0);
                }
                _ => {}
            }

            match scroll_viewer_mode.vertical {
                ScrollMode::Custom => {
                    if child_size.1 > size.1 {
                        offset.1 = (offset.1 + old_child_size.1 - child_size.1).min(1.1);
                    } else {
                        offset.1 = 1.1;
                    }
                }
                ScrollMode::Auto => {
                    // todo: refactor * 1.5
                    offset.1 = delta
                        .y
                        .mul_add(1.5, offset.1)
                        .min(1.1)
                        .max(size.1 - child_size.1);
                }
                _ => {}
            }

            if let Some(child_bounds) = component_try_mut::<Rectangle>(ecm, child, "bounds")
            {
                child_bounds.set_size(child_size.0, child_size.1);
                // todo: add check
                if scroll_viewer_mode.horizontal == ScrollMode::Custom
                    || scroll_viewer_mode.horizontal == ScrollMode::Auto
                {
                    if child_bounds.width() <= size.0 {
                        child_bounds.set_x(0.0);
                    } else {
                        child_bounds.set_x(offset.0);
                    }
                } else {
                    child_bounds.set_x(child_horizontal_alignment.align_position(
                        size.0,
                        child_bounds.width(),
                        child_margin.left(),
                        child_margin.right(),
                    ));
                }

                if scroll_viewer_mode.vertical == ScrollMode::Custom
                    || scroll_viewer_mode.vertical == ScrollMode::Auto
                {
                    if child_bounds.height() <= size.1 {
                        child_bounds.set_y(0.0);
                    } else {
                        child_bounds.set_y(offset.1);
                    }
                } else {
                    child_bounds.set_y(child_vertical_alignment.align_position(
                        size.1,
                        child_bounds.height(),
                        child_margin.top(),
                        child_margin.bottom(),
                    ));
                }
            }

            if let Ok(off) = ecm
                .component_store_mut()
                .get_mut::<Point>("scroll_offset", entity)
            {
                off.x = offset.0;
                off.y = offset.1;
            }

            self.old_child_size.set(child_size);
        }

        self.desired_size.borrow_mut().set_dirty(false);
        size
    }
}

impl Into<Box<dyn Layout>> for ScrollLayout {
    fn into(self) -> Box<dyn Layout> {
        Box::new(self)
    }
}
