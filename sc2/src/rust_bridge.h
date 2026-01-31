#ifndef RUST_BRIDGE_H
#define RUST_BRIDGE_H

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Initialize the Rust bridge logging system.
 * Creates/truncates rust-bridge.log and writes initial marker.
 * Returns 0 on success, non-zero on error.
 */
int rust_bridge_init(void);

/**
 * Log a message to the Rust bridge log file.
 * Returns 0 on success, non-zero on error.
 */
int rust_bridge_log(const char *message);

#ifdef __cplusplus
}
#endif

#endif /* RUST_BRIDGE_H */
