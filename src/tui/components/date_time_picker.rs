use std::ops::ControlFlow;

use anyhow::{Result, anyhow};
use chrono::{Datelike, Local, NaiveDateTime, Timelike};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use crate::tui::page::common::EventHandler;

/// `Month` represents the month enum for our date picker
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Month {
    January = 1,
    February = 2,
    March = 3,
    April = 4,
    May = 5,
    June = 6,
    July = 7,
    August = 8,
    September = 9,
    October = 10,
    November = 11,
    December = 12,
}

impl Month {
    pub const fn days(self) -> u8 {
        match self {
            Self::January
            | Self::March
            | Self::May
            | Self::July
            | Self::August
            | Self::October
            | Self::December => 31,
            Self::April | Self::June | Self::September | Self::November => 30,
            Self::February => 28,
        }
    }
}

impl TryFrom<u8> for Month {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::January),
            2 => Ok(Self::February),
            3 => Ok(Self::March),
            4 => Ok(Self::April),
            5 => Ok(Self::May),
            6 => Ok(Self::June),
            7 => Ok(Self::July),
            8 => Ok(Self::August),
            9 => Ok(Self::September),
            10 => Ok(Self::October),
            11 => Ok(Self::November),
            12 => Ok(Self::December),
            _ => Err(anyhow!("Unknown numeric representation of month")),
        }
    }
}

/// `DateTimePickerMode` represents the mode for the picker
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DateTimePickerMode {
    DateOnly,
    TimeOnly,
    DateAndTime,
}

/// `DateTimePickerField` represents the enum fields for our date picker
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DateTimePickerField {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

/// DateTimePickerState represents the state of our Stateful widget
#[derive(Debug, Clone)]
pub struct DateTimePickerState {
    date_time: NaiveDateTime, // state to hold the actual date/time information from the picker
    mode: DateTimePickerMode, // enum to represent the mode of picker: Date only, Time only, ...
    current_field: DateTimePickerField, // track the current field for the purpose of the widget
}

impl DateTimePickerState {
    pub fn new(mode: DateTimePickerMode) -> Self {
        let current_init_field = if mode == DateTimePickerMode::TimeOnly {
            DateTimePickerField::Hour
        } else {
            DateTimePickerField::Day
        };
        Self {
            date_time: Local::now().naive_local(),
            mode,
            current_field: current_init_field,
        }
    }

    pub fn preset_with_date_time(&mut self, date_time: NaiveDateTime) {
        self.date_time = date_time;
    }

    pub fn get_year(&self) -> i32 {
        self.date_time.year()
    }

    pub fn set_year(&mut self, year: i32) {
        if let Some(date_time) = self.date_time.with_year(year) {
            self.date_time = date_time;
        }
    }

    pub fn get_month(&self) -> Month {
        Month::try_from(self.date_time.month() as u8).unwrap()
    }

    pub fn set_month(&mut self, month: Month) {
        if let Some(date_time) = self.date_time.with_month(month as u32) {
            self.date_time = date_time;
        }
    }

    pub fn get_day(&self) -> u32 {
        self.date_time.day()
    }

    pub fn set_day(&mut self, day: u32) {
        if let Some(date_time) = self.date_time.with_day(day) {
            self.date_time = date_time;
        }
    }

    pub fn get_hour(&self) -> u32 {
        self.date_time.hour()
    }

    pub fn set_hour(&mut self, hour: u32) {
        if let Some(date_time) = self.date_time.with_hour(hour) {
            self.date_time = date_time;
        }
    }

    pub fn get_minute(&self) -> u32 {
        self.date_time.minute()
    }

    pub fn set_minute(&mut self, minute: u32) {
        if let Some(date_time) = self.date_time.with_minute(minute) {
            self.date_time = date_time;
        }
    }

    pub fn get_second(&self) -> u32 {
        self.date_time.second()
    }

    pub fn set_second(&mut self, second: u32) {
        if let Some(date_time) = self.date_time.with_second(second) {
            self.date_time = date_time;
        }
    }

    fn increment_1_based(value: u32, max: u32) -> u32 {
        (value % max) + 1
    }

    fn decrement_1_based(value: u32, max: u32) -> u32 {
        ((value + max - 2) % max) + 1
    }

    fn increment_mod(value: u32, max: u32) -> u32 {
        (value + 1) % max
    }

    fn decrement_mod(value: u32, max: u32) -> u32 {
        (value + max - 1) % max
    }

    pub fn increment_year(&mut self) {
        self.set_year(self.get_year() + 1);
    }

    pub fn decrement_year(&mut self) {
        self.set_year(self.get_year() - 1);
    }

    pub fn increment_month(&mut self) {
        let next = Self::increment_1_based(self.get_month() as u32, 12);
        self.set_month(Month::try_from(next as u8).unwrap());
    }

    pub fn decrement_month(&mut self) {
        let previous = Self::decrement_1_based(self.get_month() as u32, 12);
        self.set_month(Month::try_from(previous as u8).unwrap());
    }

    pub fn increment_day(&mut self) {
        let max_day = self.get_month().days() as u32;
        let next = Self::increment_1_based(self.get_day(), max_day);
        self.set_day(next);
    }

    pub fn decrement_day(&mut self) {
        let max_day = self.get_month().days() as u32;
        let previous = Self::decrement_1_based(self.get_day(), max_day);
        self.set_day(previous);
    }

    pub fn increment_hour(&mut self) {
        let next = Self::increment_mod(self.get_hour(), 24);
        self.set_hour(next);
    }

    pub fn decrement_hour(&mut self) {
        let previous = Self::decrement_mod(self.get_hour(), 24);
        self.set_hour(previous);
    }

    pub fn increment_minute(&mut self) {
        let next = Self::increment_mod(self.get_minute(), 60);
        self.set_minute(next);
    }

    pub fn decrement_minute(&mut self) {
        let previous = Self::decrement_mod(self.get_minute(), 60);
        self.set_minute(previous);
    }

    pub fn increment_second(&mut self) {
        let next = Self::increment_mod(self.get_second(), 60);
        self.set_second(next);
    }

    pub fn decrement_second(&mut self) {
        let previous = Self::decrement_mod(self.get_second(), 60);
        self.set_second(previous);
    }
}

impl EventHandler<NaiveDateTime, ()> for DateTimePickerState {
    fn get_event_controls(&self) -> Vec<(Event, String)> {
        vec![
            (
                Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
                String::from("Move To Next Field"),
            ),
            (
                Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
                String::from("Increment date/time value"),
            ),
            (
                Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
                String::from("Increment date/time value"),
            ),
            (
                Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
                String::from("Decrement date/time value"),
            ),
            (
                Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
                String::from("Decrement date/time value"),
            ),
        ]
    }

    fn handle_event(&mut self, event: Event) -> Result<ControlFlow<NaiveDateTime, ()>> {
        if let Event::Key(key_event) = event
            && (key_event.code == KeyCode::Enter && key_event.modifiers == KeyModifiers::NONE)
        {
            let date_time = self.date_time;
            return Ok(ControlFlow::Break(date_time));
        } else {
            match self.current_field {
                DateTimePickerField::Year => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => match self.mode {
                                DateTimePickerMode::DateAndTime => {
                                    self.current_field = DateTimePickerField::Hour;
                                }
                                DateTimePickerMode::DateOnly => {
                                    self.current_field = DateTimePickerField::Day;
                                }
                                DateTimePickerMode::TimeOnly => {
                                    panic!("invalid state, should not reach this logic execution");
                                }
                            },
                            (KeyCode::Up, KeyModifiers::NONE)
                            | (KeyCode::Right, KeyModifiers::NONE) => {
                                self.increment_year();
                            }
                            (KeyCode::Down, KeyModifiers::NONE)
                            | (KeyCode::Left, KeyModifiers::NONE) => {
                                self.decrement_year();
                            }
                            _ => {}
                        }
                    }
                }
                DateTimePickerField::Month => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => match self.mode {
                                DateTimePickerMode::DateAndTime => {
                                    self.current_field = DateTimePickerField::Year;
                                }
                                DateTimePickerMode::DateOnly => {
                                    self.current_field = DateTimePickerField::Year;
                                }
                                DateTimePickerMode::TimeOnly => {
                                    panic!("invalid state, should not reach this logic execution");
                                }
                            },
                            (KeyCode::Up, KeyModifiers::NONE)
                            | (KeyCode::Right, KeyModifiers::NONE) => {
                                self.increment_month();
                            }
                            (KeyCode::Down, KeyModifiers::NONE)
                            | (KeyCode::Left, KeyModifiers::NONE) => {
                                self.decrement_month();
                            }
                            _ => {}
                        }
                    }
                }
                DateTimePickerField::Day => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => match self.mode {
                                DateTimePickerMode::DateAndTime => {
                                    self.current_field = DateTimePickerField::Month;
                                }
                                DateTimePickerMode::DateOnly => {
                                    self.current_field = DateTimePickerField::Month;
                                }
                                DateTimePickerMode::TimeOnly => {
                                    panic!("invalid state, should not reach this logic execution");
                                }
                            },
                            (KeyCode::Up, KeyModifiers::NONE)
                            | (KeyCode::Right, KeyModifiers::NONE) => {
                                self.increment_day();
                            }
                            (KeyCode::Down, KeyModifiers::NONE)
                            | (KeyCode::Left, KeyModifiers::NONE) => {
                                self.decrement_day();
                            }
                            _ => {}
                        }
                    }
                }
                DateTimePickerField::Hour => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => match self.mode {
                                DateTimePickerMode::DateAndTime => {
                                    self.current_field = DateTimePickerField::Minute;
                                }
                                DateTimePickerMode::DateOnly => {
                                    panic!("invalid state, should not reach this logic execution");
                                }
                                DateTimePickerMode::TimeOnly => {
                                    self.current_field = DateTimePickerField::Minute;
                                }
                            },
                            (KeyCode::Up, KeyModifiers::NONE)
                            | (KeyCode::Right, KeyModifiers::NONE) => {
                                self.increment_hour();
                            }
                            (KeyCode::Down, KeyModifiers::NONE)
                            | (KeyCode::Left, KeyModifiers::NONE) => {
                                self.decrement_hour();
                            }
                            _ => {}
                        }
                    }
                }
                DateTimePickerField::Minute => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => match self.mode {
                                DateTimePickerMode::DateAndTime => {
                                    self.current_field = DateTimePickerField::Second;
                                }
                                DateTimePickerMode::DateOnly => {
                                    panic!("invalid state, should not reach this logic execution");
                                }
                                DateTimePickerMode::TimeOnly => {
                                    self.current_field = DateTimePickerField::Second;
                                }
                            },
                            (KeyCode::Up, KeyModifiers::NONE)
                            | (KeyCode::Right, KeyModifiers::NONE) => {
                                self.increment_minute();
                            }
                            (KeyCode::Down, KeyModifiers::NONE)
                            | (KeyCode::Left, KeyModifiers::NONE) => {
                                self.decrement_minute();
                            }
                            _ => {}
                        }
                    }
                }
                DateTimePickerField::Second => {
                    if let Event::Key(key_event) = event {
                        match (key_event.code, key_event.modifiers) {
                            (KeyCode::Tab, KeyModifiers::NONE) => match self.mode {
                                DateTimePickerMode::DateAndTime => {
                                    self.current_field = DateTimePickerField::Day;
                                }
                                DateTimePickerMode::DateOnly => {
                                    panic!("invalid state, should not reach this logic execution");
                                }
                                DateTimePickerMode::TimeOnly => {
                                    self.current_field = DateTimePickerField::Hour;
                                }
                            },
                            (KeyCode::Up, KeyModifiers::NONE)
                            | (KeyCode::Right, KeyModifiers::NONE) => {
                                self.increment_second();
                            }
                            (KeyCode::Down, KeyModifiers::NONE)
                            | (KeyCode::Left, KeyModifiers::NONE) => {
                                self.decrement_second();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(ControlFlow::Continue(()))
    }
}

/// `DateTimePicker` represents the Stateful widget that will be rendered for our TUI project
#[derive(Debug, Default)]
pub struct DateTimePicker {}

impl DateTimePicker {
    pub fn new() -> Self {
        Self {}
    }
}

fn get_border_style(
    current_field: &DateTimePickerField,
    style_for_field: DateTimePickerField,
) -> Style {
    if *current_field == style_for_field {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    }
}

impl StatefulWidget for DateTimePicker {
    type State = DateTimePickerState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        match state.mode {
            DateTimePickerMode::DateAndTime => {
                let vertical_layout = Layout::vertical([Constraint::Length(3)]);
                let [area] = area.layout(&vertical_layout);
                let layout = Layout::horizontal([
                    Constraint::Percentage(15),
                    Constraint::Percentage(25),
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                ]);
                let [
                    day_area,
                    month_area,
                    year_area,
                    hour_area,
                    minute_area,
                    second_area,
                ] = area.layout(&layout);

                // render the day
                let day = state.get_day();
                let day_paragraph = Paragraph::new(format!(" ▲ {day} ▼ "))
                    .alignment(Alignment::Left)
                    .block(
                        Block::new()
                            .borders(Borders::all())
                            .border_style(get_border_style(
                                &state.current_field,
                                DateTimePickerField::Day,
                            ))
                            .title(String::from("Day:")),
                    );
                day_paragraph.render(day_area, buf);

                // render the month
                let month = state.get_month();
                let month_paragraph = Paragraph::new(format!(" ▲ {month:?} ▼ "))
                    .alignment(Alignment::Left)
                    .block(
                        Block::new()
                            .borders(Borders::all())
                            .border_style(get_border_style(
                                &state.current_field,
                                DateTimePickerField::Month,
                            ))
                            .title(String::from("Month:")),
                    );
                month_paragraph.render(month_area, buf);

                // render the year
                let year = state.get_year();
                let year_paragraph = Paragraph::new(format!(" ▲ {year} ▼ "))
                    .alignment(Alignment::Left)
                    .block(
                        Block::new()
                            .borders(Borders::all())
                            .border_style(get_border_style(
                                &state.current_field,
                                DateTimePickerField::Year,
                            ))
                            .title(String::from("Year:")),
                    );
                year_paragraph.render(year_area, buf);

                // render the hour
                let hour = state.get_hour();
                let hour_paragraph = Paragraph::new(format!(" ▲ {hour} ▼ "))
                    .alignment(Alignment::Left)
                    .block(
                        Block::new()
                            .borders(Borders::all())
                            .border_style(get_border_style(
                                &state.current_field,
                                DateTimePickerField::Hour,
                            ))
                            .title(String::from("Hour:")),
                    );
                hour_paragraph.render(hour_area, buf);

                // render the minute
                let minute = state.get_minute();
                let minute_paragraph = Paragraph::new(format!(" ▲ {minute} ▼ "))
                    .alignment(Alignment::Left)
                    .block(
                        Block::new()
                            .borders(Borders::all())
                            .border_style(get_border_style(
                                &state.current_field,
                                DateTimePickerField::Minute,
                            ))
                            .title(String::from("Minute:")),
                    );
                minute_paragraph.render(minute_area, buf);

                // render the second
                let second = state.get_second();
                let second_paragraph = Paragraph::new(format!(" ▲ {second} ▼ "))
                    .alignment(Alignment::Left)
                    .block(
                        Block::new()
                            .borders(Borders::all())
                            .border_style(get_border_style(
                                &state.current_field,
                                DateTimePickerField::Second,
                            ))
                            .title(String::from("Second:")),
                    );
                second_paragraph.render(second_area, buf);
            }
            DateTimePickerMode::DateOnly => {
                let vertical_layout = Layout::vertical([Constraint::Length(3)]);
                let [area] = area.layout(&vertical_layout);
                let layout = Layout::horizontal([
                    Constraint::Percentage(30),
                    Constraint::Percentage(40),
                    Constraint::Percentage(30),
                ]);
                let [day_area, month_area, year_area] = area.layout(&layout);

                // render the day
                let day = state.get_day();
                let day_paragraph = Paragraph::new(format!(" ▲ {day} ▼ "))
                    .alignment(Alignment::Left)
                    .block(
                        Block::new()
                            .borders(Borders::all())
                            .border_style(get_border_style(
                                &state.current_field,
                                DateTimePickerField::Day,
                            ))
                            .title(String::from("Day:")),
                    );
                day_paragraph.render(day_area, buf);

                // render the month
                let month = state.get_month();
                let month_paragraph = Paragraph::new(format!(" ▲ {month:?} ▼ "))
                    .alignment(Alignment::Left)
                    .block(
                        Block::new()
                            .borders(Borders::all())
                            .border_style(get_border_style(
                                &state.current_field,
                                DateTimePickerField::Month,
                            ))
                            .title(String::from("Month:")),
                    );
                month_paragraph.render(month_area, buf);

                // render the year
                let year = state.get_year();
                let year_paragraph = Paragraph::new(format!(" ▲ {year} ▼ "))
                    .alignment(Alignment::Left)
                    .block(
                        Block::new()
                            .borders(Borders::all())
                            .border_style(get_border_style(
                                &state.current_field,
                                DateTimePickerField::Year,
                            ))
                            .title(String::from("Year:")),
                    );
                year_paragraph.render(year_area, buf);
            }
            DateTimePickerMode::TimeOnly => {
                let vertical_layout = Layout::vertical([Constraint::Length(3)]);
                let [area] = area.layout(&vertical_layout);
                let layout = Layout::horizontal([
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                ]);
                let [hour_area, minute_area, second_area] = area.layout(&layout);

                // render the hour
                let hour = state.get_hour();
                let hour_paragraph = Paragraph::new(format!(" ▲ {hour} ▼ "))
                    .alignment(Alignment::Left)
                    .block(
                        Block::new()
                            .borders(Borders::all())
                            .border_style(get_border_style(
                                &state.current_field,
                                DateTimePickerField::Hour,
                            ))
                            .title(String::from("Hour:")),
                    );
                hour_paragraph.render(hour_area, buf);

                // render the minute
                let minute = state.get_minute();
                let minute_paragraph = Paragraph::new(format!(" ▲ {minute} ▼ "))
                    .alignment(Alignment::Left)
                    .block(
                        Block::new()
                            .borders(Borders::all())
                            .border_style(get_border_style(
                                &state.current_field,
                                DateTimePickerField::Minute,
                            ))
                            .title(String::from("Minute:")),
                    );
                minute_paragraph.render(minute_area, buf);

                // render the second
                let second = state.get_second();
                let second_paragraph = Paragraph::new(format!(" ▲ {second} ▼ "))
                    .alignment(Alignment::Left)
                    .block(
                        Block::new()
                            .borders(Borders::all())
                            .border_style(get_border_style(
                                &state.current_field,
                                DateTimePickerField::Second,
                            ))
                            .title(String::from("Second:")),
                    );
                second_paragraph.render(second_area, buf);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::buffer::{assert_rendered_output, buffer_to_string};
    use chrono::NaiveDate;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn test_month_try_from_valid_and_invalid_values() {
        assert_eq!(Month::try_from(1).unwrap(), Month::January);
        assert_eq!(Month::try_from(12).unwrap(), Month::December);
        assert!(Month::try_from(0).is_err());
        assert!(Month::try_from(13).is_err());
    }

    #[test]
    fn test_date_time_picker_state_setters_getters() {
        let mut state = DateTimePickerState::new(DateTimePickerMode::DateAndTime);
        let date_time = NaiveDate::from_ymd_opt(2024, 5, 9)
            .unwrap()
            .and_hms_opt(8, 7, 6)
            .unwrap();
        state.preset_with_date_time(date_time);

        assert_eq!(state.get_year(), 2024);
        assert_eq!(state.get_month(), Month::May);
        assert_eq!(state.get_day(), 9);
        assert_eq!(state.get_hour(), 8);
        assert_eq!(state.get_minute(), 7);
        assert_eq!(state.get_second(), 6);

        state.set_year(2030);
        state.set_month(Month::December);
        state.set_day(31);
        state.set_hour(23);
        state.set_minute(59);
        state.set_second(58);

        assert_eq!(state.get_year(), 2030);
        assert_eq!(state.get_month(), Month::December);
        assert_eq!(state.get_day(), 31);
        assert_eq!(state.get_hour(), 23);
        assert_eq!(state.get_minute(), 59);
        assert_eq!(state.get_second(), 58);
    }

    #[test]
    fn test_date_time_picker_state_increment_decrement_wraps_values() {
        let mut state = DateTimePickerState::new(DateTimePickerMode::DateAndTime);
        state.preset_with_date_time(
            NaiveDate::from_ymd_opt(2024, 12, 31)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        );

        state.increment_year();
        assert_eq!(state.get_year(), 2025);
        state.decrement_year();
        assert_eq!(state.get_year(), 2024);

        state.increment_month();
        assert_eq!(state.get_month(), Month::January);
        state.decrement_month();
        assert_eq!(state.get_month(), Month::December);

        state.increment_day();
        assert_eq!(state.get_day(), 1);
        state.decrement_day();
        assert_eq!(state.get_day(), 31);

        state.increment_hour();
        assert_eq!(state.get_hour(), 1);
        state.decrement_hour();
        assert_eq!(state.get_hour(), 0);
        state.decrement_hour();
        assert_eq!(state.get_hour(), 23);

        state.increment_minute();
        assert_eq!(state.get_minute(), 1);
        state.decrement_minute();
        assert_eq!(state.get_minute(), 0);
        state.decrement_minute();
        assert_eq!(state.get_minute(), 59);

        state.increment_second();
        assert_eq!(state.get_second(), 1);
        state.decrement_second();
        assert_eq!(state.get_second(), 0);
        state.decrement_second();
        assert_eq!(state.get_second(), 59);
    }

    #[test]
    fn test_get_event_controls_includes_tab_and_arrows() {
        let state = DateTimePickerState::new(DateTimePickerMode::DateAndTime);
        let controls = state.get_event_controls();
        assert!(controls.iter().any(|(event, label)| {
            matches!(
                event,
                Event::Key(KeyEvent {
                    code: KeyCode::Tab,
                    modifiers: KeyModifiers::NONE,
                    kind: _,
                    state: _
                })
            ) && label == "Move To Next Field"
        }));
        assert!(controls.iter().any(|(event, label)| {
            matches!(
                event,
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE,
                    kind: _,
                    state: _
                })
            ) && label == "Increment date/time value"
        }));
    }

    #[test]
    fn test_handle_event_enter_returns_break_with_current_date_time() {
        let mut state = DateTimePickerState::new(DateTimePickerMode::DateAndTime);
        let date_time = NaiveDate::from_ymd_opt(2024, 1, 2)
            .unwrap()
            .and_hms_opt(3, 4, 5)
            .unwrap();
        state.preset_with_date_time(date_time);

        let result = state
            .handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
            .unwrap();

        assert!(matches!(result, ControlFlow::Break(returned) if returned == date_time));
    }

    #[test]
    fn test_handle_event_tab_cycles_fields_in_date_and_time_mode() {
        let mut state = DateTimePickerState::new(DateTimePickerMode::DateAndTime);
        state.preset_with_date_time(
            NaiveDate::from_ymd_opt(2024, 6, 15)
                .unwrap()
                .and_hms_opt(12, 30, 45)
                .unwrap(),
        );
        state.current_field = DateTimePickerField::Year;

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.current_field, DateTimePickerField::Hour);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.current_field, DateTimePickerField::Minute);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.current_field, DateTimePickerField::Second);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.current_field, DateTimePickerField::Day);
    }

    #[test]
    fn test_handle_event_tab_cycles_fields_in_time_only_mode() {
        let mut state = DateTimePickerState::new(DateTimePickerMode::TimeOnly);
        state.preset_with_date_time(
            NaiveDate::from_ymd_opt(2024, 6, 15)
                .unwrap()
                .and_hms_opt(12, 30, 45)
                .unwrap(),
        );
        assert_eq!(state.current_field, DateTimePickerField::Hour);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.current_field, DateTimePickerField::Minute);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.current_field, DateTimePickerField::Second);

        let _ = state
            .handle_event(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)))
            .unwrap();
        assert_eq!(state.current_field, DateTimePickerField::Hour);
    }

    #[test]
    fn test_render_date_only_mode_renders_three_fields() {
        let mut state = DateTimePickerState::new(DateTimePickerMode::DateOnly);
        state.preset_with_date_time(
            NaiveDate::from_ymd_opt(2024, 5, 31)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        );
        let picker = DateTimePicker::new();
        let area = Rect::new(0, 0, 90, 3);
        let mut buf = Buffer::empty(area);

        picker.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
┌Day:─────────────────────┐┌Month:────────────────────────────┐┌Year:────────────────────┐
│ ▲ 31 ▼                  ││ ▲ May ▼                          ││ ▲ 2024 ▼                │
└─────────────────────────┘└──────────────────────────────────┘└─────────────────────────┘";
        assert_rendered_output(&rendered, expected_output);
    }

    #[test]
    fn test_render_time_only_mode_renders_three_fields() {
        let mut state = DateTimePickerState::new(DateTimePickerMode::TimeOnly);
        state.preset_with_date_time(
            NaiveDate::from_ymd_opt(2024, 1, 1)
                .unwrap()
                .and_hms_opt(5, 6, 7)
                .unwrap(),
        );
        let picker = DateTimePicker::new();
        let area = Rect::new(0, 0, 90, 3);
        let mut buf = Buffer::empty(area);

        picker.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
┌Hour:───────────────────────┐┌Minute:────────────────────┐┌Second:─────────────────────┐ 
│ ▲ 5 ▼                      ││ ▲ 6 ▼                     ││ ▲ 7 ▼                      │ 
└────────────────────────────┘└───────────────────────────┘└────────────────────────────┘ ";
        assert_rendered_output(&rendered, expected_output);
    }

    #[test]
    fn test_render_date_and_time_mode_renders_six_fields() {
        let mut state = DateTimePickerState::new(DateTimePickerMode::DateAndTime);
        state.preset_with_date_time(
            NaiveDate::from_ymd_opt(2024, 12, 31)
                .unwrap()
                .and_hms_opt(23, 59, 58)
                .unwrap(),
        );
        let picker = DateTimePicker::new();
        let area = Rect::new(0, 0, 100, 3);
        let mut buf = Buffer::empty(area);

        picker.render(area, &mut buf, &mut state);
        let rendered = buffer_to_string(&buf);
        let expected_output = "
┌Day:─────────┐┌Month:─────────────────┐┌Year:────────┐┌Hour:────────┐┌Minute:──────┐┌Second:──────┐
│ ▲ 31 ▼      ││ ▲ December ▼          ││ ▲ 2024 ▼    ││ ▲ 23 ▼      ││ ▲ 59 ▼      ││ ▲ 58 ▼      │
└─────────────┘└───────────────────────┘└─────────────┘└─────────────┘└─────────────┘└─────────────┘";
        assert_rendered_output(&rendered, expected_output);
    }

    #[test]
    fn test_date_only_mode_initial_field_is_day() {
        let state = DateTimePickerState::new(DateTimePickerMode::DateOnly);
        assert_eq!(state.current_field, DateTimePickerField::Day);
    }

    #[test]
    fn test_time_only_mode_initial_field_is_hour() {
        let state = DateTimePickerState::new(DateTimePickerMode::TimeOnly);
        assert_eq!(state.current_field, DateTimePickerField::Hour);
    }

    #[test]
    fn test_increment_day_wraps_from_30_day_month() {
        let mut state = DateTimePickerState::new(DateTimePickerMode::DateAndTime);
        state.preset_with_date_time(
            NaiveDate::from_ymd_opt(2024, 4, 30)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap(),
        );

        state.increment_day();
        assert_eq!(state.get_day(), 1);
    }

    #[test]
    fn test_increment_day_wraps_from_february() {
        let mut state = DateTimePickerState::new(DateTimePickerMode::DateAndTime);
        state.preset_with_date_time(
            NaiveDate::from_ymd_opt(2024, 2, 28)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap(),
        );

        state.increment_day();
        assert_eq!(state.get_day(), 1);
    }
}
