#include "distance_api.h"
#include "bvh_index.hpp"
#include "numeric_domain.hpp"
#if !defined(__linux__) || !defined(__x86_64__) || !defined(__SSE2__)
#error Only x86_64 SSE2 Linux floating-point control is currently supported
#endif
#include <xmmintrin.h>
#include <cfenv>
#include <cfloat>
#include <cstdio>
#include <map>
#include <mutex>

#ifndef BEAVER_DISTANCE_SOURCE_SHA
#error Build with build_backend.py to bind the compiled source identity
#endif
static_assert(sizeof(double) == 8 && std::numeric_limits<double>::is_iec559);
static_assert(sizeof(int64_t) == 8 && sizeof(void *) == 8);
static_assert(LDBL_MAX_EXP >= 2 * DBL_MAX_EXP);
static_assert(__FLT128_MANT_DIG__ == 113 && __FLT128_MAX_EXP__ == 16384);

namespace {
using exact_bvh::Index;
using exact_bvh::Node;
using exact_bvh::Triangle;
using exact_bvh::Vec;
constexpr uint64_t MAX_BYTES = 512ULL * 1024 * 1024;
constexpr int64_t MAX_GEOMETRY = 2000000;
constexpr int64_t MAX_QUERIES = 1000000;
struct Owned {
    std::unique_ptr<Index> index;
    uint64_t bytes;
    exact_bvh::CoordinateDomain domain;
};
std::map<uint64_t, Owned> registry;
std::mutex registry_mutex;
uint64_t live_bytes = 0;
uint64_t next_handle = 1;
thread_local char last_error[256] = {};

void require(bool condition, const char *message) {
    if (!condition) throw std::runtime_error(message);
}
void validate_environment() {
    require(std::fegetround() == FE_TONEAREST, "Round-to-nearest environment required");
    unsigned int mxcsr = _mm_getcsr();
    require((mxcsr & _MM_ROUND_MASK) == _MM_ROUND_NEAREST,
            "SSE round-to-nearest environment required");
    require((mxcsr & ((1U << 15) | (1U << 6))) == 0,
            "SSE FTZ/DAZ must be disabled");
    // Bounds depend on IEEE subnormals; reject a caller's FTZ/DAZ mode.
    volatile double tiny = std::numeric_limits<double>::denorm_min();
    volatile double one = 1.0;
    volatile double retained = tiny * one;
    require(retained != 0.0, "Gradual underflow required (disable FTZ/DAZ)");
}
void record(const std::exception &error) noexcept {
    std::snprintf(last_error, sizeof(last_error), "%s", error.what());
}
uint64_t allocation_bound(int64_t triangles) {
    // Counts are checked first: all products fit size_t and uint64_t.
    // Vector capacity is reserved once. Include generous registry/object cost.
    return uint64_t(triangles) * (sizeof(Triangle) + sizeof(int64_t)
                                 + 2 * sizeof(Node)) + 4096;
}
} // namespace

extern "C" {
uint32_t bmd_abi_version() { return 1; }
const char *bmd_source_sha256() { return BEAVER_DISTANCE_SOURCE_SHA; }
const char *bmd_error() { return last_error; }

uint64_t bmd_build(const double *v, int64_t nv, const int64_t *t, int64_t nt,
                   uint64_t memory_limit_bytes) {
    last_error[0] = 0;
    try {
        std::lock_guard<std::mutex> lock(registry_mutex);
        validate_environment();
        require(v && t && nv > 0 && nt > 0, "Empty or null geometry");
        require(nv <= MAX_GEOMETRY && nt <= MAX_GEOMETRY, "Geometry count limit");
        require(memory_limit_bytes <= MAX_BYTES, "Memory budget exceeds hard limit");
        uint64_t bytes = allocation_bound(nt);
        require(bytes <= memory_limit_bytes && bytes <= MAX_BYTES - live_bytes,
                "Native memory budget exceeded");
        require(registry.size() < 128, "Live handle limit");
        require(next_handle != 0, "Handle token space exhausted");
        exact_bvh::CoordinateDomain domain;
        for (int64_t i = 0; i < 3 * nv; ++i)
            domain.include(v[i], int(i % 3));
        for (int64_t i = 0; i < 3 * nt; ++i)
            require(t[i] >= 0 && t[i] < nv, "Triangle index out of range");
        auto index = std::make_unique<Index>();
        index->triangles.resize(nt);
        index->order.resize(nt);
        index->nodes.reserve(2 * nt);
        for (int64_t i = 0; i < nt; ++i) {
            auto get = [&](int k) -> Vec {
                int64_t j = 3 * t[3 * i + k];
                return {v[j], v[j + 1], v[j + 2]};
            };
            index->triangles[i] = {get(0), get(1), get(2)};
            exact_bvh::validate_triangle_domain(index->triangles[i]);
        }
        std::iota(index->order.begin(), index->order.end(), 0);
        index->build(0, nt);
        uint64_t handle = next_handle++;
        registry.emplace(handle, Owned{std::move(index), bytes, domain});
        live_bytes += bytes;
        return handle;
    } catch (const std::exception &error) {
        record(error);
        return 0;
    }
}

int bmd_free(uint64_t handle) {
    last_error[0] = 0;
    try {
        std::lock_guard<std::mutex> lock(registry_mutex);
        auto found = registry.find(handle);
        require(found != registry.end(), "Unknown or closed handle");
        live_bytes -= found->second.bytes;
        registry.erase(found);
        return 0;
    } catch (const std::exception &error) {
        record(error);
        return -1;
    }
}

int bmd_query(uint64_t handle, const double *p, int64_t count, double *d,
              int64_t *ids, double *hits, uint64_t *tested, int exhaustive) {
    last_error[0] = 0;
    try {
        std::lock_guard<std::mutex> lock(registry_mutex);
        validate_environment();
        auto found = registry.find(handle);
        require(found != registry.end(), "Unknown or closed handle");
        require(count >= 0 && count <= MAX_QUERIES, "Query count limit");
        require(exhaustive == 0 || exhaustive == 1, "Invalid exhaustive flag");
        require(!count || (p && d && ids && hits && tested), "Null query buffer");
        for (int64_t i = 0; i < 3 * count; ++i)
            found->second.domain.check_query(p[i], int(i % 3));
        for (int64_t i = 0; i < count; ++i) {
            auto result = exact_bvh::query(*found->second.index,
                                          {p[3 * i], p[3 * i + 1], p[3 * i + 2]},
                                          exhaustive != 0);
            require(std::isfinite(result.distance), "Unrepresentable distance");
            require(result.id >= 0, "No triangle witness");
            d[i] = result.distance;
            ids[i] = result.id;
            tested[i] = result.tested;
            for (int k = 0; k < 3; ++k) hits[3 * i + k] = result.hit[k];
        }
        return 0;
    } catch (const std::exception &error) {
        record(error);
        return -1;
    }
}

uint64_t bmd_live_bytes() {
    std::lock_guard<std::mutex> lock(registry_mutex);
    return live_bytes;
}
uint64_t bmd_handle_count() {
    std::lock_guard<std::mutex> lock(registry_mutex);
    return registry.size();
}
} // extern C
