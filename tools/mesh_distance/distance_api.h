#ifndef BEAVER_MESH_DISTANCE_API_H
#define BEAVER_MESH_DISTANCE_API_H
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ABI 1: native-endian binary64 xyz and int64 triangle indices, contiguous.
 * Buffers must have their declared extent; an arbitrary C pointer's allocation
 * cannot be inspected. Inputs are copied at build and never retained at query.
 * Handles are process-local non-recycled tokens. Zero denotes build failure.
 * Numeric domain v5: original metre coordinates, every axis within [-16,16],
 * plus bit-span/conditioning checks at build/query (see README); unsupported
 * coordinate ranges/conditioning fail, never become implicit line targets.
 * Calls are serialized; no computational worker threads are created.
 * Every status is 0 on success, -1 on error. Outputs on error are unspecified.
 * The thread-local error string is valid until this thread's next API call.
 */
uint32_t bmd_abi_version(void);
const char *bmd_source_sha256(void);
const char *bmd_error(void);
uint64_t bmd_build(const double *vertices, int64_t vertex_count,
                   const int64_t *triangles, int64_t triangle_count,
                   uint64_t memory_limit_bytes);
int bmd_free(uint64_t handle);
int bmd_query(uint64_t handle, const double *points, int64_t count,
              double *distance, int64_t *triangle_id, double *hits,
              uint64_t *tested_triangles, int exhaustive);
uint64_t bmd_live_bytes(void);
uint64_t bmd_handle_count(void);

#ifdef __cplusplus
}
#endif
#endif
