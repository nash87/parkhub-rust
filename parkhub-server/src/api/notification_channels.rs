//! Notification channel stubs: SMS and WhatsApp.
//!
//! These are stub implementations that log the notification instead of
//! actually sending it. They validate and store preferences, and when
//! a booking event occurs, they log what would be sent.

use crate::api::admin_ext::NotificationPreferences;
use tracing::info;

/// Notification event types that trigger channel-specific messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationEvent {
    BookingCreated,
    BookingCancelled,
    BookingReminder,
}

impl std::fmt::Display for NotificationEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BookingCreated => write!(f, "booking_created"),
            Self::BookingCancelled => write!(f, "booking_cancelled"),
            Self::BookingReminder => write!(f, "booking_reminder"),
        }
    }
}

/// Check which channels should be notified for an event and dispatch stubs.
pub fn dispatch_notification(
    prefs: &NotificationPreferences,
    event: NotificationEvent,
    user_id: &str,
    booking_id: &str,
) {
    let phone = prefs.phone_number.as_deref().unwrap_or("(not set)");

    // SMS channel
    let sms_enabled = match event {
        NotificationEvent::BookingCreated => prefs.sms_booking_confirm,
        NotificationEvent::BookingCancelled => prefs.sms_booking_cancelled,
        NotificationEvent::BookingReminder => prefs.sms_booking_reminder,
    };

    if sms_enabled {
        send_sms_stub(phone, user_id, booking_id, event);
    }

    // WhatsApp channel
    let whatsapp_enabled = match event {
        NotificationEvent::BookingCreated => prefs.whatsapp_booking_confirm,
        NotificationEvent::BookingCancelled => prefs.whatsapp_booking_cancelled,
        NotificationEvent::BookingReminder => prefs.whatsapp_booking_reminder,
    };

    if whatsapp_enabled {
        send_whatsapp_stub(phone, user_id, booking_id, event);
    }
}

/// Stub: logs "would send SMS" with the phone number and event details.
fn send_sms_stub(phone: &str, user_id: &str, booking_id: &str, event: NotificationEvent) {
    info!(
        channel = "sms",
        phone = phone,
        user_id = user_id,
        booking_id = booking_id,
        event = %event,
        "[STUB] Would send SMS notification"
    );
}

/// Stub: logs "would send WhatsApp" with the phone number and event details.
fn send_whatsapp_stub(phone: &str, user_id: &str, booking_id: &str, event: NotificationEvent) {
    info!(
        channel = "whatsapp",
        phone = phone,
        user_id = user_id,
        booking_id = booking_id,
        event = %event,
        "[STUB] Would send WhatsApp notification"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn prefs_all_enabled() -> NotificationPreferences {
        NotificationPreferences {
            email_booking_confirm: true,
            email_booking_reminder: true,
            email_swap_request: true,
            push_enabled: true,
            sms_booking_confirm: true,
            sms_booking_reminder: true,
            sms_booking_cancelled: true,
            whatsapp_booking_confirm: true,
            whatsapp_booking_reminder: true,
            whatsapp_booking_cancelled: true,
            phone_number: Some("+491234567890".to_string()),
        }
    }

    fn prefs_sms_only() -> NotificationPreferences {
        NotificationPreferences {
            sms_booking_confirm: true,
            sms_booking_reminder: true,
            sms_booking_cancelled: true,
            phone_number: Some("+491234567890".to_string()),
            ..Default::default()
        }
    }

    fn prefs_whatsapp_only() -> NotificationPreferences {
        NotificationPreferences {
            whatsapp_booking_confirm: true,
            whatsapp_booking_reminder: true,
            whatsapp_booking_cancelled: true,
            phone_number: Some("+491234567890".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_dispatch_booking_created_all_channels() {
        let prefs = prefs_all_enabled();
        // Should not panic — just logs
        dispatch_notification(&prefs, NotificationEvent::BookingCreated, "user-1", "bk-1");
    }

    #[test]
    fn test_dispatch_booking_cancelled_sms_only() {
        let prefs = prefs_sms_only();
        dispatch_notification(
            &prefs,
            NotificationEvent::BookingCancelled,
            "user-2",
            "bk-2",
        );
    }

    #[test]
    fn test_dispatch_booking_reminder_whatsapp_only() {
        let prefs = prefs_whatsapp_only();
        dispatch_notification(&prefs, NotificationEvent::BookingReminder, "user-3", "bk-3");
    }

    #[test]
    fn test_dispatch_no_channels_enabled() {
        let prefs = NotificationPreferences::default();
        // SMS and WhatsApp are disabled by default — should not log anything
        dispatch_notification(&prefs, NotificationEvent::BookingCreated, "user-4", "bk-4");
    }

    #[test]
    fn test_dispatch_no_phone_number() {
        let prefs = NotificationPreferences {
            sms_booking_confirm: true,
            phone_number: None,
            ..Default::default()
        };
        // Should still work, just logs "(not set)" for phone
        dispatch_notification(&prefs, NotificationEvent::BookingCreated, "user-5", "bk-5");
    }

    #[test]
    fn test_notification_event_display() {
        assert_eq!(
            format!("{}", NotificationEvent::BookingCreated),
            "booking_created"
        );
        assert_eq!(
            format!("{}", NotificationEvent::BookingCancelled),
            "booking_cancelled"
        );
        assert_eq!(
            format!("{}", NotificationEvent::BookingReminder),
            "booking_reminder"
        );
    }

    #[test]
    fn test_preferences_serde_with_new_fields() {
        let prefs = prefs_all_enabled();
        let json = serde_json::to_string(&prefs).unwrap();
        let back: NotificationPreferences = serde_json::from_str(&json).unwrap();
        assert!(back.sms_booking_confirm);
        assert!(back.sms_booking_reminder);
        assert!(back.sms_booking_cancelled);
        assert!(back.whatsapp_booking_confirm);
        assert!(back.whatsapp_booking_reminder);
        assert!(back.whatsapp_booking_cancelled);
        assert_eq!(back.phone_number.as_deref(), Some("+491234567890"));
    }

    #[test]
    fn test_preferences_backward_compat() {
        // Old format without SMS/WhatsApp fields should deserialize with defaults
        let json = r#"{
            "email_booking_confirm": true,
            "email_booking_reminder": false,
            "email_swap_request": true,
            "push_enabled": false
        }"#;
        let prefs: NotificationPreferences = serde_json::from_str(json).unwrap();
        assert!(prefs.email_booking_confirm);
        assert!(!prefs.email_booking_reminder);
        // New fields default to false/None
        assert!(!prefs.sms_booking_confirm);
        assert!(!prefs.whatsapp_booking_confirm);
        assert!(prefs.phone_number.is_none());
    }

    #[test]
    fn test_preferences_default() {
        let prefs = NotificationPreferences::default();
        assert!(prefs.email_booking_confirm);
        assert!(prefs.push_enabled);
        assert!(!prefs.sms_booking_confirm);
        assert!(!prefs.whatsapp_booking_confirm);
        assert!(prefs.phone_number.is_none());
    }

    #[test]
    fn test_notification_event_equality() {
        assert_eq!(
            NotificationEvent::BookingCreated,
            NotificationEvent::BookingCreated
        );
        assert_ne!(
            NotificationEvent::BookingCreated,
            NotificationEvent::BookingCancelled
        );
    }
}
