//! Fixture rows E*, L*, B* of `tmp/issue38-port/dirs/fixture-spec.md`. The
//! expected values were traced from `dirs.c` before the implementation
//! existed.

use libc::{EINVAL, ENAMETOOLONG, ENOENT, ERANGE};

use super::expand::*;
use super::test_support::{bytes_or_errno, FakeEnv};

fn expand(env: &FakeEnv, src: &[u8], len: usize, what: i32) -> Result<Vec<u8>, i32> {
    bytes_or_errno(expand_path(env, src, len, what))
}

fn ok(src: &str, what: i32, want: &str) {
    let got = expand(&FakeEnv::standard(), src.as_bytes(), 64, what);
    assert_eq!(
        got,
        Ok(want.as_bytes().to_vec()),
        "src {src:?} flags {what}"
    );
}

fn err(src: &str, what: i32, want: i32) {
    let got = expand(&FakeEnv::standard(), src.as_bytes(), 64, what);
    assert_eq!(got, Err(want), "src {src:?} flags {what}");
}

#[test]
fn absolute_and_relative_paths() {
    ok("/a/b", EP_ALL_SYSTEM, "/a/b"); // E01
    ok("a/b", EP_ALL_SYSTEM, "/w/a/b"); // E02
    ok("", EP_ALL_SYSTEM, "/w/"); // E35
    ok("", 0, ""); // E36
}

#[test]
fn home_expansion() {
    ok("~/x", EP_ALL_SYSTEM, "/h/x"); // E03
    err("~", EP_ALL_SYSTEM, EINVAL); // E04
    err("~user/x", EP_ALL_SYSTEM, EINVAL); // E05
    let mut env = FakeEnv::standard();
    env.home = None;
    assert_eq!(expand(&env, b"~/x", 64, EP_ALL_SYSTEM), Err(ENOENT)); // E06
    let mut env = FakeEnv::standard();
    env.home = Some(b"rel".to_vec());
    assert_eq!(
        expand(&env, b"~/x", 64, EP_ALL_SYSTEM),
        Ok(b"/w/rel/x".to_vec())
    ); // E07
    ok("a/~/b", EP_HOME, "a/~/b"); // E15
    ok("~/x\\y", EP_HOME | EP_SLASHES, "/h/x/y"); // E31
}

#[test]
fn environment_variables() {
    ok("$V/x", EP_ALL_SYSTEM, "/v/x"); // E08
    ok("${V}/x", EP_ALL_SYSTEM, "/v/x"); // E09
    ok("a$UNSET", EP_ALL_SYSTEM, "/w/a"); // E10
    err("${V", EP_ALL_SYSTEM, EINVAL); // E11
    ok("$", EP_ENVVARS, ""); // E12
    ok("$T/x", EP_ALL_SYSTEM, "/h/x"); // E13
    ok("$W", EP_ENVVARS, "$V"); // E14
    ok("$EMPTY/q", EP_ENVVARS, "/q");
    ok("$V_x/q", EP_ENVVARS, "/q"); // the name runs to the first non-name byte
}

#[test]
fn dot_components() {
    ok("/a/./b/../c", EP_DOTS, "/a/c"); // E16
    ok("/a/b/..", EP_DOTS, "/a"); // E17
    ok("/a/..", EP_DOTS, "/"); // E18
    err("/..", EP_DOTS, EINVAL); // E19
    err("a/../..", EP_DOTS, EINVAL); // E20
    ok("a/b/../..", EP_DOTS, ""); // E21
    ok("/a/.", EP_DOTS, "/a/"); // E22
    ok("/a/b/../", EP_DOTS, "/a/"); // E23
}

#[test]
fn separators() {
    ok("/a//b///c", EP_SINGLESEP, "/a/b/c"); // E24
    ok("/a//b///c", EP_DOTS, "/a//b///c"); // E25
    ok("//a", EP_SINGLESEP, "/a"); // E26
    ok("a\\b\\..\\c", EP_SLASHES, "a/b/../c"); // E27
    ok("a\\b\\..\\c", EP_SLASHES | EP_DOTS, "a/c"); // E28
    ok("\\a", EP_ABSOLUTE | EP_SLASHES, "/a"); // E29
    ok("\\a", EP_ABSOLUTE, "/w/\\a"); // E30
}

#[test]
fn working_directory_edges() {
    let mut env = FakeEnv::standard();
    env.cwd = Ok(b"/".to_vec());
    assert_eq!(expand(&env, b"a", 64, EP_ABSOLUTE), Ok(b"/a".to_vec())); // E32
}

#[test]
fn no_flags_leave_the_text_alone() {
    ok("a/b", 0, "a/b"); // E33
    ok("~/x", 0, "~/x");
    ok("$V", 0, "$V");
    ok("a/../b", 0, "a/../b");
    ok("a", 64 | 128, "a"); // E34
}

#[test]
fn capacity_is_the_caller_buffer_size_including_the_nul() {
    let env = FakeEnv::standard();
    assert_eq!(expand(&env, b"/abc", 5, 0), Ok(b"/abc".to_vec())); // L01
    assert_eq!(expand(&env, b"/abc", 4, 0), Err(ENAMETOOLONG)); // L02
    assert_eq!(expand(&env, b"", 0, 0), Err(ENAMETOOLONG)); // L03
    assert_eq!(expand(&env, b"$V", 2, EP_ENVVARS), Err(ENAMETOOLONG)); // L04
    assert_eq!(expand(&env, b"$V", 3, EP_ENVVARS), Ok(b"/v".to_vec()));
    assert_eq!(expand(&env, b"a", 3, EP_ABSOLUTE), Err(ENAMETOOLONG)); // L05
    assert_eq!(expand(&env, b"a", 4, EP_ABSOLUTE), Err(ENAMETOOLONG));
    assert_eq!(expand(&env, b"a", 5, EP_ABSOLUTE), Ok(b"/w/a".to_vec()));
    assert_eq!(expand(&env, b"a", 2, EP_ABSOLUTE), Err(ERANGE)); // L06
    assert_eq!(expand(&env, b"a", 0, EP_ABSOLUTE), Err(EINVAL)); // L07
}

#[test]
fn working_directory_failures_keep_their_errno() {
    let mut env = FakeEnv::standard();
    env.cwd = Err(std::io::Error::from_raw_os_error(libc::EACCES));
    assert_eq!(expand(&env, b"a", 64, EP_ABSOLUTE), Err(libc::EACCES));
    assert_eq!(expand(&env, b"/a", 64, EP_ABSOLUTE), Ok(b"/a".to_vec()));
}

#[test]
fn bytes_are_never_validated_or_replaced() {
    let env = FakeEnv::standard();
    assert_eq!(
        expand(&env, b"/a/\xff\xfe", 64, EP_ALL_SYSTEM),
        Ok(b"/a/\xff\xfe".to_vec())
    ); // B01
    let mut env = FakeEnv::standard();
    env.vars.insert(b"B".to_vec(), b"/\xff".to_vec());
    assert_eq!(
        expand(&env, b"$B/\xc3\xa9", 64, EP_ALL_SYSTEM),
        Ok(b"/\xff/\xc3\xa9".to_vec())
    ); // B02
    env.cwd = Ok(b"/w\xfe".to_vec());
    assert_eq!(
        expand(&env, b"a/\xff", 64, EP_ALL_SYSTEM),
        Ok(b"/w\xfe/a/\xff".to_vec())
    ); // B03
}
