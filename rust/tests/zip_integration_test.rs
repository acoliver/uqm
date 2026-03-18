/// Integration test for ZIP archive mounting
///
/// This test verifies that the ZIP reader can:
/// 1. Create and index a test archive
/// 2. List directory contents
/// 3. Read file entries
/// 4. Handle synthetic directories
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_zip_archive_basic_functionality() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory and ZIP file
    let temp_dir = TempDir::new()?;
    let zip_path = temp_dir.path().join("test.uqm");

    // Create a simple ZIP archive
    let file = File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // Add files in various directory structures
    zip.start_file("ships/human/cruiser.png", options)?;
    zip.write_all(b"PNG data for cruiser")?;

    zip.start_file("ships/alien/probe.png", options)?;
    zip.write_all(b"PNG data for probe")?;

    zip.start_file("sounds/battle.ogg", options)?;
    zip.write_all(b"OGG audio data")?;

    zip.start_file("readme.txt", options)?;
    zip.write_all(b"Welcome to UQM!")?;

    let _ = zip.finish()?;

    // Now test the ZIP reader
    let index = uqm_rust::io::zip_reader::ZipIndex::new(&zip_path)?;

    // Test 1: Check that files exist
    assert!(index.contains("readme.txt"), "Should find readme.txt");
    assert!(
        index.contains("ships/human/cruiser.png"),
        "Should find cruiser.png"
    );
    assert!(
        index.contains("ships/alien/probe.png"),
        "Should find probe.png"
    );
    assert!(
        index.contains("sounds/battle.ogg"),
        "Should find battle.ogg"
    );

    // Test 2: Check synthetic directories
    assert!(index.is_directory("ships"), "ships should be a directory");
    assert!(
        index.is_directory("ships/human"),
        "ships/human should be a directory"
    );
    assert!(
        index.is_directory("ships/alien"),
        "ships/alien should be a directory"
    );
    assert!(index.is_directory("sounds"), "sounds should be a directory");

    // Test 3: List root directory
    let root_entries = index.list_directory("");
    assert!(
        root_entries.contains(&"readme.txt".to_string()),
        "Root should contain readme.txt"
    );
    assert!(
        root_entries.contains(&"ships".to_string()),
        "Root should contain ships dir"
    );
    assert!(
        root_entries.contains(&"sounds".to_string()),
        "Root should contain sounds dir"
    );

    // Test 4: List ships directory
    let ships_entries = index.list_directory("ships");
    assert!(
        ships_entries.contains(&"human".to_string()),
        "ships should contain human dir"
    );
    assert!(
        ships_entries.contains(&"alien".to_string()),
        "ships should contain alien dir"
    );

    // Test 5: List ships/human directory
    let human_entries = index.list_directory("ships/human");
    assert!(
        human_entries.contains(&"cruiser.png".to_string()),
        "ships/human should contain cruiser.png"
    );

    // Test 6: Read file content
    let readme_content = index.read_entry("readme.txt")?;
    assert_eq!(
        readme_content, b"Welcome to UQM!",
        "readme.txt content should match"
    );

    let cruiser_content = index.read_entry("ships/human/cruiser.png")?;
    assert_eq!(
        cruiser_content, b"PNG data for cruiser",
        "cruiser.png content should match"
    );

    // Test 7: Check entry metadata
    let readme_entry = index
        .get_entry("readme.txt")
        .expect("Should find readme entry");
    assert_eq!(
        readme_entry.uncompressed_size, 15,
        "readme.txt size should be 15 bytes"
    );
    assert!(
        !readme_entry.is_directory,
        "readme.txt should not be a directory"
    );

    Ok(())
}

#[test]
fn test_zip_path_normalization() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let zip_path = temp_dir.path().join("normalized.zip");

    let file = File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // Add files with various path formats
    zip.start_file("normal/file.txt", options)?;
    zip.write_all(b"normal")?;

    // The zip crate will accept these, but our normalization should handle them
    zip.start_file("/leading/slash.txt", options)?;
    zip.write_all(b"leading")?;

    let _ = zip.finish()?;

    let index = uqm_rust::io::zip_reader::ZipIndex::new(&zip_path)?;

    // Test normalization: leading slashes should be stripped
    assert!(
        index.contains("normal/file.txt"),
        "Should find normal/file.txt"
    );
    assert!(
        index.contains("leading/slash.txt"),
        "Should find leading/slash.txt (normalized)"
    );

    // Synthetic directories should also be normalized
    assert!(index.is_directory("normal"), "normal should be a directory");
    assert!(
        index.is_directory("leading"),
        "leading should be a directory"
    );

    Ok(())
}

#[test]
fn test_zip_case_sensitivity() -> Result<(), Box<dyn std::error::Error>> {
    // Test that ZIP lookups are case-sensitive
    let temp_dir = TempDir::new()?;
    let zip_path = temp_dir.path().join("case_test.zip");

    let file = File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // Add files with different cases
    zip.start_file("README.txt", options)?;
    zip.write_all(b"uppercase README")?;

    zip.start_file("readme.txt", options)?;
    zip.write_all(b"lowercase readme")?;

    let _ = zip.finish()?;

    let index = uqm_rust::io::zip_reader::ZipIndex::new(&zip_path)?;

    // Both should exist as separate entries (case-sensitive)
    assert!(index.contains("README.txt"), "Should find README.txt");
    assert!(index.contains("readme.txt"), "Should find readme.txt");

    // Content should match exactly
    let uppercase_content = index.read_entry("README.txt")?;
    assert_eq!(uppercase_content, b"uppercase README");

    let lowercase_content = index.read_entry("readme.txt")?;
    assert_eq!(lowercase_content, b"lowercase readme");

    Ok(())
}
