//! Per-app Notification Badges with Privacy Blur

#[derive(Debug, Clone)]
pub struct NotificationItem {
    pub package_name: String,
    pub title: String,
    pub count: u32,
    pub icon_res_id: u32,
}

pub struct NotificationWidget {
    pub items: Vec<NotificationItem>,
    pub is_privacy_blurred: bool,
}

impl NotificationWidget {
    pub fn new(privacy_blur: bool) -> Self {
        Self {
            items: Vec::new(),
            is_privacy_blurred: privacy_blur,
        }
    }

    pub fn set_unlocked(&mut self, unlocked: bool) {
        // When unlocked by biometric auth, unblur notification content
        self.is_privacy_blurred = !unlocked;
    }

    pub fn add_notification(&mut self, item: NotificationItem) {
        if let Some(existing) = self
            .items
            .iter_mut()
            .find(|n| n.package_name == item.package_name)
        {
            existing.count += item.count;
        } else {
            self.items.push(item);
        }
    }

    pub fn total_unread_count(&self) -> u32 {
        self.items.iter().map(|n| n.count).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notifications_privacy_and_count() {
        let mut widget = NotificationWidget::new(true);
        assert!(widget.is_privacy_blurred);

        widget.add_notification(NotificationItem {
            package_name: "org.telegram.messenger".to_string(),
            title: "Alice".to_string(),
            count: 3,
            icon_res_id: 101,
        });

        assert_eq!(widget.total_unread_count(), 3);
        widget.set_unlocked(true);
        assert!(!widget.is_privacy_blurred);
    }
}
