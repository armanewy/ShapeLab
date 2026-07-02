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
        let old_package_prefix = format!("name = \"{}-", "shape");
        let old_workspace_prefix = format!("\"crates/{}-", "shape");
        assert!(
            !contents.contains(old_package_prefix.as_str()),
            "{} still declares a legacy package",
            manifest.display()
        );
        assert!(
            !contents.contains(old_workspace_prefix.as_str()),
            "{} still points at a legacy workspace member",
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
        let old_use_prefix = format!("use {}_", "shape");
        let old_extern_prefix = format!("extern crate {}_", "shape");
        for (line_number, line) in contents.lines().enumerate() {
            let trimmed = line.trim_start();
            assert!(
                !trimmed.starts_with(old_use_prefix.as_str()),
                "{}:{} still imports an old crate: {}",
                path.display(),
                line_number + 1,
                line
            );
            assert!(
                !trimmed.starts_with(old_extern_prefix.as_str()),
                "{}:{} still declares an old crate: {}",
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
        if path.ends_with("docs/CLEANUP_AND_OBJECT_ORCHARD_RENAME_INTEGRATION_REPORT.md")
            || path.ends_with("docs/RUST_CRATE_FOLDER_RENAME_REPORT.md")
        {
            continue;
        }
        let contents = fs::read_to_string(&path).expect("doc text");
        let old_cli_name = format!("{}-cli", "shape");
        assert!(
            !contents.contains(old_cli_name.as_str()),
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
        "docs/CLEANUP_AND_OBJECT_ORCHARD_RENAME_INTEGRATION_REPORT.md",
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
        let old_env_prefix = ["SHAPE", "LAB_"].join("_");
        let old_slug = ["shape", "lab"].join("-");
        assert!(
            !contents.contains(old_env_prefix.as_str()),
            "{} still documents or uses a legacy env var",
            path.display()
        );
        assert!(
            !contents.contains(old_slug.as_str()),
            "{} still uses a legacy local path or slug",
            path.display()
        );

        if !allowed_old_name_docs.contains(&relative.as_str()) {
            let old_spaced_name = "Shape ".to_owned() + "Lab";
            let old_compact_name = ["Shape", "Lab"].concat();
            assert!(
                !contents.contains(old_spaced_name.as_str()),
                "{} still contains the old spaced product name",
                path.display()
            );
            assert!(
                !contents.contains(old_compact_name.as_str()),
                "{} still contains the old compact repository name",
                path.display()
            );
        }
    }
}
