//! Native provider manifest validator and strict-link enforcement.
//!
//! This crate owns the machine-readable native provider manifest used by the
//! transitional UQM production binary. It validates:
//!
//! - Schema and identity of the manifest
//! - Repo-relative path canonicality (no basename matching, no traversal)
//! - Exact object inventory decisions (include/exclude per object)
//! - Provider ownership uniqueness (no duplicate/missing/unassigned symbols)
//! - Stale, unknown, and excluded object detection
//! - Manifest drift between manifest and on-disk object tree
//! - Dynamic or unresolved internal symbols (strict-link enforcement)
//!
//! It generates deterministic machine-readable provider reports.
//!
//! # Architecture
//!
//! The manifest is a checked-in JSON file (`native-provider-manifest.json`)
//! authorized by immutable ownership ledger v6. Object identities are derived
//! from tracked canonical sources and exact produced-object names rather than
//! an ignored assessment-era object tree. The validator checks each source
//! digest, archive decision, provider, actual archive member, and strict-link
//! symbol contract before production linking.
//!
//! All checks fail the build before linking and return deterministic typed
//! diagnostics.

pub mod error;
pub mod manifest;
pub mod native;
pub mod path;
pub mod report;
pub mod toolchain;
pub mod validate;

pub use error::{Diagnostic, DiagnosticCode, OwnershipError};
pub use manifest::{
    AcceptedProductionProfile, ArchiveDecision, ExternalImport, LedgerRef, Manifest,
    ManifestObject, NoTrackedNativeChange, ObjectProvider, ProviderKind, ProviderRef,
    RecompiledObject, SymbolContract,
};
pub use native::{
    load_native_dependencies, load_native_inputs, parse_dependency_file, target_key,
    validate_linked_test_profile, validate_native_authority, validate_observed_dependencies,
    NativeDependency, NativeDependencyManifest, NativeInput, NativeInputManifest,
    PreprocessorDefine, ProductionProfile, LINKED_TEST_PROFILE_ID, NATIVE_COMPILE_COMMAND,
    NATIVE_DEPENDENCY_SCHEMA, NATIVE_INPUT_SCHEMA, PRODUCTION_PROFILE_ID, SUPPORTED_TARGETS,
};
pub use path::{canonical_absolute, validate_repo_relative_path};
pub use report::{
    ArtifactDigest, ProductionArtifactReport, ProviderReport, ReportEntry, ReportSummary,
    SymbolReportEntry,
};
pub use toolchain::{
    apply_toolchain_environment, canonical_build_environment, discover_package_identities,
    effective_deployment_data, production_packages, read_build_evidence,
    reject_ambient_build_flags, reject_noncanonical_build_flags, resolve_toolchain,
    write_build_evidence, NativeBuildEvidence, NativeCompileProfile, PackageIdentity, ToolIdentity,
    ToolchainIdentity, BUILD_EVIDENCE_FILE, BUILD_EVIDENCE_SCHEMA, DEPENDENCY_FLAGS,
    REPOSITORY_INCLUDE_ROOTS,
};
pub use validate::{
    ObservedSymbol, ProductionArtifacts, ProductionNm, ProductionToolPaths, SymbolState,
    ValidateOptions, Validator,
};

/// The canonical path to the checked-in manifest relative to the `rust/`
/// crate root.
pub const MANIFEST_RELATIVE_PATH: &str = "ownership/native-provider-manifest.json";

/// The schema version string this validator expects.
pub const EXPECTED_SCHEMA: &str = "uqm-native-provider-manifest-v2";
pub const EXPECTED_LEDGER_SCHEMA: &str = "uqm-native-ownership-ledger-v6";
pub const EXPECTED_ASSESSMENT_COMMIT: &str = "54e1dba5f56e9f20a3aa773d5f151470a8cf0662";
pub const EXPECTED_LEDGER_RAW_REVISION: &str = "8f03fa7844feac162a3759ed768f3f38f75fbf7e";
pub const EXPECTED_LEDGER_RAW_URL: &str = "https://gist.githubusercontent.com/acoliver/03378acffcc0d62e7cfd094fc77c223c/raw/8f03fa7844feac162a3759ed768f3f38f75fbf7e/uqm-native-ownership-ledger.json";
pub const EXPECTED_LEDGER_GIST_REVISION: &str = "d7602e17c4401ed322f60ddfe6bf5e61d4754e24";
pub const EXPECTED_LEDGER_SHA256: &str =
    "ff4acff2118d169021edc7e9cf32c26662d304324e1aac35cbb4d8ec67fbe496";
pub const EXPECTED_LEDGER_PROJECTION_SHA256: &str =
    "bad95726d1ee70794a7d5e9d9ee46af736bd86d469ac13c790727371f4442d71";
pub const EXPECTED_SCAN_ROOT: &str = "native";
pub const EXPECTED_OBJECT_COUNT: usize = 338;
pub const DISPLIST_OBJECT: &str = "native/displist.c.o";
pub const REMOVED_HEAP_OBJECT: &str = "native/heap.c.o";
pub const QUEUE_RUST_PROVIDER: &str = "rust/src/collections/queue.rs";
pub const HASH_TABLE_RUST_PROVIDER: &str = "rust/src/collections/hash_table.rs";
pub const CHAR_HASH_TABLE_OBJECT: &str = "native/charhashtable.c.o";
pub const STRING_HASH_TABLE_OBJECT: &str = "native/stringhashtable.c.o";
pub const CHAR_HASH_TABLE_SOURCE: &str = "sc2/src/libs/uio/charhashtable.c";
pub const CHAR_HASH_TABLE_HEADER: &str = "sc2/src/libs/uio/charhashtable.h";
pub const STRING_HASH_TABLE_SOURCE: &str = "sc2/src/libs/strings/stringhashtable.c";
pub const STRING_HASH_TABLE_HEADER: &str = "sc2/src/libs/strings/stringhashtable.h";
pub const CHAR_HASH_TABLE_OWNER: &str = "RESOURCE/#22";
pub const STRING_HASH_TABLE_OWNER: &str = "CORE_NATIVE/#22";
pub const REMOVED_PRODUCTION_PROVIDERS: [&str; 4] = [
    DISPLIST_OBJECT,
    REMOVED_HEAP_OBJECT,
    CHAR_HASH_TABLE_OBJECT,
    STRING_HASH_TABLE_OBJECT,
];
pub const RETAINED_CANONICAL_SOURCES: [&str; 7] = [
    "sc2/src/uqm/displist.c",
    "sc2/src/uqm/displist.h",
    "sc2/src/libs/heap/heap.h",
    CHAR_HASH_TABLE_SOURCE,
    CHAR_HASH_TABLE_HEADER,
    STRING_HASH_TABLE_SOURCE,
    STRING_HASH_TABLE_HEADER,
];
pub const RETAINED_CANONICAL_OWNERS: [&str; 3] = [
    "COLLECTIONS/#37",
    CHAR_HASH_TABLE_OWNER,
    STRING_HASH_TABLE_OWNER,
];
pub const DYNAMIC_LOOKUP_FLAG: &str = "-undefined,dynamic_lookup";
pub const QUEUE_SYMBOLS: [&str; 10] = [
    "AllocLink",
    "CountLinks",
    "ForAllLinks",
    "FreeLink",
    "InitQueue",
    "InsertQueue",
    "PutQueue",
    "ReinitQueue",
    "RemoveQueue",
    "UninitQueue",
];
pub const HASH_TABLE_SYMBOLS: [&str; 24] = [
    "CharHashTable_add",
    "CharHashTable_count",
    "CharHashTable_deleteHashTable",
    "CharHashTable_find",
    "CharHashTable_freeIterator",
    "CharHashTable_getIterator",
    "CharHashTable_iteratorDone",
    "CharHashTable_iteratorKey",
    "CharHashTable_iteratorNext",
    "CharHashTable_iteratorValue",
    "CharHashTable_newHashTable",
    "CharHashTable_remove",
    "StringHashTable_add",
    "StringHashTable_count",
    "StringHashTable_deleteHashTable",
    "StringHashTable_find",
    "StringHashTable_freeIterator",
    "StringHashTable_getIterator",
    "StringHashTable_iteratorDone",
    "StringHashTable_iteratorKey",
    "StringHashTable_iteratorNext",
    "StringHashTable_iteratorValue",
    "StringHashTable_newHashTable",
    "StringHashTable_remove",
];
