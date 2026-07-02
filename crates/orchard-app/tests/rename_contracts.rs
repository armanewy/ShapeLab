use std::{fs, path::Path};

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}

fn collect_files(root: &Path, predicate: fn(&Path) -> bool, files: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(root).expect("read dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name == ".git" || name == "target" {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, predicate, files);
        } else if predicate(&path) {
            files.push(path);
        }
    }
}

#[test]
fn cargo_packages_and_workspace_members_use_orchard_names() {
    let root = repo_root();
    let mut manifests = Vec::new();
    collect_files(
        &root,
        |path| path.file_name().is_some_and(|name| name == "Cargo.toml"),
        &mut manifests,
    );

    for manifest in manifests {
        let contents = fs::read_to_string(&manifest).expect("manifest text");
        assert!(
            !contents.contains("name = \"shape-"),
            "{} still declares a shape-* package",
            manifest.display()
        );
        assert!(
            !contents.contains("\"crates/shape-"),
            "{} still points at a shape-* workspace member",
            manifest.display()
        );
    }

    let root_manifest = fs::read_to_string(root.join("Cargo.toml")).expect("root manifest");
    assert!(root_manifest.contains("crates/orchard-app"));
    assert!(root_manifest.contains("crates/orchard-cli"));
    assert!(root_manifest.contains("orchard-core-legacy"));
}

#[test]
fn rust_source_does_not_import_old_shape_crates() {
    let root = repo_root();
    let mut rust_files = Vec::new();
    collect_files(
        &root.join("crates"),
        |path| path.extension().is_some_and(|ext| ext == "rs"),
        &mut rust_files,
    );

    for path in rust_files {
        let contents = fs::read_to_string(&path).expect("rust source");
        for (line_number, line) in contents.lines().enumerate() {
            let trimmed = line.trim_start();
            assert!(
                !trimmed.starts_with("use shape_"),
                "{}:{} still imports an old shape_* crate: {}",
                path.display(),
                line_number + 1,
                line
            );
            assert!(
                !trimmed.starts_with("extern crate shape_"),
                "{}:{} still declares an old shape_* crate: {}",
                path.display(),
                line_number + 1,
                line
            );
        }
    }
}

#[test]
fn docs_command_examples_use_orchard_cli() {
    let root = repo_root();
    let mut docs = Vec::new();
    collect_files(
        &root.join("docs"),
        |path| path.extension().is_some_and(|ext| ext == "md"),
        &mut docs,
    );
    docs.push(root.join("README.md"));

    for path in docs {
        if path.ends_with("docs/RUST_CRATE_FOLDER_RENAME_REPORT.md") {
            continue;
        }
        let contents = fs::read_to_string(&path).expect("doc text");
        assert!(
            !contents.contains("shape-cli"),
            "{} still uses the old CLI command name",
            path.display()
        );
    }
}

#[test]
fn repository_docs_scripts_and_packaging_use_object_orchard_names() {
    let root = repo_root();
    let mut files = Vec::new();
    for relative_root in ["docs", "scripts", "packaging", ".github"] {
        let path = root.join(relative_root);
        if path.exists() {
            collect_files(
                &path,
                |path| {
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| {
                            matches!(
                                ext,
                                "md" | "sh"
                                    | "ps1"
                                    | "py"
                                    | "yml"
                                    | "yaml"
                                    | "toml"
                                    | "plist"
                                    | "svg"
                                    | "txt"
                            )
                        })
                },
                &mut files,
            );
        }
    }
    files.push(root.join("README.md"));
    files.push(root.join("Cargo.toml"));

    let allowed_old_name_docs = [
        "docs/OBJECT_ORCHARD_NAMING_TRANSITION.md",
        "docs/OBJECT_ORCHARD_REPOSITORY_RENAME_GUIDE.md",
    ];

    for path in files {
        let relative = path
            .strip_prefix(&root)
            .expect("repo-relative path")
            .to_string_lossy()
            .replace('\\', "/");
        let contents = fs::read_to_string(&path).expect("repo text");
        assert!(
            !contents.contains("SHAPE_LAB_"),
            "{} still documents or uses an old SHAPE_LAB_* env var",
            path.display()
        );
        assert!(
            !contents.contains("shape-lab"),
            "{} still uses an old local shape-lab path or slug",
            path.display()
        );

        if !allowed_old_name_docs.contains(&relative.as_str()) {
            assert!(
                !contents.contains("Shape Lab"),
                "{} still contains the old spaced product name",
                path.display()
            );
            assert!(
                !contents.contains("ShapeLab"),
                "{} still contains the old compact repository name",
                path.display()
            );
        }
    }
}
