//! PDF invoice generation for bookings.
//!
//! Generates professional PDF receipts using the `printpdf` crate.
//!
//! Endpoints:
//! - `GET /api/v1/bookings/:id/invoice/pdf` — download PDF receipt for a booking

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use printpdf::{
    BuiltinFont, Color, Line, LinePoint, Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, Point, Pt,
    Rgb, TextItem,
};

use parkhub_common::{ApiResponse, UserRole};

use super::{AuthUser, SharedState};

/// German standard VAT rate (19% — Umsatzsteuergesetz § 12 Abs. 1)
const VAT_RATE: f64 = 0.19;

/// Invoice number format: INV-{YEAR}-{sequential_hex}
pub fn format_invoice_number(booking_id: &str, year: i32) -> String {
    let hex_part: String = booking_id
        .replace('-', "")
        .chars()
        .take(8)
        .collect::<String>()
        .to_uppercase();
    format!("INV-{year}-{hex_part}")
}

/// `GET /api/v1/bookings/:id/invoice/pdf` — generate a PDF receipt for a booking.
#[utoipa::path(
    get,
    path = "/api/v1/bookings/{id}/invoice/pdf",
    tag = "Invoices",
    summary = "Download PDF invoice for a booking",
    params(("id" = String, Path, description = "Booking ID")),
    responses(
        (status = 200, description = "PDF invoice", content_type = "application/pdf"),
        (status = 404, description = "Booking not found"),
        (status = 403, description = "Access denied"),
    )
)]
pub async fn get_booking_invoice_pdf(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Response {
    let state_guard = state.read().await;

    // Fetch booking
    let booking = match state_guard.db.get_booking(&id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error("NOT_FOUND", "Booking not found")),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Database error fetching booking for PDF invoice: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "SERVER_ERROR",
                    "Internal server error",
                )),
            )
                .into_response();
        }
    };

    // Ownership check — only the booking owner (or admin) may fetch the invoice
    let Ok(Some(caller)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<()>::error("FORBIDDEN", "Access denied")),
        )
            .into_response();
    };

    let is_admin = caller.role == UserRole::Admin || caller.role == UserRole::SuperAdmin;
    if booking.user_id != auth_user.user_id && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<()>::error("FORBIDDEN", "Access denied")),
        )
            .into_response();
    }

    // Fetch user details
    let booking_user = match state_guard.db.get_user(&booking.user_id.to_string()).await {
        Ok(Some(u)) => u,
        _ => caller.clone(),
    };

    // Fetch lot name
    let lot_name = match state_guard
        .db
        .get_parking_lot(&booking.lot_id.to_string())
        .await
    {
        Ok(Some(lot)) => lot.name,
        _ => "Parking Lot".to_string(),
    };

    // Company info
    let org_name = state_guard.config.organization_name.clone();
    let company = if org_name.is_empty() {
        "ParkHub".to_string()
    } else {
        org_name
    };

    // Invoice metadata
    let year = booking
        .created_at
        .format("%Y")
        .to_string()
        .parse::<i32>()
        .unwrap_or(2026);
    let invoice_number = format_invoice_number(&booking.id.to_string(), year);
    let invoice_date = booking.created_at.format("%d.%m.%Y").to_string();
    let start_str = booking.start_time.format("%d.%m.%Y %H:%M").to_string();
    let end_str = booking.end_time.format("%d.%m.%Y %H:%M").to_string();

    // Duration
    let duration_minutes = (booking.end_time - booking.start_time).num_minutes();
    let duration_hours = duration_minutes / 60;
    let duration_mins_part = duration_minutes % 60;

    // Pricing
    let net_price = booking.pricing.base_price;
    let vat_amount = net_price * VAT_RATE;
    let gross_total = net_price + vat_amount;
    let currency = &booking.pricing.currency;

    drop(state_guard);

    // Generate PDF
    let pdf_bytes = match generate_pdf(
        &company,
        &invoice_number,
        &invoice_date,
        &booking_user.name,
        &booking_user.email,
        &lot_name,
        booking.slot_number,
        &booking.floor_name,
        &booking.vehicle.license_plate,
        &start_str,
        &end_str,
        duration_hours,
        duration_mins_part,
        &format!("{:?}", booking.status),
        net_price,
        vat_amount,
        gross_total,
        currency,
    ) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!("PDF generation failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "PDF_ERROR",
                    "Failed to generate PDF",
                )),
            )
                .into_response();
        }
    };

    let filename = format!("{invoice_number}.pdf");

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        pdf_bytes,
    )
        .into_response()
}

/// Generate a PDF invoice document.
#[allow(clippy::too_many_arguments)]
fn generate_pdf(
    company: &str,
    invoice_number: &str,
    invoice_date: &str,
    user_name: &str,
    user_email: &str,
    lot_name: &str,
    slot_number: i32,
    floor_name: &str,
    license_plate: &str,
    start_str: &str,
    end_str: &str,
    duration_hours: i64,
    duration_mins_part: i64,
    status: &str,
    net_price: f64,
    vat_amount: f64,
    gross_total: f64,
    currency: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut ops = Vec::new();

    // Helper: add text at position with builtin font
    fn text_at(ops: &mut Vec<Op>, text: &str, size: f32, x: Mm, y: Mm, font: BuiltinFont) {
        ops.push(Op::StartTextSection);
        ops.push(Op::SetFontSizeBuiltinFont {
            size: Pt(size),
            font,
        });
        ops.push(Op::SetTextCursor {
            pos: Point::new(x, y),
        });
        ops.push(Op::WriteTextBuiltinFont {
            items: vec![TextItem::Text(text.to_string())],
            font,
        });
        ops.push(Op::EndTextSection);
    }

    // Helper: draw horizontal line
    fn hline(ops: &mut Vec<Op>, x1: Mm, x2: Mm, y: Mm, r: f32, g: f32, b: f32, thickness: f32) {
        ops.push(Op::SetOutlineColor {
            col: Color::Rgb(Rgb::new(r, g, b, None)),
        });
        ops.push(Op::SetOutlineThickness { pt: Pt(thickness) });
        ops.push(Op::DrawLine {
            line: Line {
                points: vec![
                    LinePoint {
                        p: Point::new(x1, y),
                        bezier: false,
                    },
                    LinePoint {
                        p: Point::new(x2, y),
                        bezier: false,
                    },
                ],
                is_closed: false,
            },
        });
    }

    let bold = BuiltinFont::HelveticaBold;
    let regular = BuiltinFont::Helvetica;

    // Y positions (top of A4 = 297mm, we work top-down)
    let mut y = Mm(270.0);

    // ── Header ──
    text_at(&mut ops, company, 22.0, Mm(20.0), y, bold);
    text_at(
        &mut ops,
        "Parking Management",
        10.0,
        Mm(20.0),
        y - Mm(8.0),
        regular,
    );
    text_at(&mut ops, "INVOICE", 18.0, Mm(140.0), y, bold);
    text_at(&mut ops, invoice_number, 10.0, Mm(140.0), y - Mm(7.0), bold);
    text_at(
        &mut ops,
        &format!("Date: {invoice_date}"),
        9.0,
        Mm(140.0),
        y - Mm(14.0),
        regular,
    );

    y -= Mm(35.0);

    // ── Accent line ──
    hline(&mut ops, Mm(20.0), Mm(190.0), y, 0.1, 0.45, 0.91, 1.5);
    y -= Mm(12.0);

    // ── Bill To ──
    text_at(&mut ops, "BILL TO", 9.0, Mm(20.0), y, bold);
    y -= Mm(6.0);
    text_at(&mut ops, user_name, 11.0, Mm(20.0), y, bold);
    y -= Mm(5.0);
    text_at(&mut ops, user_email, 9.0, Mm(20.0), y, regular);
    y -= Mm(15.0);

    // ── Booking Details ──
    text_at(&mut ops, "BOOKING DETAILS", 9.0, Mm(20.0), y, bold);
    y -= Mm(8.0);

    let details: Vec<(&str, String)> = vec![
        ("Booking ID", invoice_number.to_string()),
        ("Parking Lot", lot_name.to_string()),
        ("Slot", format!("No. {slot_number} - {floor_name}")),
        ("Vehicle", license_plate.to_string()),
        ("Start", start_str.to_string()),
        ("End", end_str.to_string()),
        (
            "Duration",
            format!("{duration_hours}h {duration_mins_part}min"),
        ),
        ("Status", status.to_string()),
    ];

    for (label, value) in &details {
        text_at(&mut ops, label, 9.0, Mm(20.0), y, regular);
        text_at(&mut ops, value, 9.0, Mm(80.0), y, bold);
        y -= Mm(6.0);
    }

    y -= Mm(10.0);

    // ── Separator ──
    hline(&mut ops, Mm(20.0), Mm(190.0), y, 0.8, 0.8, 0.8, 0.5);
    y -= Mm(10.0);

    // ── Pricing ──
    text_at(&mut ops, "PRICING", 9.0, Mm(20.0), y, bold);
    y -= Mm(8.0);
    text_at(&mut ops, "Description", 9.0, Mm(20.0), y, bold);
    text_at(
        &mut ops,
        &format!("Amount ({currency})"),
        9.0,
        Mm(150.0),
        y,
        bold,
    );
    y -= Mm(6.0);
    text_at(&mut ops, "Parking Fee (Net)", 9.0, Mm(20.0), y, regular);
    text_at(
        &mut ops,
        &format!("{net_price:.2}"),
        9.0,
        Mm(155.0),
        y,
        regular,
    );
    y -= Mm(6.0);
    text_at(&mut ops, "VAT 19%", 9.0, Mm(20.0), y, regular);
    text_at(
        &mut ops,
        &format!("{vat_amount:.2}"),
        9.0,
        Mm(155.0),
        y,
        regular,
    );
    y -= Mm(8.0);

    // ── Total line ──
    hline(&mut ops, Mm(130.0), Mm(190.0), y, 0.1, 0.45, 0.91, 1.0);
    y -= Mm(7.0);
    text_at(&mut ops, "TOTAL (Gross)", 11.0, Mm(20.0), y, bold);
    text_at(
        &mut ops,
        &format!("{gross_total:.2} {currency}"),
        11.0,
        Mm(145.0),
        y,
        bold,
    );

    // ── Footer ──
    let footer_y = Mm(25.0);
    text_at(
        &mut ops,
        &format!("{company} - Parking Management System"),
        8.0,
        Mm(50.0),
        footer_y,
        regular,
    );
    text_at(
        &mut ops,
        "This invoice was automatically generated and is valid without signature.",
        7.0,
        Mm(35.0),
        footer_y - Mm(5.0),
        regular,
    );

    // Build document
    let page = PdfPage::new(Mm(210.0), Mm(297.0), ops);
    let mut doc = PdfDocument::new(&format!("Invoice {invoice_number}"));
    doc.pages.push(page);

    let mut warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    Ok(bytes)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoice_number_format() {
        let invoice = format_invoice_number("550e8400-e29b-41d4-a716-446655440000", 2026);
        assert!(invoice.starts_with("INV-2026-"));
        assert_eq!(invoice, "INV-2026-550E8400");
    }

    #[test]
    fn test_invoice_number_short_id() {
        let invoice = format_invoice_number("abcd", 2025);
        assert_eq!(invoice, "INV-2025-ABCD");
    }

    #[test]
    fn test_invoice_number_strips_hyphens() {
        let invoice = format_invoice_number("a-b-c-d-e-f-g-h-i", 2026);
        // After removing hyphens: "abcdefghi", take first 8 = "ABCDEFGH"
        assert_eq!(invoice, "INV-2026-ABCDEFGH");
    }

    #[test]
    fn test_pdf_generation_produces_valid_pdf() {
        let bytes = generate_pdf(
            "Test Company",
            "INV-2026-TEST1234",
            "22.03.2026",
            "Max Mustermann",
            "max@example.com",
            "Parkhaus A",
            42,
            "Ebene 1",
            "AB-CD-1234",
            "22.03.2026 08:00",
            "22.03.2026 18:00",
            10,
            0,
            "Confirmed",
            15.0,
            2.85,
            17.85,
            "EUR",
        )
        .expect("PDF generation should succeed");

        // PDF should start with %PDF
        assert!(bytes.len() > 100, "PDF should have reasonable size");
        assert!(
            bytes.starts_with(b"%PDF"),
            "PDF should start with PDF header"
        );
    }

    #[test]
    fn test_pdf_generation_zero_price() {
        let bytes = generate_pdf(
            "ParkHub",
            "INV-2026-00000000",
            "01.01.2026",
            "Test User",
            "test@test.com",
            "Free Lot",
            1,
            "Ground",
            "X-Y-0000",
            "01.01.2026 00:00",
            "01.01.2026 01:00",
            1,
            0,
            "Active",
            0.0,
            0.0,
            0.0,
            "EUR",
        )
        .expect("PDF generation with zero price should succeed");

        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_pdf_generation_long_names() {
        let long_name = "A".repeat(100);
        let bytes = generate_pdf(
            &long_name,
            "INV-2026-LONGTEST",
            "15.06.2026",
            &long_name,
            "verylongemail@verylong.domain.com",
            &long_name,
            999,
            &long_name,
            "AAAAAA-BB-9999",
            "15.06.2026 06:00",
            "16.06.2026 22:00",
            40,
            0,
            "Completed",
            1000.0,
            190.0,
            1190.0,
            "EUR",
        )
        .expect("PDF generation with long names should succeed");

        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_vat_rate_is_nineteen_percent() {
        assert!((VAT_RATE - 0.19).abs() < f64::EPSILON);
    }
}
