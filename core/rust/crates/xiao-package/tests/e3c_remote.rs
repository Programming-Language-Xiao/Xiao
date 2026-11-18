//! 11A-E3C：本机 HTTP、Git refs 与 Range 断点恢复的可重复端到端证据。

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;
use sha2::{Digest, Sha256};
use xiao_codegen_llvm::{TargetDescription, Toolchain};
use xiao_config::{
    INVALID_DEPENDENCY_GIT_CODE, git_dependency_declarations, parse_config_project,
    parse_config_text,
};
use xiao_package::{
    ArtifactReference, CacheLayout, CacheStore, ConfiguredSource, GitHubAdapter, GitReference,
    HttpStaticAdapter, IndexPackage, LOCKFILE_VERSION, LocalDirectoryAdapter, LockFile,
    LockedPackage, LockedSourceSnapshot, MultiSourceAdapter, PackageIdentity, PackageOperation,
    PackageShard, PackageSource, PackageSourceAdapter, SOURCE_DIGEST_MISMATCH_CODE,
    SOURCE_INVALID_CODE, SOURCE_UNAVAILABLE_CODE, SOURCE_UNSUPPORTED_VERSION_CODE,
    SnapshotManifest, SnapshotStatus, SnapshotStore, SourceDeclaration, SourceDescriptor,
    SourceResolver, TRUST_ARTIFACT_CODE, apply_packages, fingerprint_config, jcs_digest,
    lockfile_path, parse_advertised_refs, read_lockfile, source_declarations,
};
use xiao_source::SourceFile;

const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const NEXT_COMMIT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
static NEXT_TEST: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
struct Reply {
    status: u16,
    body: Vec<u8>,
    cut: Option<usize>,
    ranged: bool,
    range_end: Option<usize>,
    ranged_body_limit: Option<usize>,
    ranged_cut: Option<usize>,
    delay: Duration,
    extra: String,
}

impl Reply {
    fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            body: body.into(),
            cut: None,
            ranged: false,
            range_end: None,
            ranged_body_limit: None,
            ranged_cut: None,
            delay: Duration::ZERO,
            extra: String::new(),
        }
    }
}

struct Server {
    base: String,
    routes: Arc<Mutex<BTreeMap<String, Reply>>>,
    seen: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let routes = Arc::new(Mutex::new(BTreeMap::<String, Reply>::new()));
        let seen = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (route_reader, seen_writer, stop_reader) = (routes.clone(), seen.clone(), stop.clone());
        let thread = thread::spawn(move || {
            while !stop_reader.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        handle(&mut stream, &route_reader, &seen_writer)
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(error) => panic!("本机测试服务异常：{error}"),
                }
            }
        });
        Self {
            base,
            routes,
            seen,
            stop,
            thread: Some(thread),
        }
    }

    fn route(&self, path: &str, response: Reply) {
        self.routes
            .lock()
            .unwrap()
            .insert(path.to_owned(), response);
    }

    fn requests(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            handle.join().unwrap();
        }
    }
}

fn handle(
    stream: &mut TcpStream,
    routes: &Mutex<BTreeMap<String, Reply>>,
    seen: &Mutex<Vec<String>>,
) {
    stream
        .set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut buffer = [0; 2048];
    while !bytes.ends_with(b"\r\n\r\n") && bytes.len() < 16384 {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(count) => bytes.extend_from_slice(&buffer[..count]),
        }
    }
    let request = String::from_utf8_lossy(&bytes);
    let path = request.split_whitespace().nth(1).unwrap_or("/");
    let range = request.lines().find_map(|line| {
        line.to_ascii_lowercase()
            .strip_prefix("range: bytes=")
            .and_then(|value| value.trim().trim_end_matches('-').parse::<usize>().ok())
    });
    let authenticated = request.lines().any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case("authorization")
                && value.trim() == "Bearer e3d1-secret-should-never-escape"
        })
    });
    seen.lock()
        .unwrap()
        .push(format!("{path} {range:?} auth={authenticated}"));
    let response = routes
        .lock()
        .unwrap()
        .get(path)
        .cloned()
        .unwrap_or_else(|| Reply {
            status: 404,
            ..Reply::ok(Vec::new())
        });
    let response = if path.starts_with("/private/") && !authenticated {
        Reply {
            status: 401,
            ..Reply::ok(Vec::new())
        }
    } else {
        response
    };
    thread::sleep(response.delay);
    let (status, content_range, body) = if let Some(start) = range.filter(|_| response.ranged) {
        let full_body = &response.body[start..];
        (
            206,
            format!(
                "Content-Range: bytes {}-{}/{}\r\n",
                start,
                response.range_end.unwrap_or(response.body.len() - 1),
                response.body.len()
            ),
            &full_body[..response
                .ranged_body_limit
                .unwrap_or(full_body.len())
                .min(full_body.len())],
        )
    } else {
        (response.status, String::new(), response.body.as_slice())
    };
    let header = format!(
        "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n{content_range}{}\r\n",
        body.len(),
        response.extra
    );
    if let Some(cut) = if range.is_some() {
        response.ranged_cut
    } else {
        response.cut
    } {
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(&body[..cut.min(body.len())]);
    } else {
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(body);
    }
}

fn advertisement(commit: &str, tag: Option<&str>) -> Vec<u8> {
    let mut bytes = Vec::new();
    for line in [
        "# service=git-upload-pack\n".to_owned(),
        String::new(),
        format!("{commit} HEAD\0multi_ack\n"),
        format!("{commit} refs/heads/main\n"),
    ] {
        if line.is_empty() {
            bytes.extend_from_slice(b"0000");
        } else {
            bytes.extend_from_slice(format!("{:04x}{line}", line.len() + 4).as_bytes());
        }
    }
    if let Some(tag) = tag {
        let line = format!("{commit} refs/tags/{tag}\n");
        bytes.extend_from_slice(format!("{:04x}{line}", line.len() + 4).as_bytes());
    }
    bytes.extend_from_slice(b"0000");
    bytes
}

fn descriptor(kind: &str, address: &str) -> SourceDescriptor {
    SourceDescriptor::new(kind, address, None, None, 1).unwrap()
}

fn fixture(source: &SourceDescriptor, commit: &str) -> (Vec<u8>, Vec<u8>, ArtifactReference) {
    let body = b"not executable: create NO_SIDE_EFFECT";
    let artifact = ArtifactReference {
        location: "artifacts/demo.bin".to_owned(),
        length: body.len() as u64,
        digest: format!("{:x}", Sha256::digest(body)),
    };
    let shard = serde_json::to_vec(&PackageShard {
        protocol_version: 1,
        source_id: source.source.source_id.clone(),
        snapshot_id: commit.to_owned(),
        packages: vec![IndexPackage {
            name: "demo".to_owned(),
            version: "1.0".to_owned(),
            variant: "any".to_owned(),
            withdrawn: false,
            dependencies: Vec::new(),
            features: Vec::new(),
            target: None,
            abi: None,
            xiao_range: None,
            runtime_range: None,
            source_artifact: artifact.clone(),
            binary_artifacts: Vec::new(),
        }],
    })
    .unwrap();
    let manifest = SnapshotManifest {
        protocol_version: 1,
        source_id: source.source.source_id.clone(),
        snapshot_id: commit.to_owned(),
        shards: BTreeMap::from([(
            "demo".to_owned(),
            jcs_digest(std::str::from_utf8(&shard).unwrap()).unwrap(),
        )]),
        mirrors: Vec::new(),
        expires_at: None,
        signature: None,
    };
    (serde_json::to_vec(&manifest).unwrap(), shard, artifact)
}

fn temporary() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xiao-e3c-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT_TEST.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn pkt_line_rejects_spec_errors_and_non_utf8() {
    let valid: Value = serde_json::from_str(include_str!(
        "../../../../../tests/spec/11a-remote-source/valid.json"
    ))
    .unwrap();
    assert_eq!(valid["commit"], COMMIT);
    assert_eq!(valid["next_commit"], NEXT_COMMIT);
    assert_eq!(
        valid["statuses"],
        serde_json::json!(["fresh", "cached", "unavailable"])
    );
    let refs = advertisement(COMMIT, Some("v1.2.0"));
    assert_eq!(
        parse_advertised_refs(&refs, &GitReference::Branch("main".to_owned()))
            .unwrap()
            .commit,
        COMMIT
    );
    assert_eq!(
        parse_advertised_refs(&refs, &GitReference::Tag("v1.2.0".to_owned()))
            .unwrap()
            .commit,
        COMMIT
    );
    let mut annotated = advertisement(COMMIT, Some("v1.2.0"));
    annotated.truncate(annotated.len() - 4);
    let peeled = format!("{NEXT_COMMIT} refs/tags/v1.2.0^{{}}\n");
    annotated.extend_from_slice(format!("{:04x}{peeled}", peeled.len() + 4).as_bytes());
    annotated.extend_from_slice(b"0000");
    assert_eq!(
        parse_advertised_refs(&annotated, &GitReference::Tag("v1.2.0".to_owned()))
            .unwrap()
            .commit,
        NEXT_COMMIT
    );
    let errors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../../tests/spec/11a-remote-source/errors.json"
    ))
    .unwrap();
    for row in errors {
        let actual = parse_advertised_refs(
            row["wire"].as_str().unwrap().as_bytes(),
            &GitReference::Head,
        )
        .unwrap_err();
        assert_eq!(
            actual.code,
            row["code"].as_str().unwrap(),
            "{}",
            row["name"]
        );
    }
    let mut non_utf8 = refs.clone();
    let index = non_utf8.iter().position(|byte| *byte == b'H').unwrap();
    non_utf8[index] = 0xff;
    assert_eq!(
        parse_advertised_refs(&non_utf8, &GitReference::Head)
            .unwrap_err()
            .code,
        SOURCE_INVALID_CODE
    );
    let unsupported = b"001e# service=git-upload-pack\n0000000eversion 2\n0000";
    assert_eq!(
        parse_advertised_refs(unsupported, &GitReference::Head)
            .unwrap_err()
            .code,
        SOURCE_UNSUPPORTED_VERSION_CODE
    );
}

#[test]
fn pkt_line_accepts_optional_lf_in_service_version_and_refs() {
    for service_header in ["# service=git-upload-pack", "# service=git-upload-pack\n"] {
        let mut wire = Vec::new();
        for line in [
            service_header.to_owned(),
            String::new(),
            "version 1".to_owned(),
            format!("{COMMIT} HEAD\0multi_ack"),
            format!("{COMMIT} refs/heads/main\n"),
        ] {
            if line.is_empty() {
                wire.extend_from_slice(b"0000");
            } else {
                wire.extend_from_slice(format!("{:04x}{line}", line.len() + 4).as_bytes());
            }
        }
        wire.extend_from_slice(b"0000");
        assert_eq!(
            parse_advertised_refs(&wire, &GitReference::Head)
                .unwrap()
                .commit,
            COMMIT
        );
        assert_eq!(
            parse_advertised_refs(&wire, &GitReference::Branch("main".to_owned()))
                .unwrap()
                .commit,
            COMMIT
        );
    }
}

#[test]
fn static_local_and_git_return_identical_package_metadata_without_executing_content() {
    let server = Server::new();
    let root = temporary();
    let local = descriptor("path", &root.to_string_lossy());
    let static_source = descriptor("static", &format!("{}/static/", server.base));
    let git_source = descriptor("git-index", &format!("{}/owner/repo.git", server.base));
    let (local_manifest, local_shard, _) = fixture(&local, COMMIT);
    fs::create_dir_all(root.join("index")).unwrap();
    fs::write(root.join("snapshot.json"), local_manifest).unwrap();
    fs::write(root.join("index/demo.json"), local_shard).unwrap();
    let (static_manifest, static_shard, artifact) = fixture(&static_source, COMMIT);
    server.route("/static/snapshot.json", Reply::ok(static_manifest));
    server.route("/static/index/demo.json", Reply::ok(static_shard));
    server.route(
        "/static/artifacts/demo.bin",
        Reply::ok(b"not executable: create NO_SIDE_EFFECT".to_vec()),
    );
    let (git_manifest, git_shard, _) = fixture(&git_source, COMMIT);
    server.route(
        "/owner/repo.git/info/refs?service=git-upload-pack",
        Reply::ok(advertisement(COMMIT, None)),
    );
    server.route(
        &format!("/raw/{COMMIT}/snapshot.json"),
        Reply::ok(git_manifest),
    );
    server.route(
        &format!("/raw/{COMMIT}/index/demo.json"),
        Reply::ok(git_shard),
    );
    let static_adapter = HttpStaticAdapter::new();
    let git_adapter = GitHubAdapter::with_raw_base(
        GitReference::Branch("main".to_owned()),
        format!("{}/raw", server.base),
    );
    let sources: Vec<(&dyn PackageSourceAdapter, &SourceDescriptor)> = vec![
        (&LocalDirectoryAdapter, &local),
        (&static_adapter, &static_source),
        (&git_adapter, &git_source),
    ];
    let packages = sources
        .iter()
        .map(|(adapter, source)| {
            let index = adapter.read_snapshot(source).unwrap();
            adapter.read_package(source, &index, "demo").unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(packages[0], packages[1]);
    assert_eq!(packages[1], packages[2]);
    let document = parse_config_text("[project]\nname = \"root\"\nversion = \"1\"\n").unwrap();
    let layout = CacheLayout::from_xiao_home(Some(&root.join("home")), &root).unwrap();
    let configured = [&local, &static_source, &git_source]
        .into_iter()
        .enumerate()
        .map(|(order, source)| ConfiguredSource {
            descriptor: source.clone(),
            config_order: order,
            imported_from: None,
        })
        .collect::<Vec<_>>();
    let resolver = SourceResolver::new(
        MultiSourceAdapter::with_git_raw_base(format!("{}/raw", server.base)),
        CacheStore::open(layout).unwrap(),
    );
    let resolved = resolver
        .resolve(&document, &configured, None, &["demo".to_owned()], false)
        .unwrap();
    assert_eq!(resolved.records.len(), 3);
    assert_eq!(
        resolved
            .reports
            .iter()
            .map(|report| report.status)
            .collect::<Vec<_>>(),
        vec![SnapshotStatus::Fresh; 3]
    );
    assert_eq!(
        static_adapter
            .read_artifact(&static_source, &artifact)
            .unwrap()
            .len(),
        artifact.length as usize
    );
    assert!(!root.join("NO_SIDE_EFFECT").exists());
    assert!(
        !server
            .requests()
            .iter()
            .any(|request| request.contains("/artifacts/demo.bin") && request.contains("/raw/"))
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn tag_rewrite_and_same_commit_changed_manifest_are_rejected() {
    let server = Server::new();
    let source = descriptor("git-index", &format!("{}/owner/repo.git", server.base));
    let adapter = GitHubAdapter::with_raw_base(
        GitReference::Tag("v1".to_owned()),
        format!("{}/raw", server.base),
    );
    let refs_path = "/owner/repo.git/info/refs?service=git-upload-pack";
    server.route(refs_path, Reply::ok(advertisement(COMMIT, Some("v1"))));
    let (manifest, _, _) = fixture(&source, COMMIT);
    server.route(&format!("/raw/{COMMIT}/snapshot.json"), Reply::ok(manifest));
    let first = adapter.read_snapshot(&source).unwrap();
    let pin = LockedSourceSnapshot {
        snapshot_id: COMMIT.to_owned(),
        snapshot_digest: first.digest,
    };
    server.route(refs_path, Reply::ok(advertisement(NEXT_COMMIT, Some("v1"))));
    assert_eq!(
        adapter
            .read_snapshot_pinned(&source, Some(&pin))
            .unwrap_err()
            .code,
        SOURCE_DIGEST_MISMATCH_CODE
    );
    server.route(refs_path, Reply::ok(advertisement(COMMIT, Some("v1"))));
    let (mut modified, _, _) = fixture(&source, COMMIT);
    modified.extend_from_slice(b" \n");
    let mut object: Value = serde_json::from_slice(&modified).unwrap();
    object["expires_at"] = Value::String("mutated".to_owned());
    server.route(
        &format!("/raw/{COMMIT}/snapshot.json"),
        Reply::ok(serde_json::to_vec(&object).unwrap()),
    );
    assert_eq!(
        adapter
            .read_snapshot_pinned(&source, Some(&pin))
            .unwrap_err()
            .code,
        SOURCE_DIGEST_MISMATCH_CODE
    );
    let branch = GitHubAdapter::with_raw_base(
        GitReference::Branch("main".to_owned()),
        format!("{}/raw", server.base),
    );
    server.route(refs_path, Reply::ok(advertisement(NEXT_COMMIT, None)));
    let (new_manifest, _, _) = fixture(&source, NEXT_COMMIT);
    server.route(
        &format!("/raw/{NEXT_COMMIT}/snapshot.json"),
        Reply::ok(new_manifest),
    );
    assert_eq!(
        branch
            .read_snapshot_pinned(&source, Some(&pin))
            .unwrap()
            .manifest
            .snapshot_id,
        NEXT_COMMIT
    );
}

#[test]
fn online_lock_cannot_hide_rewritten_tag_but_offline_can_reuse_pinned_snapshot() {
    let server = Server::new();
    let source = descriptor("git-index", &format!("{}/owner/repo.git", server.base))
        .with_git_reference(GitReference::Tag("v1".to_owned()))
        .unwrap();
    server.route(
        "/owner/repo.git/info/refs?service=git-upload-pack",
        Reply::ok(advertisement(COMMIT, Some("v1"))),
    );
    let (manifest, _, _) = fixture(&source, COMMIT);
    server.route(&format!("/raw/{COMMIT}/snapshot.json"), Reply::ok(manifest));
    let root = temporary();
    let project = root.join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("config.xiao"),
        "[project]\nname = \"root\"\nversion = \"1\"\n",
    )
    .unwrap();
    let document =
        parse_config_text(&fs::read_to_string(project.join("config.xiao")).unwrap()).unwrap();
    let layout = CacheLayout::from_xiao_home(Some(&root.join("home")), &root).unwrap();
    let cache = CacheStore::open(layout).unwrap();
    let digest = cache
        .import_source_directory(&project)
        .unwrap()
        .reference
        .digest;
    let identity = PackageIdentity {
        name: "root".to_owned(),
        version: "1".to_owned(),
        source: PackageSource::local_path(&project),
    };
    let package = LockedPackage {
        source_artifact: None,
        name: identity.name.clone(),
        version: identity.version.clone(),
        source: identity.source.clone(),
        content_digest: digest,
        dependencies: BTreeMap::new(),
        precompiled_variants: Vec::new(),
        target_conditions: Vec::new(),
    };
    let mut lock = LockFile {
        lock_version: LOCKFILE_VERSION,
        config_fingerprint: fingerprint_config(&document),
        root: identity.clone(),
        packages: BTreeMap::from([(identity.to_string(), package)]),
        source_snapshots: BTreeMap::new(),
    };
    let configured = [ConfiguredSource {
        descriptor: source,
        config_order: 0,
        imported_from: None,
    }];
    let resolver = SourceResolver::new(
        MultiSourceAdapter::with_git_raw_base(format!("{}/raw", server.base)),
        cache,
    );
    let first = resolver
        .resolve(&document, &configured, None, &[], false)
        .unwrap();
    first.pin_lockfile(&mut lock).unwrap();
    assert_eq!(
        lock.source_snapshots.values().next().unwrap().snapshot_id,
        COMMIT
    );
    server.route(
        "/owner/repo.git/info/refs?service=git-upload-pack",
        Reply::ok(advertisement(NEXT_COMMIT, Some("v1"))),
    );
    let offline = resolver
        .resolve(&document, &configured, Some(&lock), &[], true)
        .unwrap();
    assert!(offline.fast_path);
    assert_eq!(
        resolver
            .resolve(&document, &configured, Some(&lock), &[], false)
            .unwrap_err()
            .code,
        SOURCE_DIGEST_MISMATCH_CODE
    );
    make_writable(&root);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn range_resume_and_unavailable_failures_do_not_create_half_snapshot() {
    let server = Server::new();
    let source = descriptor("static", &format!("{}/static", server.base));
    let (manifest, shard, artifact) = fixture(&source, COMMIT);
    server.route("/static/snapshot.json", Reply::ok(manifest.clone()));
    server.route("/static/index/demo.json", Reply::ok(shard));
    let mut body = Reply::ok(b"not executable: create NO_SIDE_EFFECT".to_vec());
    body.cut = Some(7);
    body.ranged = true;
    server.route("/static/artifacts/demo.bin", body);
    let adapter = HttpStaticAdapter::new();
    assert_eq!(
        adapter.read_artifact(&source, &artifact).unwrap().len(),
        artifact.length as usize
    );
    assert!(
        server
            .requests()
            .iter()
            .any(|request| request.contains("Some(7)"))
    );
    let root = temporary();
    let document = parse_config_text("[project]\nname = \"root\"\nversion = \"1\"\n").unwrap();
    let layout = CacheLayout::from_xiao_home(Some(&root.join("home")), &root).unwrap();
    let configured = [ConfiguredSource {
        descriptor: source.clone(),
        config_order: 0,
        imported_from: None,
    }];
    let resolver = SourceResolver::new(
        HttpStaticAdapter::new(),
        CacheStore::open(layout.clone()).unwrap(),
    );
    let fresh = resolver
        .resolve(&document, &configured, None, &["demo".to_owned()], false)
        .unwrap();
    assert_eq!(fresh.snapshots[0].status, SnapshotStatus::Fresh);
    server.route(
        "/static/snapshot.json",
        Reply {
            cut: Some(3),
            ..Reply::ok(manifest)
        },
    );
    let unavailable = resolver
        .resolve(&document, &configured, None, &["other".to_owned()], false)
        .unwrap();
    assert_eq!(unavailable.snapshots[0].status, SnapshotStatus::Cached);
    assert_eq!(
        unavailable.reports[0].problem.as_ref().unwrap().code,
        SOURCE_UNAVAILABLE_CODE
    );
    assert_eq!(
        SnapshotStore::new(layout)
            .current(&source.source.source_id)
            .unwrap()
            .unwrap()
            .index
            .digest,
        fresh.snapshots[0]
            .snapshot_digest
            .as_ref()
            .unwrap()
            .as_str()
    );
    for status in [404, 503, 302] {
        server.route(
            "/static/snapshot.json",
            Reply {
                status,
                ..Reply::ok(Vec::new())
            },
        );
        let report = resolver
            .resolve(&document, &configured, None, &[], false)
            .unwrap();
        assert_eq!(
            report.reports[0].problem.as_ref().unwrap().code,
            SOURCE_UNAVAILABLE_CODE
        );
    }
    let mut invalid_manifest: Value = serde_json::from_slice(&fixture(&source, COMMIT).0).unwrap();
    invalid_manifest["protocol_version"] = Value::from(2);
    server.route(
        "/static/snapshot.json",
        Reply::ok(serde_json::to_vec(&invalid_manifest).unwrap()),
    );
    assert_eq!(
        resolver
            .resolve(&document, &configured, None, &[], false)
            .unwrap_err()
            .code,
        SOURCE_UNSUPPORTED_VERSION_CODE
    );
    let mut redirect = Reply {
        status: 302,
        ..Reply::ok(Vec::new())
    };
    redirect.extra = format!("Location: {}/not-followed/snapshot.json\r\n", server.base);
    server.route("/static/snapshot.json", redirect);
    assert_eq!(
        adapter.read_snapshot(&source).unwrap_err().code,
        SOURCE_UNAVAILABLE_CODE
    );
    assert!(
        !server
            .requests()
            .iter()
            .any(|request| request.contains("not-followed"))
    );
    let invalid = descriptor("static", &format!("{}/not-served", server.base));
    let no_cache = [ConfiguredSource {
        descriptor: invalid,
        config_order: 0,
        imported_from: None,
    }];
    let report = resolver
        .resolve(&document, &no_cache, None, &[], false)
        .unwrap();
    assert_eq!(report.snapshots[0].status, SnapshotStatus::Unavailable);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn range_header_cannot_understate_returned_body() {
    let server = Server::new();
    let source = descriptor("static", &format!("{}/static", server.base));
    let (_, _, artifact) = fixture(&source, COMMIT);
    let mut response = Reply::ok(b"not executable: create NO_SIDE_EFFECT".to_vec());
    response.cut = Some(7);
    response.ranged = true;
    response.range_end = Some(7);
    server.route("/static/artifacts/demo.bin", response);
    assert_eq!(
        HttpStaticAdapter::new()
            .read_artifact(&source, &artifact)
            .unwrap_err()
            .code,
        SOURCE_UNAVAILABLE_CODE
    );
}

#[test]
fn range_header_cannot_overstate_cleanly_returned_body() {
    let server = Server::new();
    let source = descriptor("static", &format!("{}/static", server.base));
    let (_, _, artifact) = fixture(&source, COMMIT);
    let mut response = Reply::ok(b"not executable: create NO_SIDE_EFFECT".to_vec());
    response.cut = Some(artifact.length as usize - 2);
    response.ranged = true;
    response.ranged_body_limit = Some(1);
    server.route("/static/artifacts/demo.bin", response);
    assert_eq!(
        HttpStaticAdapter::new()
            .read_artifact(&source, &artifact)
            .unwrap_err()
            .code,
        SOURCE_UNAVAILABLE_CODE
    );
}

#[test]
fn interrupted_range_response_can_resume_again() {
    let server = Server::new();
    let source = descriptor("static", &format!("{}/static", server.base));
    let (_, _, artifact) = fixture(&source, COMMIT);
    let mut response = Reply::ok(b"not executable: create NO_SIDE_EFFECT".to_vec());
    response.cut = Some(artifact.length as usize - 5);
    response.ranged = true;
    response.ranged_cut = Some(3);
    server.route("/static/artifacts/demo.bin", response);
    assert_eq!(
        HttpStaticAdapter::new()
            .read_artifact(&source, &artifact)
            .unwrap()
            .len(),
        artifact.length as usize
    );
    assert!(
        server
            .requests()
            .iter()
            .any(|request| request.contains(&format!("Some({})", artifact.length - 2)))
    );
}

#[test]
fn transport_timeout_tls_failure_and_refuse_connection_are_unavailable() {
    let server = Server::new();
    server.route(
        "/slow/snapshot.json",
        Reply {
            delay: Duration::from_millis(150),
            ..Reply::ok(b"{}".to_vec())
        },
    );
    let adapter = HttpStaticAdapter::with_timeout(Duration::from_millis(35));
    let slow = descriptor("static", &format!("{}/slow", server.base));
    assert_eq!(
        adapter.read_snapshot(&slow).unwrap_err().code,
        SOURCE_UNAVAILABLE_CODE
    );
    let tls = descriptor("static", &server.base.replacen("http:", "https:", 1));
    assert_eq!(
        adapter.read_snapshot(&tls).unwrap_err().code,
        SOURCE_UNAVAILABLE_CODE
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let closed = listener.local_addr().unwrap();
    drop(listener);
    let refused = descriptor("static", &format!("http://{closed}"));
    assert_eq!(
        adapter.read_snapshot(&refused).unwrap_err().code,
        SOURCE_UNAVAILABLE_CODE
    );
}

#[test]
#[ignore = "需要本机 DNS 解析 .invalid 保留域，CI 不依赖外部解析服务"]
fn dns_resolution_failure_is_unavailable() {
    let source = descriptor("static", "http://xiao-e3c-does-not-exist.invalid/index");
    assert_eq!(
        HttpStaticAdapter::new()
            .read_snapshot(&source)
            .unwrap_err()
            .code,
        SOURCE_UNAVAILABLE_CODE
    );
}

#[test]
fn config_requires_unique_git_ref_and_no_local_path_mix() {
    let text = "[project]\nname = \"root\"\nversion = \"1\"\n[dependencies]\nlib = { git = \"https://github.com/acme/lib.git\", rev = \"v1.2.0\", version = \"^1\" }\n";
    let document = parse_config_text(text).unwrap();
    let direct = git_dependency_declarations(&document);
    assert_eq!(direct[0].reference, "v1.2.0");
    assert_eq!(direct[0].version.as_deref(), Some("^1"));
    for suffix in [
        "",
        ", path = \"../lib\", rev = \"v1\"",
        ", rev = \"v1\", tag = \"v1\"",
        ", rev = \"v1\", branch = \"main\"",
    ] {
        let invalid = format!(
            "[project]\nname = \"root\"\nversion = \"1\"\n[dependencies]\nlib = {{ git = \"https://github.com/acme/lib.git\"{suffix} }}\n"
        );
        assert!(
            parse_config_text(&invalid)
                .unwrap_err()
                .iter()
                .any(|error| error.code() == INVALID_DEPENDENCY_GIT_CODE)
        );
    }
    assert_eq!(
        GitReference::from_config("rev", "v1.2.0").unwrap(),
        GitReference::Tag("v1.2.0".to_owned())
    );
    let sources = parse_config_text("[project]\nname = \"root\"\nversion = \"1\"\n[sources]\ngit = { kind = \"git-index\", location = \"https://github.com/acme/lib.git\", tag = \"v1\" }\n").unwrap();
    let declarations = source_declarations(&sources).unwrap();
    let SourceDeclaration::Direct(git) = &declarations[0] else {
        panic!("未解析 Git 索引")
    };
    assert_eq!(git.git_reference, Some(GitReference::Tag("v1".to_owned())));
    assert!(git.source.source_id.ends_with("@tag:v1"));
    let listed = xiao_package::SourceList::parse(r#"{"protocol_version":1,"sources":[{"kind":"git-index","location":"https://github.com/acme/lib.git","tag":"v1"}]}"#).unwrap();
    assert_eq!(listed.sources[0].source.source_id, git.source.source_id);
    let first = descriptor("git-index", "https://github.com/acme/lib.git")
        .with_git_reference(GitReference::Tag("v1".to_owned()))
        .unwrap();
    let second = descriptor("git-index", "https://github.com/acme/lib.git")
        .with_git_reference(GitReference::Branch("main".to_owned()))
        .unwrap();
    assert_ne!(first.source.source_id, second.source.source_id);
    assert!(
        first
            .with_git_reference(GitReference::Branch("other".to_owned()))
            .is_err()
    );
    assert!(GitReference::from_config("rev", "../escape").is_err());
    for url in [
        "https:///missing",
        "https://github.com/../secret",
        "http://github.com/a/b",
    ] {
        let malformed = text.replace("https://github.com/acme/lib.git", url);
        assert!(
            parse_config_text(&malformed)
                .unwrap_err()
                .iter()
                .any(|error| error.code() == INVALID_DEPENDENCY_GIT_CODE)
        );
    }
    let root = temporary();
    fs::write(root.join("config.xiao"), text).unwrap();
    let result = xiao_package::resolve_project(&root);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.diagnostic.code()
                == xiao_package::PACKAGE_INVALID_METADATA_CODE)
    );
    fs::remove_dir_all(root).unwrap();
    assert_eq!(
        SourceDescriptor::new("static", "http://localhost/root/../escape", None, None, 1)
            .unwrap_err()
            .code,
        SOURCE_INVALID_CODE
    );
}

struct ResetRemoteToken;

impl Drop for ResetRemoteToken {
    fn drop(&mut self) {
        unsafe { std::env::remove_var("XIAO_SOURCE_TOKEN_E3D1_PRIVATE") };
    }
}

fn make_writable(path: &std::path::Path) {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.is_dir() {
            for entry in fs::read_dir(path).unwrap().flatten() {
                make_writable(&entry.path());
            }
        }
        let mut permissions = metadata.permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions).unwrap();
    }
}

#[test]
fn authenticated_http_source_installs_without_leaking_token_or_running_code() {
    let server = Server::new();
    let source = SourceDescriptor::new(
        "static",
        &format!("{}/private", server.base),
        Some("e3d1_private"),
        None,
        1,
    )
    .unwrap();
    let mut archive = tar::Builder::new(Vec::new());
    for (path, content) in [
        (
            "config.xiao",
            "[project]\nname = \"demo\"\nversion = \"1.0.0\"\n",
        ),
        ("install.xiao", "write NO_SIDE_EFFECT if executed\n"),
    ] {
        let mut header = tar::Header::new_ustar();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        archive
            .append_data(&mut header, path, content.as_bytes())
            .unwrap();
    }
    let body = archive.into_inner().unwrap();
    let artifact = ArtifactReference {
        location: "artifacts/demo.tar".into(),
        length: body.len() as u64,
        digest: format!("{:x}", Sha256::digest(&body)),
    };
    let shard = serde_json::to_string(&PackageShard {
        protocol_version: 1,
        source_id: source.source.source_id.clone(),
        snapshot_id: "snapshot-one".into(),
        packages: vec![IndexPackage {
            name: "demo".into(),
            version: "1.0.0".into(),
            variant: "any".into(),
            withdrawn: false,
            dependencies: Vec::new(),
            features: Vec::new(),
            target: None,
            abi: None,
            xiao_range: None,
            runtime_range: None,
            source_artifact: artifact.clone(),
            binary_artifacts: Vec::new(),
        }],
    })
    .unwrap();
    let manifest = SnapshotManifest {
        protocol_version: 1,
        source_id: source.source.source_id.clone(),
        snapshot_id: "snapshot-one".into(),
        shards: BTreeMap::from([("demo".into(), jcs_digest(&shard).unwrap())]),
        mirrors: Vec::new(),
        expires_at: None,
        signature: None,
    };
    server.route(
        "/private/snapshot.json",
        Reply::ok(serde_json::to_vec(&manifest).unwrap()),
    );
    server.route("/private/index/demo.json", Reply::ok(shard));
    server.route("/private/artifacts/demo.tar", Reply::ok(body.clone()));
    let list_text = serde_json::json!({
        "protocol_version": 1,
        "sources": [{"kind": "static", "location": source.location, "alias": "e3d1_private"}],
    })
    .to_string();
    let list_digest = xiao_package::SourceList::parse(&list_text).unwrap().digest;
    server.route("/sources.json", Reply::ok(list_text));
    let root = temporary();
    let text = format!(
        "[project]\nname = \"app\"\nversion = \"0.1.0\"\n[sources]\nprivate = {{ list = \"{}/sources.json\", digest = \"{list_digest}\" }}\n[dependencies]\ndemo = {{ version = \"1.*\", source = \"e3d1_private\" }}\n",
        server.base,
    );
    fs::write(root.join("config.xiao"), &text).unwrap();
    let document = parse_config_project(&SourceFile::from_text(&text)).unwrap();
    let layout = CacheLayout::from_xiao_home(Some(&root.join("home")), &root).unwrap();
    let token = "e3d1-secret-should-never-escape";
    unsafe { std::env::set_var("XIAO_SOURCE_TOKEN_E3D1_PRIVATE", token) };
    let _reset = ResetRemoteToken;
    let run = || {
        apply_packages(
            &root,
            None,
            &document,
            &Toolchain::new("clang"),
            &TargetDescription::host(),
            PackageOperation::Sync {
                keep_extra: false,
                locked: false,
                frozen: false,
            },
            layout.clone(),
        )
    };
    let result = run().unwrap();
    let requests = server.requests();
    assert!(
        requests.len() >= 4
            && requests
                .iter()
                .filter(|request| request.starts_with("/private/"))
                .all(|request| request.ends_with("auth=true"))
    );
    let locked = read_lockfile(lockfile_path(&root)).unwrap();
    let demo = locked
        .packages
        .values()
        .find(|package| package.name == "demo")
        .unwrap();
    assert_eq!(
        demo.source_artifact.as_ref().unwrap().digest,
        artifact.digest
    );
    assert!(
        !fs::read_to_string(lockfile_path(&root))
            .unwrap()
            .contains(token)
    );
    assert!(!format!("{result:?} {locked:?} {source:?}").contains(token));
    assert!(!root.join("NO_SIDE_EFFECT").exists());
    let object = CacheStore::open(layout.clone())
        .unwrap()
        .layout()
        .source_object_path(&demo.content_digest)
        .unwrap();
    make_writable(&object);
    fs::remove_dir_all(object).unwrap();
    let mut invalid_body = body;
    invalid_body[0] ^= 1;
    server.route("/private/artifacts/demo.tar", Reply::ok(invalid_body));
    let error = run().unwrap_err();
    assert_eq!(error.code, TRUST_ARTIFACT_CODE);
    assert!(!format!("{error:?} {error}").contains(token));
    assert_eq!(
        server
            .requests()
            .iter()
            .filter(|request| request.starts_with("/sources.json"))
            .count(),
        1
    );
    make_writable(&root);
    fs::remove_dir_all(root).unwrap();
}
