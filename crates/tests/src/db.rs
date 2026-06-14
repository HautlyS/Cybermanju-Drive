use cybermanju_db::Database;

fn temp_db() -> (Database, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.redb");
    let db = Database::new(path.to_str().unwrap()).unwrap();
    (db, dir)
}

#[test]
fn test_database_creation() {
    let (_db, _dir) = temp_db();
}

#[test]
fn test_table_accessors_exist() {
    let _ = Database::get_files_table();
    let _ = Database::get_accounts_table();
    let _ = Database::get_collections_table();
    let _ = Database::get_collection_items_table();
    let _ = Database::get_face_groups_table();
    let _ = Database::get_loose_groups_table();
    let _ = Database::get_encryption_keys_table();
    let _ = Database::get_locations_table();
    let _ = Database::get_users_table();
    let _ = Database::get_user_file_perms_table();
    let _ = Database::get_sync_configs_table();
    let _ = Database::get_parent_index_table();
}

#[test]
fn test_begin_read_write() {
    let (db, _dir) = temp_db();
    let _read_txn = db.begin_read().unwrap();
    let _write_txn = db.begin_write().unwrap();
}

#[test]
fn test_insert_file_with_index() {
    let (db, _dir) = temp_db();
    let file_json = r#"{"id":"f1","name":"test.txt"}"#;
    db.insert_file_with_index("f1", file_json, Some("root"))
        .unwrap();

    let children = db.list_by_parent("root").unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0], "f1");
}

#[test]
fn test_insert_multiple_files_same_parent() {
    let (db, _dir) = temp_db();
    db.insert_file_with_index("f1", "j1", Some("root")).unwrap();
    db.insert_file_with_index("f2", "j2", Some("root")).unwrap();
    db.insert_file_with_index("f3", "j3", Some("root")).unwrap();

    let children = db.list_by_parent("root").unwrap();
    assert_eq!(children.len(), 3);
}

#[test]
fn test_insert_file_no_parent() {
    let (db, _dir) = temp_db();
    db.insert_file_with_index("f1", "j1", None).unwrap();
    let children = db.list_by_parent("root").unwrap();
    assert!(children.is_empty());
}

#[test]
fn test_remove_file_with_index() {
    let (db, _dir) = temp_db();
    db.insert_file_with_index("f1", "j1", Some("root")).unwrap();
    let removed = db.remove_file_with_index("f1", Some("root")).unwrap();
    assert!(removed);
    let children = db.list_by_parent("root").unwrap();
    assert!(children.is_empty());
}

#[test]
fn test_remove_nonexistent_file() {
    let (db, _dir) = temp_db();
    let removed = db.remove_file_with_index("nonexistent", None).unwrap();
    assert!(!removed);
}

#[test]
fn test_add_to_parent_index() {
    let (db, _dir) = temp_db();
    db.add_to_parent_index("f1", "parent1").unwrap();
    db.add_to_parent_index("f2", "parent1").unwrap();

    let children = db.list_by_parent("parent1").unwrap();
    assert_eq!(children, vec!["f1".to_string(), "f2".to_string()]);
}

#[test]
fn test_add_duplicate_to_parent_index() {
    let (db, _dir) = temp_db();
    db.add_to_parent_index("f1", "parent1").unwrap();
    db.add_to_parent_index("f1", "parent1").unwrap();

    let children = db.list_by_parent("parent1").unwrap();
    assert_eq!(children.len(), 1);
}

#[test]
fn test_remove_from_parent_index() {
    let (db, _dir) = temp_db();
    db.add_to_parent_index("f1", "parent1").unwrap();
    db.add_to_parent_index("f2", "parent1").unwrap();
    db.remove_from_parent_index("f1", "parent1").unwrap();

    let children = db.list_by_parent("parent1").unwrap();
    assert_eq!(children, vec!["f2".to_string()]);
}

#[test]
fn test_remove_last_from_parent_index() {
    let (db, _dir) = temp_db();
    db.add_to_parent_index("f1", "parent1").unwrap();
    db.remove_from_parent_index("f1", "parent1").unwrap();

    let children = db.list_by_parent("parent1").unwrap();
    assert!(children.is_empty());
}

#[test]
fn test_list_by_parent_empty() {
    let (db, _dir) = temp_db();
    let children = db.list_by_parent("nonexistent").unwrap();
    assert!(children.is_empty());
}

#[test]
fn test_move_file_with_index() {
    let (db, _dir) = temp_db();
    db.insert_file_with_index("f1", "j1", Some("old_parent"))
        .unwrap();

    db.move_file_with_index("f1", "j1_updated", Some("old_parent"), "new_parent")
        .unwrap();

    let old_children = db.list_by_parent("old_parent").unwrap();
    assert!(old_children.is_empty());

    let new_children = db.list_by_parent("new_parent").unwrap();
    assert_eq!(new_children, vec!["f1".to_string()]);
}

#[test]
fn test_move_file_no_old_parent() {
    let (db, _dir) = temp_db();
    db.insert_file_with_index("f1", "j1", None).unwrap();

    db.move_file_with_index("f1", "j1", None, "new_parent")
        .unwrap();

    let children = db.list_by_parent("new_parent").unwrap();
    assert_eq!(children, vec!["f1".to_string()]);
}

#[test]
fn test_file_index_across_multiple_parents() {
    let (db, _dir) = temp_db();
    db.add_to_parent_index("f1", "p1").unwrap();
    db.add_to_parent_index("f2", "p1").unwrap();
    db.add_to_parent_index("f3", "p2").unwrap();
    db.add_to_parent_index("f4", "p2").unwrap();
    db.add_to_parent_index("f5", "p2").unwrap();

    assert_eq!(db.list_by_parent("p1").unwrap().len(), 2);
    assert_eq!(db.list_by_parent("p2").unwrap().len(), 3);
    assert!(db.list_by_parent("p3").unwrap().is_empty());
}
