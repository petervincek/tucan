use std::time::{Duration, Instant};

use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Widget},
};
use textwrap::Options;

/// `NotificationPanel` represents the widget to handle and present notification/info/error messages from
/// interactions and activities from client's side
#[derive(Debug)]
pub struct NotificationPanel {
    notifications: Vec<NotificationMessage>,
    max_lifespan: Duration,
}

/// `NotificationMessage` represents the specific notification message to be displayed in the notification panel
#[derive(Debug, Clone)]
pub enum NotificationMessage {
    InfoMsg(String, Instant),
    WarningMsg(String, Instant),
    ErrorMsg(String, Instant),
}

impl NotificationMessage {
    pub fn created_at(&self) -> Instant {
        match self {
            NotificationMessage::InfoMsg(_, instant) => *instant,
            NotificationMessage::WarningMsg(_, instant) => *instant,
            NotificationMessage::ErrorMsg(_, instant) => *instant,
        }
    }
}

impl NotificationPanel {
    pub fn new(seconds_to_live: u64) -> Self {
        Self {
            notifications: vec![],
            max_lifespan: Duration::from_secs(seconds_to_live),
        }
    }

    /// `add_notification` add the notification at the top of the panel
    pub fn add_notification(&mut self, notification: NotificationMessage) {
        // insert at the top, so the most recent notification is always first
        self.notifications.insert(0, notification);
    }

    /// `tick_cleanup` clears any notification that have outlived the lifespan
    /// can be call once per frame (rendering) loop
    pub fn tick_cleanup(&mut self) {
        let now = Instant::now();
        let lifespan = self.max_lifespan;
        // retain notifications with elapsed time less than the set lifespan
        self.notifications
            .retain(|notification| now.duration_since(notification.created_at()) < lifespan);
    }

    // `clear_notifications` clears the whole panel
    pub fn clear_notifications(&mut self) {
        self.notifications.clear();
    }
}

impl Widget for &mut NotificationPanel {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // do cleanup of old notification messages first
        self.tick_cleanup();

        // map the notification messages into ListItem(s)
        let list_items: Vec<ListItem> = self
            .notifications
            .iter()
            .map(|notification| {
                let (prefix, color, msg) = match notification {
                    NotificationMessage::InfoMsg(msg, _) => ("[i] -> ", Color::Blue, msg),
                    NotificationMessage::WarningMsg(msg, _) => ("[!] -> ", Color::Yellow, msg),
                    NotificationMessage::ErrorMsg(msg, _) => ("[✘] -> ", Color::Red, msg),
                };

                // prepare the list/accumator for the wrapped/formatted message for List widget in form of
                // multiple list items
                let mut list_items: Vec<ListItem> = vec![];
                // style the prefix
                let prefix_span = Span::styled(prefix, Style::default().fg(color).bold());
                // add it as list item
                list_items.push(ListItem::from(Text::from(Line::from(prefix_span))));
                // wrapped the message text and create list items from it
                for s in textwrap::wrap(
                    msg,
                    Options::new(area.width.saturating_sub(4).max(1) as usize),
                ) {
                    list_items.push(ListItem::from(Text::from(Line::from(Span::styled(
                        s,
                        Style::default().fg(color),
                    )))));
                }
                // add one more separator of messages list items
                list_items.push(ListItem::from(Text::from(Line::from(" "))));
                list_items
            })
            .flat_map(|list_items| list_items) // flat-map the data
            .collect();

        // create panel block
        let panel_block = Block::default()
            .title(format!(" Notifications "))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        // create list
        let list = List::new(list_items).block(panel_block);
        list.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::buffer::{assert_rendered_output, buffer_to_string};

    use super::*;
    use ratatui::{layout::Rect, prelude::Buffer};

    #[test]
    fn test_notification_panel_new_is_empty_with_correct_lifespan() {
        let panel = NotificationPanel::new(5);

        assert!(panel.notifications.is_empty());
        assert_eq!(panel.max_lifespan, Duration::from_secs(5));
    }

    #[test]
    fn test_add_notification_inserts_new_message_at_top() {
        let mut panel = NotificationPanel::new(10);

        panel.add_notification(NotificationMessage::InfoMsg(
            "first".to_string(),
            Instant::now(),
        ));
        panel.add_notification(NotificationMessage::WarningMsg(
            "second".to_string(),
            Instant::now(),
        ));

        match &panel.notifications[..] {
            [
                NotificationMessage::WarningMsg(msg2, _),
                NotificationMessage::InfoMsg(msg1, _),
            ] => {
                assert_eq!(msg1, "first");
                assert_eq!(msg2, "second");
            }
            _ => panic!("Notifications were not inserted in reverse chronological order"),
        }
    }

    #[test]
    fn test_clear_notifications_removes_all_messages() {
        let mut panel = NotificationPanel::new(10);
        panel.add_notification(NotificationMessage::InfoMsg(
            "one".to_string(),
            Instant::now(),
        ));
        panel.clear_notifications();

        assert!(panel.notifications.is_empty());
    }

    #[test]
    fn test_tick_cleanup_removes_expired_notifications() {
        let mut panel = NotificationPanel::new(5);
        panel.add_notification(NotificationMessage::InfoMsg(
            "fresh".to_string(),
            Instant::now(),
        ));
        panel.add_notification(NotificationMessage::ErrorMsg(
            "expired".to_string(),
            Instant::now() - Duration::from_secs(10),
        ));

        panel.tick_cleanup();

        assert_eq!(panel.notifications.len(), 1);
        assert!(matches!(
            panel.notifications[0],
            NotificationMessage::InfoMsg(_, _)
        ));
    }

    #[test]
    fn test_tick_cleanup_keeps_notification_just_before_lifespan() {
        let mut panel = NotificationPanel::new(2);
        panel.add_notification(NotificationMessage::InfoMsg(
            "almost_expired".to_string(),
            Instant::now() - Duration::from_millis(1000),
        ));

        panel.tick_cleanup();

        assert_eq!(panel.notifications.len(), 1);
    }

    #[test]
    fn test_render_does_not_panic_when_panel_is_empty() {
        let mut panel = NotificationPanel::new(10);
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);

        (&mut panel).render(area, &mut buf);
    }

    #[test]
    fn test_render_renders_notification_prefix_and_message() {
        let mut panel = NotificationPanel::new(10);
        panel.add_notification(NotificationMessage::InfoMsg(
            "info message".to_string(),
            Instant::now(),
        ));
        panel.add_notification(NotificationMessage::WarningMsg(
            "warning message".to_string(),
            Instant::now(),
        ));

        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        (&mut panel).render(area, &mut buf);

        let rendered = buffer_to_string(&buf);
        let expected_output = "
┌ Notifications ───────────────────────┐
│[!] ->                                │
│warning message                       │
│                                      │
│[i] ->                                │
│info message                          │
│                                      │
│                                      │
│                                      │
└──────────────────────────────────────┘";
        assert_rendered_output(&rendered, expected_output);
    }

    #[test]
    fn test_render_removes_expired_notifications_before_rendering() {
        let mut panel = NotificationPanel::new(5);
        panel.add_notification(NotificationMessage::ErrorMsg(
            "expired".to_string(),
            Instant::now() - Duration::from_secs(6),
        ));
        panel.add_notification(NotificationMessage::InfoMsg(
            "still alive".to_string(),
            Instant::now(),
        ));

        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        (&mut panel).render(area, &mut buf);

        let rendered = buffer_to_string(&buf);
        let expected_output = "
┌ Notifications ───────────────────────┐
│[i] ->                                │
│still alive                           │
│                                      │
│                                      │
│                                      │
│                                      │
│                                      │
│                                      │
└──────────────────────────────────────┘";
        assert_rendered_output(&rendered, expected_output);
        assert_eq!(panel.notifications.len(), 1);
    }

    #[test]
    fn test_render_wraps_long_messages_across_lines() {
        let mut panel = NotificationPanel::new(10);
        panel.add_notification(NotificationMessage::InfoMsg(
            "this is a long text message that should wrap inside the notification panel"
                .to_string(),
            Instant::now(),
        ));

        let area = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(area);
        (&mut panel).render(area, &mut buf);

        let rendered = buffer_to_string(&buf);
        let expected_rendered_output = "
┌ Notifications ───┐
│[i] ->            │
│this is a long    │
│text message      │
│that should       │
│wrap inside the   │
│notification      │
│panel             │
│                  │
└──────────────────┘";
        assert_rendered_output(&rendered, expected_rendered_output);
    }
}
