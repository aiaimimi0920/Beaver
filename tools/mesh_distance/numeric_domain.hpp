#pragma once
#include "triangle_kernel.hpp"
#include <climits>
#include <cstdint>
#include <cstring>
#include <quadmath.h>
#include <stdexcept>

namespace exact_bvh {
// This is an explicit supported-input domain, not a universal accuracy proof.
// Use actual represented significant-bit positions (including trailing zeros),
// not a worst-case 53-bit mantissa for float32-derived coordinates.
// H-L+2 <=113 covers the significant span and a possible subtraction carry.
struct CoordinateDomain {
    // Original metre-valued coordinates only; never normalize to evade this
    // optional backend capability limit. It is not a model acceptance gate.
    static void check_absolute_cap(double value) {
        if (std::abs(value) > 16)
            throw std::runtime_error("Unsupported numerical domain: absolute coordinate cap is 16 metres");
    }
    std::array<int, 3> low{INT_MAX, INT_MAX, INT_MAX};
    std::array<int, 3> high{INT_MIN, INT_MIN, INT_MIN};

    static std::array<int, 2> significant_bits(double value) {
        uint64_t bits;
        std::memcpy(&bits, &value, sizeof(bits));
        uint64_t mantissa = bits & ((uint64_t(1) << 52) - 1);
        unsigned exponent = unsigned((bits >> 52) & 0x7ff);
        int base = -1074;
        if (exponent) {
            mantissa |= uint64_t(1) << 52;
            base = int(exponent) - 1023 - 52;
        }
        // Called only for finite nonzero values, including subnormals.
        return {base + __builtin_ctzll(mantissa),
                base + 63 - __builtin_clzll(mantissa)};
    }

    void include(double value, int axis) {
        if (!std::isfinite(value)) throw std::runtime_error("Nonfinite vertex");
        check_absolute_cap(value);
        if (value == 0) return;
        auto bits = significant_bits(value);
        low[axis] = std::min(low[axis], bits[0]);
        high[axis] = std::max(high[axis], bits[1]);
        if (high[axis] - low[axis] + 2 > 113)
            throw std::runtime_error("Coordinate significant-bit span exceeds supported numerical domain");
    }

    void check_query(double value, int axis) const {
        if (!std::isfinite(value)) throw std::runtime_error("Nonfinite query");
        check_absolute_cap(value);
        if (value == 0 || low[axis] == INT_MAX) return;
        auto bits = significant_bits(value);
        if (std::max(high[axis], bits[1]) - std::min(low[axis], bits[0]) + 2 > 113)
            throw std::runtime_error("Query significant-bit span exceeds supported numerical domain");
    }
};

inline bool exact_zero_cross(const Wide &a, const Wide &b) {
    // For admitted coordinates the differences a,b are exact. A correctly
    // rounded binary128 product plus its fused residual is an exact expansion;
    // the exponent range of binary64 inputs prevents binary128 under/overflow.
    // Equal products have equal rounded high terms and equal residuals.
    for (int k = 0; k < 3; ++k) {
        int i = (k + 1) % 3, j = (k + 2) % 3;
        Scalar left = a[i] * b[j], right = a[j] * b[i];
        if (left != right || fmaq(a[i], b[j], -left) != fmaq(a[j], b[i], -right))
            return false;
    }
    return true;
}

inline void validate_triangle_domain(const Triangle &triangle) {
    Wide ab = minus(wide(triangle.b), wide(triangle.a));
    Wide ac = minus(wide(triangle.c), wide(triangle.a));
    Wide bc = minus(wide(triangle.c), wide(triangle.b));
    Wide normal = cross(ab, ac);
    Scalar area_squared = dot(normal, normal);
    if (area_squared == 0) {
        if (!exact_zero_cross(ab, ac))
            throw std::runtime_error("Unresolved nonzero triangle area in numerical domain");
        return; // Certified represented collinearity/repeated points, not a tolerance.
    }
    Scalar edge_squared = std::max({dot(ab, ab), dot(ac, ac), dot(bc, bc)});
    constexpr Scalar scale = Scalar(uint64_t(1) << 60);
    constexpr Scalar minimum_squared_ratio = Scalar(1) / (scale * scale);
    if (area_squared < minimum_squared_ratio * edge_squared * edge_squared)
        throw std::runtime_error("Triangle conditioning exceeds supported numerical domain");
}
} // namespace exact_bvh
