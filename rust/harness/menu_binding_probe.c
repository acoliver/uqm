/*
 * Menu Binding Probe — Initialized-Child Production Query
 *
 * This probe performs the minimal real production initialization needed to
 * query the actual `menu.down.N` binding through production resources,
 * then calls the narrow `uqm_query_menu_binding` accessor (which uses
 * production `res_IsString`/`res_GetString` and `VControl_ParseGesture`),
 * emits the resolved VCONTROL_KEY binding and alternate id, and exits.
 *
 * It owns/reaps no child processes — it IS the initialized child.
 *
 * The probe links against:
 *   - libuqm_rust.a (Rust resource system, UIO, VControl parser)
 *   - libuqm_c.a    (C archive: rust_vcontrol_impl.o for VControl_ParseGesture wrapper,
 *                    C subsystem type registration)
 *   - libp00_harness_shim.a (menu_binding_accessor.o)
 *
 * @plan PLAN-20260723-RUNTIME-AUTOMATION.P00
 */

#include "menu_binding_accessor.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* --- Production Rust FFI declarations (from libuqm_rust.a) --- */

/* Resource system — Rust #[no_mangle] exports */
extern void *InitResourceSystem(void);
extern void LoadResourceIndex(void *dir, const char *filename, const char *prefix);

/* UIO — Rust #[no_mangle] exports */
typedef void uio_Repository;
typedef void uio_DirHandle;
typedef void uio_MountHandle;
typedef void uio_AutoMount;

extern uio_Repository *uio_openRepository(int flags);
extern void uio_closeRepository(uio_Repository *repository);
extern uio_MountHandle *uio_mountDir(uio_Repository *destRep,
        const char *mountPoint, int fsType,
        uio_DirHandle *sourceDir, const char *sourcePath,
        const char *inPath, uio_AutoMount **autoMount, int flags,
        uio_MountHandle *relative);
extern uio_DirHandle *uio_openDir(uio_Repository *repository, const char *path,
        int flags);
extern void uio_closeDir(uio_DirHandle *dirHandle);

/* --- UIO constants (must match libs/uio/fstypes.h and mount.h) --- */
#define PROBE_FSTYPE_STDIO 1
#define PROBE_MOUNT_RDONLY (1 << 1)
#define PROBE_MOUNT_TOP    (1 << 2)

/* VCONTROL_KEY = 1 (matches VCONTROL_GESTURE_TYPE enum) */
#define PROBE_VCONTROL_KEY 1

int
main(int argc, char **argv)
{
    const char *content_path;

    if (argc >= 2)
    {
        content_path = argv[1];
    }
    else
    {
        /* Default: relative to repository root */
        content_path = "../../sc2/content";
    }

    fprintf(stderr, "menu_binding_probe: content_path=%s\n", content_path);

    /* 1. Initialize the production resource system (Rust InitResourceSystem).
     *    This calls C subsystem type registration (InstallGraphicResTypes,
     *    InstallStringTableResType, etc.) which are in libuqm_c.a. */
    void *res_idx = InitResourceSystem();
    if (res_idx == NULL)
    {
        fprintf(stderr, "FAIL: InitResourceSystem returned NULL\n");
        printf("RESULT=FAIL reason=init_resource_system\n");
        return 1;
    }
    fprintf(stderr, "menu_binding_probe: resource system initialized\n");

    /* 2. Create a UIO repository and mount the content directory. */
    uio_Repository *repo = uio_openRepository(0);
    if (repo == NULL)
    {
        fprintf(stderr, "FAIL: uio_openRepository returned NULL\n");
        printf("RESULT=FAIL reason=open_repository\n");
        return 1;
    }

    uio_MountHandle *mount = uio_mountDir(repo, "/",
            PROBE_FSTYPE_STDIO, NULL, NULL, content_path, NULL,
            PROBE_MOUNT_TOP | PROBE_MOUNT_RDONLY, NULL);
    if (mount == NULL)
    {
        fprintf(stderr, "FAIL: uio_mountDir returned NULL for %s\n", content_path);
        printf("RESULT=FAIL reason=mount_content_dir\n");
        uio_closeRepository(repo);
        return 1;
    }
    fprintf(stderr, "menu_binding_probe: content dir mounted\n");

    /* 3. Open the root directory handle. */
    uio_DirHandle *content_dir = uio_openDir(repo, "/", 0);
    if (content_dir == NULL)
    {
        fprintf(stderr, "FAIL: uio_openDir returned NULL\n");
        printf("RESULT=FAIL reason=open_content_dir\n");
        uio_closeRepository(repo);
        return 1;
    }
    fprintf(stderr, "menu_binding_probe: content dir opened\n");

    /* 4. Load the menu.key resource index with "menu." prefix.
     *    This is the same call as register_menu_controls/initKeyConfig
     *    in input.c, using production LoadResourceIndex. */
    LoadResourceIndex(content_dir, "menu.key", "menu.");
    fprintf(stderr, "menu_binding_probe: menu.key loaded\n");

    /* 5. Query the actual menu.down binding through the narrow accessor.
     *    uqm_query_menu_binding calls production res_IsString/res_GetString
     *    and VControl_ParseGesture to resolve the binding. */
    MenuBindingResult result = uqm_query_menu_binding("down");

    /* 6. Validate and emit the result. */
    printf("menu_binding_query=down\n");
    printf("found=%d\n", result.found);
    printf("key_code=%d\n", result.key_code);
    printf("binding_id=%d\n", result.binding_id);
    printf("num_alternates=%d\n", result.num_alternates);

    if (!result.found)
    {
        printf("RESULT=FAIL reason=no_key_binding_found\n");
        uio_closeDir(content_dir);
        uio_closeRepository(repo);
        return 1;
    }

    /* Verify it's a VCONTROL_KEY (the accessor already filters for this,
     * but we double-check the key code is a valid SDL keycode). */
    if (result.key_code <= 0)
    {
        printf("RESULT=FAIL reason=invalid_key_code\n");
        uio_closeDir(content_dir);
        uio_closeRepository(repo);
        return 1;
    }

    printf("RESULT=PASS\n");
    printf("binding_type=VCONTROL_KEY\n");

    /* 7. Teardown. */
    uio_closeDir(content_dir);
    uio_closeRepository(repo);

    return 0;
}
