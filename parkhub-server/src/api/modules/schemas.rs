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

/// `mod-recommendations` — weighted recommendation engine contract.
///
/// The defaults preserve the legacy slot-ranking behavior. Operators can
/// tune the rule weights through the generic module config editor without
/// replacing the deterministic default scorer.
pub(super) const MOD_RECOMMENDATIONS_SCHEMA: &str = r#"{
  "type": "object",
  "title": "Recommendations settings",
  "description": "Weighted recommendation engine defaults. The weighted_v1 algorithm is deterministic and emits human-readable reasons for every score.",
  "properties": {
    "algorithm": {
      "type": "string",
      "enum": ["weighted_v1", "fop_pipeline_v1"],
      "description": "Versioned scoring strategy. weighted_v1 is the rollback default; fop_pipeline_v1 calls the configured fop-pipeline HTTP adapter and falls back to weighted_v1 on any error."
    },
    "pipeline_endpoint": {
      "type": "string",
      "description": "Optional fop-pipeline base URL, for example http://fop-pipeline.fop-agents.svc:9310. Empty keeps fop_pipeline_v1 in configured-but-fallback mode."
    },
    "pipeline_name": {
      "type": "string",
      "minLength": 1,
      "description": "fop-pipeline pipeline name used by POST /pipeline/{name}/run."
    },
    "pipeline_timeout_ms": {
      "type": "integer",
      "minimum": 100,
      "maximum": 5000,
      "description": "Adapter timeout in milliseconds before falling back to weighted_v1."
    },
    "pipeline_fallback_enabled": {
      "type": "boolean",
      "const": true,
      "description": "Fail-closed guardrail. weighted_v1 fallback stays mandatory until fop_pipeline_v1 is production-certified."
    },
    "allocation_strategy": {
      "type": "string",
      "enum": ["weighted_v1", "exact_cover_v1"],
      "description": "Batch-allocation strategy. weighted_v1 keeps quick booking as the default; exact_cover_v1 is admin-only operational scheduling support for recurring/batch constraints."
    },
    "exact_cover_max_options": {
      "type": "integer",
      "minimum": 1,
      "maximum": 256,
      "description": "Maximum candidate options accepted by exact_cover_v1 before failing closed with fallback_input_limited."
    },
    "exact_cover_max_search_nodes": {
      "type": "integer",
      "minimum": 1,
      "maximum": 10000,
      "description": "Maximum Algorithm X search nodes before exact_cover_v1 fails closed with fallback_search_limited."
    },
    "weight_frequency": {
      "type": "number",
      "minimum": 0,
      "maximum": 100,
      "description": "Maximum points for repeatedly choosing the same slot."
    },
    "weight_preferred_lot": {
      "type": "number",
      "minimum": 0,
      "maximum": 100,
      "description": "Maximum points for a frequently used lot when the exact slot has no history."
    },
    "weight_availability": {
      "type": "number",
      "minimum": 0,
      "maximum": 100,
      "description": "Points awarded to currently available slots."
    },
    "weight_price": {
      "type": "number",
      "minimum": 0,
      "maximum": 100,
      "description": "Maximum points for lower-priced lots."
    },
    "weight_distance": {
      "type": "number",
      "minimum": 0,
      "maximum": 100,
      "description": "Maximum points for slots near the entrance."
    },
    "weight_accessibility_bonus": {
      "type": "number",
      "minimum": 0,
      "maximum": 25,
      "description": "Optional extra points for facility-designated accessible slots only. This must not use inferred disability, health, or other sensitive user attributes; keep at 0 until tenant DPIA/privacy review and user-facing notice approve it."
    },
    "weight_feature_bonus": {
      "type": "number",
      "minimum": 0,
      "maximum": 25,
      "description": "Optional tiebreaker points for slots with feature metadata."
    },
    "max_results": {
      "type": "integer",
      "minimum": 1,
      "maximum": 25,
      "description": "Maximum recommendations returned to the client."
    },
    "explain": {
      "type": "boolean",
      "const": true,
      "description": "Fail-closed guardrail. Reason strings and badges must stay enabled before legal/privacy review approves disabling them."
    },
    "profile_safe_mode": {
      "type": "boolean",
      "const": true,
      "description": "Fail-closed privacy guardrail. Sensitive personal attributes are blocked from scoring inputs; this must stay enabled before legal/privacy review approves disabling it."
    },
    "allocation_transparency_mode": {
      "type": "string",
      "enum": ["algorithmic", "fifo_only"],
      "description": "EU AI Act Art. 50 transparency mode. algorithmic (default): scored allocation active, transparency notice included in every response. fifo_only: algorithmic endpoints refuse with 409 ALGORITHMIC_DISABLED; use the waitlist for rule-based slot assignment to remain outside AI Act scope."
    }
  },
  "required": [
    "algorithm",
    "pipeline_endpoint",
    "pipeline_name",
    "pipeline_timeout_ms",
    "pipeline_fallback_enabled",
    "allocation_strategy",
    "exact_cover_max_options",
    "exact_cover_max_search_nodes",
    "weight_frequency",
    "weight_preferred_lot",
    "weight_availability",
    "weight_price",
    "weight_distance",
    "weight_accessibility_bonus",
    "weight_feature_bonus",
    "max_results",
    "explain",
    "profile_safe_mode",
    "allocation_transparency_mode"
  ],
  "additionalProperties": false
}"#;
