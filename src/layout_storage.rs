//! Layout Storage Module
//!
//! Handles saving and loading parking lot layouts to/from disk.

#![allow(dead_code)]

use anyhow::{Context, Result};
use chrono::Local;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// Element type matching the Slint enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ElementType {
    ParkingSlot,
    Wall,
    Pillar,
    Entry,
    Exit,
    Handicap,
    Electric,
    Motorcycle,
    Lane,
    Arrow,
}

impl ElementType {
    pub fn to_index(&self) -> i32 {
        match self {
            ElementType::ParkingSlot => 0,
            ElementType::Wall => 1,
            ElementType::Pillar => 2,
            ElementType::Entry => 3,
            ElementType::Exit => 4,
            ElementType::Handicap => 5,
            ElementType::Electric => 6,
            ElementType::Motorcycle => 7,
            ElementType::Lane => 8,
            ElementType::Arrow => 9,
        }
    }

    pub fn from_index(index: i32) -> Self {
        match index {
            0 => ElementType::ParkingSlot,
            1 => ElementType::Wall,
            2 => ElementType::Pillar,
            3 => ElementType::Entry,
            4 => ElementType::Exit,
            5 => ElementType::Handicap,
            6 => ElementType::Electric,
            7 => ElementType::Motorcycle,
            8 => ElementType::Lane,
            9 => ElementType::Arrow,
            _ => ElementType::ParkingSlot,
        }
    }
}

/// A single element in the parking lot layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutElement {
    pub id: String,
    pub element_type: ElementType,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
    pub slot_number: i32,
    pub color: String, // Hex color string
}

/// A complete parking lot layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParkingLayout {
    pub id: String,
    pub name: String,
    pub created: String,
    pub modified: String,
    pub elements: Vec<LayoutElement>,
    pub canvas_width: f32,
    pub canvas_height: f32,
    pub grid_size: f32,
}

impl ParkingLayout {
    pub fn new(name: String) -> Self {
        let now = Local::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            created: now.format("%Y-%m-%d %H:%M").to_string(),
            modified: now.format("%Y-%m-%d %H:%M").to_string(),
            elements: Vec::new(),
            canvas_width: 800.0,
            canvas_height: 600.0,
            grid_size: 20.0,
        }
    }
}

/// Layout storage manager
pub struct LayoutStorage {
    layouts_dir: PathBuf,
}

impl LayoutStorage {
    /// Create a new layout storage manager
    pub fn new() -> Result<Self> {
        let project_dirs = ProjectDirs::from("com", "securanido", "parking-desktop")
            .context("Failed to determine project directories")?;

        let layouts_dir = project_dirs.data_dir().join("layouts");

        // Create layouts directory if it doesn't exist
        fs::create_dir_all(&layouts_dir).context("Failed to create layouts directory")?;

        Ok(Self { layouts_dir })
    }

    /// Get the path for a layout file
    fn layout_path(&self, id: &str) -> PathBuf {
        self.layouts_dir.join(format!("{}.json", id))
    }

    /// Save a layout to disk
    pub fn save_layout(&self, layout: &ParkingLayout) -> Result<()> {
        let path = self.layout_path(&layout.id);
        let json = serde_json::to_string_pretty(layout).context("Failed to serialize layout")?;

        fs::write(&path, json).with_context(|| format!("Failed to write layout to {:?}", path))?;

        tracing::info!("Saved layout '{}' to {:?}", layout.name, path);
        Ok(())
    }

    /// Load a layout from disk
    pub fn load_layout(&self, id: &str) -> Result<ParkingLayout> {
        let path = self.layout_path(id);
        let json = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read layout from {:?}", path))?;

        let layout: ParkingLayout =
            serde_json::from_str(&json).context("Failed to deserialize layout")?;

        Ok(layout)
    }

    /// Delete a layout from disk
    pub fn delete_layout(&self, id: &str) -> Result<()> {
        let path = self.layout_path(id);
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to delete layout {:?}", path))?;
            tracing::info!("Deleted layout {}", id);
        }
        Ok(())
    }

    /// List all saved layouts (returns summary info, not full layouts)
    pub fn list_layouts(&self) -> Result<Vec<LayoutSummary>> {
        let mut summaries = Vec::new();

        for entry in fs::read_dir(&self.layouts_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(json) = fs::read_to_string(&path) {
                    if let Ok(layout) = serde_json::from_str::<ParkingLayout>(&json) {
                        summaries.push(LayoutSummary {
                            id: layout.id,
                            name: layout.name,
                            created: layout.created,
                            modified: layout.modified,
                            elements_count: layout.elements.len() as i32,
                        });
                    }
                }
            }
        }

        // Sort by modified date (newest first)
        summaries.sort_by(|a, b| b.modified.cmp(&a.modified));

        Ok(summaries)
    }

    /// Create a sample/demo layout
    pub fn create_demo_layout(&self) -> Result<ParkingLayout> {
        let mut layout = ParkingLayout::new("Demo Parkplatz".to_string());

        // Add some sample elements
        // Entry
        layout.elements.push(LayoutElement {
            id: Uuid::new_v4().to_string(),
            element_type: ElementType::Entry,
            x: 40.0,
            y: 280.0,
            width: 60.0,
            height: 40.0,
            rotation: 0.0,
            slot_number: 0,
            color: "#22c55e".to_string(),
        });

        // Exit
        layout.elements.push(LayoutElement {
            id: Uuid::new_v4().to_string(),
            element_type: ElementType::Exit,
            x: 700.0,
            y: 280.0,
            width: 60.0,
            height: 40.0,
            rotation: 0.0,
            slot_number: 0,
            color: "#ef4444".to_string(),
        });

        // Top row of parking slots
        for i in 0..5 {
            layout.elements.push(LayoutElement {
                id: Uuid::new_v4().to_string(),
                element_type: ElementType::ParkingSlot,
                x: 140.0 + (i as f32 * 100.0),
                y: 60.0,
                width: 80.0,
                height: 120.0,
                rotation: 0.0,
                slot_number: i + 1,
                color: "#6366f1".to_string(),
            });
        }

        // Bottom row of parking slots
        for i in 0..5 {
            layout.elements.push(LayoutElement {
                id: Uuid::new_v4().to_string(),
                element_type: ElementType::ParkingSlot,
                x: 140.0 + (i as f32 * 100.0),
                y: 420.0,
                width: 80.0,
                height: 120.0,
                rotation: 0.0,
                slot_number: i + 6,
                color: "#6366f1".to_string(),
            });
        }

        // Driving lane
        layout.elements.push(LayoutElement {
            id: Uuid::new_v4().to_string(),
            element_type: ElementType::Lane,
            x: 100.0,
            y: 260.0,
            width: 600.0,
            height: 80.0,
            rotation: 0.0,
            slot_number: 0,
            color: "#64748b".to_string(),
        });

        // Some pillars
        layout.elements.push(LayoutElement {
            id: Uuid::new_v4().to_string(),
            element_type: ElementType::Pillar,
            x: 120.0,
            y: 200.0,
            width: 20.0,
            height: 20.0,
            rotation: 0.0,
            slot_number: 0,
            color: "#374151".to_string(),
        });

        layout.elements.push(LayoutElement {
            id: Uuid::new_v4().to_string(),
            element_type: ElementType::Pillar,
            x: 680.0,
            y: 200.0,
            width: 20.0,
            height: 20.0,
            rotation: 0.0,
            slot_number: 0,
            color: "#374151".to_string(),
        });

        // Electric charging spot
        layout.elements.push(LayoutElement {
            id: Uuid::new_v4().to_string(),
            element_type: ElementType::Electric,
            x: 640.0,
            y: 60.0,
            width: 80.0,
            height: 120.0,
            rotation: 0.0,
            slot_number: 11,
            color: "#22c55e".to_string(),
        });

        // Handicap spot
        layout.elements.push(LayoutElement {
            id: Uuid::new_v4().to_string(),
            element_type: ElementType::Handicap,
            x: 640.0,
            y: 420.0,
            width: 80.0,
            height: 120.0,
            rotation: 0.0,
            slot_number: 12,
            color: "#3b82f6".to_string(),
        });

        self.save_layout(&layout)?;

        Ok(layout)
    }
}

impl Default for LayoutStorage {
    fn default() -> Self {
        Self::new().expect("Failed to initialize layout storage")
    }
}

/// Summary info for listing layouts without loading full data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSummary {
    pub id: String,
    pub name: String,
    pub created: String,
    pub modified: String,
    pub elements_count: i32,
}

// =============================================================================
// HEADLESS UNIT TESTS - State-of-the-art 2026 Rust Testing
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Create a test LayoutStorage using a temporary directory
    fn create_test_storage() -> (LayoutStorage, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let layouts_dir = temp_dir.path().join("layouts");
        fs::create_dir_all(&layouts_dir).expect("Failed to create layouts dir");

        let storage = LayoutStorage { layouts_dir };
        (storage, temp_dir)
    }

    // -------------------------------------------------------------------------
    // ElementType Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_element_type_to_index() {
        assert_eq!(ElementType::ParkingSlot.to_index(), 0);
        assert_eq!(ElementType::Wall.to_index(), 1);
        assert_eq!(ElementType::Pillar.to_index(), 2);
        assert_eq!(ElementType::Entry.to_index(), 3);
        assert_eq!(ElementType::Exit.to_index(), 4);
        assert_eq!(ElementType::Handicap.to_index(), 5);
        assert_eq!(ElementType::Electric.to_index(), 6);
        assert_eq!(ElementType::Motorcycle.to_index(), 7);
        assert_eq!(ElementType::Lane.to_index(), 8);
        assert_eq!(ElementType::Arrow.to_index(), 9);
    }

    #[test]
    fn test_element_type_from_index() {
        assert_eq!(ElementType::from_index(0), ElementType::ParkingSlot);
        assert_eq!(ElementType::from_index(1), ElementType::Wall);
        assert_eq!(ElementType::from_index(2), ElementType::Pillar);
        assert_eq!(ElementType::from_index(3), ElementType::Entry);
        assert_eq!(ElementType::from_index(4), ElementType::Exit);
        assert_eq!(ElementType::from_index(5), ElementType::Handicap);
        assert_eq!(ElementType::from_index(6), ElementType::Electric);
        assert_eq!(ElementType::from_index(7), ElementType::Motorcycle);
        assert_eq!(ElementType::from_index(8), ElementType::Lane);
        assert_eq!(ElementType::from_index(9), ElementType::Arrow);
    }

    #[test]
    fn test_element_type_from_invalid_index() {
        // Invalid indices should default to ParkingSlot
        assert_eq!(ElementType::from_index(-1), ElementType::ParkingSlot);
        assert_eq!(ElementType::from_index(99), ElementType::ParkingSlot);
    }

    #[test]
    fn test_element_type_roundtrip() {
        // Converting to index and back should give same element
        let types = vec![
            ElementType::ParkingSlot,
            ElementType::Wall,
            ElementType::Pillar,
            ElementType::Entry,
            ElementType::Exit,
            ElementType::Handicap,
            ElementType::Electric,
            ElementType::Motorcycle,
            ElementType::Lane,
            ElementType::Arrow,
        ];

        for elem_type in types {
            let index = elem_type.to_index();
            let restored = ElementType::from_index(index);
            assert_eq!(restored, elem_type);
        }
    }

    // -------------------------------------------------------------------------
    // ParkingLayout Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parking_layout_new() {
        let layout = ParkingLayout::new("Test Layout".to_string());

        assert_eq!(layout.name, "Test Layout");
        assert!(!layout.id.is_empty(), "Layout should have an ID");
        assert!(
            layout.elements.is_empty(),
            "New layout should have no elements"
        );
        assert_eq!(layout.canvas_width, 800.0);
        assert_eq!(layout.canvas_height, 600.0);
        assert_eq!(layout.grid_size, 20.0);
    }

    #[test]
    fn test_parking_layout_dates() {
        let layout = ParkingLayout::new("Date Test".to_string());

        // Dates should be in YYYY-MM-DD HH:MM format
        assert!(
            layout.created.contains("-"),
            "Created date should contain dashes"
        );
        assert!(
            layout.created.contains(":"),
            "Created date should contain time"
        );
        assert_eq!(
            layout.created, layout.modified,
            "Created and modified should be same initially"
        );
    }

    #[test]
    fn test_parking_layout_unique_ids() {
        let layout1 = ParkingLayout::new("Layout 1".to_string());
        let layout2 = ParkingLayout::new("Layout 2".to_string());

        assert_ne!(layout1.id, layout2.id, "Each layout should have unique ID");
    }

    // -------------------------------------------------------------------------
    // LayoutStorage Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_save_and_load_layout() {
        let (storage, _temp_dir) = create_test_storage();

        // Create a layout
        let mut layout = ParkingLayout::new("Save Test".to_string());
        layout.elements.push(LayoutElement {
            id: "elem1".to_string(),
            element_type: ElementType::ParkingSlot,
            x: 100.0,
            y: 200.0,
            width: 80.0,
            height: 120.0,
            rotation: 0.0,
            slot_number: 1,
            color: "#6366f1".to_string(),
        });

        // Save it
        storage.save_layout(&layout).expect("Failed to save layout");

        // Load it back
        let loaded = storage
            .load_layout(&layout.id)
            .expect("Failed to load layout");

        assert_eq!(loaded.name, "Save Test");
        assert_eq!(loaded.id, layout.id);
        assert_eq!(loaded.elements.len(), 1);
        assert_eq!(loaded.elements[0].slot_number, 1);
        assert_eq!(loaded.elements[0].x, 100.0);
    }

    #[test]
    fn test_list_layouts_empty() {
        let (storage, _temp_dir) = create_test_storage();

        let layouts = storage.list_layouts().expect("Failed to list layouts");
        assert!(layouts.is_empty(), "Should have no layouts initially");
    }

    #[test]
    fn test_list_layouts_with_layouts() {
        let (storage, _temp_dir) = create_test_storage();

        // Create multiple layouts
        let layout1 = ParkingLayout::new("Layout A".to_string());
        let layout2 = ParkingLayout::new("Layout B".to_string());

        storage
            .save_layout(&layout1)
            .expect("Failed to save layout 1");
        storage
            .save_layout(&layout2)
            .expect("Failed to save layout 2");

        let layouts = storage.list_layouts().expect("Failed to list layouts");
        assert_eq!(layouts.len(), 2, "Should have 2 layouts");

        let names: Vec<&str> = layouts.iter().map(|l| l.name.as_str()).collect();
        assert!(names.contains(&"Layout A"));
        assert!(names.contains(&"Layout B"));
    }

    #[test]
    fn test_delete_layout() {
        let (storage, _temp_dir) = create_test_storage();

        // Create and save a layout
        let layout = ParkingLayout::new("Delete Me".to_string());
        let layout_id = layout.id.clone();
        storage.save_layout(&layout).expect("Failed to save layout");

        // Verify it exists
        let layouts = storage.list_layouts().expect("Failed to list layouts");
        assert_eq!(layouts.len(), 1);

        // Delete it
        storage
            .delete_layout(&layout_id)
            .expect("Failed to delete layout");

        // Verify it's gone
        let layouts = storage.list_layouts().expect("Failed to list layouts");
        assert!(layouts.is_empty(), "Layout should be deleted");
    }

    #[test]
    fn test_delete_nonexistent_layout() {
        let (storage, _temp_dir) = create_test_storage();

        // Deleting a non-existent layout should not error
        let result = storage.delete_layout("nonexistent-id");
        assert!(
            result.is_ok(),
            "Deleting non-existent layout should succeed"
        );
    }

    #[test]
    fn test_load_nonexistent_layout() {
        let (storage, _temp_dir) = create_test_storage();

        let result = storage.load_layout("nonexistent-id");
        assert!(result.is_err(), "Loading non-existent layout should fail");
    }

    #[test]
    fn test_layout_summary_element_count() {
        let (storage, _temp_dir) = create_test_storage();

        let mut layout = ParkingLayout::new("Count Test".to_string());
        for i in 0..5 {
            layout.elements.push(LayoutElement {
                id: format!("elem{}", i),
                element_type: ElementType::ParkingSlot,
                x: (i * 100) as f32,
                y: 0.0,
                width: 80.0,
                height: 120.0,
                rotation: 0.0,
                slot_number: i + 1,
                color: "#6366f1".to_string(),
            });
        }

        storage.save_layout(&layout).expect("Failed to save layout");

        let summaries = storage.list_layouts().expect("Failed to list layouts");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].elements_count, 5);
    }

    // -------------------------------------------------------------------------
    // LayoutElement Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_layout_element_serialization() {
        let element = LayoutElement {
            id: "test-element".to_string(),
            element_type: ElementType::Electric,
            x: 150.5,
            y: 250.75,
            width: 80.0,
            height: 120.0,
            rotation: 90.0,
            slot_number: 42,
            color: "#22c55e".to_string(),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&element).expect("Failed to serialize");

        // Deserialize back
        let restored: LayoutElement = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(restored.id, "test-element");
        assert_eq!(restored.element_type, ElementType::Electric);
        assert_eq!(restored.x, 150.5);
        assert_eq!(restored.y, 250.75);
        assert_eq!(restored.rotation, 90.0);
        assert_eq!(restored.slot_number, 42);
        assert_eq!(restored.color, "#22c55e");
    }

    #[test]
    fn test_all_element_types_serialize() {
        let types = vec![
            ElementType::ParkingSlot,
            ElementType::Wall,
            ElementType::Pillar,
            ElementType::Entry,
            ElementType::Exit,
            ElementType::Handicap,
            ElementType::Electric,
            ElementType::Motorcycle,
            ElementType::Lane,
            ElementType::Arrow,
        ];

        for elem_type in types {
            let element = LayoutElement {
                id: "test".to_string(),
                element_type: elem_type.clone(),
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
                rotation: 0.0,
                slot_number: 0,
                color: "#000".to_string(),
            };

            let json = serde_json::to_string(&element).expect("Failed to serialize");
            let restored: LayoutElement =
                serde_json::from_str(&json).expect("Failed to deserialize");

            assert_eq!(restored.element_type, elem_type);
        }
    }
}
