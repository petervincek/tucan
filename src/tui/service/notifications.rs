use crate::tui::components::notification_panel::NotificationMessage;
use tokio::{sync::mpsc::Sender, task::JoinHandle};
use tracing::{debug, error};

/// `NotificationService` will be a service mainly responsible for sending `NotificationMessage`s from
/// various other services and pages in asynchronous manner in order to notify the client of
/// this application about important events and errors
pub struct NotificationService {
    sender: Sender<NotificationMessage>,
}

impl NotificationService {
    pub fn new(sender: Sender<NotificationMessage>) -> Self {
        Self { sender }
    }

    // `send_notification` sends the provided `NotificationMessage` with fire-and-forget approach
    pub fn send_notification(&self, notification: NotificationMessage) -> JoinHandle<()> {
        let sender = self.sender.clone();

        tokio::spawn(async move {
            let result = sender.send(notification.clone()).await;
            if let Err(error) = result {
                error!("Failure sending notification: {notification:?}, error: {error}");
            } else {
                debug!("Notification: {notification:?} sent successfully");
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::components::notification_panel::NotificationMessage;
    use std::time::Instant;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn send_notification_delivers_notification_to_receiver() {
        // prepare the sut and test data
        let (sender, mut receiver) = mpsc::channel(1);
        let service = NotificationService::new(sender);
        let expected_text = "hello notification".to_string();
        let notification = NotificationMessage::InfoMsg(expected_text.clone(), Instant::now());

        // exercise
        let handle = service.send_notification(notification);

        // verify
        let received = receiver
            .recv()
            .await
            .expect("expected a notification message");

        match received {
            NotificationMessage::InfoMsg(text, created_at) => {
                assert_eq!(text, expected_text);
                assert!(created_at <= Instant::now());
            }
            other => panic!("unexpected notification variant: {other:?}"),
        }

        handle
            .await
            .expect("notification task should complete successfully");
    }

    #[tokio::test]
    async fn send_notification_can_send_multiple_messages() {
        // prepare the sut and test data
        let (sender, mut receiver) = mpsc::channel(3);
        let service = NotificationService::new(sender);

        let notifications = vec![
            NotificationMessage::InfoMsg("one".into(), Instant::now()),
            NotificationMessage::WarningMsg("two".into(), Instant::now()),
            NotificationMessage::ErrorMsg("three".into(), Instant::now()),
        ];

        // exercise
        let mut handles = Vec::new();
        for notification in notifications.iter().cloned() {
            handles.push(service.send_notification(notification));
        }

        // verify
        let mut received_texts = Vec::new();
        for _ in 0..3 {
            if let Some(notification) = receiver.recv().await {
                received_texts.push(notification);
            }
        }

        for handle in handles {
            handle
                .await
                .expect("notification task should complete successfully");
        }

        assert_eq!(received_texts.len(), 3);
        assert!(
            received_texts
                .iter()
                .any(|item| matches!(item, NotificationMessage::InfoMsg(text, _) if text == "one"))
        );
        assert!(
            received_texts.iter().any(
                |item| matches!(item, NotificationMessage::WarningMsg(text, _) if text == "two")
            )
        );
        assert!(
            received_texts.iter().any(
                |item| matches!(item, NotificationMessage::ErrorMsg(text, _) if text == "three")
            )
        );
    }

    #[tokio::test]
    async fn send_notification_handles_closed_receiver_gracefully() {
        // prepare the sut and test data
        let (sender, receiver) = mpsc::channel(1);
        drop(receiver);

        let service = NotificationService::new(sender);
        let notification = NotificationMessage::ErrorMsg("fail path".into(), Instant::now());

        // exercise, verify
        let handle = service.send_notification(notification);
        handle
            .await
            .expect("notification task should complete even when receiver is closed");
    }
}
