#ifndef MEM_WRAPPER_H
#define MEM_WRAPPER_H

#ifdef __cplusplus
extern "C" {
#endif

// Entry point that Rust can call
// Returns the exit code from the game
int c_entry_point(int argc, char *argv[]);

#ifdef __cplusplus
}
#endif

#endif /* MEM_WRAPPER_H */
