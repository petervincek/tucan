use std::ops::ControlFlow;

use anyhow::Result;
use crossterm::event::Event;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};

/// `EventHandler<B, C = ()>` trait to cover common functionality
/// supported by our custom interactive components/widgets
pub trait EventHandler<B, C = ()> {
    fn handle_event(&mut self, event: Event) -> Result<ControlFlow<B, C>>;
    fn get_event_controls(&self) -> Vec<(Event, String)>;
}

#[derive(Debug)]
pub struct Center {
    area: Rect,
    vertically: bool,
    vertical_constraint: Constraint,
    horizontally: bool,
    horizontal_constraint: Constraint,
}

pub struct CenterBuilder {
    area: Rect,
    vertically: Option<bool>,
    vertical_constraint: Constraint,
    horizontally: Option<bool>,
    horizontal_constraint: Constraint,
}

impl Center {
    pub fn builder(area: Rect) -> CenterBuilder {
        CenterBuilder {
            area,
            horizontally: None,
            vertically: None,
            horizontal_constraint: Constraint::Length(60),
            vertical_constraint: Constraint::Length(12),
        }
    }

    pub fn center(self) -> Rect {
        match (self.vertically, self.horizontally) {
            (true, true) => center_area(
                self.area,
                (self.horizontal_constraint, self.vertical_constraint),
            ),
            (true, false) => center_area_vertically(self.area, self.vertical_constraint),
            (false, true) => center_area_horizontally(self.area, self.horizontal_constraint),
            _ => self.area,
        }
    }
}

impl CenterBuilder {
    pub fn vertically(&mut self, vertically: bool) -> &mut Self {
        self.vertically = Some(vertically);
        self
    }

    pub fn vertical_constraint(&mut self, constraint: Constraint) -> &mut Self {
        self.vertical_constraint = constraint;
        self
    }

    pub fn horizontally(&mut self, horizontally: bool) -> &mut Self {
        self.horizontally = Some(horizontally);
        self
    }

    pub fn horizontal_constraint(&mut self, constraint: Constraint) -> &mut Self {
        self.horizontal_constraint = constraint;
        self
    }

    pub fn build(&mut self) -> Center {
        Center {
            area: self.area,
            vertically: self.vertically.unwrap_or(false),
            vertical_constraint: self.vertical_constraint,
            horizontally: self.horizontally.unwrap_or(false),
            horizontal_constraint: self.horizontal_constraint,
        }
    }
}

/// `center_area` is utility method to center the area both vertically and horizontally
pub fn center_area(area: Rect, constraints: (Constraint, Constraint)) -> Rect {
    let v_center = center_area_vertically(area, constraints.1);
    center_area_horizontally(v_center, constraints.0)
}

/// `center_area_vertically` is utility method to center the area vertically
pub fn center_area_vertically(area: Rect, constraint: Constraint) -> Rect {
    let v_center = Layout::default()
        .direction(Direction::Vertical)
        .flex(Flex::Center)
        .constraints([constraint])
        .split(area);
    v_center[0]
}

/// `center_area_horizontally` is utility method to center the area both horizontally
pub fn center_area_horizontally(area: Rect, constraint: Constraint) -> Rect {
    let h_center = Layout::default()
        .direction(Direction::Horizontal)
        .flex(Flex::Center)
        .constraints([constraint])
        .split(area);
    h_center[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn test_center_builder_defaults_to_no_centering() {
        let area = Rect::new(0, 0, 120, 40);
        let center = Center::builder(area).build();

        assert_eq!(center.area, area);
        assert!(!center.vertically);
        assert!(!center.horizontally);
        assert_eq!(center.center(), area);
    }

    #[test]
    fn test_center_builder_builds_centered_rect_when_requested() {
        let area = Rect::new(0, 0, 120, 40);
        let mut builder = Center::builder(area);
        let center = builder.vertically(true).horizontally(true).build();

        assert!(center.vertically);
        assert!(center.horizontally);
        let centered = center.center();
        assert_eq!(centered, Rect::new(30, 14, 60, 12));
    }

    #[test]
    fn test_center_area_vertically_centers_height_only() {
        let area = Rect::new(0, 0, 80, 24);
        let centered = center_area_vertically(area, Constraint::Length(12));

        assert_eq!(centered, Rect::new(0, 6, 80, 12));
    }

    #[test]
    fn test_center_area_horizontally_centers_width_only() {
        let area = Rect::new(0, 0, 100, 20);
        let centered = center_area_horizontally(area, Constraint::Length(60));

        assert_eq!(centered, Rect::new(20, 0, 60, 20));
    }

    #[test]
    fn test_center_area_centers_both_directions() {
        let area = Rect::new(10, 5, 120, 40);
        let centered = center_area(area, (Constraint::Length(60), Constraint::Length(12)));

        assert_eq!(centered, Rect::new(40, 19, 60, 12));
    }

    #[test]
    fn test_center_builder_applies_vertical_constraint() {
        let area = Rect::new(0, 0, 120, 40);
        let mut builder = Center::builder(area);
        let center = builder
            .vertically(true)
            .vertical_constraint(Constraint::Length(20))
            .build();

        assert!(center.vertically);
        assert!(!center.horizontally);
        assert_eq!(center.center(), Rect::new(0, 10, 120, 20));
    }

    #[test]
    fn test_center_builder_applies_horizontal_constraint() {
        let area = Rect::new(0, 0, 120, 40);
        let mut builder = Center::builder(area);
        let center = builder
            .horizontally(true)
            .horizontal_constraint(Constraint::Length(80))
            .build();

        assert!(!center.vertically);
        assert!(center.horizontally);
        assert_eq!(center.center(), Rect::new(20, 0, 80, 40));
    }

    #[test]
    fn test_center_builder_applies_both_constraints() {
        let area = Rect::new(0, 0, 120, 40);
        let mut builder = Center::builder(area);
        let center = builder
            .vertically(true)
            .horizontal_constraint(Constraint::Length(70))
            .vertical_constraint(Constraint::Length(20))
            .horizontally(true)
            .build();

        assert!(center.vertically);
        assert!(center.horizontally);
        assert_eq!(center.center(), Rect::new(25, 10, 70, 20));
    }
}
