//! Email Service
//!
//! Sends transactional emails via SMTP using the `lettre` crate.
//! If SMTP is not configured the functions log a warning and return `Ok(())`
//! so callers do not need to handle the "email disabled" case specially.

use anyhow::{Context, Result};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::header::ContentType,
    transport::smtp::authentication::Credentials,
};
use tracing::{info, warn};

/// SMTP configuration read from environment variables at call time.
///
/// All fields are optional; if `SMTP_HOST` is absent, email sending is
/// silently skipped.
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
}

impl SmtpConfig {
    /// Load SMTP configuration from environment variables.
    ///
    /// Returns `None` if `SMTP_HOST` is not set (email disabled).
    pub fn from_env() -> Option<Self> {
        let host = std::env::var("SMTP_HOST").ok()?;
        let port = std::env::var("SMTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(587u16);
        let username = std::env::var("SMTP_USER").unwrap_or_default();
        let password = std::env::var("SMTP_PASS").unwrap_or_default();
        let from =
            std::env::var("SMTP_FROM").unwrap_or_else(|_| format!("ParkHub <noreply@{host}>"));

        Some(Self {
            host,
            port,
            username,
            password,
            from,
        })
    }
}

/// Send an HTML email.
///
/// If SMTP is not configured (`SMTP_HOST` env var is absent) the call is a
/// no-op and returns `Ok(())`.  This provides graceful degradation in
/// development and self-hosted environments without an SMTP relay.
pub async fn send_email(to: &str, subject: &str, html_body: &str) -> Result<()> {
    let Some(config) = SmtpConfig::from_env() else {
        warn!(
            to = %to,
            subject = %subject,
            "SMTP not configured (SMTP_HOST not set) — email skipped"
        );
        return Ok(());
    };

    let message = Message::builder()
        .from(config.from.parse().context("Invalid SMTP_FROM address")?)
        .to(to.parse().context("Invalid recipient email address")?)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(html_body.to_string())
        .context("Failed to build email message")?;

    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
            .context("Failed to create SMTP transport")?
            .port(config.port)
            .credentials(Credentials::new(
                config.username.clone(),
                config.password.clone(),
            ))
            .build();

    mailer.send(message).await.context("Failed to send email")?;

    info!(to = %to, subject = %subject, "Email sent successfully");
    Ok(())
}

/// Build a booking confirmation email body.
#[allow(clippy::too_many_arguments)]
pub fn build_booking_confirmation_email(
    user_name: &str,
    booking_id: &str,
    floor_name: &str,
    slot_number: i32,
    start_time: &str,
    end_time: &str,
    org_name: &str,
) -> String {
    use crate::utils::html_escape;
    let org_raw = if org_name.is_empty() {
        "ParkHub"
    } else {
        org_name
    };
    let org = html_escape(org_raw);
    let user_name = html_escape(user_name);
    let booking_id = html_escape(booking_id);
    let floor_name = html_escape(floor_name);
    let start_time = html_escape(start_time);
    let end_time = html_escape(end_time);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>Booking Confirmation — {org}</title>
  <style>
    body {{ font-family: Arial, sans-serif; background: #f4f4f4; margin: 0; padding: 0; }}
    .container {{ max-width: 600px; margin: 40px auto; background: #ffffff; border-radius: 8px;
                  padding: 40px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }}
    h1 {{ color: #1a73e8; margin-top: 0; }}
    p  {{ color: #333333; line-height: 1.6; }}
    .detail-table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
    .detail-table td {{ padding: 10px 12px; border-bottom: 1px solid #eeeeee; font-size: 14px; color: #333333; }}
    .detail-table td:first-child {{ font-weight: bold; width: 40%; color: #555555; }}
    .booking-ref {{ display: inline-block; background: #e8f0fe; color: #1a73e8; padding: 8px 16px;
                    border-radius: 4px; font-family: monospace; font-size: 13px; margin: 8px 0; }}
    .footer {{ margin-top: 40px; font-size: 12px; color: #888888; border-top: 1px solid #eeeeee;
               padding-top: 16px; }}
  </style>
</head>
<body>
  <div class="container">
    <h1>{org} — Booking Confirmed</h1>
    <p>Dear <strong>{user_name}</strong>,</p>
    <p>Your parking booking has been confirmed. Here are your booking details:</p>
    <div class="booking-ref">{booking_id}</div>
    <table class="detail-table">
      <tr><td>Floor</td><td>{floor_name}</td></tr>
      <tr><td>Slot Number</td><td>{slot_number}</td></tr>
      <tr><td>Start Time</td><td>{start_time}</td></tr>
      <tr><td>End Time</td><td>{end_time}</td></tr>
    </table>
    <p>Please keep this email as your booking reference. You can view or cancel your booking
       at any time from your account.</p>
    <div class="footer">
      <p>This email was sent by {org}. If you have questions, contact your administrator.</p>
    </div>
  </div>
</body>
</html>"#,
    )
}

/// Build a password-reset email body.
pub fn build_password_reset_email(reset_url: &str, org_name: &str) -> String {
    use crate::utils::html_escape;
    let org_raw = if org_name.is_empty() {
        "ParkHub"
    } else {
        org_name
    };
    let org = html_escape(org_raw);
    let reset_url = html_escape(reset_url);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>Password Reset — {org}</title>
  <style>
    body {{ font-family: Arial, sans-serif; background: #f4f4f4; margin: 0; padding: 0; }}
    .container {{ max-width: 600px; margin: 40px auto; background: #ffffff; border-radius: 8px;
                  padding: 40px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }}
    h1 {{ color: #1a73e8; margin-top: 0; }}
    p  {{ color: #333333; line-height: 1.6; }}
    .btn {{ display: inline-block; background: #1a73e8; color: #ffffff; padding: 14px 28px;
            border-radius: 6px; text-decoration: none; font-weight: bold; margin: 20px 0; }}
    .footer {{ margin-top: 40px; font-size: 12px; color: #888888; border-top: 1px solid #eeeeee;
               padding-top: 16px; }}
  </style>
</head>
<body>
  <div class="container">
    <h1>{org} — Password Reset</h1>
    <p>You requested a password reset for your <strong>{org}</strong> account.</p>
    <p>Click the button below to set a new password. The link is valid for <strong>1 hour</strong>.</p>
    <a href="{reset_url}" class="btn">Reset Password</a>
    <p>If you did not request this, please ignore this email. Your password will not change.</p>
    <div class="footer">
      <p>This email was sent by {org}. If you have questions, contact your administrator.</p>
    </div>
  </div>
</body>
</html>"#,
    )
}

/// Build a welcome email body for new user registrations.
pub fn build_welcome_email(user_name: &str, org_name: &str) -> String {
    use crate::utils::html_escape;
    let org_raw = if org_name.is_empty() {
        "ParkHub"
    } else {
        org_name
    };
    let org = html_escape(org_raw);
    let user_name = html_escape(user_name);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>Welcome to {org}</title>
  <style>
    body {{ font-family: Arial, sans-serif; background: #f4f4f4; margin: 0; padding: 0; }}
    .container {{ max-width: 600px; margin: 40px auto; background: #ffffff; border-radius: 8px;
                  padding: 40px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }}
    h1 {{ color: #1a73e8; margin-top: 0; }}
    p  {{ color: #333333; line-height: 1.6; }}
    .highlight {{ background: #e8f0fe; border-left: 4px solid #1a73e8; padding: 16px; border-radius: 4px;
                  margin: 20px 0; }}
    .footer {{ margin-top: 40px; font-size: 12px; color: #888888; border-top: 1px solid #eeeeee;
               padding-top: 16px; }}
  </style>
</head>
<body>
  <div class="container">
    <h1>Welcome to {org}!</h1>
    <p>Dear <strong>{user_name}</strong>,</p>
    <p>Your account has been created successfully. You can now log in and start booking parking slots.</p>
    <div class="highlight">
      <p><strong>Getting started:</strong></p>
      <p>Browse available parking lots, book your preferred slot, and manage your bookings from your dashboard.</p>
    </div>
    <p>If you have any questions, please contact your administrator.</p>
    <div class="footer">
      <p>This email was sent by {org}. You received this because an account was created with your email address.</p>
    </div>
  </div>
</body>
</html>"#,
    )
}

/// Build a booking reminder email body sent before the booking starts.
///
/// `minutes_until` — how many minutes until the booking begins (e.g. 30).
#[allow(clippy::too_many_arguments)]
pub fn build_booking_reminder_email(
    user_name: &str,
    booking_id: &str,
    floor_name: &str,
    slot_number: i32,
    start_time: &str,
    end_time: &str,
    minutes_until: i64,
    org_name: &str,
) -> String {
    use crate::utils::html_escape;
    let org_raw = if org_name.is_empty() {
        "ParkHub"
    } else {
        org_name
    };
    let org = html_escape(org_raw);
    let user_name = html_escape(user_name);
    let booking_id = html_escape(booking_id);
    let floor_name = html_escape(floor_name);
    let start_time = html_escape(start_time);
    let end_time = html_escape(end_time);
    let countdown = if minutes_until == 1 {
        "1 minute".to_string()
    } else {
        format!("{minutes_until} minutes")
    };
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>Booking Reminder — {org}</title>
  <style>
    body {{ font-family: Arial, sans-serif; background: #f4f4f4; margin: 0; padding: 0; }}
    .container {{ max-width: 600px; margin: 40px auto; background: #ffffff; border-radius: 8px;
                  padding: 40px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }}
    h1 {{ color: #f9a825; margin-top: 0; }}
    p  {{ color: #333333; line-height: 1.6; }}
    .detail-table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
    .detail-table td {{ padding: 10px 12px; border-bottom: 1px solid #eeeeee; font-size: 14px; color: #333333; }}
    .detail-table td:first-child {{ font-weight: bold; width: 40%; color: #555555; }}
    .booking-ref {{ display: inline-block; background: #fff8e1; color: #f9a825; padding: 8px 16px;
                    border-radius: 4px; font-family: monospace; font-size: 13px; margin: 8px 0; }}
    .highlight {{ background: #fff8e1; border-left: 4px solid #f9a825; padding: 12px 16px;
                  border-radius: 4px; margin: 16px 0; }}
    .footer {{ margin-top: 40px; font-size: 12px; color: #888888; border-top: 1px solid #eeeeee;
               padding-top: 16px; }}
  </style>
</head>
<body>
  <div class="container">
    <h1>{org} — Booking Reminder</h1>
    <p>Dear <strong>{user_name}</strong>,</p>
    <div class="highlight">
      <p>Your parking booking starts in <strong>{countdown}</strong>.</p>
    </div>
    <div class="booking-ref">{booking_id}</div>
    <table class="detail-table">
      <tr><td>Floor</td><td>{floor_name}</td></tr>
      <tr><td>Slot Number</td><td>{slot_number}</td></tr>
      <tr><td>Start Time</td><td>{start_time}</td></tr>
      <tr><td>End Time</td><td>{end_time}</td></tr>
    </table>
    <p>Please make your way to the parking area on time. The slot will be held for the duration of your booking.</p>
    <div class="footer">
      <p>This email was sent by {org}. If you have questions, contact your administrator.</p>
    </div>
  </div>
</body>
</html>"#,
    )
}

/// Build a waitlist-slot-available notification email body.
///
/// Sent to the first user on the waitlist when a slot in their desired lot
/// becomes available (e.g. after a cancellation).
pub fn build_waitlist_slot_available_email(
    user_name: &str,
    lot_name: &str,
    org_name: &str,
) -> String {
    use crate::utils::html_escape;
    let org_raw = if org_name.is_empty() {
        "ParkHub"
    } else {
        org_name
    };
    let org = html_escape(org_raw);
    let user_name = html_escape(user_name);
    let lot_name = html_escape(lot_name);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>Parking Slot Available — {org}</title>
  <style>
    body {{ font-family: Arial, sans-serif; background: #f4f4f4; margin: 0; padding: 0; }}
    .container {{ max-width: 600px; margin: 40px auto; background: #ffffff; border-radius: 8px;
                  padding: 40px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }}
    h1 {{ color: #34a853; margin-top: 0; }}
    p  {{ color: #333333; line-height: 1.6; }}
    .highlight {{ background: #e6f4ea; border-left: 4px solid #34a853; padding: 16px;
                  border-radius: 4px; margin: 20px 0; }}
    .footer {{ margin-top: 40px; font-size: 12px; color: #888888; border-top: 1px solid #eeeeee;
               padding-top: 16px; }}
  </style>
</head>
<body>
  <div class="container">
    <h1>{org} — Parking Slot Available</h1>
    <p>Dear <strong>{user_name}</strong>,</p>
    <div class="highlight">
      <p>Good news! A parking slot has become available at <strong>{lot_name}</strong>.</p>
      <p>You are on the waitlist for this parking lot. Log in now to book your slot before it is taken.</p>
    </div>
    <p>Please note that slots are available on a first-come, first-served basis. Act quickly to secure your spot.</p>
    <div class="footer">
      <p>This email was sent by {org}. You received this because you are on the waitlist for {lot_name}.</p>
      <p>To remove yourself from the waitlist, log in to your account.</p>
    </div>
  </div>
</body>
</html>"#,
    )
}

/// Build a booking cancellation confirmation email body.
#[allow(clippy::too_many_arguments)]
pub fn build_booking_cancellation_email(
    user_name: &str,
    booking_id: &str,
    floor_name: &str,
    slot_number: i32,
    start_time: &str,
    end_time: &str,
    org_name: &str,
) -> String {
    use crate::utils::html_escape;
    let org_raw = if org_name.is_empty() {
        "ParkHub"
    } else {
        org_name
    };
    let org = html_escape(org_raw);
    let user_name = html_escape(user_name);
    let booking_id = html_escape(booking_id);
    let floor_name = html_escape(floor_name);
    let start_time = html_escape(start_time);
    let end_time = html_escape(end_time);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>Booking Cancelled — {org}</title>
  <style>
    body {{ font-family: Arial, sans-serif; background: #f4f4f4; margin: 0; padding: 0; }}
    .container {{ max-width: 600px; margin: 40px auto; background: #ffffff; border-radius: 8px;
                  padding: 40px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }}
    h1 {{ color: #d93025; margin-top: 0; }}
    p  {{ color: #333333; line-height: 1.6; }}
    .detail-table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
    .detail-table td {{ padding: 10px 12px; border-bottom: 1px solid #eeeeee; font-size: 14px; color: #333333; }}
    .detail-table td:first-child {{ font-weight: bold; width: 40%; color: #555555; }}
    .booking-ref {{ display: inline-block; background: #fce8e6; color: #d93025; padding: 8px 16px;
                    border-radius: 4px; font-family: monospace; font-size: 13px; margin: 8px 0; }}
    .footer {{ margin-top: 40px; font-size: 12px; color: #888888; border-top: 1px solid #eeeeee;
               padding-top: 16px; }}
  </style>
</head>
<body>
  <div class="container">
    <h1>{org} — Booking Cancelled</h1>
    <p>Dear <strong>{user_name}</strong>,</p>
    <p>Your parking booking has been cancelled. The slot has been released and is available for others.</p>
    <div class="booking-ref">{booking_id}</div>
    <table class="detail-table">
      <tr><td>Floor</td><td>{floor_name}</td></tr>
      <tr><td>Slot Number</td><td>{slot_number}</td></tr>
      <tr><td>Original Start</td><td>{start_time}</td></tr>
      <tr><td>Original End</td><td>{end_time}</td></tr>
      <tr><td>Status</td><td>Cancelled</td></tr>
    </table>
    <p>If credits were deducted for this booking, they have been refunded to your account.</p>
    <div class="footer">
      <p>This email was sent by {org}. If you have questions, contact your administrator.</p>
    </div>
  </div>
</body>
</html>"#,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SmtpConfig::from_env ──

    #[test]
    #[allow(unsafe_code)]
    fn smtp_config_returns_none_when_host_missing() {
        // Ensure SMTP_HOST is not set (tests run without it by default)
        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::remove_var("SMTP_HOST") };
        assert!(SmtpConfig::from_env().is_none());
    }

    #[test]
    #[allow(unsafe_code)]
    fn smtp_config_parses_env_vars() {
        // Set up temporary env vars
        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::set_var("SMTP_HOST", "mail.example.com") };
        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::set_var("SMTP_PORT", "465") };
        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::set_var("SMTP_USER", "user@example.com") };
        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::set_var("SMTP_PASS", "secret") };
        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::set_var("SMTP_FROM", "Test <test@example.com>") };

        let config = SmtpConfig::from_env().expect("should parse SMTP config");
        assert_eq!(config.host, "mail.example.com");
        assert_eq!(config.port, 465);
        assert_eq!(config.username, "user@example.com");
        assert_eq!(config.password, "secret");
        assert_eq!(config.from, "Test <test@example.com>");

        // Clean up
        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::remove_var("SMTP_HOST") };
        unsafe { std::env::remove_var("SMTP_PORT") };
        unsafe { std::env::remove_var("SMTP_USER") };
        unsafe { std::env::remove_var("SMTP_PASS") };
        unsafe { std::env::remove_var("SMTP_FROM") };
    }

    #[test]
    #[allow(unsafe_code)]
    fn smtp_config_defaults_port_to_587() {
        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::set_var("SMTP_HOST", "mail.test.io") };
        unsafe { std::env::remove_var("SMTP_PORT") };
        unsafe { std::env::remove_var("SMTP_USER") };
        unsafe { std::env::remove_var("SMTP_PASS") };
        unsafe { std::env::remove_var("SMTP_FROM") };

        let config = SmtpConfig::from_env().unwrap();
        assert_eq!(config.port, 587);
        assert_eq!(config.from, "ParkHub <noreply@mail.test.io>");

        unsafe { std::env::remove_var("SMTP_HOST") };
    }

    // ── build_booking_confirmation_email ──

    #[test]
    fn booking_email_contains_user_name_and_booking_id() {
        let html = build_booking_confirmation_email(
            "Alice",
            "BK-001",
            "Ground Floor",
            5,
            "2026-03-20 09:00",
            "2026-03-20 17:00",
            "Acme",
        );
        assert!(html.contains("Alice"));
        assert!(html.contains("BK-001"));
        assert!(html.contains("Ground Floor"));
        assert!(html.contains("2026-03-20 09:00"));
        assert!(html.contains("2026-03-20 17:00"));
        assert!(html.contains("Acme"));
    }

    #[test]
    fn booking_email_defaults_org_to_parkhub() {
        let html =
            build_booking_confirmation_email("Bob", "BK-002", "Level 2", 3, "09:00", "12:00", "");
        assert!(html.contains("ParkHub"));
        assert!(!html.contains("Acme"));
    }

    #[test]
    fn booking_email_escapes_html_in_user_name() {
        let html = build_booking_confirmation_email(
            "<script>alert(1)</script>",
            "BK-XSS",
            "Floor",
            1,
            "09:00",
            "10:00",
            "",
        );
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn booking_email_contains_slot_number() {
        let html = build_booking_confirmation_email(
            "Carol", "BK-003", "Deck A", 42, "08:00", "18:00", "ParkCo",
        );
        assert!(html.contains("42"));
    }

    #[test]
    fn booking_email_is_valid_html() {
        let html = build_booking_confirmation_email(
            "Dave", "BK-004", "B1", 7, "10:00", "11:00", "TestOrg",
        );
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<title>Booking Confirmation"));
    }

    // ── build_password_reset_email ──

    #[test]
    fn reset_email_contains_url() {
        let html =
            build_password_reset_email("https://park.example.com/reset?token=abc123", "MyOrg");
        assert!(html.contains("https://park.example.com/reset?token=abc123"));
        assert!(html.contains("MyOrg"));
    }

    #[test]
    fn reset_email_defaults_org_to_parkhub() {
        let html = build_password_reset_email("https://example.com/reset", "");
        assert!(html.contains("ParkHub"));
    }

    #[test]
    fn reset_email_escapes_html_in_url() {
        let html = build_password_reset_email("https://evil.com?a=1&b=2", "");
        assert!(html.contains("&amp;b=2"));
    }

    #[test]
    fn reset_email_is_valid_html() {
        let html = build_password_reset_email("https://example.com/reset", "Corp");
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<title>Password Reset"));
    }

    #[test]
    fn reset_email_contains_button_with_href() {
        let html = build_password_reset_email("https://example.com/reset?t=xyz", "");
        assert!(html.contains(r#"href="https://example.com/reset?t=xyz""#));
        assert!(html.contains("Reset Password"));
    }

    #[test]
    fn reset_email_mentions_one_hour_validity() {
        let html = build_password_reset_email("https://example.com/r", "");
        assert!(html.contains("1 hour"));
    }

    // ── build_welcome_email ──

    #[test]
    fn welcome_email_contains_user_name() {
        let html = build_welcome_email("Alice", "Acme Corp");
        assert!(html.contains("Alice"));
        assert!(html.contains("Acme Corp"));
    }

    #[test]
    fn welcome_email_defaults_org_to_parkhub() {
        let html = build_welcome_email("Bob", "");
        assert!(html.contains("ParkHub"));
    }

    #[test]
    fn welcome_email_escapes_html() {
        let html = build_welcome_email("<script>xss</script>", "");
        assert!(!html.contains("<script>xss"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn welcome_email_is_valid_html() {
        let html = build_welcome_email("Carol", "TestOrg");
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<title>Welcome to TestOrg</title>"));
    }

    #[test]
    fn welcome_email_mentions_getting_started() {
        let html = build_welcome_email("Dave", "");
        assert!(html.contains("Getting started"));
    }

    // ── build_booking_reminder_email ──

    #[test]
    fn reminder_email_contains_booking_details() {
        let html = build_booking_reminder_email(
            "Alice",
            "BK-001",
            "Ground Floor",
            5,
            "2026-03-20 09:00",
            "2026-03-20 17:00",
            30,
            "Acme",
        );
        assert!(html.contains("Alice"));
        assert!(html.contains("BK-001"));
        assert!(html.contains("Ground Floor"));
        assert!(html.contains("30 minutes"));
        assert!(html.contains("Acme"));
    }

    #[test]
    fn reminder_email_singular_minute() {
        let html =
            build_booking_reminder_email("Bob", "BK-002", "Level 1", 3, "09:00", "10:00", 1, "");
        assert!(html.contains("1 minute"));
        assert!(!html.contains("1 minutes"));
    }

    #[test]
    fn reminder_email_escapes_html() {
        let html = build_booking_reminder_email(
            "<b>Hacker</b>",
            "BK-XSS",
            "Floor",
            1,
            "09:00",
            "10:00",
            30,
            "",
        );
        assert!(!html.contains("<b>Hacker</b>"));
        assert!(html.contains("&lt;b&gt;"));
    }

    #[test]
    fn reminder_email_is_valid_html() {
        let html = build_booking_reminder_email(
            "Carol", "BK-003", "A", 42, "08:00", "18:00", 30, "ParkCo",
        );
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<title>Booking Reminder"));
    }

    // ── build_waitlist_slot_available_email ──

    #[test]
    fn waitlist_email_contains_lot_name() {
        let html = build_waitlist_slot_available_email("Alice", "Lot A", "ParkCo");
        assert!(html.contains("Alice"));
        assert!(html.contains("Lot A"));
        assert!(html.contains("ParkCo"));
    }

    #[test]
    fn waitlist_email_defaults_org_to_parkhub() {
        let html = build_waitlist_slot_available_email("Bob", "Lot B", "");
        assert!(html.contains("ParkHub"));
    }

    #[test]
    fn waitlist_email_escapes_html() {
        let html = build_waitlist_slot_available_email("<script>xss</script>", "Lot", "");
        assert!(!html.contains("<script>xss"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn waitlist_email_is_valid_html() {
        let html = build_waitlist_slot_available_email("Carol", "Main Lot", "TestOrg");
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
        assert!(html.contains("Parking Slot Available"));
    }

    #[test]
    fn waitlist_email_mentions_waitlist() {
        let html = build_waitlist_slot_available_email("Dave", "Lot D", "");
        assert!(html.contains("waitlist"));
    }

    // ── build_booking_cancellation_email ──

    #[test]
    fn cancellation_email_contains_booking_details() {
        let html = build_booking_cancellation_email(
            "Alice",
            "BK-001",
            "Ground Floor",
            5,
            "2026-03-20 09:00",
            "2026-03-20 17:00",
            "Acme",
        );
        assert!(html.contains("Alice"));
        assert!(html.contains("BK-001"));
        assert!(html.contains("Ground Floor"));
        assert!(html.contains("Cancelled"));
        assert!(html.contains("Acme"));
    }

    #[test]
    fn cancellation_email_defaults_org_to_parkhub() {
        let html =
            build_booking_cancellation_email("Bob", "BK-002", "Level 2", 3, "09:00", "12:00", "");
        assert!(html.contains("ParkHub"));
    }

    #[test]
    fn cancellation_email_escapes_html() {
        let html =
            build_booking_cancellation_email("<img src=x>", "BK-XSS", "F", 1, "09:00", "10:00", "");
        assert!(!html.contains("<img src=x>"));
        assert!(html.contains("&lt;img"));
    }

    #[test]
    fn cancellation_email_is_valid_html() {
        let html = build_booking_cancellation_email(
            "Carol", "BK-003", "A", 42, "08:00", "18:00", "ParkCo",
        );
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<title>Booking Cancelled"));
    }

    #[test]
    fn cancellation_email_mentions_credit_refund() {
        let html = build_booking_cancellation_email("Eve", "BK-004", "B1", 7, "10:00", "11:00", "");
        assert!(html.contains("refunded"));
    }

    // ── send_email (no SMTP configured) ──

    #[tokio::test]
    #[allow(unsafe_code)]
    async fn send_email_noop_when_smtp_not_configured() {
        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::remove_var("SMTP_HOST") };
        let result = send_email("user@example.com", "Test", "<p>Hello</p>").await;
        assert!(result.is_ok());
    }
}
