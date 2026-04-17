//! Module config schemas (T-1720 v3)
//!
//! One const string per module that exposes a config editor. We keep the
//! schemas as literal JSON so (a) they are trivially reviewable in diffs,
//! (b) they cost zero runtime parsing effort beyond the one-time registry
//! materialisation, and (c) the set of declared modules is obvious from
//! `rg MOD_.*_SCHEMA`.
//!
//! Every schema is draft 2020-12, `additionalProperties: false`, and only
//! enumerates fields that the Settings store actually consumes today. We
//! deliberately do *not* invent aspirational fields — better to extend
//! the schema when a setting is added than to ship a UI for a
//! non-existent key.

/// `mod-themes` — tenant default theme + per-user override flag.
pub(super) const MOD_THEMES_SCHEMA: &str = r#"{
  "type": "object",
  "title": "Themes settings",
  "description": "Default theme for the tenant plus a flag that gates per-user theme override.",
  "properties": {
    "default_theme": {
      "type": "string",
      "enum": ["light", "dark", "classic"],
      "description": "Theme used when a user has not picked one."
    },
    "allow_user_override": {
      "type": "boolean",
      "description": "When true, individual users can pick their own theme."
    }
  },
  "required": ["default_theme", "allow_user_override"],
  "additionalProperties": false
}"#;

/// `mod-announcements` — admin banner policy.
pub(super) const MOD_ANNOUNCEMENTS_SCHEMA: &str = r#"{
  "type": "object",
  "title": "Announcements settings",
  "description": "Controls the admin-published banner system.",
  "properties": {
    "max_announcements": {
      "type": "integer",
      "minimum": 1,
      "maximum": 50,
      "description": "Maximum number of simultaneously active announcements."
    },
    "default_ttl_days": {
      "type": "integer",
      "minimum": 1,
      "maximum": 365,
      "description": "Default days until an announcement auto-expires."
    },
    "show_on_login": {
      "type": "boolean",
      "description": "Display the active announcement list on the login page."
    }
  },
  "required": ["max_announcements", "default_ttl_days", "show_on_login"],
  "additionalProperties": false
}"#;

/// `mod-notifications` — delivery channels and quiet hours.
///
/// `quiet_hours_start` / `quiet_hours_end` use an `HH:MM` 24-hour pattern
/// rather than the full RFC 3339 `time` format because we only ever
/// consume minutes-of-day; seconds + timezone would be silently
/// discarded otherwise.
pub(super) const MOD_NOTIFICATIONS_SCHEMA: &str = r#"{
  "type": "object",
  "title": "Notifications settings",
  "description": "Master switches per channel plus nightly quiet-hours window.",
  "properties": {
    "push_enabled": {
      "type": "boolean",
      "description": "Send Web Push notifications to subscribed clients."
    },
    "email_enabled": {
      "type": "boolean",
      "description": "Send transactional email notifications."
    },
    "quiet_hours_start": {
      "type": "string",
      "pattern": "^([01][0-9]|2[0-3]):[0-5][0-9]$",
      "description": "Start of the nightly quiet window (HH:MM, 24h)."
    },
    "quiet_hours_end": {
      "type": "string",
      "pattern": "^([01][0-9]|2[0-3]):[0-5][0-9]$",
      "description": "End of the nightly quiet window (HH:MM, 24h)."
    }
  },
  "required": ["push_enabled", "email_enabled", "quiet_hours_start", "quiet_hours_end"],
  "additionalProperties": false
}"#;

/// `mod-email-templates` — outbound envelope identity.
pub(super) const MOD_EMAIL_TEMPLATES_SCHEMA: &str = r#"{
  "type": "object",
  "title": "Email template settings",
  "description": "Envelope identity applied to every transactional email.",
  "properties": {
    "from_address": {
      "type": "string",
      "format": "email",
      "description": "`From:` address on outbound email."
    },
    "from_name": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128,
      "description": "Human-readable display name on outbound email."
    },
    "reply_to": {
      "type": "string",
      "format": "email",
      "description": "`Reply-To:` address, typically a monitored inbox."
    }
  },
  "required": ["from_address", "from_name", "reply_to"],
  "additionalProperties": false
}"#;

/// `mod-widgets` — embeddable dashboard widget cap.
pub(super) const MOD_WIDGETS_SCHEMA: &str = r#"{
  "type": "object",
  "title": "Widgets settings",
  "description": "Limits for the embeddable widgets subsystem.",
  "properties": {
    "max_widgets_per_dashboard": {
      "type": "integer",
      "minimum": 1,
      "maximum": 20,
      "description": "Maximum number of widgets that can be pinned to one dashboard."
    }
  },
  "required": ["max_widgets_per_dashboard"],
  "additionalProperties": false
}"#;
