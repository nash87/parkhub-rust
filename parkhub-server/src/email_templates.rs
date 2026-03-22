//! HTML email templates for ParkHub transactional emails.
//!
//! All templates use inline CSS (no external stylesheets) for maximum
//! email client compatibility. Template variables use `{{key}}` syntax
//! and are replaced via simple string substitution.

use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════════
// Template Engine
// ═══════════════════════════════════════════════════════════════════════════════

/// Render a template string by replacing `{{key}}` placeholders with values.
pub fn render_template(template: &str, vars: &HashMap<&str, &str>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{key}}}}}"), value);
    }
    result
}

// ═══════════════════════════════════════════════════════════════════════════════
// Shared Layout
// ═══════════════════════════════════════════════════════════════════════════════

const HEADER: &str = r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"></head>
<body style="margin:0;padding:0;background-color:#f4f5f7;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;">
<table width="100%" cellpadding="0" cellspacing="0" style="background-color:#f4f5f7;padding:24px 0;">
<tr><td align="center">
<table width="600" cellpadding="0" cellspacing="0" style="background-color:#ffffff;border-radius:12px;overflow:hidden;box-shadow:0 2px 8px rgba(0,0,0,0.06);">
<!-- Header -->
<tr><td style="background:linear-gradient(135deg,#6366f1,#4f46e5);padding:28px 32px;text-align:center;">
<div style="font-size:24px;font-weight:700;color:#ffffff;letter-spacing:-0.02em;">{{company_name}}</div>
</td></tr>
<!-- Body -->
<tr><td style="padding:32px;">"#;

const FOOTER: &str = r#"</td></tr>
<!-- Footer -->
<tr><td style="padding:20px 32px;background-color:#f9fafb;border-top:1px solid #e5e7eb;text-align:center;">
<p style="margin:0;font-size:12px;color:#9ca3af;">{{company_name}} — Self-hosted parking management</p>
<p style="margin:4px 0 0;font-size:11px;color:#d1d5db;">This is an automated message. Please do not reply.</p>
</td></tr>
</table>
</td></tr>
</table>
</body>
</html>"#;

fn wrap(body: &str) -> String {
    format!("{HEADER}{body}{FOOTER}")
}

// ═══════════════════════════════════════════════════════════════════════════════
// Templates
// ═══════════════════════════════════════════════════════════════════════════════

/// Booking confirmation email.
pub fn booking_confirmation(vars: &HashMap<&str, &str>) -> String {
    let body = r#"
<h2 style="margin:0 0 16px;font-size:20px;color:#111827;">Booking Confirmed!</h2>
<p style="margin:0 0 20px;color:#4b5563;line-height:1.6;">Hi {{name}}, your parking spot has been reserved.</p>
<table width="100%" cellpadding="0" cellspacing="0" style="background-color:#f9fafb;border-radius:8px;padding:16px;margin-bottom:20px;">
<tr><td style="padding:8px 16px;"><strong style="color:#374151;">Parking Lot</strong></td><td style="padding:8px 16px;color:#6b7280;">{{lot_name}}</td></tr>
<tr><td style="padding:8px 16px;"><strong style="color:#374151;">Slot</strong></td><td style="padding:8px 16px;color:#6b7280;">{{slot}}</td></tr>
<tr><td style="padding:8px 16px;"><strong style="color:#374151;">Date & Time</strong></td><td style="padding:8px 16px;color:#6b7280;">{{start_time}} — {{end_time}}</td></tr>
<tr><td style="padding:8px 16px;"><strong style="color:#374151;">Vehicle</strong></td><td style="padding:8px 16px;color:#6b7280;">{{vehicle}}</td></tr>
</table>
<a href="{{qr_link}}" style="display:inline-block;background-color:#6366f1;color:#ffffff;padding:12px 24px;border-radius:8px;text-decoration:none;font-weight:600;font-size:14px;">View QR Pass</a>
"#;
    render_template(&wrap(body), vars)
}

/// Booking reminder (1 hour before).
pub fn booking_reminder(vars: &HashMap<&str, &str>) -> String {
    let body = r#"
<h2 style="margin:0 0 16px;font-size:20px;color:#111827;">Reminder: Booking Starting Soon</h2>
<p style="margin:0 0 20px;color:#4b5563;line-height:1.6;">Hi {{name}}, your parking booking starts in about 1 hour.</p>
<table width="100%" cellpadding="0" cellspacing="0" style="background-color:#fef3c7;border-radius:8px;padding:16px;margin-bottom:20px;border:1px solid #fbbf24;">
<tr><td style="padding:8px 16px;"><strong style="color:#92400e;">Lot</strong></td><td style="padding:8px 16px;color:#92400e;">{{lot_name}}</td></tr>
<tr><td style="padding:8px 16px;"><strong style="color:#92400e;">Slot</strong></td><td style="padding:8px 16px;color:#92400e;">{{slot}}</td></tr>
<tr><td style="padding:8px 16px;"><strong style="color:#92400e;">Time</strong></td><td style="padding:8px 16px;color:#92400e;">{{start_time}} — {{end_time}}</td></tr>
</table>
<p style="margin:0;color:#6b7280;font-size:13px;">Don't forget to check in when you arrive!</p>
"#;
    render_template(&wrap(body), vars)
}

/// Booking cancellation confirmation.
pub fn booking_cancelled(vars: &HashMap<&str, &str>) -> String {
    let body = r#"
<h2 style="margin:0 0 16px;font-size:20px;color:#111827;">Booking Cancelled</h2>
<p style="margin:0 0 20px;color:#4b5563;line-height:1.6;">Hi {{name}}, your booking has been cancelled.</p>
<table width="100%" cellpadding="0" cellspacing="0" style="background-color:#fef2f2;border-radius:8px;padding:16px;margin-bottom:20px;border:1px solid #fca5a5;">
<tr><td style="padding:8px 16px;"><strong style="color:#991b1b;">Lot</strong></td><td style="padding:8px 16px;color:#991b1b;">{{lot_name}}</td></tr>
<tr><td style="padding:8px 16px;"><strong style="color:#991b1b;">Slot</strong></td><td style="padding:8px 16px;color:#991b1b;">{{slot}}</td></tr>
<tr><td style="padding:8px 16px;"><strong style="color:#991b1b;">Was scheduled</strong></td><td style="padding:8px 16px;color:#991b1b;">{{start_time}} — {{end_time}}</td></tr>
</table>
<p style="margin:0;color:#6b7280;font-size:13px;">If this was a mistake, you can book a new spot anytime.</p>
"#;
    render_template(&wrap(body), vars)
}

/// Password reset email with expiring link.
pub fn password_reset(vars: &HashMap<&str, &str>) -> String {
    let body = r#"
<h2 style="margin:0 0 16px;font-size:20px;color:#111827;">Reset Your Password</h2>
<p style="margin:0 0 20px;color:#4b5563;line-height:1.6;">Hi {{name}}, we received a request to reset your password. Click the button below to set a new one.</p>
<div style="text-align:center;margin:24px 0;">
<a href="{{reset_link}}" style="display:inline-block;background-color:#6366f1;color:#ffffff;padding:14px 32px;border-radius:8px;text-decoration:none;font-weight:600;font-size:15px;">Reset Password</a>
</div>
<p style="margin:0 0 8px;color:#9ca3af;font-size:12px;">This link expires in 1 hour.</p>
<p style="margin:0;color:#9ca3af;font-size:12px;">If you didn't request this, you can safely ignore this email.</p>
"#;
    render_template(&wrap(body), vars)
}

/// Welcome email for new users.
pub fn welcome(vars: &HashMap<&str, &str>) -> String {
    let body = r#"
<h2 style="margin:0 0 16px;font-size:20px;color:#111827;">Welcome to {{company_name}}!</h2>
<p style="margin:0 0 20px;color:#4b5563;line-height:1.6;">Hi {{name}}, your account has been created. Here's how to get started:</p>
<div style="margin-bottom:20px;">
<div style="display:flex;align-items:flex-start;margin-bottom:12px;">
<div style="min-width:28px;height:28px;border-radius:50%;background-color:#6366f1;color:#fff;text-align:center;line-height:28px;font-weight:700;font-size:13px;margin-right:12px;">1</div>
<div><strong style="color:#374151;">Log in</strong><br><span style="color:#6b7280;font-size:13px;">Sign in with your email and password.</span></div>
</div>
<div style="display:flex;align-items:flex-start;margin-bottom:12px;">
<div style="min-width:28px;height:28px;border-radius:50%;background-color:#6366f1;color:#fff;text-align:center;line-height:28px;font-weight:700;font-size:13px;margin-right:12px;">2</div>
<div><strong style="color:#374151;">Add your vehicle</strong><br><span style="color:#6b7280;font-size:13px;">Register your car for easier bookings.</span></div>
</div>
<div style="display:flex;align-items:flex-start;">
<div style="min-width:28px;height:28px;border-radius:50%;background-color:#6366f1;color:#fff;text-align:center;line-height:28px;font-weight:700;font-size:13px;margin-right:12px;">3</div>
<div><strong style="color:#374151;">Book a spot</strong><br><span style="color:#6b7280;font-size:13px;">Reserve your parking in seconds.</span></div>
</div>
</div>
<a href="{{login_link}}" style="display:inline-block;background-color:#6366f1;color:#ffffff;padding:12px 24px;border-radius:8px;text-decoration:none;font-weight:600;font-size:14px;">Get Started</a>
"#;
    render_template(&wrap(body), vars)
}

/// Weekly admin summary email.
pub fn weekly_summary(vars: &HashMap<&str, &str>) -> String {
    let body = r#"
<h2 style="margin:0 0 16px;font-size:20px;color:#111827;">Weekly Summary</h2>
<p style="margin:0 0 20px;color:#4b5563;line-height:1.6;">Here's your parking overview for the past week.</p>
<table width="100%" cellpadding="0" cellspacing="0" style="margin-bottom:20px;">
<tr>
<td style="padding:16px;background-color:#eff6ff;border-radius:8px;text-align:center;width:33%;">
<div style="font-size:24px;font-weight:700;color:#1d4ed8;">{{bookings_count}}</div>
<div style="font-size:12px;color:#3b82f6;margin-top:4px;">Bookings</div>
</td>
<td style="width:8px;"></td>
<td style="padding:16px;background-color:#f0fdf4;border-radius:8px;text-align:center;width:33%;">
<div style="font-size:24px;font-weight:700;color:#15803d;">{{revenue}}</div>
<div style="font-size:12px;color:#22c55e;margin-top:4px;">Revenue</div>
</td>
<td style="width:8px;"></td>
<td style="padding:16px;background-color:#faf5ff;border-radius:8px;text-align:center;width:33%;">
<div style="font-size:24px;font-weight:700;color:#7e22ce;">{{active_users}}</div>
<div style="font-size:12px;color:#a855f7;margin-top:4px;">Active Users</div>
</td>
</tr>
</table>
<h3 style="margin:0 0 12px;font-size:16px;color:#374151;">Top Lots</h3>
<p style="margin:0 0 16px;color:#6b7280;font-size:14px;line-height:1.5;">{{top_lots}}</p>
<a href="{{dashboard_link}}" style="display:inline-block;background-color:#6366f1;color:#ffffff;padding:12px 24px;border-radius:8px;text-decoration:none;font-weight:600;font-size:14px;">View Dashboard</a>
"#;
    render_template(&wrap(body), vars)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vars() -> HashMap<&'static str, &'static str> {
        let mut vars = HashMap::new();
        vars.insert("company_name", "ParkHub");
        vars.insert("name", "Alice");
        vars.insert("lot_name", "Main Garage");
        vars.insert("slot", "A-12");
        vars.insert("start_time", "2026-03-22 09:00");
        vars.insert("end_time", "2026-03-22 17:00");
        vars.insert("vehicle", "ABC-1234");
        vars.insert("qr_link", "https://parkhub.test/pass/abc");
        vars.insert("reset_link", "https://parkhub.test/reset/xyz");
        vars.insert("login_link", "https://parkhub.test/login");
        vars.insert("dashboard_link", "https://parkhub.test/admin");
        vars.insert("bookings_count", "42");
        vars.insert("revenue", "1,250.00");
        vars.insert("active_users", "18");
        vars.insert("top_lots", "1. Main Garage (85%)\n2. Annex (60%)");
        vars
    }

    #[test]
    fn render_template_replaces_vars() {
        let mut vars = HashMap::new();
        vars.insert("name", "Bob");
        vars.insert("count", "5");
        let result = render_template("Hello {{name}}, you have {{count}} items.", &vars);
        assert_eq!(result, "Hello Bob, you have 5 items.");
    }

    #[test]
    fn render_template_preserves_unknown_vars() {
        let vars = HashMap::new();
        let result = render_template("Hello {{unknown}}!", &vars);
        assert_eq!(result, "Hello {{unknown}}!");
    }

    #[test]
    fn booking_confirmation_contains_slot() {
        let vars = sample_vars();
        let html = booking_confirmation(&vars);
        assert!(html.contains("A-12"));
        assert!(html.contains("Main Garage"));
        assert!(html.contains("Alice"));
        assert!(html.contains("Booking Confirmed"));
        assert!(html.contains("pass/abc"));
    }

    #[test]
    fn booking_reminder_contains_warning_style() {
        let vars = sample_vars();
        let html = booking_reminder(&vars);
        assert!(html.contains("Reminder"));
        assert!(html.contains("fef3c7")); // amber background
        assert!(html.contains("Main Garage"));
    }

    #[test]
    fn booking_cancelled_contains_red_style() {
        let vars = sample_vars();
        let html = booking_cancelled(&vars);
        assert!(html.contains("Cancelled"));
        assert!(html.contains("fef2f2")); // red background
        assert!(html.contains("A-12"));
    }

    #[test]
    fn password_reset_contains_link_and_expiry() {
        let vars = sample_vars();
        let html = password_reset(&vars);
        assert!(html.contains("Reset Your Password"));
        assert!(html.contains("reset/xyz"));
        assert!(html.contains("expires in 1 hour"));
    }

    #[test]
    fn welcome_contains_getting_started() {
        let vars = sample_vars();
        let html = welcome(&vars);
        assert!(html.contains("Welcome to ParkHub"));
        assert!(html.contains("Log in"));
        assert!(html.contains("Add your vehicle"));
        assert!(html.contains("Book a spot"));
    }

    #[test]
    fn weekly_summary_contains_stats() {
        let vars = sample_vars();
        let html = weekly_summary(&vars);
        assert!(html.contains("Weekly Summary"));
        assert!(html.contains("42"));
        assert!(html.contains("1,250.00"));
        assert!(html.contains("18"));
        assert!(html.contains("Top Lots"));
    }

    #[test]
    fn all_templates_have_header_and_footer() {
        let vars = sample_vars();
        for html in [
            booking_confirmation(&vars),
            booking_reminder(&vars),
            booking_cancelled(&vars),
            password_reset(&vars),
            welcome(&vars),
            weekly_summary(&vars),
        ] {
            assert!(html.contains("<!DOCTYPE html>"), "Missing DOCTYPE");
            assert!(html.contains("ParkHub"), "Missing company name in header");
            assert!(html.contains("automated message"), "Missing footer");
        }
    }
}
