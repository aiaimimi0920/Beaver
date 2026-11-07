#include "../distance_api.h"
#include <cassert>
#include <cfenv>
#include <cmath>
#include <cstdint>
#include <iostream>
#include <limits>
#include <vector>
#if defined(__SSE__)
#include <xmmintrin.h>
#endif

int main() {
    const uint64_t budget = 512ULL * 1024 * 1024;
    double vertices[] = {0, 0, 0, 1, 0, 0, 0, 1, 0};
    int64_t triangle[] = {0, 1, 2};
    double point[] = {.25, .25, .001};
    double distance = 0, hit[3] = {};
    int64_t id = -1;
    uint64_t tested = 0;
    auto query = [&](uint64_t handle) {
        return bmd_query(handle, point, 1, &distance, &id, hit, &tested, 0);
    };
    assert(bmd_abi_version() == 1 && bmd_live_bytes() == 0);
    assert(!bmd_build(nullptr, 3, triangle, 1, budget));
    assert(!bmd_build(vertices, INT64_MAX, triangle, 1, budget));
    assert(!bmd_build(vertices, 3, triangle, INT64_MAX, budget));
    assert(!bmd_build(vertices, 3, triangle, 2000000, 0));
    assert(!bmd_build(vertices, 3, triangle, 1, budget + 1));
    double unsupported[] = {1e40, 1e40, 0, 0, 0, 0, 1, 0, 0};
    assert(!bmd_build(unsupported, 3, triangle, 1, budget));
    double remote[] = {0x1p59, 0x1p59, 0, 0, 0, 0, 0x1p59, 0, 0};
    assert(!bmd_build(remote, 3, triangle, 1, budget)); // V5 explicitly unsupported.
    double remote_edge[] = {1, 1, 0, 0x1p50, 1, 0, 1, 0x1p50, 0};
    assert(!bmd_build(remote_edge, 3, triangle, 1, budget));
    double remote_interior[] = {-0x1p40, -0x1p41, 0, 3 * 0x1p40, -0x1p40, 0,
                                -0x1p41, 0x1p42, 0};
    assert(!bmd_build(remote_interior, 3, triangle, 1, budget));
    double bounded_interior[] = {-4, -8, 0, 12, -4, 0, -8, 16, 0};
    auto stable = bmd_build(bounded_interior, 3, triangle, 1, budget);
    point[0] = .5;
    assert(stable && query(stable) == 0 && distance == .001);
    assert(hit[0] == .5 && hit[1] == .25 && hit[2] == 0);
    assert(bmd_free(stable) == 0);
    point[0] = .25;
    auto handle = bmd_build(vertices, 3, triangle, 1, budget);
    assert(handle && bmd_handle_count() == 1);
    assert(query(handle) == 0 && distance == .001 && id == 0 && tested == 1);
    point[0] = std::nextafter(16., 17.);
    assert(query(handle) == -1);
    point[0] = .25;
    assert(bmd_query(handle, nullptr, 0, nullptr, nullptr, nullptr, nullptr, 0) == 0);
    assert(bmd_query(handle, nullptr, 1, &distance, &id, hit, &tested, 0) == -1);
    assert(bmd_query(handle, point, 1, nullptr, &id, hit, &tested, 0) == -1);
    assert(bmd_query(handle, point, 1, &distance, nullptr, hit, &tested, 0) == -1);
    assert(bmd_query(handle, point, 1, &distance, &id, nullptr, &tested, 0) == -1);
    assert(bmd_query(handle, point, 1, &distance, &id, hit, nullptr, 0) == -1);
    assert(bmd_query(handle, point, INT64_MAX, &distance, &id, hit, &tested, 0) == -1);
    assert(bmd_query(handle, point, -1, &distance, &id, hit, &tested, 0) == -1);
    assert(bmd_query(handle, point, 1, &distance, &id, hit, &tested, 2) == -1);
    std::fesetround(FE_DOWNWARD);
    assert(query(handle) == -1);
    assert(!bmd_build(vertices, 3, triangle, 1, budget));
    std::fesetround(FE_TONEAREST);
#if defined(__SSE__)
    unsigned int mode = _mm_getcsr();
    for (unsigned int rounding : {_MM_ROUND_DOWN, _MM_ROUND_UP, _MM_ROUND_TOWARD_ZERO}) {
        _mm_setcsr((mode & ~_MM_ROUND_MASK) | rounding);
        assert(std::fegetround() == FE_TONEAREST); // x87 alone cannot see this.
        assert(query(handle) == -1);
        assert(!bmd_build(vertices, 3, triangle, 1, budget));
        _mm_setcsr(mode);
    }
    _mm_setcsr(mode | (1 << 15) | (1 << 6)); // Deliberate caller FTZ + DAZ.
    assert(query(handle) == -1);
    _mm_setcsr(mode);
#endif
    assert(query(handle) == 0);
    assert(bmd_free(handle) == 0 && bmd_live_bytes() == 0);
    assert(query(handle) == -1 && bmd_free(handle) == -1 && bmd_free(0) == -1);
    std::vector<uint64_t> handles;
    for (int i = 0; i < 128; ++i) {
        auto next = bmd_build(vertices, 3, triangle, 1, budget);
        assert(next && next != handle);
        handles.push_back(next);
    }
    assert(!bmd_build(vertices, 3, triangle, 1, budget));
    for (auto next : handles) assert(bmd_free(next) == 0);
    assert(bmd_handle_count() == 0 && bmd_live_bytes() == 0);
    triangle[0] = -1;
    assert(!bmd_build(vertices, 3, triangle, 1, budget));
    triangle[0] = 3;
    assert(!bmd_build(vertices, 3, triangle, 1, budget));
    triangle[0] = 0;
    vertices[0] = std::numeric_limits<double>::quiet_NaN();
    assert(!bmd_build(vertices, 3, triangle, 1, budget));
    std::cout << "C ABI input, resource, fenv and lifetime negatives passed\n";
}
