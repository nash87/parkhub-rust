//! Email Service
//!
//! Sends transactional emails via SMTP using the `lettre` crate.
//! If SMTP is not configured the functions log a warning and return `Ok(())`
//! so callers do not need to handle the "email disabled" case specially.

use anyhow::{Context, Result};
use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
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
        let from = std::env::var("SMTP_FROM")
            .unwrap_or_else(|_| format!("ParkHub <noreply@{}>", host));

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
    let config = match SmtpConfig::from_env() {
        Some(c) => c,
        None => {
            warn!(
                to = %to,
                subject = %subject,
                "SMTP not configured (SMTP_HOST not set) — email skipped"
            );
            return Ok(());
        }
    };

    let message = Message::builder()
        .from(
            config
                .from
                .parse()
                .context("Invalid SMTP_FROM address")?,
        )
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

    mailer
        .send(message)
        .await
        .context("Failed to send email")?;

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
    let org = if org_name.is_empty() { "ParkHub" } else { org_name };
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
        org = org,
        user_name = user_name,
        booking_id = booking_id,
        floor_name = floor_name,
        slot_number = slot_number,
        start_time = start_time,
        end_time = end_time,
    )
}

/// Build a password-reset email body.
pub fn build_password_reset_email(reset_url: &str, org_name: &str) -> String {
    let org = if org_name.is_empty() { "ParkHub" } else { org_name };
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
        org = org,
        reset_url = reset_url,
    )
}
